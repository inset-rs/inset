//! Flutter counterpart: `widgets/form.dart`.
//!
//! A [`Form`] groups the [`FormField`]s below it: each field's state registers with the form's
//! [`FormState`] while it builds, and the form saves, resets and validates them together.
//!
//! [`FormField`] and [`FormFieldState`] are Dart classes that Material subclasses
//! (`TextFormField`, `DropdownButtonFormField`, `DropdownMenuFormField`). A subclass is a leaf:
//! its widget carries a [`FormFieldData`] and implements [`FormFieldLeaf`], its state carries a
//! [`FormFieldStateData`] and implements [`FormFieldStateLeaf`], and the bodies Dart inherits
//! are associated functions on [`FormFieldState`] that a leaf calls where Dart writes `super`.
//! A form holds its fields erased, as [`DynamicFormFieldState`]s, Dart's
//! `FormFieldState<dynamic>`, and a field's builder is handed its state as
//! [`AnyFormFieldState`], Dart's `FormFieldState<T>`; both dispatch the virtuals to the leaf.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;

use indexmap::IndexSet;
use waft_embedder::{SemanticsRole, SemanticsValidationResult};
use waft_foundation::{App, Handle, HandleId, OptionRetained, VoidCallback};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_services::RestorationBucket;

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::view::View;
use crate::widgets::basic::Semantics;
use crate::widgets::focus_manager::FocusNode;
use crate::widgets::focus_manager::FocusNodeLeaf;
use crate::widgets::focus_scope::Focus;
use crate::widgets::navigator::RouteResult;
use crate::widgets::pop_scope::PopScope;
use crate::widgets::restoration::{RestorableProperty, RestorationMixin, RestorationMixinData};
use crate::widgets::restoration_properties::{RestorableBool, RestorableStringN, RestorableValue};
use crate::widgets::routes::PopInvokedWithResultCallback;
use waft_foundation::Ref;

// ---------------------------------------------------------------------------------------------
// Form

/// An optional container for grouping together multiple form field widgets (e.g. `TextField`
/// widgets).
///
/// Each individual form field should be wrapped in a [`FormField`] widget, with the [`Form`]
/// widget as a common ancestor of all of those. Call methods on [`FormState`] to save, reset, or
/// validate each [`FormField`] that is a descendant of this [`Form`]. To obtain the
/// [`FormState`], you may use [`Form::of`] with a context whose ancestor is the [`Form`], or pass
/// a `GlobalKey` to the [`Form`] constructor and call `GlobalKey::current_state`.
///
/// See also:
///
///  * `GlobalKey`, a key that is unique across the entire app.
///  * [`FormField`], a single form field widget that maintains the current state.
///  * `TextFormField`, a convenience widget that wraps a `TextField` widget in a [`FormField`].
pub struct Form {
    /// See [`StatefulWidget::key`].
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    ///
    /// This is the root of the widget hierarchy that contains this form.
    pub child: WidgetRef,

    /// When false, blocks the current route from being popped.
    ///
    /// See also:
    ///
    ///  * [`on_pop_invoked_with_result`](Self::on_pop_invoked_with_result), which also comes
    ///    from [`PopScope`] and is often used in conjunction with this parameter.
    ///  * [`PopScope::can_pop`], which is what [`Form`] delegates to internally.
    pub can_pop: Option<bool>,

    /// Called after a route pop was handled; see [`PopScope::on_pop_invoked_with_result`].
    ///
    /// See also:
    ///
    ///  * [`can_pop`](Self::can_pop), which also comes from [`PopScope`] and is often used in
    ///    conjunction with this parameter.
    ///  * [`PopScope::on_pop_invoked_with_result`], which is what [`Form`] delegates to
    ///    internally.
    pub on_pop_invoked_with_result: Option<PopInvokedWithResultCallback>,

    /// Called when one of the form fields changes.
    ///
    /// In addition to this callback being invoked, all the form fields themselves will rebuild.
    pub on_changed: Option<VoidCallback>,

    /// Used to enable/disable form fields auto validation and update their error text.
    ///
    /// If [`AutovalidateMode::OnUserInteraction`], this form will only auto-validate after its
    /// content changes. If [`AutovalidateMode::Always`], it will auto-validate even without user
    /// interaction. If [`AutovalidateMode::Disabled`], auto-validation will be disabled.
    ///
    /// Defaults to [`AutovalidateMode::Disabled`].
    pub autovalidate_mode: AutovalidateMode,
}

impl Form {
    /// Creates a container for form fields.
    pub fn new<K>(child: impl IntoWidget<K>) -> Form {
        Form {
            key: None,
            child: child.into_widget(),
            can_pop: None,
            on_pop_invoked_with_result: None,
            on_changed: None,
            autovalidate_mode: AutovalidateMode::Disabled,
        }
    }

    /// Dart `Form(key:)`.
    pub fn key(mut self, key: KeyRef) -> Form {
        self.key = Some(key);
        self
    }

    /// Dart `Form(canPop:)`.
    pub fn can_pop(mut self, can_pop: bool) -> Form {
        self.can_pop = Some(can_pop);
        self
    }

    /// Dart `Form(onPopInvokedWithResult:)`.
    pub fn on_pop_invoked_with_result(
        mut self,
        on_pop_invoked_with_result: PopInvokedWithResultCallback,
    ) -> Form {
        self.on_pop_invoked_with_result = Some(on_pop_invoked_with_result);
        self
    }

    /// Dart `Form(onChanged:)`.
    pub fn on_changed(mut self, on_changed: impl Fn(&mut App) + 'static) -> Form {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `Form(autovalidateMode:)`.
    pub fn autovalidate_mode(mut self, autovalidate_mode: AutovalidateMode) -> Form {
        self.autovalidate_mode = autovalidate_mode;
        self
    }

    /// Returns the [`FormState`] of the closest [`Form`] widget which encloses the given
    /// context, or `None` if none is found.
    ///
    /// Calling this method will create a dependency on the closest [`Form`] in the `context`,
    /// if there is one.
    ///
    /// See also:
    ///
    /// * [`Form::of`], which is similar to this method, but panics if no [`Form`] ancestor is
    ///   found.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<FormState>> {
        context
            .depend_on_inherited_widget_of_exact_type::<FormScope>(app)
            .map(|scope| scope.form_state)
    }

    /// Returns the [`FormState`] of the closest [`Form`] widget which encloses the given
    /// context.
    ///
    /// If no [`Form`] ancestor is found, this panics.
    ///
    /// Calling this method will create a dependency on the closest [`Form`] in the `context`.
    ///
    /// See also:
    ///
    /// * [`Form::maybe_of`], which is similar to this method, but returns `None` if no [`Form`]
    ///   ancestor is found.
    pub fn of(app: &mut App, context: BuildContext) -> Handle<FormState> {
        let form_state = Form::maybe_of(app, context);
        form_state.unwrap_or_else(|| {
            panic!(
                "Form.of() was called with a context that does not contain a Form widget.\n\
                 No Form widget ancestor could be found starting from the context that was \
                 passed to Form.of(). This can happen because you are using a widget that looks \
                 for a Form ancestor, but no such ancestor exists.\n\
                 The context used was:\n  {}",
                context.widget(app).type_name()
            )
        })
    }

    /// Dart's `_callPopInvoked` tear-off, which the form hands its [`PopScope`].
    fn call_pop_invoked(&self) -> PopInvokedWithResultCallback {
        let on_pop_invoked_with_result = self.on_pop_invoked_with_result.clone();
        Rc::new(move |app: &mut App, did_pop: bool, result: RouteResult| {
            if let Some(on_pop_invoked_with_result) = &on_pop_invoked_with_result {
                on_pop_invoked_with_result(app, did_pop, result);
            }
        })
    }
}

impl Debug for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Form")
            .field("can_pop", &self.can_pop)
            .field("autovalidate_mode", &self.autovalidate_mode)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Form {
    type State = FormState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> FormState {
        FormState {
            state: StateData::new(),
            generation: 0,
            has_interacted_by_user: false,
            fields: IndexSet::new(),
        }
    }
}

/// State associated with a [`Form`] widget.
///
/// A [`FormState`] object can be used to [`save`](Self::save), [`reset`](Self::reset), and
/// [`validate`](Self::validate) every [`FormField`] that is a descendant of the associated
/// [`Form`].
///
/// Typically obtained via [`Form::of`].
pub struct FormState {
    state: StateData<Form>,
    generation: i64,
    has_interacted_by_user: bool,
    fields: IndexSet<DynamicFormFieldState>,
}

impl FormState {
    /// The [`FormFieldState`] objects that are currently registered with this [`Form`].
    pub fn fields(self: Handle<Self>, app: &App) -> Vec<DynamicFormFieldState> {
        app.get(self).fields.iter().copied().collect()
    }

    // Called when a form field has changed. This will cause all form fields
    // to rebuild, useful if form fields have interdependencies.
    fn field_did_change(self: Handle<Self>, app: &mut App) {
        if let Some(on_changed) = self.widget(app).on_changed.clone() {
            on_changed(app);
        }

        let fields = self.fields(app);
        let has_interacted_by_user = fields
            .into_iter()
            .any(|field| field.has_interacted_by_user(app));
        app.get_mut(self).has_interacted_by_user = has_interacted_by_user;

        self.force_rebuild(app);
    }

    fn force_rebuild(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.generation += 1;
        });
    }

    fn register(self: Handle<Self>, app: &mut App, field: DynamicFormFieldState) {
        app.get_mut(self).fields.insert(field);
    }

    fn unregister(self: Handle<Self>, app: &mut App, field: DynamicFormFieldState) {
        app.get_mut(self).fields.shift_remove(&field);
    }

    /// Saves every [`FormField`] that is a descendant of this [`Form`].
    pub fn save(self: Handle<Self>, app: &mut App) {
        for field in self.fields(app) {
            field.save(app);
        }
    }

    /// Resets every [`FormField`] that is a descendant of this [`Form`] back to its
    /// [`FormFieldData::initial_value`].
    ///
    /// The [`Form::on_changed`] callback will be called.
    ///
    /// If the form's [`Form::autovalidate_mode`] property is [`AutovalidateMode::Always`], the
    /// fields will all be revalidated after being reset.
    pub fn reset(self: Handle<Self>, app: &mut App) {
        for field in self.fields(app) {
            field.reset(app);
        }
        app.get_mut(self).has_interacted_by_user = false;
        self.field_did_change(app);
    }

    /// Clears the validation errors for all [`FormField`]s in this [`Form`] without resetting
    /// their values.
    ///
    /// See also:
    ///
    ///  * [`FormFieldStateLeaf::clear_error`], which clears the error for a single form field.
    pub fn clear_error(self: Handle<Self>, app: &mut App) {
        for field in self.fields(app) {
            field.clear_error_internal(app);
        }
        self.field_did_change(app);
    }

    /// Validates every [`FormField`] that is a descendant of this [`Form`], and returns true if
    /// there are no errors.
    ///
    /// The form will rebuild to report the results.
    ///
    /// See also:
    ///  * [`validate_granularly`](Self::validate_granularly), which also validates descendant
    ///    [`FormField`]s, but instead returns a set of fields with errors.
    pub fn validate(self: Handle<Self>, app: &mut App) -> bool {
        app.get_mut(self).has_interacted_by_user = true;
        self.force_rebuild(app);
        self.validate_fields(app, None)
    }

    /// Validates every [`FormField`] that is a descendant of this [`Form`], and returns a set
    /// of [`DynamicFormFieldState`] of the invalid field(s) only, if any.
    ///
    /// This method can be useful to highlight field(s) with errors.
    ///
    /// The form will rebuild to report the results.
    ///
    /// See also:
    ///  * [`validate`](Self::validate), which also validates descendant [`FormField`]s, and
    ///    return true if there are no errors.
    pub fn validate_granularly(
        self: Handle<Self>,
        app: &mut App,
    ) -> IndexSet<DynamicFormFieldState> {
        let mut invalid_fields = IndexSet::new();
        app.get_mut(self).has_interacted_by_user = true;
        self.force_rebuild(app);
        self.validate_fields(app, Some(&mut invalid_fields));
        invalid_fields
    }

    /// Dart's `_validate`.
    fn validate_fields(
        self: Handle<Self>,
        app: &mut App,
        mut invalid_fields: Option<&mut IndexSet<DynamicFormFieldState>>,
    ) -> bool {
        let mut has_error = false;
        let validate_on_focus_change =
            self.widget(app).autovalidate_mode == AutovalidateMode::OnUnfocus;

        for field in self.fields(app) {
            let has_focus = field.focus_node(app).as_node().has_focus(app);

            if !validate_on_focus_change || !has_focus || (validate_on_focus_change && has_focus) {
                let is_field_valid = field.validate(app);
                has_error |= !is_field_valid;
                if let Some(invalid_fields) = invalid_fields.as_deref_mut()
                    && !is_field_valid
                {
                    invalid_fields.insert(field);
                }
            }
        }

        !has_error
    }
}

impl State for FormState {
    type Widget = Form;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let has_error = self
            .fields(app)
            .into_iter()
            .any(|field| field.has_error(app));

        match self.widget(app).autovalidate_mode {
            AutovalidateMode::Always => {
                self.validate_fields(app, None);
            }
            AutovalidateMode::OnUserInteraction => {
                if app.get(self).has_interacted_by_user {
                    self.validate_fields(app, None);
                }
            }
            AutovalidateMode::OnUserInteractionIfError => {
                if app.get(self).has_interacted_by_user && has_error {
                    self.validate_fields(app, None);
                }
            }
            AutovalidateMode::OnUnfocus | AutovalidateMode::Disabled => {}
        }

        let generation = app.get(self).generation;
        let widget = self.widget(app);
        let scope = FormScope {
            form_state: self,
            generation,
            child: widget.child.clone(),
        };
        let form = if widget.can_pop.is_some() || widget.on_pop_invoked_with_result.is_some() {
            PopScope::new(scope)
                .can_pop(widget.can_pop.unwrap_or(true))
                .on_pop_invoked_with_result(widget.call_pop_invoked())
                .into_widget()
        } else {
            scope.into_widget()
        };
        Semantics::new()
            .container(true)
            .explicit_child_nodes(true)
            .role(SemanticsRole::Form)
            .child(form)
            .into_widget()
    }
}

/// Dart's `_FormScope`: hands a [`Form`]'s state to the fields below it, and rebuilds them
/// each time a field changes.
#[derive(Debug)]
struct FormScope {
    form_state: Handle<FormState>,

    /// Incremented every time a form field has changed. This lets us know when to rebuild the
    /// form.
    generation: i64,

    child: WidgetRef,
}

impl InheritedWidget for FormScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &FormScope) -> bool {
        self.generation != old.generation
    }
}

// ---------------------------------------------------------------------------------------------
// FormField

/// Signature for validating a form field.
///
/// Returns an error string to display if the input is invalid, or `None` otherwise.
///
/// Used by [`FormFieldData::validator`].
pub type FormFieldValidator<T> = Rc<dyn Fn(&mut App, Option<&T>) -> Option<String>>;

/// Signature for a callback that builds an error widget.
///
/// See also:
///
///  * [`FormFieldData::error_builder`], which is of this type, and passes the result error
///    given by `TextFormField.validator`.
pub type FormFieldErrorBuilder = Rc<dyn Fn(&mut App, BuildContext, String) -> WidgetRef>;

/// Signature for being notified when a form field changes value.
///
/// Used by [`FormFieldData::on_saved`].
pub type FormFieldSetter<T> = Rc<dyn Fn(&mut App, Option<T>)>;

/// Signature for building the widget representing the form field.
///
/// Used by [`FormFieldData::builder`].
pub type FormFieldBuilder<T> = Rc<dyn Fn(&mut App, AnyFormFieldState<T>) -> WidgetRef>;

/// The fields of Dart's `FormField<T>`, which [`FormField`] and its subclasses carry under the
/// field `form_field`.
pub struct FormFieldData<T: 'static> {
    /// See [`StatefulWidget::key`].
    pub key: Option<KeyRef>,

    /// Function that returns the widget representing this form field.
    ///
    /// It is passed the form field state as input, containing the current value and validation
    /// state of this field.
    pub builder: FormFieldBuilder<T>,

    /// An optional method to call with the final value when the form is saved via
    /// [`FormState::save`].
    pub on_saved: Option<FormFieldSetter<T>>,

    /// An optional method to call when the form field is reset via
    /// [`FormFieldStateLeaf::reset`].
    pub on_reset: Option<VoidCallback>,

    /// An optional property that forces the [`FormFieldState`] into an error state by directly
    /// setting the [`FormFieldStateLeaf::error_text`] property without running the validator
    /// function.
    ///
    /// When the [`force_error_text`](Self::force_error_text) property is provided, the
    /// [`FormFieldStateLeaf::error_text`] will be set to the provided value, causing the form
    /// field to be considered invalid and to display the error message specified.
    ///
    /// When [`validator`](Self::validator) is provided,
    /// [`force_error_text`](Self::force_error_text) will override any error that it returns.
    /// [`validator`](Self::validator) will not be called unless
    /// [`force_error_text`](Self::force_error_text) is `None`.
    pub force_error_text: Option<String>,

    /// An optional method that validates an input. Returns an error string to display if the
    /// input is invalid, or `None` otherwise.
    ///
    /// The returned value is exposed by the [`FormFieldStateLeaf::error_text`] property. The
    /// `TextFormField` uses this to override the `InputDecoration.errorText` value.
    ///
    /// Alternating between error and normal state can cause the height of the `TextFormField`
    /// to change if no other subtext decoration is set on the field. To create a field whose
    /// height is fixed regardless of whether or not an error is displayed, either wrap the
    /// `TextFormField` in a fixed height parent like `SizedBox`, or set the
    /// `InputDecoration.helperText` parameter to a space.
    pub validator: Option<FormFieldValidator<T>>,

    /// Function that returns the widget representing the error to display.
    ///
    /// It is passed the form field validator error string as input. The resulting widget is
    /// passed to `InputDecoration.error`.
    ///
    /// If `None`, the validator error string is passed to `InputDecoration.errorText`.
    pub error_builder: Option<FormFieldErrorBuilder>,

    /// An optional value to initialize the form field to, or `None` otherwise.
    ///
    /// The `initial_value` affects the form field's state in two cases:
    /// 1. When the form field is first built, `initial_value` determines the field's initial
    ///    state.
    /// 2. When [`FormFieldStateLeaf::reset`] is called (either directly or by calling
    ///    [`FormState::reset`]), the form field is reset to this `initial_value`.
    pub initial_value: Option<T>,

    /// Whether the form is able to receive user input.
    ///
    /// Defaults to true. If [`autovalidate_mode`](Self::autovalidate_mode) is not
    /// [`AutovalidateMode::Disabled`], the field will be auto validated. Likewise, if this field
    /// is false, the widget will not be validated regardless of
    /// [`autovalidate_mode`](Self::autovalidate_mode).
    pub enabled: bool,

    /// Used to enable/disable this form field auto validation and update its error text.
    ///
    /// If [`AutovalidateMode::OnUserInteraction`], this FormField will only auto-validate after
    /// its content changes. If [`AutovalidateMode::Always`], it will auto-validate even without
    /// user interaction. If [`AutovalidateMode::Disabled`], auto-validation will be disabled.
    ///
    /// Defaults to [`AutovalidateMode::Disabled`].
    pub autovalidate_mode: AutovalidateMode,

    /// Restoration ID to save and restore the state of the form field.
    ///
    /// Setting the restoration ID to a non-`None` value results in whether or not the form field
    /// validation persists.
    ///
    /// The state of this widget is persisted in a [`RestorationBucket`] claimed from the
    /// surrounding `RestorationScope` using the provided restoration ID.
    ///
    /// See also:
    ///
    ///  * `RestorationManager`, which explains how state restoration works in Flutter.
    pub restoration_id: Option<String>,
}

impl<T: 'static> FormFieldData<T> {
    /// The fields of a form field built by `builder`, with Dart's defaults.
    pub fn new(builder: FormFieldBuilder<T>) -> FormFieldData<T> {
        FormFieldData {
            key: None,
            builder,
            on_saved: None,
            on_reset: None,
            force_error_text: None,
            validator: None,
            error_builder: None,
            initial_value: None,
            enabled: true,
            autovalidate_mode: AutovalidateMode::Disabled,
            restoration_id: None,
        }
    }
}

impl<T: 'static> Debug for FormFieldData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FormField")
            .field("force_error_text", &self.force_error_text)
            .field("enabled", &self.enabled)
            .field("autovalidate_mode", &self.autovalidate_mode)
            .field("restoration_id", &self.restoration_id)
            .finish_non_exhaustive()
    }
}

/// The [`FormFieldData`] accessor a [`FormFieldLeaf`] implementor writes; the bag lives under
/// the field `form_field`.
#[macro_export]
macro_rules! form_field_accessors {
    () => {
        fn form_field_data(&self) -> &$crate::FormFieldData<Self::Value> {
            &self.form_field
        }
    };
}

/// The trait every form-field widget implements: [`FormField`] and its subclasses, such as
/// `TextFormField`.
///
/// Its [`State`] is a [`FormFieldStateLeaf`].
pub trait FormFieldLeaf: StatefulWidget {
    /// Dart's `T` of `FormField<T>`.
    type Value: Clone + 'static;

    /// The [`FormFieldData`] bag ([`form_field_accessors!`](crate::form_field_accessors)).
    fn form_field_data(&self) -> &FormFieldData<Self::Value>;
}

/// A single form field.
///
/// This widget maintains the current state of the form field, so that updates and validation
/// errors are visually reflected in the UI.
///
/// When used inside a [`Form`], you can use methods on [`FormState`] to query or manipulate the
/// form data as a whole. For example, calling [`FormState::save`] will invoke each
/// [`FormField`]'s [`on_saved`](FormFieldData::on_saved) callback in turn.
///
/// Use a `GlobalKey` with [`FormField`] if you want to retrieve its current state, for example
/// if you want one form field to depend on another.
///
/// A [`Form`] ancestor is not required. The [`Form`] allows one to save, reset, or validate
/// multiple fields at once. To use without a [`Form`], pass a `GlobalKey` to the constructor
/// and use `GlobalKey::current_state` to save or reset the form field.
///
/// See also:
///
///  * [`Form`], which is the widget that aggregates the form fields.
///  * `TextField`, which is a commonly used form field for entering text.
pub struct FormField<T: 'static> {
    form_field: FormFieldData<T>,
}

impl<T: Clone + 'static> FormField<T> {
    /// Creates a single form field.
    pub fn new(
        builder: impl Fn(&mut App, AnyFormFieldState<T>) -> WidgetRef + 'static,
    ) -> FormField<T> {
        FormField {
            form_field: FormFieldData::new(Rc::new(builder)),
        }
    }

    /// Dart `FormField(key:)`.
    pub fn key(mut self, key: KeyRef) -> FormField<T> {
        self.form_field.key = Some(key);
        self
    }

    /// Dart `FormField(onSaved:)`.
    pub fn on_saved(mut self, on_saved: impl Fn(&mut App, Option<T>) + 'static) -> FormField<T> {
        self.form_field.on_saved = Some(Rc::new(on_saved));
        self
    }

    /// Dart `FormField(onReset:)`.
    pub fn on_reset(mut self, on_reset: impl Fn(&mut App) + 'static) -> FormField<T> {
        self.form_field.on_reset = Some(Rc::new(on_reset));
        self
    }

    /// Dart `FormField(forceErrorText:)`.
    pub fn force_error_text(mut self, force_error_text: impl Into<String>) -> FormField<T> {
        self.form_field.force_error_text = Some(force_error_text.into());
        self
    }

    /// Dart `FormField(validator:)`.
    pub fn validator(
        mut self,
        validator: impl Fn(&mut App, Option<&T>) -> Option<String> + 'static,
    ) -> FormField<T> {
        self.form_field.validator = Some(Rc::new(validator));
        self
    }

    /// Dart `FormField(errorBuilder:)`.
    pub fn error_builder(
        mut self,
        error_builder: impl Fn(&mut App, BuildContext, String) -> WidgetRef + 'static,
    ) -> FormField<T> {
        self.form_field.error_builder = Some(Rc::new(error_builder));
        self
    }

    /// Dart `FormField(initialValue:)`.
    pub fn initial_value(mut self, initial_value: T) -> FormField<T> {
        self.form_field.initial_value = Some(initial_value);
        self
    }

    /// Dart `FormField(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> FormField<T> {
        self.form_field.enabled = enabled;
        self
    }

    /// Dart `FormField(autovalidateMode:)`.
    pub fn autovalidate_mode(mut self, autovalidate_mode: AutovalidateMode) -> FormField<T> {
        self.form_field.autovalidate_mode = autovalidate_mode;
        self
    }

    /// Dart `FormField(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> FormField<T> {
        self.form_field.restoration_id = Some(restoration_id.into());
        self
    }
}

impl<T: 'static> Debug for FormField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.form_field.fmt(f)
    }
}

impl<T: Clone + 'static> FormFieldLeaf for FormField<T> {
    type Value = T;
    crate::form_field_accessors!();
}

impl<T: Clone + 'static> StatefulWidget for FormField<T> {
    type State = FormFieldState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.form_field.key.as_ref()
    }

    fn create_state(&self) -> FormFieldState<T> {
        FormFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            form_field_state: FormFieldStateData::new(&self.form_field),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// FormFieldState

/// The value type of a form-field state: its widget's `T`.
pub type FormFieldValue<S> = <<S as State>::Widget as FormFieldLeaf>::Value;

/// The fields of Dart's `FormFieldState<T>`, which [`FormFieldState`] and its subclasses carry
/// under the field `form_field_state`.
///
/// The restorable properties and the focus node need the [`App`], so
/// [`FormFieldState::init_state`] creates them.
pub struct FormFieldStateData<T> {
    value: Option<T>,
    error_text: Option<Ref<RestorableStringN>>,
    has_interacted_by_user: Option<Ref<RestorableBool>>,
    focus_node: Option<Ref<FocusNode>>,
}

impl<T: Clone + 'static> FormFieldStateData<T> {
    /// The state of a field created for `widget`: Dart's `late T? _value = widget.initialValue`,
    /// read when the state is created.
    pub fn new(widget: &FormFieldData<T>) -> FormFieldStateData<T> {
        FormFieldStateData {
            value: widget.initial_value.clone(),
            error_text: None,
            has_interacted_by_user: None,
            focus_node: None,
        }
    }

    fn error_text_property(&self) -> Handle<RestorableStringN> {
        self.error_text.handle().expect("created in init_state")
    }

    fn has_interacted_by_user_property(&self) -> Handle<RestorableBool> {
        self.has_interacted_by_user
            .handle()
            .expect("created in init_state")
    }

    fn focus_node(&self) -> Handle<FocusNode> {
        self.focus_node.handle().expect("created in init_state")
    }
}

/// The [`FormFieldStateData`] accessors a [`FormFieldStateLeaf`] implementor writes; the bag
/// lives under the field `form_field_state`.
#[macro_export]
macro_rules! form_field_state_accessors {
    () => {
        fn form_field_state_data(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> &$crate::FormFieldStateData<$crate::FormFieldValue<Self>> {
            &app.get(self).form_field_state
        }

        fn form_field_state_data_mut(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> &mut $crate::FormFieldStateData<$crate::FormFieldValue<Self>> {
            &mut app.get_mut(self).form_field_state
        }
    };
}

/// The trait every form-field state implements: [`FormFieldState`] and its subclasses, such as
/// `TextFormField`'s state.
///
/// The virtuals a subclass overrides are the members with a default body here; the shared
/// bodies, and the lifecycle bodies a leaf's own [`State`] and [`RestorationMixin`] impls call,
/// are associated functions on [`FormFieldState`], which an override calls where Dart writes
/// `super.…`.
pub trait FormFieldStateLeaf: RestorationMixin<Widget: FormFieldLeaf> {
    /// Dart's `FormFieldState` fields, held under the field `form_field_state`
    /// ([`form_field_state_accessors!`](crate::form_field_state_accessors)).
    fn form_field_state_data(
        self: Handle<Self>,
        app: &App,
    ) -> &FormFieldStateData<FormFieldValue<Self>>;

    /// See [`form_field_state_data`](Self::form_field_state_data).
    fn form_field_state_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut FormFieldStateData<FormFieldValue<Self>>;

    /// The current value of the form field.
    fn value(self: Handle<Self>, app: &App) -> Option<&FormFieldValue<Self>> {
        self.form_field_state_data(app).value.as_ref()
    }

    /// The current validation error returned by the [`FormFieldData::validator`] callback, or
    /// the manually provided error message using the [`FormFieldData::force_error_text`]
    /// property.
    ///
    /// This property is automatically updated when [`validate`](Self::validate) is called and
    /// the [`FormFieldData::validator`] callback is invoked, or If
    /// [`FormFieldData::force_error_text`] is set directly to a non-`None` value.
    fn error_text(self: Handle<Self>, app: &App) -> Option<String> {
        let property = self.form_field_state_data(app).error_text_property();
        RestorableValue::value(property, app).clone()
    }

    /// True if this field has any validation errors.
    fn has_error(self: Handle<Self>, app: &App) -> bool {
        let property = self.form_field_state_data(app).error_text_property();
        RestorableValue::value(property, app).is_some()
    }

    /// Returns true if the user has modified the value of this field.
    ///
    /// This only updates to true once [`did_change`](Self::did_change) has been called and
    /// resets to false when [`reset`](Self::reset) is called.
    fn has_interacted_by_user(self: Handle<Self>, app: &App) -> bool {
        let property = self
            .form_field_state_data(app)
            .has_interacted_by_user_property();
        *RestorableValue::value(property, app)
    }

    /// True if the current value is valid.
    ///
    /// This will not set [`error_text`](Self::error_text) or [`has_error`](Self::has_error) and
    /// it will not update error display.
    ///
    /// See also:
    ///
    ///  * [`validate`](Self::validate), which may update [`error_text`](Self::error_text) and
    ///    [`has_error`](Self::has_error).
    ///
    ///  * [`FormFieldData::force_error_text`], which also may update
    ///    [`error_text`](Self::error_text) and [`has_error`](Self::has_error).
    fn is_valid(self: Handle<Self>, app: &mut App) -> bool {
        let widget = self.widget(app).form_field_data();
        if widget.force_error_text.is_some() {
            return false;
        }
        let Some(validator) = widget.validator.clone() else {
            return true;
        };
        let value = self.value(app).cloned();
        validator(app, value.as_ref()).is_none()
    }

    /// Calls the [`FormField`]'s on_saved method with the current value.
    fn save(self: Handle<Self>, app: &mut App) {
        FormFieldState::save(self, app);
    }

    /// Resets the field to its initial value.
    fn reset(self: Handle<Self>, app: &mut App) {
        FormFieldState::reset(self, app);
    }

    /// Clears any visible validation error for this field without resetting the field's value.
    ///
    /// This sets [`error_text`](Self::error_text) to `None` and
    /// [`has_interacted_by_user`](Self::has_interacted_by_user) to false.
    ///
    /// If [`AutovalidateMode::Always`] is used, the error may reappear immediately because the
    /// field will trigger a new validation cycle during the next build.
    /// See also:
    ///
    ///  * [`FormState::clear_error`], which clears errors across all fields in the form.
    fn clear_error(self: Handle<Self>, app: &mut App) {
        FormFieldState::clear_error(self, app);
    }

    /// Calls [`FormFieldData::validator`] to set the [`error_text`](Self::error_text) only if
    /// [`FormFieldData::force_error_text`] is `None`. When
    /// [`FormFieldData::force_error_text`] is not `None`, [`FormFieldData::validator`] will not
    /// be called.
    ///
    /// Returns true if there were no errors.
    /// See also:
    ///
    ///  * [`is_valid`](Self::is_valid), which passively gets the validity without setting
    ///    [`error_text`](Self::error_text) or [`has_error`](Self::has_error).
    fn validate(self: Handle<Self>, app: &mut App) -> bool {
        FormFieldState::validate(self, app)
    }

    /// Updates this field's state to the new value. Useful for responding to child widget
    /// changes, e.g. `Slider`'s `Slider.onChanged` argument.
    ///
    /// Triggers the [`Form::on_changed`] callback and, if [`Form::autovalidate_mode`] is
    /// [`AutovalidateMode::Always`] or [`AutovalidateMode::OnUserInteraction`], revalidates all
    /// the fields of the form.
    fn did_change(self: Handle<Self>, app: &mut App, value: Option<FormFieldValue<Self>>) {
        FormFieldState::did_change(self, app, value);
    }

    /// Sets the value associated with this form field.
    ///
    /// This method should only be called by subclasses that need to update the form field value
    /// due to state changes identified during the widget build phase, when calling `set_state`
    /// is prohibited. In all other cases, the value should be set by a call to
    /// [`did_change`](Self::did_change), which ensures that `set_state` is called.
    fn set_value(self: Handle<Self>, app: &mut App, value: Option<FormFieldValue<Self>>) {
        self.form_field_state_data_mut(app).value = value;
    }

    /// This state as the erased [`AnyFormFieldState`]: what to pass where a Dart API takes a
    /// `FormFieldState<T>`.
    fn as_form_field_state(self: Handle<Self>) -> AnyFormFieldState<FormFieldValue<Self>> {
        AnyFormFieldState {
            id: self.id(),
            vtable: const { &FormFieldStateVTable::<FormFieldValue<Self>>::of::<Self>() },
        }
    }

    /// This state as the erased [`DynamicFormFieldState`]: what to pass where a Dart API takes
    /// a `FormFieldState<dynamic>`.
    fn as_dynamic_form_field_state(self: Handle<Self>) -> DynamicFormFieldState {
        self.as_form_field_state().as_dynamic()
    }
}

/// The current state of a [`FormField`]. Passed to the [`FormFieldBuilder`] method for use in
/// constructing the form field's widget.
///
/// Its associated functions are the bodies Dart's `FormFieldState<T>` gives its subclasses:
/// the virtuals' defaults, which an override calls where Dart writes `super.…`, and the
/// lifecycle a subclass's [`State`] and [`RestorationMixin`] impls run at Dart's
/// `super.initState()` and the rest.
pub struct FormFieldState<T: Clone + 'static> {
    state: StateData<FormField<T>>,
    restoration: RestorationMixinData,
    form_field_state: FormFieldStateData<T>,
}

impl<T: Clone + 'static> FormFieldState<T> {
    /// See [`FormFieldStateLeaf::save`].
    pub fn save<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        if let Some(on_saved) = this.widget(app).form_field_data().on_saved.clone() {
            let value = this.value(app).cloned();
            on_saved(app, value);
        }
    }

    /// See [`FormFieldStateLeaf::reset`].
    pub fn reset<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        let initial_value = this.widget(app).form_field_data().initial_value.clone();
        this.form_field_state_data_mut(app).value = initial_value;
        clear_error_internal(this, app);
        this.set_state(app, |_state| {});
        if let Some(on_reset) = this.widget(app).form_field_data().on_reset.clone() {
            on_reset(app);
        }
        let context = this.context(app);
        if let Some(form) = Form::maybe_of(app, context) {
            form.field_did_change(app);
        }
    }

    /// See [`FormFieldStateLeaf::clear_error`].
    pub fn clear_error<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        clear_error_internal(this, app);
        this.set_state(app, |_state| {});
        let context = this.context(app);
        if let Some(form) = Form::maybe_of(app, context) {
            form.field_did_change(app);
        }
    }

    /// See [`FormFieldStateLeaf::validate`].
    pub fn validate<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) -> bool {
        validate_field(this, app);
        this.set_state(app, |_state| {});
        !this.has_error(app)
    }

    /// See [`FormFieldStateLeaf::did_change`].
    pub fn did_change<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
        value: Option<T>,
    ) {
        this.form_field_state_data_mut(app).value = value;
        let has_interacted_by_user = this
            .form_field_state_data(app)
            .has_interacted_by_user_property();
        has_interacted_by_user.set_value(app, true);
        this.set_state(app, |_state| {});
        let context = this.context(app);
        if let Some(form) = Form::maybe_of(app, context) {
            form.field_did_change(app);
        }
    }

    /// Dart's `FormFieldState.restorationId`.
    pub fn restoration_id<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &App,
    ) -> Option<&str> {
        this.widget(app).form_field_data().restoration_id.as_deref()
    }

    /// Dart's `FormFieldState.restoreState`.
    pub fn restore_state<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
        _old_bucket: Option<Handle<RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let error_text = this.form_field_state_data(app).error_text_property();
        this.register_for_restoration(app, error_text.as_property(), "error_text");
        let has_interacted_by_user = this
            .form_field_state_data(app)
            .has_interacted_by_user_property();
        this.register_for_restoration(
            app,
            has_interacted_by_user.as_property(),
            "has_interacted_by_user",
        );
    }

    /// Dart's `FormFieldState.deactivate`, run where the leaf's [`State::deactivate`] would call
    /// `super.deactivate()`.
    pub fn deactivate<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        let context = this.context(app);
        if let Some(form) = Form::maybe_of(app, context) {
            form.unregister(app, this.as_dynamic_form_field_state());
        }
    }

    /// Dart's `FormFieldState.initState`, with the field initializers Dart runs as the state is
    /// constructed.
    pub fn init_state<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        let has_interacted_by_user = RestorableBool::new(app, false);
        let focus_node = FocusNode::new(app);
        let force_error_text = this.widget(app).form_field_data().force_error_text.clone();
        let error_text = RestorableStringN::new(app, force_error_text);
        let data = this.form_field_state_data_mut(app);
        data.has_interacted_by_user = Some(has_interacted_by_user);
        data.focus_node = Some(focus_node);
        data.error_text = Some(error_text);
    }

    /// Dart's `FormFieldState.didUpdateWidget`.
    pub fn did_update_widget<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
        old_widget: &S::Widget,
    ) {
        this.did_update_restoration_id(app);
        let force_error_text = this.widget(app).form_field_data().force_error_text.clone();
        if force_error_text != old_widget.form_field_data().force_error_text {
            let error_text = this.form_field_state_data(app).error_text_property();
            error_text.set_value(app, force_error_text);
        }
    }

    /// Dart's `FormFieldState.didChangeDependencies`.
    pub fn did_change_dependencies<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        this.did_change_dependencies_restoration(app);
        let context = this.context(app);
        let form = Form::maybe_of(app, context);
        match form.map(|form| form.widget(app).autovalidate_mode) {
            Some(AutovalidateMode::Always) => {
                let view = View::maybe_id_of(app, context);
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _time_stamp| {
                        // If the form is already validated, don't validate again.
                        if this.widget(app).form_field_data().enabled
                            && !this.has_error(app)
                            && !this.is_valid(app)
                        {
                            this.validate(app);
                        }
                    }),
                    view,
                );
            }
            Some(
                AutovalidateMode::OnUnfocus
                | AutovalidateMode::OnUserInteraction
                | AutovalidateMode::OnUserInteractionIfError
                | AutovalidateMode::Disabled,
            )
            | None => {}
        }
    }

    /// Dart's `FormFieldState.dispose`.
    pub fn dispose<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        let data = this.form_field_state_data(app);
        let (error_text, focus_node, has_interacted_by_user) = (
            data.error_text_property(),
            data.focus_node(),
            data.has_interacted_by_user_property(),
        );
        RestorableProperty::dispose(error_text, app);
        focus_node.as_node().dispose(app);
        RestorableProperty::dispose(has_interacted_by_user, app);
        this.dispose_restoration(app);
    }

    /// Dart's `FormFieldState.build`.
    pub fn build<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
        context: BuildContext,
    ) -> WidgetRef {
        let widget = this.widget(app).form_field_data();
        let (enabled, autovalidate_mode) = (widget.enabled, widget.autovalidate_mode);
        if enabled {
            match autovalidate_mode {
                AutovalidateMode::Always => {
                    validate_field(this, app);
                }
                AutovalidateMode::OnUserInteraction => {
                    if this.has_interacted_by_user(app) {
                        validate_field(this, app);
                    }
                }
                AutovalidateMode::OnUserInteractionIfError => {
                    if this.has_interacted_by_user(app) && this.has_error(app) {
                        validate_field(this, app);
                    }
                }
                AutovalidateMode::OnUnfocus | AutovalidateMode::Disabled => {}
            }
        }

        if let Some(form) = Form::maybe_of(app, context) {
            form.register(app, this.as_dynamic_form_field_state());
        }

        let builder = Rc::clone(&this.widget(app).form_field_data().builder);
        let validation_result = if this.has_error(app) {
            SemanticsValidationResult::Invalid
        } else {
            SemanticsValidationResult::Valid
        };
        let child = Semantics::new()
            .validation_result(validation_result)
            .child(builder(app, this.as_form_field_state()))
            .into_widget();

        let form_autovalidate_mode =
            Form::maybe_of(app, context).map(|form| form.widget(app).autovalidate_mode);
        if form_autovalidate_mode == Some(AutovalidateMode::OnUnfocus)
            && autovalidate_mode != AutovalidateMode::Always
            || autovalidate_mode == AutovalidateMode::OnUnfocus
        {
            let focus_node = this
                .form_field_state_data(app)
                .focus_node
                .as_ref()
                .expect("created in init_state")
                .erase();
            return Focus::new(child)
                .can_request_focus(false)
                .skip_traversal(true)
                .on_focus_change(move |app, value| {
                    if !value {
                        validate_field(this, app);
                        this.set_state(app, |_state| {});
                    }
                })
                .focus_node(focus_node)
                .into_widget();
        }

        child
    }
}

/// Dart's private `_clearErrorInternal`.
fn clear_error_internal<S: FormFieldStateLeaf>(this: Handle<S>, app: &mut App) {
    let data = this.form_field_state_data(app);
    let (error_text, has_interacted_by_user) = (
        data.error_text_property(),
        data.has_interacted_by_user_property(),
    );
    error_text.set_value(app, None);
    has_interacted_by_user.set_value(app, false);
}

/// Dart's private `FormFieldState._validate`.
fn validate_field<S: FormFieldStateLeaf>(this: Handle<S>, app: &mut App) {
    let error_text = this.form_field_state_data(app).error_text_property();
    let widget = this.widget(app).form_field_data();
    if let Some(force_error_text) = widget.force_error_text.clone() {
        error_text.set_value(app, Some(force_error_text));
        // Skip validating if error is forced.
        return;
    }
    if let Some(validator) = widget.validator.clone() {
        let value = this.value(app).cloned();
        let result = validator(app, value.as_ref());
        error_text.set_value(app, result);
    } else {
        error_text.set_value(app, None);
    }
}

impl<T: Clone + 'static> FormFieldStateLeaf for FormFieldState<T> {
    crate::form_field_state_accessors!();
}

impl<T: Clone + 'static> RestorationMixin for FormFieldState<T> {
    crate::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        FormFieldState::restoration_id(self, app)
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<RestorationBucket>>,
        initial_restore: bool,
    ) {
        FormFieldState::restore_state(self, app, old_bucket, initial_restore);
    }
}

impl<T: Clone + 'static> State for FormFieldState<T> {
    type Widget = FormField<T>;
    crate::state_accessors!();

    fn deactivate(self: Handle<Self>, app: &mut App) {
        FormFieldState::deactivate(self, app);
    }

    fn init_state(self: Handle<Self>, app: &mut App) {
        FormFieldState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &FormField<T>) {
        FormFieldState::did_update_widget(self, app, old_widget);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        FormFieldState::did_change_dependencies(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        FormFieldState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        FormFieldState::build(self, app, context)
    }
}

// ---------------------------------------------------------------------------------------------
// The erased form-field states

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<S: 'static>(id: HandleId) -> Handle<S> {
    Handle::from_id(id)
}

/// The vtable of an erased [`DynamicFormFieldState`]: what does not name the value type.
struct DynamicFormFieldStateVTable {
    type_name: fn() -> &'static str,
    typed: fn() -> &'static dyn Any,
    value: fn(&App, HandleId) -> Option<&dyn Any>,
    context: fn(&App, HandleId) -> BuildContext,
    mounted: fn(&App, HandleId) -> bool,
    error_text: fn(&App, HandleId) -> Option<String>,
    has_error: fn(&App, HandleId) -> bool,
    has_interacted_by_user: fn(&App, HandleId) -> bool,
    is_valid: fn(&mut App, HandleId) -> bool,
    save: fn(&mut App, HandleId),
    reset: fn(&mut App, HandleId),
    clear_error: fn(&mut App, HandleId),
    validate: fn(&mut App, HandleId) -> bool,
    clear_error_internal: fn(&mut App, HandleId),
    focus_node: fn(&App, HandleId) -> Handle<FocusNode>,
}

/// The vtable of an erased [`AnyFormFieldState`]: the members that name the value type, and the
/// [`DynamicFormFieldStateVTable`] of the same leaf.
struct FormFieldStateVTable<T: 'static> {
    dynamic: DynamicFormFieldStateVTable,
    value: fn(&App, HandleId) -> Option<&T>,
    did_change: fn(&mut App, HandleId, Option<T>),
}

impl<T: Clone + 'static> FormFieldStateVTable<T> {
    const fn of<S: FormFieldStateLeaf<Widget: FormFieldLeaf<Value = T>>>() -> FormFieldStateVTable<T>
    {
        FormFieldStateVTable {
            dynamic: DynamicFormFieldStateVTable {
                type_name: std::any::type_name::<S>,
                typed: || const { &FormFieldStateVTable::<T>::of::<S>() } as &dyn Any,
                value: |app, id| S::value(resolve(id), app).map(|value| value as &dyn Any),
                context: |app, id| S::context(resolve(id), app),
                mounted: |app, id| S::mounted(resolve(id), app),
                error_text: |app, id| S::error_text(resolve(id), app),
                has_error: |app, id| S::has_error(resolve(id), app),
                has_interacted_by_user: |app, id| S::has_interacted_by_user(resolve(id), app),
                is_valid: |app, id| S::is_valid(resolve(id), app),
                save: |app, id| S::save(resolve(id), app),
                reset: |app, id| S::reset(resolve(id), app),
                clear_error: |app, id| S::clear_error(resolve(id), app),
                validate: |app, id| S::validate(resolve(id), app),
                clear_error_internal: |app, id| clear_error_internal::<S>(resolve(id), app),
                focus_node: |app, id| S::form_field_state_data(resolve(id), app).focus_node(),
            },
            value: |app, id| S::value(resolve(id), app),
            did_change: |app, id, value| S::did_change(resolve(id), app, value),
        }
    }
}

/// Erased [`FormFieldStateLeaf`] whose value type is not known: what a field or parameter Dart
/// types as `FormFieldState<dynamic>` becomes, such as the fields a [`FormState`] holds.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct DynamicFormFieldState {
    id: HandleId,
    vtable: &'static DynamicFormFieldStateVTable,
}

impl PartialEq for DynamicFormFieldState {
    fn eq(&self, other: &DynamicFormFieldState) -> bool {
        self.id == other.id
    }
}

impl Eq for DynamicFormFieldState {}

impl std::hash::Hash for DynamicFormFieldState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for DynamicFormFieldState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl DynamicFormFieldState {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// The leaf state behind this handle, if it is an `S`: Dart's `field as S`.
    pub fn downcast<S: 'static>(self, app: &App) -> Option<Handle<S>> {
        app.handle::<S>(self.id)
    }

    /// This field as a `FormFieldState<T>`, if its value type is `T`: Dart's
    /// `field is FormFieldState<T>`.
    pub fn typed<T: 'static>(self) -> Option<AnyFormFieldState<T>> {
        (self.vtable.typed)()
            .downcast_ref::<FormFieldStateVTable<T>>()
            .map(|vtable| AnyFormFieldState {
                id: self.id,
                vtable,
            })
    }

    /// See [`FormFieldStateLeaf::value`].
    pub fn value(self, app: &App) -> Option<&dyn Any> {
        (self.vtable.value)(app, self.id)
    }

    /// See [`State::context`].
    pub fn context(self, app: &App) -> BuildContext {
        (self.vtable.context)(app, self.id)
    }

    /// See [`State::mounted`].
    pub fn mounted(self, app: &App) -> bool {
        (self.vtable.mounted)(app, self.id)
    }

    /// See [`FormFieldStateLeaf::error_text`].
    pub fn error_text(self, app: &App) -> Option<String> {
        (self.vtable.error_text)(app, self.id)
    }

    /// See [`FormFieldStateLeaf::has_error`].
    pub fn has_error(self, app: &App) -> bool {
        (self.vtable.has_error)(app, self.id)
    }

    /// See [`FormFieldStateLeaf::has_interacted_by_user`].
    pub fn has_interacted_by_user(self, app: &App) -> bool {
        (self.vtable.has_interacted_by_user)(app, self.id)
    }

    /// See [`FormFieldStateLeaf::is_valid`].
    pub fn is_valid(self, app: &mut App) -> bool {
        (self.vtable.is_valid)(app, self.id)
    }

    /// See [`FormFieldStateLeaf::save`].
    pub fn save(self, app: &mut App) {
        (self.vtable.save)(app, self.id);
    }

    /// See [`FormFieldStateLeaf::reset`].
    pub fn reset(self, app: &mut App) {
        (self.vtable.reset)(app, self.id);
    }

    /// See [`FormFieldStateLeaf::clear_error`].
    pub fn clear_error(self, app: &mut App) {
        (self.vtable.clear_error)(app, self.id);
    }

    /// See [`FormFieldStateLeaf::validate`].
    pub fn validate(self, app: &mut App) -> bool {
        (self.vtable.validate)(app, self.id)
    }

    fn clear_error_internal(self, app: &mut App) {
        (self.vtable.clear_error_internal)(app, self.id);
    }

    fn focus_node(self, app: &App) -> Handle<FocusNode> {
        (self.vtable.focus_node)(app, self.id)
    }
}

/// Erased [`FormFieldStateLeaf`] with value type `T`: what a field or parameter Dart types as
/// `FormFieldState<T>` becomes, such as what a [`FormFieldBuilder`] is handed.
///
/// Equality is Dart's `==` on an object reference.
pub struct AnyFormFieldState<T: 'static> {
    id: HandleId,
    vtable: &'static FormFieldStateVTable<T>,
}

impl<T: 'static> Clone for AnyFormFieldState<T> {
    fn clone(&self) -> AnyFormFieldState<T> {
        *self
    }
}

impl<T: 'static> Copy for AnyFormFieldState<T> {}

impl<T: 'static> PartialEq for AnyFormFieldState<T> {
    fn eq(&self, other: &AnyFormFieldState<T>) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> Eq for AnyFormFieldState<T> {}

impl<T: 'static> Debug for AnyFormFieldState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_dynamic().fmt(f)
    }
}

impl<T: 'static> AnyFormFieldState<T> {
    /// This field as a `FormFieldState<dynamic>`.
    pub fn as_dynamic(self) -> DynamicFormFieldState {
        DynamicFormFieldState {
            id: self.id,
            vtable: &self.vtable.dynamic,
        }
    }

    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// The leaf state behind this handle, if it is an `S`: Dart's `field as S`.
    pub fn downcast<S: 'static>(self, app: &App) -> Option<Handle<S>> {
        app.handle::<S>(self.id)
    }

    /// See [`FormFieldStateLeaf::value`].
    pub fn value(self, app: &App) -> Option<&T> {
        (self.vtable.value)(app, self.id)
    }

    /// See [`FormFieldStateLeaf::did_change`].
    pub fn did_change(self, app: &mut App, value: Option<T>) {
        (self.vtable.did_change)(app, self.id, value);
    }

    /// See [`State::context`].
    pub fn context(self, app: &App) -> BuildContext {
        self.as_dynamic().context(app)
    }

    /// See [`State::mounted`].
    pub fn mounted(self, app: &App) -> bool {
        self.as_dynamic().mounted(app)
    }

    /// See [`FormFieldStateLeaf::error_text`].
    pub fn error_text(self, app: &App) -> Option<String> {
        self.as_dynamic().error_text(app)
    }

    /// See [`FormFieldStateLeaf::has_error`].
    pub fn has_error(self, app: &App) -> bool {
        self.as_dynamic().has_error(app)
    }

    /// See [`FormFieldStateLeaf::has_interacted_by_user`].
    pub fn has_interacted_by_user(self, app: &App) -> bool {
        self.as_dynamic().has_interacted_by_user(app)
    }

    /// See [`FormFieldStateLeaf::is_valid`].
    pub fn is_valid(self, app: &mut App) -> bool {
        self.as_dynamic().is_valid(app)
    }

    /// See [`FormFieldStateLeaf::save`].
    pub fn save(self, app: &mut App) {
        self.as_dynamic().save(app);
    }

    /// See [`FormFieldStateLeaf::reset`].
    pub fn reset(self, app: &mut App) {
        self.as_dynamic().reset(app);
    }

    /// See [`FormFieldStateLeaf::clear_error`].
    pub fn clear_error(self, app: &mut App) {
        self.as_dynamic().clear_error(app);
    }

    /// See [`FormFieldStateLeaf::validate`].
    pub fn validate(self, app: &mut App) -> bool {
        self.as_dynamic().validate(app)
    }
}

// ---------------------------------------------------------------------------------------------
// AutovalidateMode

/// Used to configure the auto validation of [`FormField`] and [`Form`] widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutovalidateMode {
    /// No auto validation will occur.
    Disabled,

    /// Used to auto-validate [`Form`] and [`FormField`] even without user interaction.
    Always,

    /// Used to auto-validate [`Form`] and [`FormField`] only after each user interaction.
    OnUserInteraction,

    /// Used to auto-validate [`Form`] and [`FormField`] only after the field has lost focus.
    ///
    /// In order to validate all fields of a [`Form`] after the first time the user interacts
    /// with one, use [`Always`](Self::Always) instead.
    OnUnfocus,

    /// Used to auto-validate [`Form`] and [`FormField`] after each user interaction, only if
    /// the field already has an error.
    ///
    /// This is useful for reducing unnecessary validation calls while still ensuring errors are
    /// re-checked when the user attempts to fix them.
    OnUserInteractionIfError,
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell, RefMut};
    use std::time::Duration;
    use waft_foundation::OptionRetained;
    use waft_foundation::Ref;

    use waft_embedder::{Restoration, Size, TargetPlatform, TextDirection};
    use waft_foundation::{AppCell, UniqueKey, ValueKey};
    use waft_services::{RestorationMap, RestorationUpdate};
    use waft_test::{TestPlatform, TestView};

    use super::*;
    use crate::binding::WidgetsBinding;
    use crate::framework::StatelessWidget;
    use crate::framework::{AnyElement, GlobalKey, downcast_widget};
    use crate::test_harness::{platform_view, pump_every_view};
    use crate::widgets::basic::{Center, Column, Directionality, SizedBox, StatefulBuilder};
    use crate::widgets::focus_manager::{AnyFocusNode, FocusNodeRef};
    use crate::widgets::restoration::RootRestorationScope;
    use crate::widgets::scroll_view::ListView;

    // ---- flutter_test's `WidgetTester`, as far as these tests use it ----

    /// A host that answers `Restoration::get` with what it was handed and records every
    /// `Restoration::put`.
    struct RecordingRestoration {
        stored: RefCell<Option<RestorationUpdate>>,
        puts: RefCell<Vec<RestorationMap>>,
    }

    impl Restoration for RecordingRestoration {
        fn get(&self) -> Option<RestorationUpdate> {
            self.stored.borrow().clone()
        }

        fn put(&self, data: RestorationMap) {
            self.puts.borrow_mut().push(data);
        }
    }

    struct WidgetTester {
        cell: Rc<AppCell>,
        restoration: Rc<RecordingRestoration>,
        now: Duration,
    }

    impl WidgetTester {
        fn new() -> WidgetTester {
            WidgetTester::restoring(None)
        }

        /// An app whose host restores `data`, as `restoreFrom` hands it.
        fn restoring(data: Option<RestorationMap>) -> WidgetTester {
            let restoration = Rc::new(RecordingRestoration {
                stored: RefCell::new(Some(RestorationUpdate {
                    enabled: true,
                    data,
                })),
                puts: RefCell::new(Vec::new()),
            });
            let platform = TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_view(Rc::new(TestView::new(800.0, 600.0)))
                .with_restoration(restoration.clone());
            WidgetTester {
                cell: AppCell::with_platform(Rc::new(platform)),
                restoration,
                now: Duration::ZERO,
            }
        }

        fn app(&self) -> RefMut<'_, App> {
            self.cell.borrow_mut()
        }

        /// `tester.pumpWidget`: attaches `widget` as the root, then one frame.
        fn pump_widget(&mut self, widget: WidgetRef) {
            let widget = Directionality::new(TextDirection::Ltr, widget).into_widget();
            {
                let mut app = self.app();
                let view = platform_view(&app);
                let binding = WidgetsBinding::instance(&mut app);
                binding.attach_root_widget(
                    &mut app,
                    crate::view::View::new(view, widget).into_widget(),
                );
            }
            self.pump();
        }

        /// `tester.pump`: one frame of every view, asked for or not.
        fn pump(&mut self) {
            self.cell.elapse(Duration::ZERO);
            {
                let mut app = self.app();
                pump_every_view(&mut app, self.now);
                app.drain_microtasks();
            }
            self.cell.checkpoint();
        }

        /// The restoration data the host was last handed: `getRestorationData`.
        fn restoration_data(&self) -> Option<RestorationMap> {
            self.restoration.puts.borrow().last().cloned()
        }

        /// Every mounted element, in tree order.
        fn elements(&self) -> Vec<AnyElement> {
            let mut app = self.cell.borrow_mut();
            let root = WidgetsBinding::instance(&mut app)
                .root_element(&app)
                .expect("a mounted tree");
            let mut found = Vec::new();
            let mut stack = vec![root];
            while let Some(element) = stack.pop() {
                found.push(element);
                let mut children = Vec::new();
                element.visit_children(&app, &mut |child| children.push(child));
                stack.extend(children.into_iter().rev());
            }
            found
        }

        /// How many mounted [`Shown`]s show `text`: `find.text(text)`.
        fn count_text(&self, text: &str) -> usize {
            let elements = self.elements();
            let app = self.cell.borrow();
            elements
                .into_iter()
                .filter(|element| {
                    downcast_widget::<Shown>(&**element.widget(&app))
                        .is_some_and(|widget| widget.0 == text)
                })
                .count()
        }

        fn form(&self, key: &GlobalKey) -> Handle<FormState> {
            key.current_state::<FormState>(&self.app())
                .expect("the form is mounted")
        }

        fn field<T: Clone + 'static>(&self, key: &GlobalKey) -> Handle<FormFieldState<T>> {
            key.current_state::<FormFieldState<T>>(&self.app())
                .expect("the field is mounted")
        }

        /// `tester.enterText` on a field: the value its text field would hand `didChange`.
        fn enter_text(&self, key: &GlobalKey, text: &str) {
            let field = self.field::<String>(key);
            field.did_change(&mut self.app(), Some(text.to_string()));
        }
    }

    /// A string on screen, where flutter_test's `find.text` looks: a `Text` without the fonts
    /// a `Text` needs to lay out.
    #[derive(Debug)]
    struct Shown(String);

    impl Shown {
        fn new(text: impl Into<String>) -> Shown {
            Shown(text.into())
        }
    }

    impl StatelessWidget for Shown {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::shrink().into_widget()
        }
    }

    // ---- a minimal `TextFormField`: a `FormField<String>` showing its value and error ----

    fn show_field(app: &mut App, field: AnyFormFieldState<String>) -> WidgetRef {
        let mut children =
            vec![Shown::new(field.value(app).cloned().unwrap_or_default()).into_widget()];
        if let Some(error_text) = field.error_text(app) {
            children.push(Shown::new(error_text).into_widget());
        }
        Column::new().children(children).into_widget()
    }

    fn text_field(key: &GlobalKey) -> FormField<String> {
        FormField::new(show_field).key(key.clone().into())
    }

    /// A field whose subtree takes the focus through `node`, as a text field's does on a tap.
    fn focusable_text_field(key: &GlobalKey, node: FocusNodeRef) -> FormField<String> {
        FormField::new(move |app, field| {
            Focus::new(show_field(app, field))
                .focus_node(node.clone())
                .into_widget()
        })
        .key(key.clone().into())
    }

    fn error_text(value: Option<&String>) -> Option<String> {
        Some(format!(
            "{}/error",
            value.map(String::as_str).unwrap_or("null")
        ))
    }

    #[test]
    fn on_saved_is_called_with_the_value_the_field_holds() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        let field_value = Rc::new(RefCell::new(None));
        let saved = Rc::clone(&field_value);
        tester.pump_widget(
            Form::new(text_field(&field_key).on_saved(move |_app, value| {
                *saved.borrow_mut() = value;
            }))
            .key(form_key.clone().into())
            .into_widget(),
        );

        assert_eq!(*field_value.borrow(), None);

        for test_value in ["Test", ""] {
            tester.enter_text(&field_key, test_value);
            let form = tester.form(&form_key);
            form.save(&mut tester.app());
            // Pumping is unnecessary because callback happens regardless of frames.
            assert_eq!(field_value.borrow().as_deref(), Some(test_value));
        }
    }

    #[test]
    fn on_reset_is_called_when_the_form_resets() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let reset_called = Rc::new(Cell::new(false));
        let called = Rc::clone(&reset_called);
        tester.pump_widget(
            Form::new(
                FormField::<String>::new(|_app, _field| SizedBox::shrink().into_widget())
                    .on_reset(move |_app| called.set(true)),
            )
            .key(form_key.clone().into())
            .into_widget(),
        );

        assert!(!reset_called.get());

        let form = tester.form(&form_key);
        form.reset(&mut tester.app());
        tester.pump();

        assert!(reset_called.get());
    }

    #[test]
    fn the_validator_sets_the_error_text_only_when_validate_is_called() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        let error_text = |value: Option<&String>| {
            Some(format!("{}/error", value.map(String::as_str).unwrap_or("")))
        };
        let builder = |autovalidate_mode| {
            Form::new(text_field(&field_key).validator(move |_app, value| error_text(value)))
                .key(form_key.clone().into())
                .autovalidate_mode(autovalidate_mode)
                .into_widget()
        };

        // Start off not autovalidating.
        tester.pump_widget(builder(AutovalidateMode::Disabled));

        for test_value in ["Test", ""] {
            let expected = error_text(Some(&test_value.to_string())).unwrap();
            let form = tester.form(&form_key);
            form.reset(&mut tester.app());
            tester.pump_widget(builder(AutovalidateMode::Disabled));
            tester.enter_text(&field_key, test_value);
            tester.pump();

            // We have to manually validate if we're not autovalidating.
            assert_eq!(tester.count_text(&expected), 0);
            form.validate(&mut tester.app());
            tester.pump();
            assert_eq!(tester.count_text(&expected), 1);

            // Try again with autovalidation. Should validate immediately.
            form.reset(&mut tester.app());
            tester.pump_widget(builder(AutovalidateMode::Always));
            tester.enter_text(&field_key, test_value);
            tester.pump();

            assert_eq!(tester.count_text(&expected), 1);
        }
    }

    fn valid_string_validator(value: Option<&String>) -> Option<String> {
        (value.map(String::as_str) != Some("Valid string")).then(|| "Error text".to_string())
    }

    #[test]
    fn is_valid_is_true_for_fields_that_are_valid() {
        let mut tester = WidgetTester::new();
        let field_key1 = GlobalKey::new();
        let field_key2 = GlobalKey::new();
        let field = |key: &GlobalKey| {
            text_field(key)
                .initial_value("Valid string".to_string())
                .validator(|_app, value| valid_string_validator(value))
                .autovalidate_mode(AutovalidateMode::Always)
                .into_widget()
        };
        tester.pump_widget(
            Form::new(ListView::new().children([field(&field_key1), field(&field_key2)]))
                .into_widget(),
        );

        let (field1, field2) = (
            tester.field::<String>(&field_key1),
            tester.field::<String>(&field_key2),
        );
        assert!(field1.is_valid(&mut tester.app()));
        assert!(field2.is_valid(&mut tester.app()));
    }

    #[test]
    fn is_valid_is_false_for_an_invalid_field_and_does_not_change_its_error_display() {
        let mut tester = WidgetTester::new();
        let field_key1 = GlobalKey::new();
        let field_key2 = GlobalKey::new();
        let field = |key: &GlobalKey, initial_value: &str| {
            text_field(key)
                .initial_value(initial_value.to_string())
                .validator(|_app, value| valid_string_validator(value))
                .autovalidate_mode(AutovalidateMode::Disabled)
                .into_widget()
        };
        tester.pump_widget(
            Form::new(
                ListView::new()
                    .children([field(&field_key1, "Valid string"), field(&field_key2, "")]),
            )
            .into_widget(),
        );

        let (field1, field2) = (
            tester.field::<String>(&field_key1),
            tester.field::<String>(&field_key2),
        );
        assert!(field1.is_valid(&mut tester.app()));
        assert!(!field2.is_valid(&mut tester.app()));
        assert!(!field2.has_error(&tester.app()));
    }

    #[test]
    fn validate_granularly_returns_a_set_of_all_and_only_the_invalid_fields() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let valid_fields_key = UniqueKey::new();
        let invalid_fields_key = UniqueKey::new();
        let field = |key: UniqueKey, initial_value: &str| {
            FormField::new(show_field)
                .key(key.into())
                .initial_value(initial_value.to_string())
                .validator(|_app, value| valid_string_validator(value))
                .autovalidate_mode(AutovalidateMode::Disabled)
                .into_widget()
        };
        tester.pump_widget(
            Form::new(ListView::new().children([
                field(valid_fields_key, "Valid string"),
                field(invalid_fields_key, "Invalid string"),
                field(invalid_fields_key, "Invalid string"),
            ]))
            .key(form_key.clone().into())
            .into_widget(),
        );

        let form = tester.form(&form_key);
        let validation_result = form.validate_granularly(&mut tester.app());

        let app = tester.app();
        let with_key = |key: UniqueKey| {
            let key = KeyRef::from(key);
            validation_result
                .iter()
                .filter(|field| field.context(&app).widget(&app).key() == Some(&key))
                .count()
        };
        assert_eq!(validation_result.len(), 2);
        assert_eq!(with_key(invalid_fields_key), 2);
        assert_eq!(with_key(valid_fields_key), 0);
    }

    #[test]
    fn multiple_fields_communicate() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        let second_key = GlobalKey::new();
        let read_key = field_key.clone();
        // Input 2's validator depends on a input 1's value.
        let builder = || {
            let read_key = read_key.clone();
            Form::new(
                ListView::new().children([
                    text_field(&field_key).into_widget(),
                    text_field(&second_key)
                        .validator(move |app, _input| {
                            let field = read_key
                                .current_state::<FormFieldState<String>>(app)
                                .expect("the first field is mounted");
                            Some(format!(
                                "{}/error",
                                field.value(app).cloned().unwrap_or_default()
                            ))
                        })
                        .into_widget(),
                ]),
            )
            .key(form_key.clone().into())
            .autovalidate_mode(AutovalidateMode::Always)
            .into_widget()
        };
        tester.pump_widget(builder());

        for test_value in ["Test", ""] {
            tester.enter_text(&field_key, test_value);
            tester.pump();

            // Check for a new Text widget with our error text.
            assert_eq!(tester.count_text(&format!("{test_value}/error")), 1);
        }
    }

    #[test]
    fn removing_a_field_from_the_tree_does_not_crash_the_form() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        let field_value = Rc::new(RefCell::new(None::<String>));
        let builder = |remove: bool| {
            let saved = Rc::clone(&field_value);
            let child = if remove {
                SizedBox::new().into_widget()
            } else {
                text_field(&field_key)
                    .on_saved(move |_app, value| *saved.borrow_mut() = value)
                    .validator(|_app, value| {
                        value
                            .is_some_and(|value| !value.is_empty())
                            .then(|| "yes".to_string())
                    })
                    .into_widget()
            };
            Form::new(child).key(form_key.clone().into()).into_widget()
        };

        tester.pump_widget(builder(false));
        let form = tester.form(&form_key);

        assert_eq!(*field_value.borrow(), None);
        assert!(form.validate(&mut tester.app()));

        tester.enter_text(&field_key, "Test");
        tester.pump_widget(builder(false));

        // Form wasn't saved yet.
        assert_eq!(*field_value.borrow(), None);
        assert!(!form.validate(&mut tester.app()));

        form.save(&mut tester.app());

        // Now fieldValue is saved.
        assert_eq!(field_value.borrow().as_deref(), Some("Test"));
        assert!(!form.validate(&mut tester.app()));

        // Now remove the field with an error.
        tester.pump_widget(builder(true));

        // Reset the form. Should not crash.
        form.reset(&mut tester.app());
        form.save(&mut tester.app());
        assert!(form.validate(&mut tester.app()));
    }

    /// A lone field, no form above it, that records its state as its builder is handed it.
    fn recording_field(
        recorded: &Rc<Cell<Option<AnyFormFieldState<String>>>>,
        show_error: bool,
    ) -> FormField<String> {
        let recorded = Rc::clone(recorded);
        FormField::new(move |app, field| {
            recorded.set(Some(field));
            let mut children = vec![SizedBox::new().into_widget()];
            if show_error && let Some(error_text) = field.error_text(app) {
                children.push(Shown::new(error_text).into_widget());
            }
            Column::new().children(children).into_widget()
        })
    }

    fn required(value: Option<&String>) -> Option<String> {
        value
            .is_none_or(|value| value.is_empty())
            .then(|| "Required".to_string())
    }

    #[test]
    fn a_field_does_not_autovalidate_before_its_value_changes_on_user_interaction() {
        let mut tester = WidgetTester::new();
        let recorded = Rc::new(Cell::new(None));
        tester.pump_widget(
            recording_field(&recorded, true)
                .initial_value("foo".to_string())
                .autovalidate_mode(AutovalidateMode::OnUserInteraction)
                .validator(|_app, value| error_text(value))
                .into_widget(),
        );
        let field = recorded.get().expect("built");
        // The form field has no error.
        assert!(!field.has_error(&tester.app()));
        // No error widget is visible.
        assert_eq!(tester.count_text("foo/error"), 0);
    }

    #[test]
    fn a_field_does_not_autovalidate_before_its_value_changes_on_user_interaction_if_error() {
        let mut tester = WidgetTester::new();
        let recorded = Rc::new(Cell::new(None));
        tester.pump_widget(
            recording_field(&recorded, true)
                .initial_value("foo".to_string())
                .autovalidate_mode(AutovalidateMode::OnUserInteractionIfError)
                .validator(|_app, value| required(value))
                .into_widget(),
        );
        let field = recorded.get().expect("built");
        // The form field has no error.
        assert!(!field.has_error(&tester.app()));
        // No "Required" error is visible.
        assert_eq!(tester.count_text("Required"), 0);
    }

    #[test]
    fn a_field_with_an_empty_value_does_not_autovalidate_before_it_changes_on_user_interaction_if_error()
     {
        let mut tester = WidgetTester::new();
        let recorded = Rc::new(Cell::new(None));
        tester.pump_widget(
            recording_field(&recorded, true)
                .autovalidate_mode(AutovalidateMode::OnUserInteractionIfError)
                .validator(|_app, value| required(value))
                .into_widget(),
        );
        let field = recorded.get().expect("built");

        assert!(!field.has_error(&tester.app()));
        assert_eq!(tester.count_text("Required"), 0);
        assert_eq!(field.error_text(&tester.app()), None);

        field.validate(&mut tester.app());
        tester.pump();

        assert!(field.has_error(&tester.app()));
        assert_eq!(tester.count_text("Required"), 1);
    }

    #[test]
    fn a_field_autovalidates_before_its_value_changes_when_its_mode_is_always() {
        let mut tester = WidgetTester::new();
        let recorded = Rc::new(Cell::new(None));
        tester.pump_widget(
            recording_field(&recorded, false)
                .initial_value("foo".to_string())
                .autovalidate_mode(AutovalidateMode::Always)
                .validator(|_app, value| error_text(value))
                .into_widget(),
        );
        let field = recorded.get().expect("built");
        assert!(field.has_error(&tester.app()));
    }

    fn error_slash(value: Option<&String>) -> Option<String> {
        Some(format!(
            "error/{}",
            value.map(String::as_str).unwrap_or("null")
        ))
    }

    #[test]
    fn a_form_autovalidates_its_fields_only_after_one_changes_on_user_interaction() {
        let mut tester = WidgetTester::new();
        let keys = [GlobalKey::new(), GlobalKey::new(), GlobalKey::new()];
        let builder = || {
            Form::new(Column::new().children(keys.iter().map(|key| {
                text_field(key)
                    .initial_value("foo".to_string())
                    .validator(|_app, value| error_slash(value))
                    .into_widget()
            })))
            .autovalidate_mode(AutovalidateMode::OnUserInteraction)
            .into_widget()
        };

        // Makes sure the Form widget won't auto-validate the form fields
        // after rebuilds if there is not user interaction.
        tester.pump_widget(builder());
        tester.pump_widget(builder());

        // We expect no validation error text being shown.
        assert_eq!(tester.count_text("error/foo"), 0);

        // Set a empty string into the first form field to
        // trigger the fields validators.
        tester.enter_text(&keys[0], "");
        tester.pump();

        // Now we expect the errors to be shown for the first Text Field and
        // for the next two form fields that have their contents unchanged.
        assert_eq!(tester.count_text("error/"), 1);
        assert_eq!(tester.count_text("error/foo"), 2);
    }

    #[test]
    fn a_form_autovalidates_its_fields_before_any_changes_when_its_mode_is_always() {
        let mut tester = WidgetTester::new();
        let field_key = GlobalKey::new();
        let builder = || {
            Form::new(
                text_field(&field_key)
                    // A `TextFormField` without an initial value holds an empty string.
                    .initial_value(String::new())
                    .validator(|_app, value| error_slash(value)),
            )
            .autovalidate_mode(AutovalidateMode::Always)
            .into_widget()
        };

        // The issue only happens on the second build so we
        // need to rebuild the tree twice.
        tester.pump_widget(builder());
        tester.pump_widget(builder());

        // We expect validation error text being shown.
        assert_eq!(tester.count_text("error/"), 1);
    }

    #[test]
    fn form_reset_clears_errors_and_autovalidates_again_only_on_the_next_user_interaction() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(
            Form::new(
                text_field(&field_key)
                    .initial_value("foo".to_string())
                    .validator(|_app, value| error_text(value)),
            )
            .key(form_key.clone().into())
            .autovalidate_mode(AutovalidateMode::OnUserInteraction)
            .into_widget(),
        );

        // No error text is visible yet.
        assert_eq!(tester.count_text("foo/error"), 0);

        tester.enter_text(&field_key, "bar");
        tester.pump();
        tester.pump();
        assert_eq!(tester.count_text("bar/error"), 1);

        // Resetting the form state should remove the error text.
        let form = tester.form(&form_key);
        form.reset(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text("bar/error"), 0);
    }

    #[test]
    fn a_form_on_user_interaction_if_error_revalidates_only_once_an_error_exists() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(
            Form::new(
                text_field(&field_key)
                    .initial_value("foo".to_string())
                    .validator(|_app, value| required(value)),
            )
            .key(form_key.clone().into())
            .autovalidate_mode(AutovalidateMode::OnUserInteractionIfError)
            .into_widget(),
        );
        let form = tester.form(&form_key);

        // No error text is visible yet. (Initial valid state).
        assert_eq!(tester.count_text("Required"), 0);

        // User types valid input 'bar' → autovalidate is disabled → still no error.
        tester.enter_text(&field_key, "bar");
        tester.pump();
        assert_eq!(tester.count_text("Required"), 0);

        // Clear the input (invalid).
        tester.enter_text(&field_key, "");
        tester.pump();

        // Manually submit form to show the initial error (AutovalidateMode is now active).
        form.validate(&mut tester.app());
        assert_eq!(tester.count_text("Required"), 0);
        tester.pump();

        // Verify error is shown.
        assert_eq!(tester.count_text("Required"), 1);

        // Now user interacts again with valid text ('baz') → validation auto-runs and clears
        // the error.
        tester.enter_text(&field_key, "baz");
        tester.pump();
        assert_eq!(tester.count_text("Required"), 0);

        // Check the behavior of a manual validate when the text is already valid.
        // This should *confirm* the error is cleared, not re-introduce it.
        form.validate(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text("Required"), 0);

        // Resetting should clear form (already cleared, but a safety check).
        tester.enter_text(&field_key, "");
        form.reset(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text("Required"), 0);
    }

    #[test]
    fn has_interacted_by_user_is_false_while_the_input_has_not_changed() {
        let mut tester = WidgetTester::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(text_field(&field_key).into_widget());

        let field = tester.field::<String>(&field_key);
        assert!(!field.has_interacted_by_user(&tester.app()));
    }

    #[test]
    fn has_interacted_by_user_is_true_after_the_input_changed() {
        let mut tester = WidgetTester::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(text_field(&field_key).into_widget());
        let field = tester.field::<String>(&field_key);

        // initially, the field has not been interacted with
        assert!(!field.has_interacted_by_user(&tester.app()));

        // after entering text, the field has been interacted with
        tester.enter_text(&field_key, "foo");
        assert!(field.has_interacted_by_user(&tester.app()));
    }

    #[test]
    fn has_interacted_by_user_is_false_after_the_field_is_reset() {
        let mut tester = WidgetTester::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(text_field(&field_key).into_widget());
        let field = tester.field::<String>(&field_key);

        // initially, the field has not been interacted with
        assert!(!field.has_interacted_by_user(&tester.app()));

        // after entering text, the field has been interacted with
        tester.enter_text(&field_key, "foo");
        assert!(field.has_interacted_by_user(&tester.app()));

        // after resetting the field, it has not been interacted with again
        field.reset(&mut tester.app());
        assert!(!field.has_interacted_by_user(&tester.app()));
    }

    #[test]
    fn force_error_text_forces_an_error_state_from_the_first_build() {
        let mut tester = WidgetTester::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(
            Form::new(text_field(&field_key).force_error_text("Forcing error."))
                .autovalidate_mode(AutovalidateMode::Disabled)
                .into_widget(),
        );
        assert_eq!(tester.count_text("Forcing error."), 1);
    }

    #[test]
    fn validate_is_false_with_a_force_error_text_even_when_the_validator_finds_no_error() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(
            Form::new(
                text_field(&field_key)
                    .force_error_text("Forcing error")
                    .validator(|_app, _value| None),
            )
            .key(form_key.clone().into())
            .into_widget(),
        );

        assert_eq!(tester.count_text("Forcing error"), 1);
        let form = tester.form(&form_key);
        let is_valid = form.validate(&mut tester.app());
        assert!(!is_valid);

        tester.pump();
        assert_eq!(tester.count_text("Forcing error"), 1);
    }

    #[test]
    fn force_error_text_forces_an_error_state_only_once_it_is_set() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        let builder = |force_error_text: Option<&str>| {
            let mut field = text_field(&field_key);
            if let Some(force_error_text) = force_error_text {
                field = field.force_error_text(force_error_text);
            }
            Form::new(field)
                .key(form_key.clone().into())
                .autovalidate_mode(AutovalidateMode::Disabled)
                .into_widget()
        };

        tester.pump_widget(builder(None));
        let form = tester.form(&form_key);
        let is_valid = form.validate(&mut tester.app());
        assert!(is_valid);
        assert_eq!(tester.count_text("Forcing Error Text"), 0);
        tester.pump_widget(builder(Some("Forcing Error Text")));
        assert_eq!(tester.count_text("Forcing Error Text"), 1);
    }

    #[test]
    fn the_validator_is_not_called_while_a_force_error_text_is_given() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        let force_error_text = "Forcing error.";
        let validator_error_text =
            "this error should not appear as we override it with forceErrorText";
        let did_call_validator = Rc::new(Cell::new(false));
        let builder = |autovalidate_mode| {
            let called = Rc::clone(&did_call_validator);
            Form::new(
                text_field(&field_key)
                    .force_error_text(force_error_text)
                    .validator(move |_app, _value| {
                        called.set(true);
                        Some(validator_error_text.to_string())
                    }),
            )
            .key(form_key.clone().into())
            .autovalidate_mode(autovalidate_mode)
            .into_widget()
        };

        // Start off not autovalidating.
        tester.pump_widget(builder(AutovalidateMode::Disabled));
        assert_eq!(tester.count_text(force_error_text), 1);
        assert_eq!(tester.count_text(validator_error_text), 0);

        let form = tester.form(&form_key);
        form.reset(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text(force_error_text), 0);
        assert_eq!(tester.count_text(validator_error_text), 0);

        // We have to manually validate if we're not autovalidating.
        form.validate(&mut tester.app());
        tester.pump();

        assert!(!did_call_validator.get());
        assert_eq!(tester.count_text(force_error_text), 1);
        assert_eq!(tester.count_text(validator_error_text), 0);

        // Try again with autovalidation. Should validate immediately.
        tester.pump_widget(builder(AutovalidateMode::Always));

        assert!(!did_call_validator.get());
        assert_eq!(tester.count_text(force_error_text), 1);
        assert_eq!(tester.count_text(validator_error_text), 0);
    }

    #[test]
    fn removing_the_validator_takes_its_error_text_away() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        let use_validator = Rc::new(Cell::new(false));
        let set_state = Rc::new(RefCell::new(None));
        let builder = {
            let (form_key, field_key) = (form_key.clone(), field_key.clone());
            let (use_validator, set_state) = (Rc::clone(&use_validator), Rc::clone(&set_state));
            StatefulBuilder::new(move |_app, _context, setter| {
                *set_state.borrow_mut() = Some(setter);
                let mut field = text_field(&field_key);
                if use_validator.get() {
                    field = field.validator(|_app, value| {
                        value
                            .is_none_or(|value| value.is_empty())
                            .then(|| "test_error".to_string())
                    });
                }
                Form::new(field).key(form_key.clone().into()).into_widget()
            })
        };
        tester.pump_widget(builder.into_widget());
        let form = tester.form(&form_key);
        let set_use_validator = |tester: &mut WidgetTester, value: bool| {
            let setter = set_state.borrow().clone().expect("built");
            let use_validator = Rc::clone(&use_validator);
            setter(&mut tester.app(), &mut move |_app| use_validator.set(value));
            tester.pump();
        };

        // Start with no validator.
        tester.enter_text(&field_key, "");
        tester.pump();
        form.validate(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text("test_error"), 0);

        // Now use the validator.
        set_use_validator(&mut tester, true);
        form.validate(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text("test_error"), 1);

        // Remove the validator again and expect the error to disappear.
        set_use_validator(&mut tester, false);
        form.validate(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text("test_error"), 0);
    }

    /// `tester.tap` on a text field: its subtree takes the primary focus.
    fn focus(tester: &mut WidgetTester, node: AnyFocusNode) {
        node.request_focus(&mut tester.app(), None);
        tester.pump();
        tester.pump();
    }

    #[test]
    fn a_form_on_unfocus_validates_a_field_as_it_loses_the_focus() {
        let mut tester = WidgetTester::new();
        let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
        let (node1, node2): (FocusNodeRef, FocusNodeRef) = {
            let mut app = tester.app();
            (
                FocusNode::new(&mut app).into_erased(),
                FocusNode::new(&mut app).into_erased(),
            )
        };
        let field = |key: &GlobalKey, node| {
            focusable_text_field(key, node)
                .initial_value("bar".to_string())
                .validator(|_app, value| error_text(value))
                .into_widget()
        };
        tester.pump_widget(
            Form::new(
                Column::new().children([field(&key1, node1.clone()), field(&key2, node2.clone())]),
            )
            .autovalidate_mode(AutovalidateMode::OnUnfocus)
            .into_widget(),
        );

        // No error text is visible yet.
        assert_eq!(tester.count_text("foo/error"), 0);

        // Enter text in the first TextFormField.
        focus(&mut tester, *node1);
        tester.enter_text(&key1, "foo");
        tester.pump();

        // No error text is visible yet.
        assert_eq!(tester.count_text("foo/error"), 0);

        // Tap on the second TextFormField to trigger validation.
        // This should trigger validation for the first TextFormField as well.
        focus(&mut tester, *node2);

        // Verify that the error text is displayed for the first TextFormField.
        assert_eq!(tester.count_text("foo/error"), 1);
        assert_eq!(tester.count_text("bar/error"), 0);

        // Tap on the first TextFormField to trigger validation.
        focus(&mut tester, *node1);

        // Verify that the both error texts are displayed.
        assert_eq!(tester.count_text("foo/error"), 1);
        assert_eq!(tester.count_text("bar/error"), 1);
    }

    #[test]
    fn a_field_mode_of_its_own_wins_over_the_form_on_unfocus() {
        let mut tester = WidgetTester::new();
        let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
        let (node1, node2): (FocusNodeRef, FocusNodeRef) = {
            let mut app = tester.app();
            (
                FocusNode::new(&mut app).into_erased(),
                FocusNode::new(&mut app).into_erased(),
            )
        };
        tester.pump_widget(
            Form::new(
                Column::new().children([
                    focusable_text_field(&key1, node1.clone())
                        .autovalidate_mode(AutovalidateMode::Always)
                        .initial_value("foo".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                    focusable_text_field(&key2, node2.clone())
                        .autovalidate_mode(AutovalidateMode::Disabled)
                        .initial_value("bar".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                ]),
            )
            .autovalidate_mode(AutovalidateMode::OnUnfocus)
            .into_widget(),
        );

        // Verify that the error text is displayed for the first TextFormField.
        assert_eq!(tester.count_text("foo/error"), 1);

        // Enter text in the TextFormField.
        focus(&mut tester, *node1);
        tester.enter_text(&key1, "foo");
        tester.pump();

        // Click in the second TextFormField to trigger validation.
        focus(&mut tester, *node2);

        // No error text is visible yet for the second TextFormField.
        assert_eq!(tester.count_text("bar/error"), 0);

        // Now click in the first TextFormField to trigger validation for the second
        // TextFormField.
        focus(&mut tester, *node1);

        // Verify that the error text is displayed for the second TextFormField.
        assert_eq!(tester.count_text("bar/error"), 1);
    }

    #[test]
    fn a_form_on_unfocus_validates_every_field_when_validated_by_hand() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
        let (node1, node2): (FocusNodeRef, FocusNodeRef) = {
            let mut app = tester.app();
            (
                FocusNode::new(&mut app).into_erased(),
                FocusNode::new(&mut app).into_erased(),
            )
        };
        tester.pump_widget(
            Form::new(
                Column::new().children([
                    focusable_text_field(&key1, node1.clone())
                        .initial_value("foo".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                    focusable_text_field(&key2, node2.clone())
                        .initial_value("bar".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                ]),
            )
            .key(form_key.clone().into())
            .autovalidate_mode(AutovalidateMode::OnUnfocus)
            .into_widget(),
        );

        // Focus on the first field.
        focus(&mut tester, *node1);

        // Check no error messages are displayed initially.
        assert_eq!(tester.count_text("foo/error"), 0);
        assert_eq!(tester.count_text("bar/error"), 0);

        // Validate all fields manually using FormState.
        let form = tester.form(&form_key);
        assert!(!form.validate(&mut tester.app()));
        tester.pump();

        // Check error messages are displayed.
        assert_eq!(tester.count_text("foo/error"), 1);
        assert_eq!(tester.count_text("bar/error"), 1);
    }

    #[test]
    fn a_form_always_validates_its_fields_at_the_second_frame() {
        let mut tester = WidgetTester::new();
        let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
        tester.pump_widget(
            Form::new(
                Column::new().children([
                    text_field(&key1)
                        .initial_value("foo".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                    text_field(&key2)
                        .initial_value("bar".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                ]),
            )
            .autovalidate_mode(AutovalidateMode::Always)
            .into_widget(),
        );

        // The validation happens in a post frame callback, so the error
        // doesn't show up until the second frame.
        assert_eq!(tester.count_text("foo/error"), 0);
        assert_eq!(tester.count_text("bar/error"), 0);

        tester.pump();

        // The error shows up on the second frame.
        assert_eq!(tester.count_text("foo/error"), 1);
        assert_eq!(tester.count_text("bar/error"), 1);
    }

    /// `tester.getSize(find.byType(W))`: the size of the first box at or below the one `W`.
    fn size_of_widget<W: 'static>(tester: &WidgetTester) -> Size {
        let elements = tester.elements();
        let app = tester.app();
        let mut found = elements
            .into_iter()
            .filter(|element| downcast_widget::<W>(&**element.widget(&app)).is_some());
        let element = found.next().expect("one widget of the type");
        assert!(found.next().is_none(), "one widget of the type");
        element
            .render_object(&app)
            .and_then(|object| object.as_box())
            .expect("a box below the widget")
            .size(&app)
    }

    /// `Text('X')`: a box that would be bigger than zero, which zero constraints squeeze.
    fn sized_text() -> WidgetRef {
        SizedBox::new()
            .width(14.0)
            .height(14.0)
            .child(Shown::new("X"))
            .into_widget()
    }

    #[test]
    fn a_form_does_not_crash_at_zero_area() {
        let mut tester = WidgetTester::new();
        tester.pump_widget(
            Center::new()
                .child(SizedBox::shrink().child(Form::new(sized_text())))
                .into_widget(),
        );
        assert_eq!(size_of_widget::<Form>(&tester), Size::ZERO);
    }

    #[test]
    fn a_form_field_does_not_crash_at_zero_area() {
        let mut tester = WidgetTester::new();
        tester.pump_widget(
            Center::new()
                .child(
                    SizedBox::shrink().child(FormField::<String>::new(|_app, _field| sized_text())),
                )
                .into_widget(),
        );
        assert_eq!(size_of_widget::<FormField<String>>(&tester), Size::ZERO);
    }

    #[test]
    fn clear_error_clears_the_error_and_keeps_the_value() {
        let mut tester = WidgetTester::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(
            Form::new(
                text_field(&field_key)
                    .initial_value("foo".to_string())
                    .validator(|_app, value| error_text(value)),
            )
            .into_widget(),
        );
        let field = tester.field::<String>(&field_key);

        field.validate(&mut tester.app());
        tester.pump();
        assert_eq!(tester.count_text("foo/error"), 1);

        field.clear_error(&mut tester.app());
        tester.pump();

        assert_eq!(tester.count_text("foo/error"), 0);

        // Value is preserved.
        assert_eq!(tester.count_text("foo"), 1);
    }

    #[test]
    fn clear_error_on_the_form_clears_every_field_error() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
        tester.pump_widget(
            Form::new(
                Column::new().children([
                    text_field(&key1)
                        .initial_value("foo".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                    text_field(&key2)
                        .initial_value("bar".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                ]),
            )
            .key(form_key.clone().into())
            .into_widget(),
        );
        let form = tester.form(&form_key);

        form.validate(&mut tester.app());
        tester.pump();

        assert_eq!(tester.count_text("foo/error"), 1);
        assert_eq!(tester.count_text("bar/error"), 1);

        form.clear_error(&mut tester.app());
        tester.pump();

        assert_eq!(tester.count_text("foo/error"), 0);
        assert_eq!(tester.count_text("bar/error"), 0);
        assert_eq!(tester.count_text("foo"), 1);
        assert_eq!(tester.count_text("bar"), 1);
    }

    #[test]
    fn fields_exposes_every_registered_field_state_with_its_value() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
        tester.pump_widget(
            Form::new(
                Column::new().children([
                    text_field(&key1)
                        .initial_value("A".to_string())
                        .into_widget(),
                    text_field(&key2)
                        .initial_value("B".to_string())
                        .into_widget(),
                ]),
            )
            .key(form_key.clone().into())
            .into_widget(),
        );

        let form = tester.form(&form_key);
        let app = tester.app();
        let fields = form.fields(&app);
        assert_eq!(fields.len(), 2);
        let values: Vec<&String> = fields
            .iter()
            .map(|field| {
                field
                    .value(&app)
                    .and_then(|value| value.downcast_ref::<String>())
                    .unwrap()
            })
            .collect();
        assert!(values.contains(&&"A".to_string()));
        assert!(values.contains(&&"B".to_string()));
    }

    #[test]
    fn fields_reports_every_field_invalid_after_validation_errors() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let (key1, key2) = (GlobalKey::new(), GlobalKey::new());
        tester.pump_widget(
            Form::new(
                Column::new().children([
                    text_field(&key1)
                        .initial_value("foo".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                    text_field(&key2)
                        .initial_value("bar".to_string())
                        .validator(|_app, value| error_text(value))
                        .into_widget(),
                ]),
            )
            .key(form_key.clone().into())
            .into_widget(),
        );
        let form = tester.form(&form_key);

        form.validate(&mut tester.app());
        tester.pump();

        assert_eq!(tester.count_text("foo/error"), 1);
        assert_eq!(tester.count_text("bar/error"), 1);

        let mut app = tester.app();
        let fields = form.fields(&app);

        // Expect all fields to be invalid.
        assert!(fields.into_iter().all(|field| !field.is_valid(&mut app)));
    }

    #[test]
    fn is_valid_evaluates_validity_without_updating_the_error_display() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let field_key = GlobalKey::new();
        tester.pump_widget(
            Form::new(
                text_field(&field_key)
                    .initial_value("foo".to_string())
                    .validator(|_app, value| error_text(value)),
            )
            .key(form_key.clone().into())
            .into_widget(),
        );

        let form = tester.form(&form_key);
        let fields = form.fields(&tester.app());
        assert_eq!(fields.len(), 1);
        let field = fields[0];

        assert!(!field.is_valid(&mut tester.app()));

        // No error UI should be shown.
        assert_eq!(tester.count_text("foo/error"), 0);
        assert!(!field.has_error(&tester.app()));
    }

    #[test]
    fn fields_collects_and_updates_the_values_of_fields_of_every_type() {
        let mut tester = WidgetTester::new();
        let form_key = GlobalKey::new();
        let keyed = |name: &str| KeyRef::from(ValueKey::new(name.to_string()));
        tester.pump_widget(
            Form::new(
                Column::new().children([
                    FormField::new(show_field)
                        .key(keyed("name"))
                        .initial_value("Name".to_string())
                        .into_widget(),
                    FormField::new(show_field)
                        .key(keyed("email"))
                        .initial_value("Email".to_string())
                        .into_widget(),
                    FormField::<i64>::new(|_app, _field| SizedBox::shrink().into_widget())
                        .key(keyed("age"))
                        .initial_value(18)
                        .into_widget(),
                ]),
            )
            .key(form_key.clone().into())
            .into_widget(),
        );
        let form = tester.form(&form_key);

        let collect_data = |tester: &WidgetTester| {
            let app = tester.app();
            let mut data = Vec::new();
            for field in form.fields(&app) {
                let key = field.context(&app).widget(&app).key().cloned();
                let value = field.value(&app).expect("every field holds a value");
                let value = value
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| value.downcast_ref::<i64>().map(i64::to_string))
                    .expect("a String or an i64");
                data.push((key.expect("every field is keyed"), value));
            }
            data
        };

        assert_eq!(
            collect_data(&tester),
            [
                (keyed("name"), "Name".to_string()),
                (keyed("email"), "Email".to_string()),
                (keyed("age"), "18".to_string()),
            ]
        );

        let field = |tester: &WidgetTester, name: &str| {
            let app = tester.app();
            form.fields(&app)
                .into_iter()
                .find(|field| field.context(&app).widget(&app).key() == Some(&keyed(name)))
                .expect("a field keyed so")
        };
        let name = field(&tester, "name")
            .typed::<String>()
            .expect("a String field");
        let email = field(&tester, "email")
            .typed::<String>()
            .expect("a String field");
        let age = field(&tester, "age").typed::<i64>().expect("an i64 field");
        assert!(field(&tester, "age").typed::<String>().is_none());

        name.did_change(&mut tester.app(), Some("New Name".to_string()));
        email.did_change(&mut tester.app(), Some("new@email.com".to_string()));
        age.did_change(&mut tester.app(), Some(30));

        tester.pump();

        assert_eq!(
            collect_data(&tester),
            [
                (keyed("name"), "New Name".to_string()),
                (keyed("email"), "new@email.com".to_string()),
                (keyed("age"), "30".to_string()),
            ]
        );
    }

    // ---- restoration, through a field that restores its value as `TextFormField` does ----

    /// A [`FormField`] subclass whose state also restores its value, as `TextFormField`'s
    /// restorable controller does.
    struct RestorableTextField {
        form_field: FormFieldData<String>,
    }

    impl Debug for RestorableTextField {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.form_field.fmt(f)
        }
    }

    impl FormFieldLeaf for RestorableTextField {
        type Value = String;
        crate::form_field_accessors!();
    }

    impl StatefulWidget for RestorableTextField {
        type State = RestorableTextFieldState;

        fn key(&self) -> Option<&KeyRef> {
            self.form_field.key.as_ref()
        }

        fn create_state(&self) -> RestorableTextFieldState {
            RestorableTextFieldState {
                state: StateData::new(),
                restoration: RestorationMixinData::new(),
                form_field_state: FormFieldStateData::new(&self.form_field),
                text: None,
            }
        }
    }

    struct RestorableTextFieldState {
        state: StateData<RestorableTextField>,
        restoration: RestorationMixinData,
        form_field_state: FormFieldStateData<String>,
        text: Option<Ref<RestorableStringN>>,
    }

    impl RestorableTextFieldState {
        fn text(self: Handle<Self>, app: &App) -> Handle<RestorableStringN> {
            app.get(self).text.handle().expect("created in init_state")
        }
    }

    impl FormFieldStateLeaf for RestorableTextFieldState {
        crate::form_field_state_accessors!();

        fn did_change(self: Handle<Self>, app: &mut App, value: Option<String>) {
            FormFieldState::did_change(self, app, value.clone());
            self.text(app).set_value(app, value);
        }

        fn reset(self: Handle<Self>, app: &mut App) {
            let initial_value = self.widget(app).form_field.initial_value.clone();
            self.text(app).set_value(app, initial_value);
            FormFieldState::reset(self, app);
        }
    }

    impl RestorationMixin for RestorableTextFieldState {
        crate::restoration_mixin_accessors!();

        fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
            FormFieldState::restoration_id(self, app)
        }

        fn restore_state(
            self: Handle<Self>,
            app: &mut App,
            old_bucket: Option<Handle<RestorationBucket>>,
            initial_restore: bool,
        ) {
            FormFieldState::restore_state(self, app, old_bucket, initial_restore);
            let text = self.text(app);
            self.register_for_restoration(app, text.as_property(), "text");
            let value = RestorableValue::value(text, app).clone();
            self.set_value(app, value);
        }
    }

    impl State for RestorableTextFieldState {
        type Widget = RestorableTextField;
        crate::state_accessors!();

        fn deactivate(self: Handle<Self>, app: &mut App) {
            FormFieldState::deactivate(self, app);
        }

        fn init_state(self: Handle<Self>, app: &mut App) {
            FormFieldState::init_state(self, app);
            let initial_value = self.widget(app).form_field.initial_value.clone();
            let text = RestorableStringN::new(app, initial_value);
            app.get_mut(self).text = Some(text);
        }

        fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RestorableTextField) {
            FormFieldState::did_update_widget(self, app, old_widget);
        }

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            FormFieldState::did_change_dependencies(self, app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            let text = self.text(app);
            RestorableProperty::dispose(text, app);
            FormFieldState::dispose(self, app);
        }

        fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
            FormFieldState::build(self, app, context)
        }
    }

    #[test]
    fn a_lone_field_restores_its_error_text_on_user_interaction() {
        let field_key = GlobalKey::new();
        let builder = || {
            let mut form_field = FormFieldData::new(Rc::new(show_field));
            form_field.key = Some(field_key.clone().into());
            form_field.autovalidate_mode = AutovalidateMode::OnUserInteraction;
            form_field.restoration_id = Some("text_form_field".to_string());
            form_field.initial_value = Some("foo".to_string());
            form_field.validator = Some(Rc::new(|_app, value| error_text(value)));
            RootRestorationScope::new(Some("app".to_string()), RestorableTextField { form_field })
                .into_widget()
        };
        let restart_and_restore = |data: Option<RestorationMap>| {
            let mut tester = WidgetTester::restoring(data);
            tester.pump_widget(builder());
            tester
        };

        let mut tester = WidgetTester::new();
        tester.pump_widget(builder());

        // No error text is visible yet.
        assert_eq!(tester.count_text("foo/error"), 0);

        let field = key_state::<RestorableTextFieldState>(&tester, &field_key);
        field.did_change(&mut tester.app(), Some("bar".to_string()));
        tester.pump();
        tester.pump();
        assert_eq!(tester.count_text("bar/error"), 1);

        let data = tester.restoration_data();
        let mut tester = restart_and_restore(data.clone());
        // Error text should be present after restart and restore.
        assert_eq!(tester.count_text("bar/error"), 1);

        // Resetting the form state should remove the error text.
        let field = key_state::<RestorableTextFieldState>(&tester, &field_key);
        field.reset(&mut tester.app());
        tester.pump();
        tester.pump();
        assert_eq!(tester.count_text("bar/error"), 0);
        let tester = restart_and_restore(tester.restoration_data());
        // Error text should still be removed after restart and restore.
        assert_eq!(tester.count_text("bar/error"), 0);

        let tester = restart_and_restore(data);
        assert_eq!(tester.count_text("bar/error"), 1);
    }

    fn key_state<S: State>(tester: &WidgetTester, key: &GlobalKey) -> Handle<S> {
        key.current_state::<S>(&tester.app())
            .expect("the field is mounted")
    }

    #[test]
    fn a_form_with_can_pop_wraps_its_fields_in_a_pop_scope() {
        let mut tester = WidgetTester::new();
        tester.pump_widget(Form::new(SizedBox::shrink()).can_pop(false).into_widget());
        let pop_scopes: Vec<bool> = {
            let elements = tester.elements();
            let app = tester.app();
            elements
                .into_iter()
                .filter_map(|element| {
                    downcast_widget::<PopScope>(&**element.widget(&app)).map(|scope| scope.can_pop)
                })
                .collect()
        };
        assert_eq!(pop_scopes, [false]);

        tester.pump_widget(Form::new(SizedBox::shrink()).into_widget());
        let elements = tester.elements();
        let app = tester.app();
        assert!(
            elements
                .into_iter()
                .all(|element| downcast_widget::<PopScope>(&**element.widget(&app)).is_none())
        );
    }
}
