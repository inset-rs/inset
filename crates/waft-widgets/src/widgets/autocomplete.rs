//! Flutter counterpart: `widgets/autocomplete.dart`.
//!
//! [`RawAutocompleteState`] keeps the options of the last [`RawAutocomplete::options_builder`]
//! call and shows them through an [`OverlayPortal`] laid out against the field. The builder's
//! result arrives in a continuation that runs at the next checkpoint, as Dart's `await` runs
//! it in a microtask; a continuation whose call id is no longer the latest is dropped, so a
//! slow older call never replaces the options of a newer one.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::rc::Rc;

use waft_embedder::{Offset, TargetPlatform, TextAffinity, TextSelection};
use waft_foundation::{
    App, Handle, ListenableObject, Listener, Ref, Task, ValueNotifier, VoidCallback,
};
use waft_painting::{AlignmentGeometry, transform_rect};
use waft_rendering::BoxConstraints;
use waft_services::{LogicalKeyboardKey, TextEditingValue};

use crate::framework::{
    BuildContext, GlobalKey, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    WidgetRef,
};
use crate::widgets::actions::{
    Action, ActionData, ActionRef, Actions, CallbackAction, DismissIntent, Intent, IntentRef,
    OnInvokeCallback,
};
use crate::widgets::basic::{Align, Builder, ConstrainedBox, SizedBox, Transform};
use crate::widgets::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::widgets::editable_text::{TextEditingController, TextEditingControllerRef};
use crate::widgets::focus_manager::{FocusNode, FocusNodeLeaf, FocusNodeRef};
use crate::widgets::focus_scope::ExcludeFocus;
use crate::widgets::inherited_notifier::{InheritedNotifier, InheritedNotifierKind};
use crate::widgets::media_query::MediaQuery;
use crate::widgets::overlay::{OverlayChildLayoutInfo, OverlayPortal, OverlayPortalController};
use crate::widgets::shortcuts::{ShortcutActivatorRef, ShortcutMap, Shortcuts, SingleActivator};
use crate::widgets::tap_region::TextFieldTapRegion;

/// The type of the [`RawAutocomplete`] callback which computes the list of optional
/// completions for the widget's field, based on the text the user has entered so far.
///
/// Dart's `FutureOr<Iterable<T>>`: a builder that has its options at hand returns
/// [`Task::ready`].
///
/// See also:
///
///   * [`RawAutocomplete::options_builder`], which is of this type.
pub type AutocompleteOptionsBuilder<T> = Rc<dyn Fn(&mut App, TextEditingValue) -> Task<Vec<T>>>;

/// The type of the callback used by the [`RawAutocomplete`] widget to indicate that the user
/// has selected an option.
///
/// See also:
///
///   * [`RawAutocomplete::on_selected`], which is of this type.
pub type AutocompleteOnSelected<T> = Rc<dyn Fn(&mut App, T)>;

/// The type of the [`RawAutocomplete`] callback which returns a widget that displays the
/// specified `options` and calls `on_selected` if the user selects an option.
///
/// The returned widget from this callback will be wrapped in an
/// [`AutocompleteHighlightedOption`] inherited widget. This will allow this callback to
/// determine which option is currently highlighted for keyboard navigation.
///
/// See also:
///
///   * [`RawAutocomplete::options_view_builder`], which is of this type.
pub type AutocompleteOptionsViewBuilder<T> =
    Rc<dyn Fn(&mut App, BuildContext, AutocompleteOnSelected<T>, &[T]) -> WidgetRef>;

/// The type of the Autocomplete callback which returns the widget that contains the input
/// `TextField` or `TextFormField`.
///
/// See also:
///
///   * [`RawAutocomplete::field_view_builder`], which is of this type.
pub type AutocompleteFieldViewBuilder = Rc<
    dyn Fn(
        &mut App,
        BuildContext,
        TextEditingControllerRef,
        FocusNodeRef,
        VoidCallback,
    ) -> WidgetRef,
>;

/// The type of the [`RawAutocomplete`] callback that converts an option value to a string
/// which can be displayed in the widget's options menu.
///
/// See also:
///
///   * [`RawAutocomplete::display_string_for_option`], which is of this type.
pub type AutocompleteOptionToString<T> = Rc<dyn Fn(&T) -> String>;

/// A direction in which to open the options-view overlay.
///
/// See also:
///
///  * [`RawAutocomplete::options_view_open_direction`], which is of this type.
///  * [`RawAutocomplete::options_view_builder`] to specify how to build the selectable-options
///    widget.
///  * [`RawAutocomplete::field_view_builder`] to optionally specify how to build the
///    corresponding field widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsViewOpenDirection {
    /// Open upward.
    ///
    /// The bottom edge of the options view will align with the top edge of the text field
    /// built by [`RawAutocomplete::field_view_builder`].
    Up,

    /// Open downward.
    ///
    /// The top edge of the options view will align with the bottom edge of the text field
    /// built by [`RawAutocomplete::field_view_builder`].
    Down,

    /// Open in the direction with the most available space within the overlay.
    ///
    /// The available space is calculated as the distance from the field's top edge to the
    /// overlay's top edge (for upward opening) or from the field's bottom edge to the
    /// overlay's bottom edge (for downward opening).
    ///
    /// If both directions have the same available space, the options view opens downward.
    MostSpace,
}

/// A widget for helping the user make a selection by entering some text and choosing from
/// among a list of options.
///
/// The user's text input is received in a field built with the
/// [`field_view_builder`](Self::field_view_builder) parameter. The options to be displayed are
/// determined using [`options_builder`](Self::options_builder) and rendered with
/// [`options_view_builder`](Self::options_view_builder).
///
/// The options view opens when the field gains focus or when the field's text changes, as
/// long as [`options_builder`](Self::options_builder) returns at least one option. The options
/// view closes when the user selects an option, when there are no matching options, or when
/// the field loses focus.
///
/// This is a core framework widget with very basic UI.
///
/// The type parameter `T` represents the type of the options. Most commonly this is a
/// `String`. However, it's also possible to use another type with a `Display` form, or a
/// custom [`display_string_for_option`](Self::display_string_for_option). Options will be
/// compared using `==`.
///
/// See also:
///
///  * `Autocomplete`, which is a Material-styled implementation that is based on
///    RawAutocomplete.
pub struct RawAutocomplete<T: Clone + PartialEq + 'static> {
    /// See [`StatefulWidget::key`].
    pub key: Option<KeyRef>,

    /// Builds the field whose input is used to get the options.
    ///
    /// Pass the provided [`TextEditingControllerRef`] to the field built here so that
    /// RawAutocomplete can listen for changes.
    ///
    /// If this parameter is `None`, then a [`SizedBox::shrink`] is built instead. For how that
    /// pattern can be useful, see [`text_editing_controller`](Self::text_editing_controller).
    pub field_view_builder: Option<AutocompleteFieldViewBuilder>,

    /// The [`FocusNode`] that is used for the text field.
    ///
    /// The main purpose of this parameter is to allow the use of a separate text field
    /// located in another part of the widget tree instead of the text field built by
    /// [`field_view_builder`](Self::field_view_builder). For example, it may be desirable to
    /// place the text field in the AppBar and the options below in the main body.
    ///
    /// When following this pattern, [`field_view_builder`](Self::field_view_builder) can be
    /// omitted, so that a text field is not drawn where it would normally be. A separate text
    /// field can be created elsewhere, and a FocusNode and TextEditingController can be passed
    /// both to that text field and to RawAutocomplete.
    ///
    /// If this parameter is not `None`, then
    /// [`text_editing_controller`](Self::text_editing_controller) must also be set.
    pub focus_node: Option<FocusNodeRef>,

    /// Builds the selectable options widgets from a list of options objects.
    ///
    /// The options are displayed floating below or above the field inside of an `Overlay`,
    /// not at the same place in the widget tree as [`RawAutocomplete`]. To control whether it
    /// opens upward or downward, use
    /// [`options_view_open_direction`](Self::options_view_open_direction).
    ///
    /// In order to track which item is highlighted by keyboard navigation, the resulting
    /// options will be wrapped in an inherited [`AutocompleteHighlightedOption`] widget.
    /// Inside this callback, the index of the highlighted option can be obtained from
    /// [`AutocompleteHighlightedOption::of`] to display the highlighted option with a visual
    /// highlight to indicate it will be the option selected from the keyboard.
    pub options_view_builder: AutocompleteOptionsViewBuilder<T>,

    /// Determines the direction in which to open the options view.
    ///
    /// Defaults to [`OptionsViewOpenDirection::Down`].
    pub options_view_open_direction: OptionsViewOpenDirection,

    /// Returns the string to display in the field when the option is selected.
    ///
    /// This is useful when using a custom T type and the string to display is different
    /// than the string to search by.
    ///
    /// If not provided, will use the option's `Display` form.
    pub display_string_for_option: AutocompleteOptionToString<T>,

    /// Called when an option is selected by the user.
    pub on_selected: Option<AutocompleteOnSelected<T>>,

    /// A function that returns the current selectable options objects given the current
    /// TextEditingValue.
    pub options_builder: AutocompleteOptionsBuilder<T>,

    /// The [`TextEditingController`] that is used for the text field.
    ///
    /// See [`focus_node`](Self::focus_node) for the split field pattern.
    ///
    /// If this parameter is not `None`, then [`focus_node`](Self::focus_node) must also be
    /// set.
    pub text_editing_controller: Option<TextEditingControllerRef>,

    /// The initial value to use for the text field.
    ///
    /// Setting the initial value does not notify
    /// [`text_editing_controller`](Self::text_editing_controller)'s listeners, and thus will
    /// not cause the options UI to appear.
    ///
    /// This parameter is ignored if [`text_editing_controller`](Self::text_editing_controller)
    /// is defined.
    pub initial_value: Option<TextEditingValue>,
}

impl<T: Clone + PartialEq + Display + 'static> RawAutocomplete<T> {
    /// Create an instance of RawAutocomplete.
    ///
    /// [`display_string_for_option`](Self::display_string_for_option) defaults to
    /// [`default_string_for_option`](Self::default_string_for_option), which needs `T`'s
    /// `Display` form.
    pub fn new(
        options_view_builder: impl Fn(
            &mut App,
            BuildContext,
            AutocompleteOnSelected<T>,
            &[T],
        ) -> WidgetRef
        + 'static,
        options_builder: impl Fn(&mut App, TextEditingValue) -> Task<Vec<T>> + 'static,
    ) -> RawAutocomplete<T> {
        RawAutocomplete {
            key: None,
            field_view_builder: None,
            focus_node: None,
            options_view_builder: Rc::new(options_view_builder),
            options_view_open_direction: OptionsViewOpenDirection::Down,
            display_string_for_option: Rc::new(|option: &T| {
                RawAutocomplete::<T>::default_string_for_option(option)
            }),
            on_selected: None,
            options_builder: Rc::new(options_builder),
            text_editing_controller: None,
            initial_value: None,
        }
    }

    /// The default way to convert an option to a string in
    /// [`display_string_for_option`](Self::display_string_for_option).
    ///
    /// Uses the `Display` form of the given `option`.
    pub fn default_string_for_option(option: &dyn Display) -> String {
        option.to_string()
    }
}

impl<T: Clone + PartialEq + 'static> RawAutocomplete<T> {
    /// Dart `RawAutocomplete(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawAutocomplete<T> {
        self.key = Some(key);
        self
    }

    /// Dart `RawAutocomplete(optionsViewOpenDirection:)`.
    pub fn options_view_open_direction(
        mut self,
        options_view_open_direction: OptionsViewOpenDirection,
    ) -> RawAutocomplete<T> {
        self.options_view_open_direction = options_view_open_direction;
        self
    }

    /// Dart `RawAutocomplete(displayStringForOption:)`.
    pub fn display_string_for_option(
        mut self,
        display_string_for_option: impl Fn(&T) -> String + 'static,
    ) -> RawAutocomplete<T> {
        self.display_string_for_option = Rc::new(display_string_for_option);
        self
    }

    /// Dart `RawAutocomplete(fieldViewBuilder:)`.
    pub fn field_view_builder(
        mut self,
        field_view_builder: impl Fn(
            &mut App,
            BuildContext,
            TextEditingControllerRef,
            FocusNodeRef,
            VoidCallback,
        ) -> WidgetRef
        + 'static,
    ) -> RawAutocomplete<T> {
        self.field_view_builder = Some(Rc::new(field_view_builder));
        self
    }

    /// Dart `RawAutocomplete(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> RawAutocomplete<T> {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `RawAutocomplete(onSelected:)`.
    pub fn on_selected(
        mut self,
        on_selected: impl Fn(&mut App, T) + 'static,
    ) -> RawAutocomplete<T> {
        self.on_selected = Some(Rc::new(on_selected));
        self
    }

    /// Dart `RawAutocomplete(textEditingController:)`.
    pub fn text_editing_controller(
        mut self,
        text_editing_controller: TextEditingControllerRef,
    ) -> RawAutocomplete<T> {
        self.text_editing_controller = Some(text_editing_controller);
        self
    }

    /// Dart `RawAutocomplete(initialValue:)`.
    pub fn initial_value(mut self, initial_value: TextEditingValue) -> RawAutocomplete<T> {
        self.initial_value = Some(initial_value);
        self
    }

    /// Calls [`AutocompleteFieldViewBuilder`]'s `on_field_submitted` callback for the
    /// RawAutocomplete widget indicated by the given [`GlobalKey`].
    ///
    /// This is not typically used unless a custom field is implemented instead of using
    /// [`field_view_builder`](Self::field_view_builder). In the typical case, the
    /// `on_field_submitted` callback is passed via the [`AutocompleteFieldViewBuilder`]
    /// signature. When not using `field_view_builder`, the same callback can be called by
    /// using this associated function.
    ///
    /// See also:
    ///
    ///  * [`focus_node`](Self::focus_node) and
    ///    [`text_editing_controller`](Self::text_editing_controller), which describe how to
    ///    create a separate field outside of `field_view_builder`.
    pub fn on_field_submitted(app: &mut App, key: &GlobalKey) {
        let raw_autocomplete = key
            .current_state::<RawAutocompleteState<T>>(app)
            .expect("the key's widget is a mounted RawAutocomplete of this option type");
        raw_autocomplete.on_field_submitted(app);
    }
}

impl<T: Clone + PartialEq + 'static> Debug for RawAutocomplete<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawAutocomplete")
            .field(
                "options_view_open_direction",
                &self.options_view_open_direction,
            )
            .field("focus_node", &self.focus_node)
            .field("text_editing_controller", &self.text_editing_controller)
            .field("initial_value", &self.initial_value)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for RawAutocomplete<T> {
    type State = RawAutocompleteState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.field_view_builder.is_some()
                || (self.key.is_some()
                    && self.focus_node.is_some()
                    && self.text_editing_controller.is_some()),
            "Pass in a fieldViewBuilder, or otherwise create a separate field and pass in the \
             FocusNode, TextEditingController, and a key. Use the key with \
             RawAutocomplete.onFieldSubmitted."
        );
        debug_assert!(self.focus_node.is_none() == self.text_editing_controller.is_none());
        debug_assert!(
            !(self.text_editing_controller.is_some() && self.initial_value.is_some()),
            "textEditingController and initialValue cannot be simultaneously defined."
        );
    }

    fn create_state(&self) -> RawAutocompleteState<T> {
        RawAutocompleteState {
            state: StateData::new(),
            options_view_controller: None,
            has_focus: false,
            selecting: false,
            internal_text_editing_controller: None,
            internal_focus_node: None,
            action_map: None,
            options: Vec::new(),
            selection: None,
            last_field_text: None,
            highlighted_option_index: None,
            on_changed_call_id: 0,
        }
    }
}

/// Dart's `_RawAutocompleteState`; public because [`RawAutocomplete`]'s
/// `StatefulWidget::State` names it.
pub struct RawAutocompleteState<T: Clone + PartialEq + 'static> {
    state: StateData<RawAutocomplete<T>>,
    options_view_controller: Option<Ref<OverlayPortalController>>,

    /// Whether the field currently has focus.
    ///
    /// This is used to determine whether the focus state has changed.
    has_focus: bool,

    /// Whether an option is currently being selected.
    selecting: bool,

    internal_text_editing_controller: Option<Ref<TextEditingController>>,
    internal_focus_node: Option<Ref<FocusNode>>,
    action_map: Option<HashMap<TypeId, ActionRef>>,
    options: Vec<T>,
    selection: Option<T>,
    // Set the initial value to None so when this widget gets focused for the first time it
    // will try to run the options view builder.
    last_field_text: Option<String>,
    highlighted_option_index: Option<Ref<ValueNotifier<usize>>>,
    // Assigning an ID to every call of on_changed_field is necessary to avoid a situation
    // where options is updated by an older call when multiple on_changed_field calls are
    // running simultaneously.
    on_changed_call_id: u64,
}

impl<T: Clone + PartialEq + 'static> RawAutocompleteState<T> {
    // The number of options to scroll by "page", such as when using the page up/down keys.
    const PAGE_SIZE: i64 = 4;

    // A big enough height for about one item in the default Autocomplete.optionsViewBuilder.
    // The assumption is that the user likely wants the list of options to move to stay on the
    // screen rather than get any smaller than this. Allows Autocomplete to work when it has
    // very little screen height available (as in b/317115348) by positioning itself on top of
    // the field, while in other cases to size itself based on the height under the field.
    const MIN_USABLE_HEIGHT: f64 = K_MIN_INTERACTIVE_DIMENSION;

    fn options_view_controller(self: Handle<Self>, app: &App) -> Handle<OverlayPortalController> {
        app.get(self)
            .options_view_controller
            .as_ref()
            .expect("made in init_state")
            .handle()
    }

    fn highlighted_option_index(self: Handle<Self>, app: &App) -> Handle<ValueNotifier<usize>> {
        app.get(self)
            .highlighted_option_index
            .as_ref()
            .expect("made in init_state")
            .handle()
    }

    fn text_editing_controller(self: Handle<Self>, app: &mut App) -> TextEditingControllerRef {
        if let Some(controller) = &self.widget(app).text_editing_controller {
            return controller.clone();
        }
        if app.get(self).internal_text_editing_controller.is_none() {
            let controller = TextEditingController::new(app);
            ListenableObject::add_listener(
                controller.handle(),
                app,
                Listener::handle_method(self, Self::on_changed_field),
            );
            app.get_mut(self).internal_text_editing_controller = Some(controller);
        }
        app.get(self)
            .internal_text_editing_controller
            .as_ref()
            .expect("made above")
            .erase()
    }

    fn focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = &self.widget(app).focus_node {
            return focus_node.clone();
        }
        if app.get(self).internal_focus_node.is_none() {
            let focus_node = FocusNode::new(app);
            let erased: FocusNodeRef = focus_node.erase();
            erased
                .handle()
                .add_listener(app, Listener::handle_method(self, Self::on_focus_change));
            app.get_mut(self).internal_focus_node = Some(focus_node);
        }
        app.get(self)
            .internal_focus_node
            .as_ref()
            .expect("made above")
            .erase()
    }

    /// Dart's `late final Map<Type, CallbackAction<Intent>> _actionMap`, made on first read.
    fn action_map(self: Handle<Self>, app: &mut App) -> HashMap<TypeId, ActionRef> {
        if app.get(self).action_map.is_none() {
            let action_map = self.make_action_map(app);
            app.get_mut(self).action_map = Some(action_map);
        }
        app.get(self).action_map.clone().expect("made above")
    }

    fn make_action_map(self: Handle<Self>, app: &mut App) -> HashMap<TypeId, ActionRef> {
        let is_enabled_callback: Rc<dyn Fn(&mut App) -> bool> =
            Rc::new(move |app: &mut App| self.can_show_options_view(app));
        let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
        actions.insert(
            TypeId::of::<AutocompletePreviousOptionIntent>(),
            AutocompleteCallbackAction::<AutocompletePreviousOptionIntent>::new(
                app,
                Rc::new(
                    move |app: &mut App, intent: &AutocompletePreviousOptionIntent| {
                        self.highlight_previous_option(app, intent);
                        None
                    },
                ),
                Rc::clone(&is_enabled_callback),
            )
            .into_erased(),
        );
        actions.insert(
            TypeId::of::<AutocompleteNextOptionIntent>(),
            AutocompleteCallbackAction::<AutocompleteNextOptionIntent>::new(
                app,
                Rc::new(
                    move |app: &mut App, intent: &AutocompleteNextOptionIntent| {
                        self.highlight_next_option(app, intent);
                        None
                    },
                ),
                Rc::clone(&is_enabled_callback),
            )
            .into_erased(),
        );
        actions.insert(
            TypeId::of::<AutocompleteFirstOptionIntent>(),
            AutocompleteCallbackAction::<AutocompleteFirstOptionIntent>::new(
                app,
                Rc::new(
                    move |app: &mut App, intent: &AutocompleteFirstOptionIntent| {
                        self.highlight_first_option(app, intent);
                        None
                    },
                ),
                Rc::clone(&is_enabled_callback),
            )
            .into_erased(),
        );
        actions.insert(
            TypeId::of::<AutocompleteLastOptionIntent>(),
            AutocompleteCallbackAction::<AutocompleteLastOptionIntent>::new(
                app,
                Rc::new(
                    move |app: &mut App, intent: &AutocompleteLastOptionIntent| {
                        self.highlight_last_option(app, intent);
                        None
                    },
                ),
                Rc::clone(&is_enabled_callback),
            )
            .into_erased(),
        );
        actions.insert(
            TypeId::of::<AutocompleteNextPageOptionIntent>(),
            AutocompleteCallbackAction::<AutocompleteNextPageOptionIntent>::new(
                app,
                Rc::new(
                    move |app: &mut App, intent: &AutocompleteNextPageOptionIntent| {
                        self.highlight_next_page_option(app, intent);
                        None
                    },
                ),
                Rc::clone(&is_enabled_callback),
            )
            .into_erased(),
        );
        actions.insert(
            TypeId::of::<AutocompletePreviousPageOptionIntent>(),
            AutocompleteCallbackAction::<AutocompletePreviousPageOptionIntent>::new(
                app,
                Rc::new(
                    move |app: &mut App, intent: &AutocompletePreviousPageOptionIntent| {
                        self.highlight_previous_page_option(app, intent);
                        None
                    },
                ),
                is_enabled_callback,
            )
            .into_erased(),
        );
        actions.insert(
            TypeId::of::<DismissIntent>(),
            CallbackAction::<DismissIntent>::new(
                app,
                Rc::new(move |app: &mut App, intent: &DismissIntent| {
                    self.hide_options(app, intent)
                }),
            )
            .into_erased(),
        );
        actions
    }

    /// Dart's `_appleShortcuts`.
    fn apple_shortcuts() -> ShortcutMap {
        vec![
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::ARROW_UP).meta(true),
                AutocompleteFirstOptionIntent::new(),
            ),
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN).meta(true),
                AutocompleteLastOptionIntent::new(),
            ),
        ]
    }

    /// Dart's `_nonAppleShortcuts`.
    fn non_apple_shortcuts() -> ShortcutMap {
        vec![
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::ARROW_UP).control(true),
                AutocompleteFirstOptionIntent::new(),
            ),
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN).control(true),
                AutocompleteLastOptionIntent::new(),
            ),
        ]
    }

    /// Dart's `_commonShortcuts`.
    fn common_shortcuts() -> ShortcutMap {
        vec![
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::ARROW_UP),
                AutocompletePreviousOptionIntent::new(),
            ),
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN),
                AutocompleteNextOptionIntent::new(),
            ),
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::PAGE_UP),
                AutocompletePreviousPageOptionIntent::new(),
            ),
            shortcut(
                SingleActivator::new(LogicalKeyboardKey::PAGE_DOWN),
                AutocompleteNextPageOptionIntent::new(),
            ),
        ]
    }

    /// Dart's `_shortcuts`.
    fn shortcuts(app: &App) -> ShortcutMap {
        let mut shortcuts = Self::common_shortcuts();
        shortcuts.extend(match app.platform().target_platform() {
            TargetPlatform::IOS => Self::apple_shortcuts(),
            TargetPlatform::MacOS => Self::apple_shortcuts(),
            TargetPlatform::Android => Self::non_apple_shortcuts(),
            TargetPlatform::Linux => Self::non_apple_shortcuts(),
            TargetPlatform::Windows => Self::non_apple_shortcuts(),
            TargetPlatform::Fuchsia => Self::non_apple_shortcuts(),
        });
        shortcuts
    }

    /// The options view is considered eligible to show only while the field has focus and
    /// there is at least one option to display.
    fn can_show_options_view(self: Handle<Self>, app: &mut App) -> bool {
        self.focus_node(app).handle().has_focus(app) && !app.get(self).options.is_empty()
    }

    fn on_focus_change(self: Handle<Self>, app: &mut App) {
        let has_focus = self.focus_node(app).handle().has_focus(app);
        if has_focus != app.get(self).has_focus {
            app.get_mut(self).has_focus = has_focus;
            // Gaining focus can open the options view (if there are options). Losing focus
            // always closes it.
            self.update_options_view_visibility(app);
        }
    }

    /// Shows the options view when the field is focused and there is at least one option to
    /// display; otherwise hides the options view.
    fn update_options_view_visibility(self: Handle<Self>, app: &mut App) {
        if self.can_show_options_view(app) {
            self.options_view_controller(app).show(app);
        } else if self.options_view_controller(app).is_showing(app) {
            self.options_view_controller(app).hide(app);
        }
    }

    // Called when text_editing_controller changes. Dart's `Future<void>` is only ever dropped
    // by the listener list, so the continuation's task is dropped here.
    fn on_changed_field(self: Handle<Self>, app: &mut App) {
        // During a selection, changes to the field text should not trigger options update.
        if app.get(self).selecting {
            return;
        }
        let value = self.text_editing_controller(app).value(app).clone();

        // Makes sure that options change only when content of the field changes.
        let mut should_update_options = false;
        if app.get(self).last_field_text.as_deref() != Some(value.text.as_str()) {
            should_update_options = true;
            app.get_mut(self).on_changed_call_id += 1;
        }
        app.get_mut(self).last_field_text = Some(value.text.clone());
        let call_id = app.get(self).on_changed_call_id;
        let options_builder = Rc::clone(&self.widget(app).options_builder);
        let options = options_builder(app, value.clone());

        drop(app.spawn(async move |cx| {
            let options = options.await;
            cx.update(|app| {
                if !self.mounted(app) {
                    return;
                }

                // Makes sure that previous call results do not replace new ones.
                if call_id != app.get(self).on_changed_call_id || !should_update_options {
                    return;
                }
                app.get_mut(self).options = options;
                let highlighted = *self.highlighted_option_index(app).value(app) as i64;
                self.update_highlight(app, highlighted);
                if let Some(selection) = app.get(self).selection.clone()
                    && value.text != (self.widget(app).display_string_for_option)(&selection)
                {
                    app.get_mut(self).selection = None;
                }

                self.update_options_view_visibility(app);
            });
        }));
    }

    // Called from field_view_builder when the user submits the field.
    fn on_field_submitted(self: Handle<Self>, app: &mut App) {
        if self.options_view_controller(app).is_showing(app) {
            let index = *self.highlighted_option_index(app).value(app);
            let option = app.get(self).options[index].clone();
            self.select(app, option);
        }
    }

    // Select the given option and update the widget.
    fn select(self: Handle<Self>, app: &mut App, next_selection: T) {
        if Some(&next_selection) == app.get(self).selection.as_ref() {
            return;
        }
        app.get_mut(self).selecting = true;
        app.get_mut(self).selection = Some(next_selection.clone());
        let selection_string = (self.widget(app).display_string_for_option)(&next_selection);
        let selection_length = selection_string.encode_utf16().count() as i32;
        self.text_editing_controller(app).handle().set_value(
            app,
            TextEditingValue::new()
                .selection(TextSelection::collapsed(
                    selection_length,
                    TextAffinity::Downstream,
                ))
                .text(selection_string.clone()),
        );
        app.get_mut(self).last_field_text = Some(selection_string);
        if let Some(on_selected) = self.widget(app).on_selected.clone() {
            on_selected(app, next_selection);
        }
        if self.options_view_controller(app).is_showing(app) {
            // Close the options view after a selection is made.
            self.options_view_controller(app).hide(app);
        }
        app.get_mut(self).selecting = false;
    }

    fn update_highlight(self: Handle<Self>, app: &mut App, next_index: i64) {
        let length = app.get(self).options.len();
        let index = if length == 0 {
            0
        } else {
            next_index.clamp(0, length as i64 - 1) as usize
        };
        self.highlighted_option_index(app).set_value(app, index);
    }

    fn highlight_previous_option(
        self: Handle<Self>,
        app: &mut App,
        _intent: &AutocompletePreviousOptionIntent,
    ) {
        let index = *self.highlighted_option_index(app).value(app) as i64;
        self.highlight_option(app, index - 1);
    }

    fn highlight_next_option(
        self: Handle<Self>,
        app: &mut App,
        _intent: &AutocompleteNextOptionIntent,
    ) {
        let index = *self.highlighted_option_index(app).value(app) as i64;
        self.highlight_option(app, index + 1);
    }

    fn highlight_first_option(
        self: Handle<Self>,
        app: &mut App,
        _intent: &AutocompleteFirstOptionIntent,
    ) {
        self.highlight_option(app, 0);
    }

    fn highlight_last_option(
        self: Handle<Self>,
        app: &mut App,
        _intent: &AutocompleteLastOptionIntent,
    ) {
        let length = app.get(self).options.len() as i64;
        self.highlight_option(app, length - 1);
    }

    fn highlight_next_page_option(
        self: Handle<Self>,
        app: &mut App,
        _intent: &AutocompleteNextPageOptionIntent,
    ) {
        let index = *self.highlighted_option_index(app).value(app) as i64;
        self.highlight_option(app, index + Self::PAGE_SIZE);
    }

    fn highlight_previous_page_option(
        self: Handle<Self>,
        app: &mut App,
        _intent: &AutocompletePreviousPageOptionIntent,
    ) {
        let index = *self.highlighted_option_index(app).value(app) as i64;
        self.highlight_option(app, index - Self::PAGE_SIZE);
    }

    fn highlight_option(self: Handle<Self>, app: &mut App, index: i64) {
        debug_assert!(self.can_show_options_view(app));
        self.update_options_view_visibility(app);
        debug_assert!(self.options_view_controller(app).is_showing(app));
        self.update_highlight(app, index);
    }

    fn hide_options(
        self: Handle<Self>,
        app: &mut App,
        intent: &DismissIntent,
    ) -> Option<Rc<dyn Any>> {
        if self.options_view_controller(app).is_showing(app) {
            self.options_view_controller(app).hide(app);
            None
        } else {
            let context = self.context(app);
            Actions::invoke(app, context, intent)
        }
    }

    fn build_options_view(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        layout_info: OverlayChildLayoutInfo,
    ) -> WidgetRef {
        if layout_info.child_paint_transform().determinant() == 0.0 {
            // The child is not visible.
            return SizedBox::shrink().into_widget();
        }
        let field_size = layout_info.child_size();
        // Dart's `invert()` leaves a matrix it cannot invert as it is.
        let invert_transform = layout_info
            .child_paint_transform()
            .invert()
            .unwrap_or(layout_info.child_paint_transform());

        let media_query_padding = MediaQuery::padding_of(app, context);
        let view_insets = MediaQuery::view_insets_of(app, context);

        let overlay_rect = media_query_padding
            .deflate_rect(view_insets.deflate_rect(Offset::ZERO & layout_info.overlay_size()));

        // This may not work well if the paint transform has rotation in it.
        // transform_rect returns the bounding rect of the rotated overlay rect.
        let overlay_rect_in_field = transform_rect(&invert_transform, overlay_rect);

        let space_above = -overlay_rect_in_field.top;
        let space_below = overlay_rect_in_field.bottom - field_size.height();
        let opens_up = match self.widget(app).options_view_open_direction {
            OptionsViewOpenDirection::Up => true,
            OptionsViewOpenDirection::Down => false,
            OptionsViewOpenDirection::MostSpace => space_above > space_below,
        };

        let options_view_max_height = if opens_up {
            -overlay_rect_in_field.top
        } else {
            overlay_rect_in_field.bottom - field_size.height()
        };

        let options_view_bounding_box = waft_embedder::Size::new(
            field_size.width(),
            options_view_max_height.max(Self::MIN_USABLE_HEIGHT),
        );

        let origin_y = if opens_up {
            overlay_rect_in_field.top
        } else {
            overlay_rect_in_field.bottom - options_view_bounding_box.height()
        };

        let transform = layout_info
            .child_paint_transform()
            .then(&waft_embedder::Matrix4::translation(0.0, origin_y as f32));
        let child = Builder::new(move |app, context| {
            let options_view_builder = Rc::clone(&self.widget(app).options_view_builder);
            let options = app.get(self).options.clone();
            let on_selected: AutocompleteOnSelected<T> =
                Rc::new(move |app: &mut App, option: T| self.select(app, option));
            options_view_builder(app, context, on_selected, &options)
        });
        let highlighted_option_index = app
            .get(self)
            .highlighted_option_index
            .clone()
            .expect("made in init_state");
        Transform::new(transform)
            .child(
                Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                    ConstrainedBox::new(BoxConstraints::tight(options_view_bounding_box)).child(
                        Align::new()
                            .alignment(if opens_up {
                                AlignmentGeometry::BOTTOM_START
                            } else {
                                AlignmentGeometry::TOP_START
                            })
                            .child(TextFieldTapRegion::new(
                                AutocompleteHighlightedOption::new(
                                    highlighted_option_index,
                                    // Exclude the options overlay from the ambient focus
                                    // traversal tree. Autocomplete options are navigated by
                                    // arrow keys (via the widget's own shortcuts) and selected
                                    // via Enter or tap, so they don't participate in TAB
                                    // traversal. Without this, TAB from the field would detour
                                    // into focusable items in the options overlay instead of
                                    // advancing to the next form field.
                                    ExcludeFocus::new(child),
                                )
                                .into_widget(),
                            )),
                    ),
                ),
            )
            .into_widget()
    }
}

impl<T: Clone + PartialEq + 'static> State for RawAutocompleteState<T> {
    type Widget = RawAutocomplete<T>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).options_view_controller = Some(OverlayPortalController::new(
            app,
            Some("_RawAutocompleteState".to_string()),
        ));
        app.get_mut(self).highlighted_option_index = Some(app.create(ValueNotifier::new(0)));
        let initial_controller = match self.widget(app).text_editing_controller.clone() {
            Some(controller) => controller,
            None => {
                let initial_value = self.widget(app).initial_value.clone();
                let controller = TextEditingController::from_value(app, initial_value);
                let erased = controller.erase();
                app.get_mut(self).internal_text_editing_controller = Some(controller);
                erased
            }
        };
        initial_controller
            .handle()
            .add_listener(app, Listener::handle_method(self, Self::on_changed_field));
        let has_focus = self.focus_node(app).handle().has_focus(app);
        app.get_mut(self).has_focus = has_focus;
        if let Some(focus_node) = self.widget(app).focus_node.clone() {
            focus_node
                .handle()
                .add_listener(app, Listener::handle_method(self, Self::on_focus_change));
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RawAutocomplete<T>) {
        if old_widget.text_editing_controller != self.widget(app).text_editing_controller {
            if let Some(controller) = &old_widget.text_editing_controller {
                controller
                    .handle()
                    .remove_listener(app, &Listener::handle_method(self, Self::on_changed_field));
            }
            if old_widget.text_editing_controller.is_none()
                && let Some(controller) = app.get_mut(self).internal_text_editing_controller.take()
            {
                controller.dispose(app);
            }
            if let Some(controller) = self.widget(app).text_editing_controller.clone() {
                controller
                    .handle()
                    .add_listener(app, Listener::handle_method(self, Self::on_changed_field));
            }
        }
        if old_widget.focus_node != self.widget(app).focus_node {
            if let Some(focus_node) = &old_widget.focus_node {
                focus_node.handle().remove_listener(
                    app,
                    &Listener::handle_method(self, Self::update_options_view_visibility),
                );
            }
            if old_widget.focus_node.is_none()
                && let Some(focus_node) = app.get_mut(self).internal_focus_node.take()
            {
                focus_node.dispose(app);
            }
            if let Some(focus_node) = self.widget(app).focus_node.clone() {
                focus_node.handle().add_listener(
                    app,
                    Listener::handle_method(self, Self::update_options_view_visibility),
                );
            }
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = self.widget(app).text_editing_controller.clone() {
            controller
                .handle()
                .remove_listener(app, &Listener::handle_method(self, Self::on_changed_field));
        }
        if let Some(controller) = app.get(self).internal_text_editing_controller.clone() {
            controller.dispose(app);
        }
        if let Some(focus_node) = self.widget(app).focus_node.clone() {
            focus_node.handle().remove_listener(
                app,
                &Listener::handle_method(self, Self::update_options_view_visibility),
            );
        }
        if let Some(focus_node) = app.get(self).internal_focus_node.clone() {
            focus_node.dispose(app);
        }
        self.highlighted_option_index(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let field_view = match self.widget(app).field_view_builder.clone() {
            Some(field_view_builder) => {
                let text_editing_controller = self.text_editing_controller(app);
                let focus_node = self.focus_node(app);
                let on_field_submitted: VoidCallback =
                    Rc::new(move |app: &mut App| self.on_field_submitted(app));
                field_view_builder(
                    app,
                    context,
                    text_editing_controller,
                    focus_node,
                    on_field_submitted,
                )
            }
            // Horizontally expand to make sure the options view's width won't be zero.
            None => SizedBox::new()
                .width(f64::INFINITY)
                .height(0.0)
                .into_widget(),
        };
        let controller = app
            .get(self)
            .options_view_controller
            .clone()
            .expect("made in init_state");
        let action_map = self.action_map(app);
        OverlayPortal::overlay_child_layout_builder(controller, move |app, context, layout_info| {
            self.build_options_view(app, context, layout_info)
        })
        .child(TextFieldTapRegion::new(Shortcuts::new(
            Self::shortcuts(app),
            Actions::new(action_map, field_view),
        )))
        .into_widget()
    }
}

fn shortcut(activator: SingleActivator, intent: impl Intent) -> (ShortcutActivatorRef, IntentRef) {
    (Rc::new(activator), Rc::new(intent))
}

/// Dart's `_AutocompleteCallbackAction`: a [`CallbackAction`] whose enabled state, and so
/// whether it consumes its key, is asked of a callback.
struct AutocompleteCallbackAction<I: Intent> {
    action: ActionData,
    on_invoke: OnInvokeCallback<I>,

    // The enabled state determines whether the action will consume the key shortcut or let
    // it continue on to the underlying text field. They should only be enabled when the
    // options are showing so shortcuts can be used to navigate them.
    is_enabled_callback: Rc<dyn Fn(&mut App) -> bool>,
}

impl<I: Intent> AutocompleteCallbackAction<I> {
    fn new(
        app: &mut App,
        on_invoke: OnInvokeCallback<I>,
        is_enabled_callback: Rc<dyn Fn(&mut App) -> bool>,
    ) -> Ref<AutocompleteCallbackAction<I>> {
        app.create(AutocompleteCallbackAction {
            action: ActionData::new(),
            on_invoke,
            is_enabled_callback,
        })
    }
}

impl<I: Intent> Action for AutocompleteCallbackAction<I> {
    type Intent = I;
    crate::action_accessors!();

    fn is_enabled(self: Handle<Self>, app: &mut App, _intent: &I) -> bool {
        let is_enabled_callback = Rc::clone(&app.get(self).is_enabled_callback);
        is_enabled_callback(app)
    }

    fn consumes_key(self: Handle<Self>, app: &mut App, intent: &I) -> bool {
        self.is_enabled(app, intent)
    }

    fn invoke(self: Handle<Self>, app: &mut App, intent: &I) -> Option<Rc<dyn Any>> {
        let on_invoke = Rc::clone(&app.get(self).on_invoke);
        on_invoke(app, intent)
    }
}

/// An [`Intent`] to highlight the previous option in the autocomplete list.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutocompletePreviousOptionIntent;

impl AutocompletePreviousOptionIntent {
    /// Creates an instance of AutocompletePreviousOptionIntent.
    pub const fn new() -> AutocompletePreviousOptionIntent {
        AutocompletePreviousOptionIntent
    }
}

impl Intent for AutocompletePreviousOptionIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] to highlight the next option in the autocomplete list.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutocompleteNextOptionIntent;

impl AutocompleteNextOptionIntent {
    /// Creates an instance of AutocompleteNextOptionIntent.
    pub const fn new() -> AutocompleteNextOptionIntent {
        AutocompleteNextOptionIntent
    }
}

impl Intent for AutocompleteNextOptionIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] to highlight the first option in the autocomplete list.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutocompleteFirstOptionIntent;

impl AutocompleteFirstOptionIntent {
    /// Creates an instance of AutocompleteFirstOptionIntent.
    pub const fn new() -> AutocompleteFirstOptionIntent {
        AutocompleteFirstOptionIntent
    }
}

impl Intent for AutocompleteFirstOptionIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] to highlight the last option in the autocomplete list.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutocompleteLastOptionIntent;

impl AutocompleteLastOptionIntent {
    /// Creates an instance of AutocompleteLastOptionIntent.
    pub const fn new() -> AutocompleteLastOptionIntent {
        AutocompleteLastOptionIntent
    }
}

impl Intent for AutocompleteLastOptionIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] to highlight the option one page after the currently highlighted option in
/// the autocomplete list.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutocompleteNextPageOptionIntent;

impl AutocompleteNextPageOptionIntent {
    /// Creates an instance of AutocompleteNextPageOptionIntent.
    pub const fn new() -> AutocompleteNextPageOptionIntent {
        AutocompleteNextPageOptionIntent
    }
}

impl Intent for AutocompleteNextPageOptionIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] to highlight the option one page before the currently highlighted option in
/// the autocomplete list.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutocompletePreviousPageOptionIntent;

impl AutocompletePreviousPageOptionIntent {
    /// Creates an instance of AutocompletePreviousPageOptionIntent.
    pub const fn new() -> AutocompletePreviousPageOptionIntent {
        AutocompletePreviousPageOptionIntent
    }
}

impl Intent for AutocompletePreviousPageOptionIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An inherited widget used to indicate which autocomplete option should be highlighted for
/// keyboard navigation.
///
/// The [`RawAutocomplete`] widget will wrap the options view generated by the
/// `options_view_builder` with this widget to provide the highlighted option's index to the
/// builder.
///
/// In the builder callback the index of the highlighted option can be obtained by using the
/// associated function [`of`](Self::of):
///
/// ```text
/// let highlighted_index = AutocompleteHighlightedOption::of(app, context);
/// ```
///
/// which can then be used to tell which option should be given a visual indication that will
/// be the option selected with the keyboard.
///
/// Both [`InheritedWidget`] and [`InheritedNotifier`] offer an `IntoWidget` kind, so this
/// widget names the conversion itself ([`into_widget`](Self::into_widget)).
#[derive(Debug)]
pub struct AutocompleteHighlightedOption {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// Dart's `InheritedNotifier.notifier`: the highlighted option's index.
    pub notifier: Ref<ValueNotifier<usize>>,
    /// See [`InheritedWidget::child`].
    pub child: WidgetRef,
}

impl AutocompleteHighlightedOption {
    /// Create an instance of AutocompleteHighlightedOption inherited widget.
    pub fn new<K>(
        highlight_index_notifier: Ref<ValueNotifier<usize>>,
        child: impl IntoWidget<K>,
    ) -> AutocompleteHighlightedOption {
        AutocompleteHighlightedOption {
            key: None,
            notifier: highlight_index_notifier,
            child: child.into_widget(),
        }
    }

    /// Dart `AutocompleteHighlightedOption(key:)`.
    pub fn key(mut self, key: KeyRef) -> AutocompleteHighlightedOption {
        self.key = Some(key);
        self
    }

    /// The tree node, as an [`InheritedNotifier`].
    pub fn into_widget(self) -> WidgetRef {
        IntoWidget::<InheritedNotifierKind>::into_widget(self)
    }

    /// Returns the index of the highlighted option from the closest
    /// [`AutocompleteHighlightedOption`] ancestor.
    ///
    /// If there is no ancestor, it returns 0.
    pub fn of(app: &mut App, context: BuildContext) -> usize {
        let Some(widget) =
            context.depend_on_inherited_widget_of_exact_type::<AutocompleteHighlightedOption>(app)
        else {
            return 0;
        };
        let notifier = widget.notifier.handle();
        *notifier.value(app)
    }
}

impl InheritedWidget for AutocompleteHighlightedOption {
    crate::inherited_notifier_overrides!();

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }
}

impl InheritedNotifier for AutocompleteHighlightedOption {
    type Notifier = ValueNotifier<usize>;

    fn notifier(&self) -> Option<Handle<ValueNotifier<usize>>> {
        Some(self.notifier.handle())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use waft_embedder::{Color, Offset, Size, TextDirection, TextInputAction, ViewPadding};
    use waft_foundation::{Completer, Timer};
    use waft_painting::{EdgeInsetsGeometry, TextStyle};
    use waft_services::{PhysicalKeyboardKey, TextInputClient};

    use super::*;
    use crate::PageRouteRef;
    use crate::framework::StatelessWidget;
    use crate::framework::{AnyElement, IntoWidget};
    use crate::testing::{
        Finder, WidgetTester, assert_more_or_less_equals, assert_more_or_less_equals_within,
        expect, find, finds_n_widgets, finds_nothing, finds_one_widget, finds_widgets,
        test_widgets,
    };
    use crate::widgets::app::WidgetsApp;
    use crate::widgets::basic::Semantics;
    use crate::widgets::basic::WidgetBuilder;
    use crate::widgets::basic::{
        Center, Column, Directionality, Expanded, Padding, Row, StatefulBuilder,
    };
    use crate::widgets::container::Container;
    use crate::widgets::editable_text::{EditableText, EditableTextState};
    use crate::widgets::focus_manager::{AnyFocusNode, UnfocusDisposition};
    use crate::widgets::focus_scope::Focus;
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::navigator::RouteSettings;
    use crate::widgets::navigator::RouteSettingsRef;
    use crate::widgets::pages::PageRouteBuilder;
    use crate::widgets::placeholder::Placeholder;
    use crate::widgets::routes::RoutePageBuilder;
    use crate::widgets::scroll_view::ListView;
    use crate::widgets::text::Text;
    use crate::widgets::text_selection::{
        ClipboardStatusNotifier, TextSelectionControls, TextSelectionGestureDetectorBuilder,
        TextSelectionGestureDetectorBuilderData, TextSelectionGestureDetectorBuilderDelegate,
        TextSelectionHandleType,
    };
    use waft_rendering::HitTestBehavior;

    // Matches the Material kMinInteractiveDimension (48.0).
    const K_MIN_INTERACTIVE_DIMENSION: f64 = 48.0;

    const K_OPTIONS: [&str; 14] = [
        "aardvark",
        "bobcat",
        "chameleon",
        "dingo",
        "elephant",
        "flamingo",
        "goose",
        "hippopotamus",
        "iguana",
        "jaguar",
        "koala",
        "lemur",
        "mouse",
        "northern white rhinoceros",
    ];

    const OPEN_DIRECTIONS: [OptionsViewOpenDirection; 3] = [
        OptionsViewOpenDirection::Up,
        OptionsViewOpenDirection::Down,
        OptionsViewOpenDirection::MostSpace,
    ];

    #[derive(Clone, Debug, PartialEq)]
    struct User {
        email: &'static str,
        name: &'static str,
    }

    impl Display for User {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}, {}", self.name, self.email)
        }
    }

    fn k_options_users() -> [User; 3] {
        [
            User {
                name: "Alice",
                email: "alice@example.com",
            },
            User {
                name: "Bob",
                email: "bob@example.com",
            },
            User {
                name: "Charlie",
                email: "charlie123@gmail.com",
            },
        ]
    }

    /// Dart's `kOptions.where((option) => option.contains(text.toLowerCase()))`.
    fn matching(text: &str) -> Vec<String> {
        let query = text.to_lowercase();
        K_OPTIONS
            .iter()
            .filter(|option| option.contains(&query))
            .map(|option| option.to_string())
            .collect()
    }

    /// The `optionsBuilder` most cases share: the options containing the field's text.
    fn string_options(_app: &mut App, value: TextEditingValue) -> Task<Vec<String>> {
        Task::ready(matching(&value.text))
    }

    /// The `pageRouteBuilder` flutter_test's `TestWidgetsApp` hands to [`WidgetsApp`]: a page
    /// with no transition.
    fn page_route(app: &mut App, settings: &RouteSettings, builder: WidgetBuilder) -> PageRouteRef {
        let page_builder: RoutePageBuilder =
            Rc::new(move |app, context, _animation, _secondary| builder(app, context));
        PageRouteBuilder::new(app, page_builder)
            .settings(app, RouteSettingsRef::Settings(settings.clone()))
            .into_erased()
    }

    /// flutter_test's `TestWidgetsApp(home: home)`.
    fn test_widgets_app<K>(home: impl IntoWidget<K>) -> WidgetRef {
        WidgetsApp::new(Color::new(0xFFFFFFFF))
            .page_route_builder(page_route)
            .home(home)
            .into_widget()
    }

    /// Dart's `TestTextField` (`editable_text_tester.dart`): an [`EditableText`] under a
    /// selection gesture detector, with its required members filled, owning a controller and a
    /// focus node unless given them.
    #[derive(Default)]
    struct TestTextField {
        key: Option<KeyRef>,
        controller: Option<TextEditingControllerRef>,
        focus_node: Option<FocusNodeRef>,
        on_submitted: Option<SubmittedCallback>,
    }

    /// `TestTextField.onSubmitted`'s type.
    type SubmittedCallback = Rc<dyn Fn(&mut App, String)>;

    impl TestTextField {
        fn new() -> TestTextField {
            TestTextField::default()
        }

        fn key(mut self, key: &GlobalKey) -> TestTextField {
            self.key = Some(key.clone().into());
            self
        }

        fn controller(mut self, controller: TextEditingControllerRef) -> TestTextField {
            self.controller = Some(controller);
            self
        }

        fn focus_node(mut self, focus_node: FocusNodeRef) -> TestTextField {
            self.focus_node = Some(focus_node);
            self
        }

        fn on_submitted(
            mut self,
            on_submitted: impl Fn(&mut App, String) + 'static,
        ) -> TestTextField {
            self.on_submitted = Some(Rc::new(on_submitted));
            self
        }
    }

    impl Debug for TestTextField {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("TestTextField").finish_non_exhaustive()
        }
    }

    impl StatefulWidget for TestTextField {
        type State = TestTextFieldState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> TestTextFieldState {
            TestTextFieldState {
                state: StateData::new(),
                controller: None,
                focus_node: None,
                editable_text_key: GlobalKey::new(),
                selection_gesture_detector_builder: None,
            }
        }
    }

    struct TestTextFieldState {
        state: StateData<TestTextField>,
        controller: Option<Ref<TextEditingController>>,
        focus_node: Option<Ref<FocusNode>>,
        editable_text_key: GlobalKey,
        selection_gesture_detector_builder:
            Option<Ref<TestTextFieldSelectionGestureDetectorBuilder>>,
    }

    impl TestTextFieldState {
        fn effective_controller(self: Handle<Self>, app: &mut App) -> TextEditingControllerRef {
            if let Some(controller) = &self.widget(app).controller {
                return controller.clone();
            }
            if app.get(self).controller.is_none() {
                let controller = TextEditingController::new(app);
                app.get_mut(self).controller = Some(controller);
            }
            app.get(self)
                .controller
                .as_ref()
                .expect("made above")
                .erase()
        }

        fn effective_focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
            if let Some(focus_node) = &self.widget(app).focus_node {
                return focus_node.clone();
            }
            if app.get(self).focus_node.is_none() {
                let focus_node = FocusNode::new(app);
                app.get_mut(self).focus_node = Some(focus_node);
            }
            app.get(self)
                .focus_node
                .as_ref()
                .expect("made above")
                .erase()
        }
    }

    impl TextSelectionGestureDetectorBuilderDelegate for TestTextFieldState {
        fn editable_text_key(self: Handle<Self>, app: &App) -> GlobalKey {
            app.get(self).editable_text_key.clone()
        }

        fn force_press_enabled(self: Handle<Self>, _app: &App) -> bool {
            false
        }

        fn selection_enabled(self: Handle<Self>, _app: &App) -> bool {
            true
        }
    }

    impl State for TestTextFieldState {
        type Widget = TestTextField;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let builder = app.create(TestTextFieldSelectionGestureDetectorBuilder {
                builder: TextSelectionGestureDetectorBuilderData::new(
                    self.as_text_selection_gesture_detector_builder_delegate(),
                ),
            });
            app.get_mut(self).selection_gesture_detector_builder = Some(builder);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            if let Some(focus_node) = app.get(self).focus_node.clone() {
                focus_node.dispose(app);
            }
            if let Some(controller) = app.get(self).controller.clone() {
                controller.dispose(app);
            }
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            const RED: Color = Color::new(0xFFF44336);
            let controller = self.effective_controller(app);
            let focus_node = self.effective_focus_node(app);
            let mut field =
                EditableText::new(controller, focus_node.clone(), TextStyle::new(), RED, RED)
                    .key(app.get(self).editable_text_key.clone().into())
                    .renderer_ignores_pointer(true)
                    .selection_controls(Rc::new(TestTextSelectionHandleControls));
            if let Some(on_submitted) = self.widget(app).on_submitted.clone() {
                field = field.on_submitted(move |app, value| on_submitted(app, value));
            }
            let builder = app
                .get(self)
                .selection_gesture_detector_builder
                .as_ref()
                .expect("made in init_state")
                .handle();
            let gestured = TextSelectionGestureDetectorBuilder::build_gesture_detector(
                builder,
                app,
                None,
                Some(HitTestBehavior::Translucent),
                field.into_widget(),
            );
            Semantics::new()
                .enabled(true)
                .on_tap(Rc::new(move |app: &mut App| {
                    focus_node.handle().request_focus(app, None)
                }))
                .child(gestured)
                .into_widget()
        }
    }

    /// Dart's `_TestTextFieldSelectionGestureDetectorBuilder`: the base builder, with no
    /// `onTap` of its own.
    struct TestTextFieldSelectionGestureDetectorBuilder {
        builder: TextSelectionGestureDetectorBuilderData,
    }

    impl TextSelectionGestureDetectorBuilder for TestTextFieldSelectionGestureDetectorBuilder {
        crate::text_selection_gesture_detector_builder_accessors!(builder);
    }

    /// Dart's `TestTextSelectionHandleControls`: handles that take no room.
    struct TestTextSelectionHandleControls;

    #[allow(deprecated)]
    impl TextSelectionControls for TestTextSelectionHandleControls {
        fn build_handle(
            &self,
            _app: &mut App,
            _context: BuildContext,
            _handle_type: TextSelectionHandleType,
            _text_line_height: f64,
            _on_tap: Option<Listener>,
        ) -> WidgetRef {
            SizedBox::shrink().into_widget()
        }

        fn get_handle_anchor(
            &self,
            _handle_type: TextSelectionHandleType,
            _text_line_height: f64,
        ) -> Offset {
            Offset::ZERO
        }

        fn build_toolbar(
            &self,
            _app: &mut App,
            _context: BuildContext,
            _global_editable_region: waft_embedder::Rect,
            _text_line_height: f64,
            _selection_midpoint: Offset,
            _endpoints: &[waft_rendering::TextSelectionPoint],
            _delegate: waft_services::AnyTextSelectionDelegate,
            _clipboard_status: Option<Ref<ClipboardStatusNotifier>>,
            _last_secondary_tap_down_position: Option<Offset>,
        ) -> WidgetRef {
            SizedBox::shrink().into_widget()
        }

        fn get_handle_size(&self, _text_line_height: f64) -> Size {
            Size::ZERO
        }
    }

    /// Dart's `_OptionTile`: a tappable box at least [`K_MIN_INTERACTIVE_DIMENSION`] tall.
    #[derive(Debug)]
    struct OptionTile {
        on_tap: Listener,
        child: WidgetRef,
    }

    impl StatelessWidget for OptionTile {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            GestureDetector::new()
                .on_tap(self.on_tap.clone())
                .child(
                    ConstrainedBox::new(BoxConstraints::new().min_height(48.0))
                        .child(self.child.clone()),
                )
                .into_widget()
        }
    }

    /// Dart's `TestListTile(title: Text(option))` (`list_tile_tester.dart`).
    fn test_list_tile(title: impl Into<String>) -> WidgetRef {
        Container::new()
            .constraints(BoxConstraints::new().min_height(56.0))
            .padding(EdgeInsetsGeometry::symmetric(4.0, 16.0))
            .child(Text::new(title))
            .into_widget()
    }

    /// What a case's builders hand out, as Dart's `late` locals capture them.
    struct Captured<T> {
        focus_node: RefCell<Option<FocusNodeRef>>,
        controller: RefCell<Option<TextEditingControllerRef>>,
        on_field_submitted: RefCell<Option<VoidCallback>>,
        last_options: RefCell<Option<Vec<T>>>,
        last_on_selected: RefCell<Option<AutocompleteOnSelected<T>>>,
        last_highlighted: Cell<usize>,
    }

    impl<T> Captured<T> {
        fn new() -> Rc<Captured<T>> {
            Rc::new(Captured {
                focus_node: RefCell::new(None),
                controller: RefCell::new(None),
                on_field_submitted: RefCell::new(None),
                last_options: RefCell::new(None),
                last_on_selected: RefCell::new(None),
                last_highlighted: Cell::new(0),
            })
        }

        fn focus_node(&self) -> AnyFocusNode {
            self.focus_node
                .borrow()
                .as_ref()
                .expect("the field was built")
                .handle()
        }

        fn controller(&self) -> TextEditingControllerRef {
            self.controller
                .borrow()
                .clone()
                .expect("the field was built")
        }

        fn on_selected(&self) -> AutocompleteOnSelected<T> {
            self.last_on_selected
                .borrow()
                .clone()
                .expect("the options were built")
        }
    }

    impl<T: Clone> Captured<T> {
        fn options(&self) -> Vec<T> {
            self.last_options
                .borrow()
                .clone()
                .expect("the options were built")
        }
    }

    /// The `fieldViewBuilder` most cases share: a [`TestTextField`] keyed `field_key` whose
    /// controller, focus node and submit callback land in `captured`.
    fn field_view<T: 'static>(
        captured: &Rc<Captured<T>>,
        field_key: &GlobalKey,
    ) -> impl Fn(
        &mut App,
        BuildContext,
        TextEditingControllerRef,
        FocusNodeRef,
        VoidCallback,
    ) -> WidgetRef
    + 'static {
        let captured = Rc::clone(captured);
        let field_key = field_key.clone();
        move |_app, _context, controller, focus_node, on_field_submitted| {
            *captured.focus_node.borrow_mut() = Some(focus_node.clone());
            *captured.controller.borrow_mut() = Some(controller.clone());
            *captured.on_field_submitted.borrow_mut() = Some(Rc::clone(&on_field_submitted));
            TestTextField::new()
                .key(&field_key)
                .focus_node(focus_node)
                .controller(controller)
                .on_submitted(move |app, _value| on_field_submitted(app))
                .into_widget()
        }
    }

    /// The `optionsViewBuilder` most cases share: a `Container` keyed `options_key`, with the
    /// options, the select callback and the highlighted index landing in `captured`.
    fn options_view<T: Clone + 'static>(
        captured: &Rc<Captured<T>>,
        options_key: &GlobalKey,
    ) -> impl Fn(&mut App, BuildContext, AutocompleteOnSelected<T>, &[T]) -> WidgetRef + 'static
    {
        let captured = Rc::clone(captured);
        let options_key = options_key.clone();
        move |app, context, on_selected, options| {
            captured
                .last_highlighted
                .set(AutocompleteHighlightedOption::of(app, context));
            *captured.last_options.borrow_mut() = Some(options.to_vec());
            *captured.last_on_selected.borrow_mut() = Some(on_selected);
            Container::new()
                .key(options_key.clone().into())
                .into_widget()
        }
    }

    /// The `RawAutocomplete<String>` most cases build: the shared options, field and options
    /// view.
    fn string_autocomplete(
        captured: &Rc<Captured<String>>,
        field_key: &GlobalKey,
        options_key: &GlobalKey,
    ) -> RawAutocomplete<String> {
        RawAutocomplete::new(options_view(captured, options_key), string_options)
            .field_view_builder(field_view(captured, field_key))
    }

    /// The case's own synchronous statements up to its next `await`, then the end of that
    /// event: the checkpoint runs the microtasks and the continuations they left, which
    /// flutter_test's next `pump` flushes before its frame.
    fn act<R>(tester: &mut WidgetTester, body: impl FnOnce(&mut App) -> R) -> R {
        let result = tester.update(body);
        tester.cell.checkpoint();
        result
    }

    /// Dart's `const TextEditingValue(text: text, selection: TextSelection.collapsed(..))`
    /// with the caret at `offset`.
    fn value_at(text: &str, offset: i32) -> TextEditingValue {
        TextEditingValue::new()
            .text(text)
            .selection(TextSelection::collapsed(offset, TextAffinity::Downstream))
    }

    fn editable_text_state(tester: &WidgetTester, field: Finder) -> Handle<EditableTextState> {
        let element: AnyElement = tester.element(find::descendant_or_self(
            field,
            find::by_type::<EditableText>(),
        ));
        element
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state")
    }

    /// Dart's `tester.showKeyboard(finder)`.
    fn show_keyboard(tester: &mut WidgetTester, field: Finder) {
        let state = editable_text_state(tester, field);
        state.request_keyboard(&mut tester.cell.borrow_mut());
        tester.pump();
    }

    /// Dart's `tester.enterText(finder, text)`: the keyboard is shown, then the platform sends
    /// `text` with the caret at its end.
    fn enter_text(tester: &mut WidgetTester, field: Finder, text: &str) {
        show_keyboard(tester, field.clone());
        let state = editable_text_state(tester, field);
        let value = value_at(text, text.encode_utf16().count() as i32);
        TextInputClient::update_editing_value(state, &mut tester.cell.borrow_mut(), value);
        tester.cell.checkpoint();
    }

    /// Dart's `tester.testTextInput.receiveAction(action)` on the field with the connection.
    fn receive_action(tester: &mut WidgetTester, field: Finder, action: TextInputAction) {
        let state = editable_text_state(tester, field);
        TextInputClient::perform_action(state, &mut tester.cell.borrow_mut(), action);
        tester.cell.checkpoint();
    }

    /// Dart's `tester.sendKeyDownEvent(key)` or `sendKeyUpEvent(key)`: one key event.
    fn send_key_down_or_up(
        tester: &mut WidgetTester,
        physical: PhysicalKeyboardKey,
        logical: LogicalKeyboardKey,
        event_type: waft_embedder::KeyEventType,
    ) {
        use waft_embedder::{KeyData, KeyEventDeviceType};
        {
            let mut app = tester.cell.borrow_mut();
            waft_services::KeyEventManager::instance(&mut app).handle_key_data(
                &mut app,
                KeyData {
                    time_stamp: Duration::ZERO,
                    event_type,
                    device_type: KeyEventDeviceType::Keyboard,
                    physical: physical.usb_hid_usage,
                    logical: logical.key_id,
                    character: None,
                    synthesized: false,
                },
            );
            app.drain_microtasks();
        }
        tester.cell.checkpoint();
    }

    fn send_key(
        tester: &mut WidgetTester,
        physical: PhysicalKeyboardKey,
        logical: LogicalKeyboardKey,
    ) {
        tester.send_key_event(physical, logical);
    }

    fn arrow_down(tester: &mut WidgetTester) {
        send_key(
            tester,
            PhysicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ARROW_DOWN,
        );
    }

    fn arrow_up(tester: &mut WidgetTester) {
        send_key(
            tester,
            PhysicalKeyboardKey::ARROW_UP,
            LogicalKeyboardKey::ARROW_UP,
        );
    }

    fn escape(tester: &mut WidgetTester) {
        send_key(
            tester,
            PhysicalKeyboardKey::ESCAPE,
            LogicalKeyboardKey::ESCAPE,
        );
    }

    /// Dart's `sendKeyDownEvent(control); sendKeyEvent(key); sendKeyUpEvent(control)`.
    fn with_control(
        tester: &mut WidgetTester,
        physical: PhysicalKeyboardKey,
        logical: LogicalKeyboardKey,
    ) {
        send_key_down_or_up(
            tester,
            PhysicalKeyboardKey::CONTROL_LEFT,
            LogicalKeyboardKey::CONTROL_LEFT,
            waft_embedder::KeyEventType::Down,
        );
        send_key(tester, physical, logical);
        send_key_down_or_up(
            tester,
            PhysicalKeyboardKey::CONTROL_LEFT,
            LogicalKeyboardKey::CONTROL_LEFT,
            waft_embedder::KeyEventType::Up,
        );
    }

    /// Dart's `Future<T>.delayed(delay, () => value)`.
    fn delayed<T: Clone + 'static>(app: &mut App, delay: Duration, value: T) -> Task<T> {
        let completer: Completer<T> = Completer::new();
        let future = completer.future();
        Timer::new(
            app,
            delay,
            Listener::new(move |app| completer.complete(app, value.clone())),
        );
        app.spawn(async move |_cx| future.await)
    }

    /// Dart's `textEditingController.text`.
    fn text_of(tester: &WidgetTester, controller: &TextEditingControllerRef) -> String {
        controller
            .handle()
            .text_value(&tester.cell.borrow())
            .to_string()
    }

    fn strings(options: &[&str]) -> Vec<String> {
        options.iter().map(|option| option.to_string()).collect()
    }

    #[test]
    fn can_filter_and_select_a_list_of_string_options() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // The field is always rendered, but the options are not unless needed.
            expect(tester, &find::by_global_key(&field_key), finds_one_widget());
            expect(tester, &find::by_global_key(&options_key), finds_nothing());

            // Focus the empty field. All the options are displayed.
            let focus_node = captured.focus_node();
            act(tester, |app| focus_node.request_focus(app, None));
            tester.pump();
            expect(
                tester,
                &find::by_global_key(&options_key),
                finds_one_widget(),
            );
            assert_eq!(captured.options().len(), K_OPTIONS.len());

            // Enter text. The options are filtered by the text.
            let controller = captured.controller();
            act(tester, |app| {
                controller.handle().set_value(app, value_at("ele", 3))
            });
            tester.pump();
            expect(tester, &find::by_global_key(&field_key), finds_one_widget());
            expect(
                tester,
                &find::by_global_key(&options_key),
                finds_one_widget(),
            );
            assert_eq!(captured.options(), strings(&["chameleon", "elephant"]));

            // Select an option. The options hide and the field updates to show the selection.
            let selection = captured.options()[1].clone();
            let on_selected = captured.on_selected();
            act(tester, |app| on_selected(app, selection.clone()));
            tester.pump();
            expect(tester, &find::by_global_key(&field_key), finds_one_widget());
            expect(tester, &find::by_global_key(&options_key), finds_nothing());
            assert_eq!(text_of(tester, &controller), selection);

            // Modify the field text. The options appear again and are filtered.
            act(tester, |app| {
                controller.handle().set_value(app, value_at("e", 1))
            });
            tester.pump();
            expect(tester, &find::by_global_key(&field_key), finds_one_widget());
            expect(
                tester,
                &find::by_global_key(&options_key),
                finds_one_widget(),
            );
            assert_eq!(
                captured.options(),
                strings(&[
                    "chameleon",
                    "elephant",
                    "goose",
                    "lemur",
                    "mouse",
                    "northern white rhinoceros"
                ])
            );
        });
    }

    #[test]
    fn options_are_shown_one_frame_after_tapping_in_field() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let options_key_for_view = options_key.clone();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(
                Align::new().alignment(AlignmentGeometry::TOP_CENTER).child(
                    RawAutocomplete::new(
                        move |_app, _context, _on_selected, options: &[String]| {
                            ListView::new()
                                .key(options_key_for_view.clone().into())
                                .children(
                                    options
                                        .iter()
                                        .map(|option| Text::new(option.clone()).into_widget()),
                                )
                                .into_widget()
                        },
                        string_options,
                    )
                    .field_view_builder(field_view(&captured, &field_key)),
                ),
            ));

            // Field is shown but not options.
            expect(tester, &find::by_global_key(&field_key), finds_one_widget());
            expect(tester, &find::by_global_key(&options_key), finds_nothing());
            expect(tester, &find::text("aardvark"), finds_nothing());

            // Tap to show the options.
            tester.tap(find::by_global_key(&field_key));
            tester.pump();
            expect(tester, &find::by_global_key(&field_key), finds_one_widget());
            expect(
                tester,
                &find::by_global_key(&options_key),
                finds_one_widget(),
            );
            expect(tester, &find::text("aardvark"), finds_one_widget());
        });
    }

    /// The `optionsViewBuilder` of `_OptionTile`s, each padded text selecting its option:
    /// `ListView.builder(padding: EdgeInsets.zero, shrinkWrap: true, ..)`.
    fn option_tiles(
        options_key: &GlobalKey,
    ) -> impl Fn(&mut App, BuildContext, AutocompleteOnSelected<String>, &[String]) -> WidgetRef + 'static
    {
        let options_key = options_key.clone();
        move |_app, _context, on_selected, options| {
            let options = options.to_vec();
            let count = options.len() as i32;
            ListView::builder(move |_app, _context, index| {
                let option = options[index as usize].clone();
                let on_selected = Rc::clone(&on_selected);
                let tapped = option.clone();
                Some(
                    OptionTile {
                        on_tap: Listener::new(move |app| on_selected(app, tapped.clone())),
                        child: Padding::new(EdgeInsetsGeometry::all(16.0))
                            .child(Text::new(option))
                            .into_widget(),
                    }
                    .into_widget(),
                )
            })
            .item_count(count)
            .build()
            .key(options_key.clone().into())
            .padding(EdgeInsetsGeometry::ZERO)
            .shrink_wrap(true)
            .into_widget()
        }
    }

    /// A `StatefulBuilder`'s `setState`, captured as Dart's `late StateSetter setState`.
    type CapturedSetter = Rc<RefCell<Option<crate::widgets::basic::StateSetter>>>;

    fn set_state(tester: &mut WidgetTester, setter: &CapturedSetter, mut change: impl FnMut()) {
        let setter = setter.borrow().clone().expect("the builder ran");
        act(tester, |app| setter(app, &mut |_app| change()));
    }

    fn by_key(key: &GlobalKey) -> Finder {
        find::by_global_key(key)
    }

    /// A field and its focus node made by the case, as Dart's `TextEditingController()` and
    /// `FocusNode()` with `addTearDown(dispose)`.
    fn own_field(tester: &WidgetTester) -> (Ref<TextEditingController>, Ref<FocusNode>) {
        tester.update(|app| (TextEditingController::new(app), FocusNode::new(app)))
    }

    fn focus_of(node: &Ref<FocusNode>) -> AnyFocusNode {
        let erased: FocusNodeRef = node.erase();
        erased.handle()
    }

    #[test]
    fn can_split_the_field_and_options() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let autocomplete_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let (text_editing_controller, focus_node) = own_field(tester);

            let submitted_key = autocomplete_key.clone();
            let view_key = options_key.clone();
            let recorded = Rc::clone(&captured);
            tester.pump_widget(test_widgets_app(
                Column::new().children([
                    // The field that is not actually a child of RawAutocomplete.
                    TestTextField::new()
                        .key(&field_key)
                        .controller(text_editing_controller.erase())
                        .focus_node(focus_node.erase())
                        .on_submitted(move |app, _value| {
                            RawAutocomplete::<String>::on_field_submitted(app, &submitted_key)
                        })
                        .into_widget(),
                    Expanded::new(
                        Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                            RawAutocomplete::new(
                                move |_app,
                                      _context,
                                      on_selected: AutocompleteOnSelected<String>,
                                      options: &[String]| {
                                    *recorded.last_options.borrow_mut() = Some(options.to_vec());
                                    *recorded.last_on_selected.borrow_mut() =
                                        Some(Rc::clone(&on_selected));
                                    ListView::new()
                                        .key(view_key.clone().into())
                                        .children(options.iter().map(|option| {
                                            let on_selected = Rc::clone(&on_selected);
                                            let tapped = option.clone();
                                            GestureDetector::new()
                                                .on_tap(Listener::new(move |app| {
                                                    on_selected(app, tapped.clone())
                                                }))
                                                .child(test_list_tile(option.clone()))
                                                .into_widget()
                                        }))
                                        .into_widget()
                                },
                                string_options,
                            )
                            .key(autocomplete_key.clone().into())
                            .focus_node(focus_node.erase())
                            .text_editing_controller(text_editing_controller.erase()),
                        ),
                    )
                    .into_widget(),
                ]),
            ));

            // The field is always rendered, but the options are not unless needed.
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Focus the empty field. All the options are displayed.
            act(tester, |app| focus_of(&focus_node).request_focus(app, None));
            tester.pump();
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options().len(), K_OPTIONS.len());
            assert!(tester.get_size(by_key(&options_key)).width() > 0.0);

            // Enter text. The options are filtered by the text.
            let controller: TextEditingControllerRef = text_editing_controller.erase();
            act(tester, |app| {
                controller.handle().set_value(app, value_at("ele", 3))
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), strings(&["chameleon", "elephant"]));

            // Select an option. The options hide and the field updates to show the selection.
            let selection = captured.options()[1].clone();
            let on_selected = captured.on_selected();
            act(tester, |app| on_selected(app, selection.clone()));
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(text_of(tester, &controller), selection);

            // Modify the field text. The options appear again and are filtered.
            act(tester, |app| {
                controller.handle().set_value(app, value_at("e", 1))
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(
                captured.options(),
                strings(&[
                    "chameleon",
                    "elephant",
                    "goose",
                    "lemur",
                    "mouse",
                    "northern white rhinoceros"
                ])
            );
        });
    }

    #[test]
    fn tapping_on_an_option_selects_it_in_each_open_direction() {
        for open_direction in OPEN_DIRECTIONS {
            test_widgets(|tester| {
                let field_key = GlobalKey::new();
                let options_key = GlobalKey::new();
                let captured = Captured::<String>::new();
                let view_key = options_key.clone();
                let recorded = Rc::clone(&captured);
                tester.pump_widget(test_widgets_app(
                    Column::new().children([
                        SizedBox::new().height(200.0).into_widget(),
                        RawAutocomplete::new(
                            move |_app,
                                  _context,
                                  on_selected: AutocompleteOnSelected<String>,
                                  options: &[String]| {
                                *recorded.last_options.borrow_mut() = Some(options.to_vec());
                                let options = options.to_vec();
                                let count = options.len() as i32;
                                ListView::builder(move |_app, _context, index| {
                                    let option = options[index as usize].clone();
                                    let on_selected = Rc::clone(&on_selected);
                                    let tapped = option.clone();
                                    Some(
                                        GestureDetector::new()
                                            .on_tap(Listener::new(move |app| {
                                                on_selected(app, tapped.clone())
                                            }))
                                            .child(test_list_tile(option))
                                            .into_widget(),
                                    )
                                })
                                .item_count(count)
                                .build()
                                .key(view_key.clone().into())
                                .padding(EdgeInsetsGeometry::all(8.0))
                                .into_widget()
                            },
                            string_options,
                        )
                        .options_view_open_direction(open_direction)
                        .field_view_builder(field_view(&captured, &field_key))
                        .into_widget(),
                    ]),
                ));

                // The field is always rendered, but the options are not unless needed.
                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_nothing());

                // Tap on the text field to open the options.
                tester.tap(by_key(&field_key));
                tester.pump();
                expect(tester, &by_key(&options_key), finds_one_widget());
                assert_eq!(captured.options().len(), K_OPTIONS.len());

                tester.tap(find::text(K_OPTIONS[2]));
                tester.pump();

                expect(tester, &by_key(&options_key), finds_nothing());

                assert_eq!(
                    text_of(tester, &captured.controller()),
                    K_OPTIONS[2],
                    "{open_direction:?}"
                );
            });
        }
    }

    #[test]
    fn when_not_enough_room_for_options_options_cover_field_in_each_open_direction() {
        for open_direction in OPEN_DIRECTIONS {
            test_widgets(|tester| {
                const PADDING: f64 = 32.0;
                let field_key = GlobalKey::new();
                let options_key = GlobalKey::new();
                let setter: CapturedSetter = Rc::default();
                let alignment = Rc::new(Cell::new(AlignmentGeometry::BOTTOM_CENTER));

                let captured = Captured::<String>::new();
                let (stored, aligned) = (Rc::clone(&setter), Rc::clone(&alignment));
                let (field, options) = (field_key.clone(), options_key.clone());
                tester.pump_widget(test_widgets_app(StatefulBuilder::new(
                    move |_app, _context, set_state| {
                        *stored.borrow_mut() = Some(set_state);
                        Padding::new(EdgeInsetsGeometry::symmetric(0.0, PADDING))
                            .child(
                                Align::new().alignment(aligned.get()).child(
                                    RawAutocomplete::new(option_tiles(&options), string_options)
                                        .options_view_open_direction(open_direction)
                                        .field_view_builder(field_view(&captured, &field)),
                                ),
                            )
                            .into_widget()
                    },
                )));

                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_nothing());

                tester.tap(by_key(&field_key));
                tester.pump();

                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_one_widget());

                enter_text(tester, by_key(&field_key), "go"); // 3 results.
                tester.pump();

                let tiles = || find::by_type::<OptionTile>();
                match open_direction {
                    OptionsViewOpenDirection::Up | OptionsViewOpenDirection::MostSpace => {
                        // Options are positioned and sized like normal.
                        expect(tester, &tiles(), finds_n_widgets(3));
                        let option_height = tester.get_size(tiles().first()).height();
                        let top_of_field = tester.get_top_left(by_key(&field_key)).dy();
                        assert_eq!(
                            tester.get_top_left(tiles().first()),
                            Offset::new(PADDING, top_of_field - 3.0 * option_height)
                        );
                        assert_eq!(
                            tester.get_bottom_left(tiles().at(2)),
                            Offset::new(PADDING, top_of_field)
                        );
                    }
                    OptionsViewOpenDirection::Down => {
                        expect(tester, &tiles(), finds_n_widgets(1));
                        let options_size = tester.get_size(by_key(&options_key));
                        assert_eq!(options_size.height(), K_MIN_INTERACTIVE_DIMENSION);
                        // Options are positioned as low as possible while still fitting on
                        // screen.
                        let bottom_of_field = tester.get_bottom_left(by_key(&options_key)).dy();
                        assert_eq!(
                            tester.get_top_left(by_key(&options_key)),
                            Offset::new(PADDING, bottom_of_field - options_size.height())
                        );
                    }
                }

                set_state(tester, &setter, || {
                    alignment.set(AlignmentGeometry::TOP_CENTER)
                });

                tester.pump();

                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_one_widget());

                match open_direction {
                    OptionsViewOpenDirection::Up => {
                        // Options are positioned as high as possible while still fitting on
                        // the screen.
                        expect(tester, &tiles(), finds_n_widgets(1));
                        let options_size = tester.get_size(by_key(&options_key));
                        assert_eq!(options_size.height(), K_MIN_INTERACTIVE_DIMENSION);
                        assert_eq!(
                            tester.get_top_left(by_key(&options_key)),
                            Offset::new(PADDING, 0.0)
                        );
                        assert_eq!(
                            tester.get_bottom_left(by_key(&options_key)),
                            Offset::new(PADDING, options_size.height())
                        );
                    }
                    OptionsViewOpenDirection::Down | OptionsViewOpenDirection::MostSpace => {
                        // Options are positioned and sized like normal.
                        expect(tester, &tiles(), finds_n_widgets(3));
                        let option_height = tester.get_size(tiles().first()).height();
                        let bottom_of_field = tester.get_bottom_left(by_key(&field_key)).dy();
                        assert_eq!(
                            tester.get_top_left(tiles().first()),
                            Offset::new(PADDING, bottom_of_field)
                        );
                        assert_eq!(
                            tester.get_bottom_left(tiles().at(2)),
                            Offset::new(PADDING, bottom_of_field + 3.0 * option_height)
                        );
                    }
                }
            });
        }
    }

    #[test]
    fn correct_options_alignment_for_rtl_in_each_open_direction() {
        for open_direction in OPEN_DIRECTIONS {
            test_widgets(|tester| {
                const K_OPTIONS_WIDTH: f64 = 100.0;
                let field_key = GlobalKey::new();
                let options_key = GlobalKey::new();
                let captured = Captured::<String>::new();
                let view_key = options_key.clone();
                tester.pump_widget(test_widgets_app(Directionality::new(
                    TextDirection::Rtl,
                    Padding::new(EdgeInsetsGeometry::symmetric(0.0, 32.0)).child(
                        RawAutocomplete::new(
                            move |_app, _context, _on_selected, _options: &[String]| {
                                SizedBox::new()
                                    .width(K_OPTIONS_WIDTH)
                                    .key(view_key.clone().into())
                                    .into_widget()
                            },
                            string_options,
                        )
                        .options_view_open_direction(open_direction)
                        .field_view_builder(field_view(&captured, &field_key)),
                    ),
                )));

                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_nothing());

                tester.tap(find::by_type::<TestTextField>());
                tester.pump();

                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_one_widget());

                assert_eq!(
                    tester.get_size(by_key(&options_key)).width(),
                    K_OPTIONS_WIDTH
                );
                assert_eq!(
                    tester.get_top_right(by_key(&options_key)).dx(),
                    tester.get_top_right(by_key(&field_key)).dx()
                );
            });
        }
    }

    #[test]
    fn options_width_matches_field_width_in_each_open_direction() {
        for open_direction in OPEN_DIRECTIONS {
            test_widgets(|tester| {
                let field_key = GlobalKey::new();
                let options_key = GlobalKey::new();
                let captured = Captured::<String>::new();
                tester.pump_widget(test_widgets_app(
                    Padding::new(EdgeInsetsGeometry::symmetric(0.0, 32.0)).child(
                        Center::new().child(
                            string_autocomplete(&captured, &field_key, &options_key)
                                .options_view_open_direction(open_direction),
                        ),
                    ),
                ));

                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_nothing());

                tester.tap(find::by_type::<TestTextField>());
                tester.pump();

                expect(tester, &by_key(&field_key), finds_one_widget());
                expect(tester, &by_key(&options_key), finds_one_widget());

                let field_size = tester.get_size(by_key(&field_key));
                let options_size = tester.get_size(by_key(&options_key));
                assert_eq!(options_size.width(), field_size.width());
                let field_top = tester.get_top_left(by_key(&field_key)).dy();
                assert_eq!(
                    tester.get_top_left(by_key(&options_key)).dy(),
                    match open_direction {
                        OptionsViewOpenDirection::Down => field_top + field_size.height(),
                        OptionsViewOpenDirection::Up => field_top - options_size.height(),
                        OptionsViewOpenDirection::MostSpace => field_top + field_size.height(),
                    }
                );
            });
        }
    }

    /// The options builder of the `User` cases: users whose `Display` form contains the text.
    fn user_options(_app: &mut App, value: TextEditingValue) -> Task<Vec<User>> {
        let query = value.text.to_lowercase();
        Task::ready(
            k_options_users()
                .into_iter()
                .filter(|option| option.to_string().contains(&query))
                .collect(),
        )
    }

    #[test]
    fn can_filter_and_select_a_list_of_custom_user_options() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<User>::new();
            let last_user_selected: Rc<RefCell<Option<User>>> = Rc::default();
            let selected = Rc::clone(&last_user_selected);
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(options_view(&captured, &options_key), user_options)
                    .on_selected(move |_app, user| *selected.borrow_mut() = Some(user))
                    .field_view_builder(field_view(&captured, &field_key)),
            ));

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Enter text. The options are filtered by the text.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().set_value(app, value_at("example", 7));
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            let users = k_options_users();
            assert_eq!(captured.options(), vec![users[0].clone(), users[1].clone()]);

            // Select an option. The options hide and onSelected is called.
            let selection = captured.options()[1].clone();
            let on_selected = captured.on_selected();
            act(tester, |app| on_selected(app, selection.clone()));
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(last_user_selected.borrow().as_ref(), Some(&selection));
            assert_eq!(text_of(tester, &controller), selection.to_string());

            // Modify the field text. The options appear again and are filtered, this time by
            // name instead of email.
            act(tester, |app| {
                controller.handle().set_value(app, value_at("B", 1))
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), vec![users[1].clone()]);
        });
    }

    #[test]
    fn can_specify_a_custom_display_string_for_a_list_of_custom_user_options() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<User>::new();
            let last_user_selected: Rc<RefCell<Option<User>>> = Rc::default();
            let selected = Rc::clone(&last_user_selected);
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(options_view(&captured, &options_key), user_options)
                    .display_string_for_option(|option: &User| option.name.to_string())
                    .on_selected(move |_app, user| *selected.borrow_mut() = Some(user))
                    .field_view_builder(field_view(&captured, &field_key)),
            ));

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Enter text. The options are filtered by the text.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().set_value(app, value_at("example", 7));
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            let users = k_options_users();
            assert_eq!(captured.options(), vec![users[0].clone(), users[1].clone()]);

            // Select an option. The options hide and onSelected is called. The field has its
            // text set to the selection's display string.
            let selection = captured.options()[1].clone();
            let on_selected = captured.on_selected();
            act(tester, |app| on_selected(app, selection.clone()));
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(last_user_selected.borrow().as_ref(), Some(&selection));
            assert_eq!(text_of(tester, &controller), selection.name);

            // Modify the field text. The options appear again and are filtered, this time by
            // name instead of email.
            act(tester, |app| {
                controller.handle().set_value(app, value_at("B", 1))
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), vec![users[1].clone()]);
        });
    }

    #[test]
    fn on_field_submitted_selects_the_first_option() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Enter text. The options are filtered by the text.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().set_value(app, value_at("ele", 3));
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), strings(&["chameleon", "elephant"]));

            // Select the current string, as if the field was submitted. The options hide and
            // the field updates to show the selection.
            let on_field_submitted = captured.on_field_submitted.borrow().clone().expect("built");
            act(tester, |app| on_field_submitted(app));
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(text_of(tester, &controller), captured.options()[0]);
        });
    }

    /// The `optionsViewOpenDirection` group's autocomplete: one option `a`, shown as a text.
    fn one_option_autocomplete() -> RawAutocomplete<String> {
        RawAutocomplete::new(
            |_app, _context, _on_selected, _options: &[String]| Text::new("a").into_widget(),
            |_app, _value| Task::ready(strings(&["a"])),
        )
        .field_view_builder(
            |_app, _context, controller, focus_node, _on_field_submitted| {
                TestTextField::new()
                    .controller(controller)
                    .focus_node(focus_node)
                    .into_widget()
            },
        )
    }

    #[test]
    fn options_view_open_direction_unset_opens_downward() {
        test_widgets(|tester| {
            tester.pump_widget(test_widgets_app(
                Center::new().child(one_option_autocomplete()),
            ));
            show_keyboard(tester, find::by_type::<TestTextField>());
            tester.pump();
            assert_more_or_less_equals(
                tester.get_bottom_left(find::by_type::<TestTextField>()),
                tester.get_top_left(find::text("a")),
            );
        });
    }

    #[test]
    fn options_view_open_direction_down_opens_downward() {
        test_widgets(|tester| {
            tester.pump_widget(test_widgets_app(
                Center::new().child(
                    one_option_autocomplete()
                        .options_view_open_direction(OptionsViewOpenDirection::Down),
                ),
            ));
            show_keyboard(tester, find::by_type::<TestTextField>());
            tester.pump();
            assert_more_or_less_equals(
                tester.get_bottom_left(find::by_type::<TestTextField>()),
                tester.get_top_left(find::text("a")),
            );
        });
    }

    #[test]
    fn options_view_open_direction_up_opens_upward() {
        test_widgets(|tester| {
            tester.pump_widget(test_widgets_app(Center::new().child(
                one_option_autocomplete().options_view_open_direction(OptionsViewOpenDirection::Up),
            )));
            show_keyboard(tester, find::by_type::<TestTextField>());
            tester.pump();
            assert_more_or_less_equals(
                tester.get_top_left(find::by_type::<TestTextField>()),
                tester.get_bottom_left(find::text("a")),
            );
        });
    }

    #[test]
    fn options_view_open_direction_most_space_opens_in_the_direction_with_more_space() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let setter: CapturedSetter = Rc::default();
            let alignment = Rc::new(Cell::new(AlignmentGeometry::TOP_CENTER));

            let (stored, aligned) = (Rc::clone(&setter), Rc::clone(&alignment));
            let (field, options) = (field_key.clone(), options_key.clone());
            tester.pump_widget(test_widgets_app(StatefulBuilder::new(
                move |_app, _context, set_state| {
                    *stored.borrow_mut() = Some(set_state);
                    let options = options.clone();
                    Align::new()
                        .alignment(aligned.get())
                        .child(
                            RawAutocomplete::new(
                                move |_app, _context, _on_selected, values: &[String]| {
                                    ListView::new()
                                        .key(options.clone().into())
                                        .children(
                                            values.iter().map(|value| {
                                                Text::new(value.clone()).into_widget()
                                            }),
                                        )
                                        .into_widget()
                                },
                                |_app, _value| Task::ready(strings(&["a", "b", "c"])),
                            )
                            .key(field.clone().into())
                            .options_view_open_direction(OptionsViewOpenDirection::MostSpace)
                            .field_view_builder(
                                |_app, _context, controller, focus_node, _on_field_submitted| {
                                    TestTextField::new()
                                        .controller(controller)
                                        .focus_node(focus_node)
                                        .into_widget()
                                },
                            ),
                        )
                        .into_widget()
                },
            )));

            // Show the options. It should open downwards since there is more space.
            tester.tap(find::by_type::<TestTextField>());
            tester.pump();

            assert_more_or_less_equals(
                tester.get_bottom_left(by_key(&field_key)),
                tester.get_top_left(by_key(&options_key)),
            );

            // Move the field to the bottom.
            set_state(tester, &setter, || {
                alignment.set(AlignmentGeometry::BOTTOM_CENTER)
            });
            tester.pump();

            // The options should now open upwards, since there is more space above.
            assert_more_or_less_equals(
                tester.get_top_left(by_key(&field_key)),
                tester.get_bottom_left(by_key(&options_key)),
            );
        });
    }

    /// The `fieldViewBuilder not passed` cases' autocomplete over the case's own field.
    fn split_one_option_autocomplete(
        autocomplete_key: &GlobalKey,
        controller: &Ref<TextEditingController>,
        focus_node: &Ref<FocusNode>,
        open_direction: OptionsViewOpenDirection,
    ) -> WidgetRef {
        RawAutocomplete::new(
            |_app, _context, _on_selected, _options: &[String]| Text::new("a").into_widget(),
            |_app, _value| Task::ready(strings(&["a"])),
        )
        .key(autocomplete_key.clone().into())
        .text_editing_controller(controller.erase())
        .focus_node(focus_node.erase())
        .options_view_open_direction(open_direction)
        .into_widget()
    }

    #[test]
    fn options_view_open_direction_down_without_a_field_view_builder() {
        test_widgets(|tester| {
            let autocomplete_key = GlobalKey::new();
            let (controller, focus_node) = own_field(tester);
            tester.pump_widget(test_widgets_app(
                Column::new()
                    .cross_axis_alignment(waft_rendering::CrossAxisAlignment::Start)
                    .children([
                        TestTextField::new()
                            .controller(controller.erase())
                            .focus_node(focus_node.erase())
                            .into_widget(),
                        split_one_option_autocomplete(
                            &autocomplete_key,
                            &controller,
                            &focus_node,
                            OptionsViewOpenDirection::Down,
                        ),
                    ]),
            ));
            show_keyboard(tester, find::by_type::<TestTextField>());
            tester.pump();
            assert_more_or_less_equals(
                tester.get_bottom_left(by_key(&autocomplete_key)),
                tester.get_top_left(find::text("a")),
            );
        });
    }

    #[test]
    fn options_view_open_direction_up_without_a_field_view_builder() {
        test_widgets(|tester| {
            let autocomplete_key = GlobalKey::new();
            let (controller, focus_node) = own_field(tester);
            tester.pump_widget(test_widgets_app(
                Column::new()
                    .main_axis_alignment(waft_rendering::MainAxisAlignment::End)
                    .cross_axis_alignment(waft_rendering::CrossAxisAlignment::Start)
                    .children([
                        split_one_option_autocomplete(
                            &autocomplete_key,
                            &controller,
                            &focus_node,
                            OptionsViewOpenDirection::Up,
                        ),
                        TestTextField::new()
                            .controller(controller.erase())
                            .focus_node(focus_node.erase())
                            .into_widget(),
                    ]),
            ));
            show_keyboard(tester, find::by_type::<TestTextField>());
            tester.pump();
            assert_more_or_less_equals(
                tester.get_top_left(by_key(&autocomplete_key)),
                tester.get_bottom_left(find::text("a")),
            );
        });
    }

    #[test]
    fn options_follow_field_when_it_moves() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let setter: CapturedSetter = Rc::default();
            let alignment = Rc::new(Cell::new(AlignmentGeometry::CENTER));
            let captured = Captured::<String>::new();

            let (stored, aligned, recorded) = (
                Rc::clone(&setter),
                Rc::clone(&alignment),
                Rc::clone(&captured),
            );
            let (field, options) = (field_key.clone(), options_key.clone());
            tester.pump_widget(test_widgets_app(StatefulBuilder::new(
                move |_app, _context, set_state| {
                    *stored.borrow_mut() = Some(set_state);
                    Align::new()
                        .alignment(aligned.get())
                        .child(string_autocomplete(&recorded, &field, &options))
                        .into_widget()
                },
            )));

            // Field is shown but not options.
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Enter text to show the options.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().set_value(app, value_at("ele", 3));
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());

            // Options are just below the field.
            let options_top_left = tester.get_top_left(by_key(&options_key));
            let field_offset = tester.get_top_left(by_key(&field_key));
            let field_size = tester.get_size(by_key(&field_key));
            assert_eq!(
                options_top_left.dy(),
                field_offset.dy() + field_size.height()
            );

            // Move the field (similar to as if the keyboard opened). The options move to
            // follow the field.
            set_state(tester, &setter, || {
                alignment.set(AlignmentGeometry::TOP_CENTER)
            });
            tester.pump();
            let field_offset_frame1 = tester.get_top_left(by_key(&field_key));
            let options_top_left_open_frame1 = tester.get_top_left(by_key(&options_key));

            assert!(field_offset_frame1.dy() < field_offset.dy());
            assert_ne!(options_top_left_open_frame1.dy(), options_top_left.dy());
            assert_eq!(
                options_top_left_open_frame1.dy(),
                field_offset_frame1.dy() + field_size.height()
            );
        });
    }

    #[test]
    fn can_prevent_options_from_showing_by_returning_an_empty_iterable() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(
                    options_view(&captured, &options_key),
                    |_app, value: TextEditingValue| {
                        if value.text.is_empty() {
                            return Task::ready(Vec::new());
                        }
                        Task::ready(matching(&value.text))
                    },
                )
                .field_view_builder(field_view(&captured, &field_key)),
            ));

            // The field is always rendered, but the options are not unless needed.
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Focus the empty field. The options are not displayed because optionsBuilder
            // returns nothing for an empty field query.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().set_value(app, value_at("", 0));
            });
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            // Enter text. Now the options appear, filtered by the text.
            act(tester, |app| {
                controller.handle().set_value(app, value_at("ele", 3))
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), strings(&["chameleon", "elephant"]));
        });
    }

    #[test]
    fn can_create_a_field_outside_of_field_view_builder() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let autocomplete_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let (text_editing_controller, focus_node) = own_field(tester);

            let submitted_key = autocomplete_key.clone();
            tester.pump_widget(test_widgets_app(
                Column::new().children([
                    // This is where the real field is being built.
                    TestTextField::new()
                        .key(&field_key)
                        .controller(text_editing_controller.erase())
                        .focus_node(focus_node.erase())
                        .on_submitted(move |app, _value| {
                            RawAutocomplete::<String>::on_field_submitted(app, &submitted_key)
                        })
                        .into_widget(),
                    Expanded::new(
                        RawAutocomplete::new(options_view(&captured, &options_key), string_options)
                            .key(autocomplete_key.clone().into())
                            .focus_node(focus_node.erase())
                            .text_editing_controller(text_editing_controller.erase()),
                    )
                    .into_widget(),
                ]),
            ));

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Enter text. The options are filtered by the text.
            let controller: TextEditingControllerRef = text_editing_controller.erase();
            act(tester, |app| {
                focus_of(&focus_node).request_focus(app, None);
                controller.handle().set_value(app, value_at("ele", 3));
            });
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), strings(&["chameleon", "elephant"]));

            // Submit the field. The options hide and the field updates to show the selection.
            show_keyboard(tester, find::by_type::<TestTextField>());
            receive_action(
                tester,
                find::by_type::<TestTextField>(),
                TextInputAction::Done,
            );
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(text_of(tester, &controller), captured.options()[0]);
        });
    }

    #[test]
    fn initial_value_sets_initial_text_field_value() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(
                string_autocomplete(&captured, &field_key, &options_key)
                    // Should initialize text field with 'lem'.
                    .initial_value(TextEditingValue::new().text("lem")),
            ));

            // The field is always rendered, but the options are not unless needed.
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            // The text editing controller value starts off with initialized value.
            let controller = captured.controller();
            assert_eq!(text_of(tester, &controller), "lem");

            // Focus the empty field. All the options are displayed.
            let focus_node = captured.focus_node();
            act(tester, |app| focus_node.request_focus(app, None));
            tester.pump();
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options()[0], "lemur");

            // Select an option. The options hide and the field updates to show the selection.
            let selection = captured.options()[0].clone();
            let on_selected = captured.on_selected();
            act(tester, |app| on_selected(app, selection.clone()));
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(text_of(tester, &controller), selection);
        });
    }

    #[test]
    #[should_panic(
        expected = "textEditingController and initialValue cannot be simultaneously defined."
    )]
    fn initial_value_cannot_be_defined_if_text_editing_controller_is_defined() {
        test_widgets(|tester| {
            let (text_editing_controller, focus_node) = own_field(tester);
            let field_controller: TextEditingControllerRef = text_editing_controller.erase();
            let field_focus_node: FocusNodeRef = focus_node.erase();
            RawAutocomplete::new(
                |_app, _context, _on_selected, _options: &[String]| Container::new().into_widget(),
                string_options,
            )
            .focus_node(focus_node.erase())
            // Both initialValue and textEditingController cannot be simultaneously defined.
            .initial_value(TextEditingValue::new().text("lemur"))
            .text_editing_controller(text_editing_controller.erase())
            .field_view_builder(
                move |_app, _context, _controller, _focus_node, _on_field_submitted| {
                    TestTextField::new()
                        .focus_node(field_focus_node.clone())
                        .controller(field_controller.clone())
                        .into_widget()
                },
            )
            .into_widget();
        });
    }

    /// The `optionsBuilder` of the asynchronous cases: the matching options, after `delay`
    /// when one is set.
    fn delayed_options(
        delay: &Rc<Cell<Option<Duration>>>,
    ) -> impl Fn(&mut App, TextEditingValue) -> Task<Vec<String>> + 'static {
        let delay = Rc::clone(delay);
        move |app, value| {
            let options = matching(&value.text);
            match delay.get() {
                None => Task::ready(options),
                Some(delay) => delayed(app, delay, options),
            }
        }
    }

    fn last_options(captured: &Captured<String>) -> Option<Vec<String>> {
        captured.last_options.borrow().clone()
    }

    #[test]
    fn support_asynchronous_options_builder() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let delay: Rc<Cell<Option<Duration>>> = Rc::default();
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(
                    options_view(&captured, &options_key),
                    delayed_options(&delay),
                )
                .field_view_builder(field_view(&captured, &field_key)),
            ));

            // Enter text to build the options with delay.
            let focus_node = captured.focus_node();
            act(tester, |app| {
                focus_node.request_focus(app, None);
                delay.set(Some(Duration::from_millis(500)));
            });
            enter_text(tester, by_key(&field_key), "go");
            tester.pump_and_settle();

            // The options have not yet been built.
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(last_options(&captured), None);

            // Await asynchronous options builder.
            tester.pump_and_settle_by(Duration::from_millis(500));
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(
                last_options(&captured),
                Some(strings(&["dingo", "flamingo", "goose"]))
            );

            // Enter text to rebuild the options without delay.
            delay.set(None);
            enter_text(tester, by_key(&field_key), "ngo");
            tester.pump();
            assert_eq!(
                last_options(&captured),
                Some(strings(&["dingo", "flamingo"]))
            );
        });
    }

    #[test]
    fn can_navigate_options_with_the_arrow_keys() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // Enter text. The options are filtered by the text.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| focus_node.request_focus(app, None));
            enter_text(tester, by_key(&field_key), "ele");
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), strings(&["chameleon", "elephant"]));

            // Move the highlighted option to the second item 'elephant' and select it.
            arrow_down(tester);
            // Can't use the key event for enter to submit to the text field using the test
            // framework, so this appears to be the equivalent.
            receive_action(tester, by_key(&field_key), TextInputAction::Done);
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(text_of(tester, &controller), "elephant");

            // Modify the field text. The options appear again and are filtered.
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().clear(app);
            });
            enter_text(tester, by_key(&field_key), "e");
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(
                captured.options(),
                strings(&[
                    "chameleon",
                    "elephant",
                    "goose",
                    "lemur",
                    "mouse",
                    "northern white rhinoceros"
                ])
            );

            // Selection does not wrap (going up).
            arrow_up(tester);
            tester.pump();
            assert_eq!(captured.last_highlighted.get(), 0);
            arrow_up(tester);
            tester.pump();
            assert_eq!(captured.last_highlighted.get(), 0);

            // Selection does not wrap (going down).
            for expected in [1, 2, 3, 4, 5, 5] {
                arrow_down(tester);
                tester.pump();
                assert_eq!(captured.last_highlighted.get(), expected);
            }
        });
    }

    #[test]
    fn can_jump_to_ends_with_keyboard() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // Enter text. The options are filtered by the text.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().clear(app);
            });
            enter_text(tester, by_key(&field_key), "e");
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options().len(), 6);

            // Jump to the bottom.
            with_control(
                tester,
                PhysicalKeyboardKey::ARROW_DOWN,
                LogicalKeyboardKey::ARROW_DOWN,
            );
            tester.pump();
            assert_eq!(captured.last_highlighted.get(), 5);

            // Doesn't wrap down.
            with_control(
                tester,
                PhysicalKeyboardKey::ARROW_DOWN,
                LogicalKeyboardKey::ARROW_DOWN,
            );
            tester.pump();
            assert_eq!(captured.last_highlighted.get(), 5);

            // Jump to the top.
            with_control(
                tester,
                PhysicalKeyboardKey::ARROW_UP,
                LogicalKeyboardKey::ARROW_UP,
            );
            tester.pump();
            assert_eq!(captured.last_highlighted.get(), 0);

            // Doesn't wrap up.
            with_control(
                tester,
                PhysicalKeyboardKey::ARROW_UP,
                LogicalKeyboardKey::ARROW_UP,
            );
            tester.pump();
            assert_eq!(captured.last_highlighted.get(), 0);
        });
    }

    #[test]
    fn can_navigate_with_page_up_down_keys() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // Enter text. The options are filtered by the text.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| {
                focus_node.request_focus(app, None);
                controller.handle().clear(app);
            });
            enter_text(tester, by_key(&field_key), "e");
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options().len(), 6);

            // Jump down. Stops at the bottom and doesn't wrap.
            for expected in [4, 5, 5] {
                send_key(
                    tester,
                    PhysicalKeyboardKey::PAGE_DOWN,
                    LogicalKeyboardKey::PAGE_DOWN,
                );
                tester.pump();
                assert_eq!(captured.last_highlighted.get(), expected);
            }

            // Jump to the bottom and then jump up a page.
            for expected in [1, 0, 0] {
                send_key(
                    tester,
                    PhysicalKeyboardKey::PAGE_UP,
                    LogicalKeyboardKey::PAGE_UP,
                );
                tester.pump();
                assert_eq!(captured.last_highlighted.get(), expected);
            }
        });
    }

    #[test]
    fn can_hide_and_show_options_with_the_keyboard() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // Enter text. The options are filtered by the text.
            let (focus_node, controller) = (captured.focus_node(), captured.controller());
            act(tester, |app| focus_node.request_focus(app, None));
            enter_text(tester, by_key(&field_key), "ele");
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options(), strings(&["chameleon", "elephant"]));

            // Hide the options.
            escape(tester);
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            // Show the options again by pressing arrow keys.
            arrow_down(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_one_widget());
            escape(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());
            arrow_up(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_one_widget());
            escape(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            // Show the options again by re-focusing the field.
            act(tester, |app| {
                focus_node.unfocus(app, UnfocusDisposition::Scope)
            });
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());
            act(tester, |app| focus_node.request_focus(app, None));
            tester.pump();
            expect(tester, &by_key(&options_key), finds_one_widget());
            escape(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            // Show the options again by editing the text (but not when selecting text or
            // moving the caret).
            enter_text(tester, by_key(&field_key), "elep");
            tester.pump();
            expect(tester, &by_key(&options_key), finds_one_widget());
            escape(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());
            act(tester, |app| {
                controller
                    .handle()
                    .set_selection(app, TextSelection::collapsed(3, TextAffinity::Downstream))
            });
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());
        });
    }

    #[test]
    fn re_invokes_dismiss_intent_if_options_not_shown() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let wrapping_action_invoked = Rc::new(Cell::new(false));
            let invoked = Rc::clone(&wrapping_action_invoked);
            let dismiss = tester.update(|app| {
                CallbackAction::<DismissIntent>::new(
                    app,
                    Rc::new(move |_app, _intent| {
                        invoked.set(true);
                        None
                    }),
                )
            });
            tester.pump_widget(test_widgets_app(Actions::new(
                HashMap::from([(TypeId::of::<DismissIntent>(), dismiss.into_erased())]),
                string_autocomplete(&captured, &field_key, &options_key),
            )));

            // Enter text to show options.
            let focus_node = captured.focus_node();
            act(tester, |app| focus_node.request_focus(app, None));
            enter_text(tester, by_key(&field_key), "ele");
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());

            // Hide the options.
            escape(tester);
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());
            assert!(!wrapping_action_invoked.get());

            // Ensure the wrapping Actions can receive the DismissIntent.
            escape(tester);
            assert!(wrapping_action_invoked.get());
        });
    }

    #[test]
    fn options_view_builders_can_use_autocomplete_highlighted_option_to_highlight_selected_option()
    {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // Enter text. The options are filtered by the text.
            let focus_node = captured.focus_node();
            act(tester, |app| focus_node.request_focus(app, None));
            enter_text(tester, by_key(&field_key), "e");
            tester.pump();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(captured.options().len(), 6);

            // Move the highlighted option down and check the highlighted index.
            assert_eq!(captured.last_highlighted.get(), 0);
            for expected in [1, 2, 3, 4, 5] {
                arrow_down(tester);
                tester.pump();
                assert_eq!(captured.last_highlighted.get(), expected);
            }

            // And move it back up.
            for expected in [4, 3, 2, 1, 0] {
                arrow_up(tester);
                tester.pump();
                assert_eq!(captured.last_highlighted.get(), expected);
            }

            // Arrow keys do not wrap.
            arrow_up(tester);
            tester.pump();
            assert_eq!(captured.last_highlighted.get(), 0);
        });
    }

    #[test]
    fn floating_menu_goes_away_on_select() {
        // Regression test for https://github.com/flutter/flutter/issues/99749.
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // The field is always rendered, but the options are not unless needed.
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            enter_text(tester, by_key(&field_key), K_OPTIONS[0]);
            tester.pump_and_settle();
            expect(tester, &by_key(&options_key), finds_one_widget());

            // Pretend that the only option is selected. This does not change the text in the
            // text field.
            let on_selected = captured.on_selected();
            act(tester, |app| on_selected(app, K_OPTIONS[0].to_string()));
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());
        });
    }

    #[test]
    fn options_width_matches_field_width_after_rebuilding() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let setter: CapturedSetter = Rc::default();
            let width = Rc::new(Cell::new(100.0));
            let captured = Captured::<String>::new();

            let (stored, sized, recorded) =
                (Rc::clone(&setter), Rc::clone(&width), Rc::clone(&captured));
            let (field, options) = (field_key.clone(), options_key.clone());
            tester.pump_widget(test_widgets_app(
                Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                    Padding::new(EdgeInsetsGeometry::symmetric(0.0, 32.0)).child(
                        StatefulBuilder::new(move |_app, _context, set_state| {
                            *stored.borrow_mut() = Some(set_state);
                            SizedBox::new()
                                .width(sized.get())
                                .child(string_autocomplete(&recorded, &field, &options))
                                .into_widget()
                        }),
                    ),
                ),
            ));

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            assert_eq!(tester.get_size(by_key(&field_key)).width(), 100.0);

            tester.tap(find::by_type::<TestTextField>());
            tester.pump();

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());

            assert_eq!(tester.get_size(by_key(&options_key)).width(), 100.0);

            set_state(tester, &setter, || width.set(200.0));
            tester.pump();

            // The options width changes to match the field width.
            assert_eq!(tester.get_size(by_key(&field_key)).width(), 200.0);
            assert_eq!(tester.get_size(by_key(&options_key)).width(), 200.0);
        });
    }

    #[test]
    fn options_width_matches_field_width_after_changing() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let setter: CapturedSetter = Rc::default();
            let width = Rc::new(Cell::new(100.0));
            let captured = Captured::<String>::new();

            let (stored, sized) = (Rc::clone(&setter), Rc::clone(&width));
            let field = field_key.clone();
            let autocomplete =
                RawAutocomplete::new(options_view(&captured, &options_key), string_options)
                    .field_view_builder(
                        move |_app, _context, controller, focus_node, _on_submitted| {
                            let (stored, sized, field) =
                                (Rc::clone(&stored), Rc::clone(&sized), field.clone());
                            StatefulBuilder::new(move |_app, _context, set_state| {
                                *stored.borrow_mut() = Some(set_state);
                                SizedBox::new()
                                    .width(sized.get())
                                    .child(
                                        TestTextField::new()
                                            .key(&field)
                                            .focus_node(focus_node.clone())
                                            .controller(controller.clone()),
                                    )
                                    .into_widget()
                            })
                            .into_widget()
                        },
                    );

            tester.pump_widget(test_widgets_app(
                Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                    Padding::new(EdgeInsetsGeometry::symmetric(0.0, 32.0)).child(autocomplete),
                ),
            ));

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            assert_eq!(tester.get_size(by_key(&field_key)).width(), 100.0);

            tester.tap(find::by_type::<TestTextField>());
            tester.pump();

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());

            assert_eq!(tester.get_size(by_key(&field_key)).width(), 100.0);
            assert_eq!(tester.get_size(by_key(&options_key)).width(), 100.0);

            set_state(tester, &setter, || width.set(200.0));
            tester.pump();

            // The options width changes to match the field width.
            assert_eq!(tester.get_size(by_key(&field_key)).width(), 200.0);
            assert_eq!(tester.get_size(by_key(&options_key)).width(), 200.0);
        });
    }

    /// The `screen size` group's `pumpRawAutocomplete`.
    fn pump_raw_autocomplete(
        tester: &mut WidgetTester,
        field_key: &GlobalKey,
        options_key: &GlobalKey,
        options_view_open_direction: OptionsViewOpenDirection,
        alignment: AlignmentGeometry,
    ) {
        let field_key = field_key.clone();
        tester.pump_widget(test_widgets_app(
            Padding::new(EdgeInsetsGeometry::symmetric(0.0, 32.0)).child(
                Align::new().alignment(alignment).child(
                    RawAutocomplete::new(option_tiles(options_key), string_options)
                        .options_view_open_direction(options_view_open_direction)
                        .field_view_builder(
                            move |_app, _context, controller, focus_node, _on_submitted| {
                                TestTextField::new()
                                    .key(&field_key)
                                    .focus_node(focus_node)
                                    .controller(controller)
                                    .into_widget()
                            },
                        ),
                ),
            ),
        ));
    }

    /// Dart's `tester.view.physicalSize = size; tester.view.devicePixelRatio = 1.0;`.
    fn set_window_size(tester: &WidgetTester, size: Size) {
        tester.view().set_physical_size(size);
        tester.view().set_device_pixel_ratio(1.0);
    }

    #[test]
    fn options_when_screen_changes_landscape_to_portrait() {
        test_widgets(|tester| {
            // Start with a portrait-sized window, with enough space for all of the options.
            let wide_window_size = Size::new(1920.0, 1080.0);
            set_window_size(tester, wide_window_size);

            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();

            pump_raw_autocomplete(
                tester,
                &field_key,
                &options_key,
                OptionsViewOpenDirection::Down,
                AlignmentGeometry::TOP_LEFT,
            );

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            tester.tap(find::by_type::<TestTextField>());
            tester.pump_and_settle();

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(K_OPTIONS.len()),
            );

            let field_size1 = tester.get_size(by_key(&field_key));
            let field_top_left1 = tester.get_top_left(by_key(&field_key));
            let options_top_left1 = tester.get_top_left(by_key(&options_key));
            assert_eq!(
                options_top_left1,
                Offset::new(
                    field_top_left1.dx(),
                    field_top_left1.dy() + field_size1.height()
                )
            );
            let options_bottom_right1 = tester.get_bottom_right(by_key(&options_key));
            let option_height = tester
                .get_size(find::by_type::<OptionTile>().first())
                .height();
            assert_eq!(
                options_bottom_right1,
                Offset::new(
                    field_top_left1.dx() + field_size1.width(),
                    field_top_left1.dy()
                        + field_size1.height()
                        + option_height * K_OPTIONS.len() as f64
                )
            );

            // Change the screen size to portrait.
            let narrow_window_size = Size::new(1070.0, 1770.0);
            set_window_size(tester, narrow_window_size);
            tester.pump_and_settle();

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(K_OPTIONS.len()),
            );
            assert_eq!(tester.get_top_left(by_key(&options_key)), options_top_left1);
            let field_size2 = tester.get_size(by_key(&field_key));
            assert!(field_size1.width() > field_size2.width());
            assert_eq!(field_size1.height(), field_size2.height());
            let options_bottom_right2 = tester.get_bottom_right(by_key(&options_key));
            let field_top_left2 = tester.get_top_left(by_key(&field_key));
            assert!(options_bottom_right2.dx() < options_bottom_right1.dx());
            assert_eq!(options_bottom_right2.dy(), options_bottom_right1.dy());
            assert_eq!(
                options_bottom_right2,
                Offset::new(
                    field_top_left2.dx() + field_size2.width(),
                    field_top_left2.dy()
                        + field_size2.height()
                        + option_height * K_OPTIONS.len() as f64
                )
            );
        });
    }

    #[test]
    fn options_when_screen_changes_portrait_to_landscape_and_overflows() {
        test_widgets(|tester| {
            // Start with a portrait-sized window, with enough space for all of the options.
            let narrow_window_size = Size::new(1070.0, 1770.0);
            set_window_size(tester, narrow_window_size);

            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();

            pump_raw_autocomplete(
                tester,
                &field_key,
                &options_key,
                OptionsViewOpenDirection::Down,
                AlignmentGeometry::TOP_LEFT,
            );

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            tester.tap(find::by_type::<TestTextField>());
            tester.pump_and_settle();

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(K_OPTIONS.len()),
            );

            let field_size1 = tester.get_size(by_key(&field_key));
            let field_top_left1 = tester.get_top_left(by_key(&field_key));
            let options_top_left1 = tester.get_top_left(by_key(&options_key));
            assert_eq!(
                options_top_left1,
                Offset::new(
                    field_top_left1.dx(),
                    field_top_left1.dy() + field_size1.height()
                )
            );
            let options_bottom_right1 = tester.get_bottom_right(by_key(&options_key));
            let option_height = tester
                .get_size(find::by_type::<OptionTile>().first())
                .height();
            assert_eq!(
                options_bottom_right1,
                Offset::new(
                    field_top_left1.dx() + field_size1.width(),
                    field_top_left1.dy()
                        + field_size1.height()
                        + option_height * K_OPTIONS.len() as f64
                )
            );

            // Change the screen size to landscape where the options can't all fit on the
            // screen.
            let wide_window_size = Size::new(1920.0, 580.0);
            set_window_size(tester, wide_window_size);
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());

            let visible_options = (wide_window_size.height() / option_height).floor() as usize;
            assert!(visible_options < K_OPTIONS.len());
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(visible_options),
            );
            assert_eq!(tester.get_top_left(by_key(&options_key)), options_top_left1);
            let field_size2 = tester.get_size(by_key(&field_key));
            assert!(field_size1.width() < field_size2.width());
            assert_eq!(field_size1.height(), field_size2.height());
            let options_bottom_right2 = tester.get_bottom_right(by_key(&options_key));
            let field_top_left2 = tester.get_top_left(by_key(&field_key));
            assert!(options_bottom_right2.dx() > options_bottom_right1.dx());
            assert!(options_bottom_right2.dy() < options_bottom_right1.dy());
            assert_eq!(
                options_bottom_right2,
                Offset::new(
                    field_top_left2.dx() + field_size2.width(),
                    // Options are taking all available space below the field.
                    wide_window_size.height()
                )
            );
        });
    }

    #[test]
    fn screen_changes_portrait_to_landscape_and_overflows() {
        test_widgets(|tester| {
            // Start with a portrait-sized window, with enough space for all of the options.
            let narrow_window_size = Size::new(1070.0, 1770.0);
            set_window_size(tester, narrow_window_size);

            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();

            pump_raw_autocomplete(
                tester,
                &field_key,
                &options_key,
                OptionsViewOpenDirection::Down,
                AlignmentGeometry::TOP_LEFT,
            );

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            tester.tap(find::by_type::<TestTextField>());
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());

            let option_height = tester
                .get_size(find::by_type::<OptionTile>().first())
                .height();
            let options_height1 = tester.get_size(by_key(&options_key)).height();
            let visible_options1 = (options_height1 / option_height).ceil() as usize;
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(visible_options1),
            );
            let field_size1 = tester.get_size(by_key(&field_key));
            let options_top_left1 = tester.get_top_left(by_key(&options_key));
            let field_top_left1 = tester.get_top_left(by_key(&field_key));
            assert_eq!(
                options_top_left1,
                Offset::new(
                    field_top_left1.dx(),
                    field_top_left1.dy() + field_size1.height()
                )
            );
            let options_bottom_right1 = tester.get_bottom_right(by_key(&options_key));
            assert_eq!(
                options_bottom_right1,
                Offset::new(
                    field_top_left1.dx() + field_size1.width(),
                    field_top_left1.dy() + field_size1.height() + options_height1
                )
            );

            // Change the screen size to landscape where the options can't all fit on the
            // screen.
            let wide_window_size = Size::new(1920.0, 580.0);
            set_window_size(tester, wide_window_size);
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());

            let options_height2 = tester.get_size(by_key(&options_key)).height();
            let visible_options2 = (options_height2 / option_height).ceil() as usize;
            assert!(visible_options2 < K_OPTIONS.len());
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(visible_options2),
            );
            let options_top_left2 = tester.get_top_left(by_key(&options_key));
            assert_eq!(options_top_left2, options_top_left1);
            let field_size2 = tester.get_size(by_key(&field_key));
            assert!(field_size1.width() < field_size2.width());
            assert_eq!(field_size1.height(), field_size2.height());
            let options_bottom_right2 = tester.get_bottom_right(by_key(&options_key));
            let field_top_left2 = tester.get_top_left(by_key(&field_key));
            assert!(options_bottom_right2.dx() > options_bottom_right1.dx());
            assert_eq!(
                options_bottom_right2,
                Offset::new(
                    field_top_left2.dx() + field_size2.width(),
                    // Options are taking all available space below the field.
                    wide_window_size.height()
                )
            );

            // Shrink the screen further so that the options become smaller than
            // kMinInteractiveDimension and move to overlap the field.
            let short_window_size = Size::new(1920.0, 48.0);
            set_window_size(tester, short_window_size);
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());

            const VISIBLE_OPTIONS3: usize = 1;
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(VISIBLE_OPTIONS3),
            );
            let options_top_left3 = tester.get_top_left(by_key(&options_key));
            assert_eq!(options_top_left3.dx(), options_top_left1.dx());
            // The options have moved up, overlapping the field, to still be able to show
            // kMinInteractiveDimension.
            assert!(options_top_left3.dy() < options_top_left1.dy());
            let field_size3 = tester.get_size(by_key(&field_key));
            let field_top_left3 = tester.get_top_left(by_key(&field_key));
            assert!(options_top_left3.dy() < field_top_left3.dy() + field_size3.height());
            assert_eq!(field_size3.width(), field_size2.width());
            assert_eq!(field_size1.height(), field_size3.height());
            let options_bottom_right3 = tester.get_bottom_right(by_key(&options_key));
            assert!(options_bottom_right3.dx() > options_bottom_right1.dx());
            assert_eq!(
                options_bottom_right3,
                Offset::new(
                    field_top_left3.dx() + field_size3.width(),
                    short_window_size.height()
                )
            );
        });
    }

    #[test]
    fn when_opening_up_screen_changes_portrait_to_landscape_and_overflows() {
        test_widgets(|tester| {
            // Start with a portrait-sized window, with enough space for all of the options.
            let narrow_window_size = Size::new(1070.0, 1770.0);
            set_window_size(tester, narrow_window_size);

            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();

            pump_raw_autocomplete(
                tester,
                &field_key,
                &options_key,
                OptionsViewOpenDirection::Up,
                AlignmentGeometry::CENTER,
            );

            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            tester.tap(find::by_type::<TestTextField>());
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_one_widget());

            let option_height = tester
                .get_size(find::by_type::<OptionTile>().first())
                .height();
            let options_height1 = tester.get_size(by_key(&options_key)).height();
            let visible_options1 = (options_height1 / option_height).ceil() as usize;
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(visible_options1),
            );
            let field_size1 = tester.get_size(by_key(&field_key));
            let options_top_left1 = tester.get_top_left(by_key(&options_key));
            let field_top_left1 = tester.get_top_left(by_key(&field_key));
            assert_eq!(
                options_top_left1,
                Offset::new(field_top_left1.dx(), field_top_left1.dy() - options_height1)
            );
            assert!(options_top_left1.dy() > 0.0);
            let options_bottom_right1 = tester.get_bottom_right(by_key(&options_key));
            assert_eq!(
                options_bottom_right1,
                Offset::new(
                    field_top_left1.dx() + field_size1.width(),
                    field_top_left1.dy()
                )
            );

            // Change the screen size to landscape where the options can't all fit on the
            // screen.
            let wide_window_size = Size::new(1920.0, 580.0);
            set_window_size(tester, wide_window_size);
            tester.pump_and_settle();
            expect(tester, &by_key(&field_key), finds_one_widget());

            let options_height2 = tester.get_size(by_key(&options_key)).height();
            assert!(options_height2 < options_height1);
            let visible_options2 = (options_height2 / option_height).ceil() as usize;
            assert!(visible_options2 < visible_options1);
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(visible_options2),
            );
            let options_top_left2 = tester.get_top_left(by_key(&options_key));
            let field_top_left2 = tester.get_top_left(by_key(&field_key));
            assert_eq!(
                options_top_left2,
                Offset::new(
                    options_top_left1.dx(),
                    field_top_left2.dy() - options_height2
                )
            );
            let field_size2 = tester.get_size(by_key(&field_key));
            assert!(field_size1.width() < field_size2.width());
            assert_eq!(field_size1.height(), field_size2.height());
            let options_bottom_right2 = tester.get_bottom_right(by_key(&options_key));
            assert!(options_bottom_right2.dx() > options_bottom_right1.dx());
            assert_eq!(
                options_bottom_right2,
                Offset::new(
                    field_top_left2.dx() + field_size2.width(),
                    field_top_left2.dy()
                )
            );

            // Shrink the screen further so that the options become smaller than
            // kMinInteractiveDimension and move to overlap the field.
            let short_window_size = Size::new(1920.0, 90.0);
            set_window_size(tester, short_window_size);
            tester.pump_and_settle();

            expect(tester, &by_key(&field_key), finds_one_widget());
            const VISIBLE_OPTIONS3: usize = 1;
            expect(
                tester,
                &find::by_type::<OptionTile>(),
                finds_n_widgets(VISIBLE_OPTIONS3),
            );
            let options_top_left3 = tester.get_top_left(by_key(&options_key));
            assert_eq!(options_top_left3.dx(), options_top_left1.dx());
            // The options have moved down, overlapping the field, to still be able to show
            // kMinInteractiveDimension.
            assert!(options_top_left3.dy() < options_top_left1.dy());
            let field_size3 = tester.get_size(by_key(&field_key));
            let field_top_left3 = tester.get_top_left(by_key(&field_key));
            assert!(options_top_left3.dy() < field_top_left3.dy() + field_size3.height());
            assert_eq!(field_size3.width(), field_size2.width());
            assert_eq!(field_size1.height(), field_size3.height());
            let options_bottom_right3 = tester.get_bottom_right(by_key(&options_key));
            assert!(options_bottom_right3.dx() > options_bottom_right1.dx());
            assert!(options_bottom_right3.dy() > field_top_left3.dy());
            assert_eq!(
                options_bottom_right3.dx(),
                field_top_left3.dx() + field_size3.width()
            );
        });
    }

    #[test]
    fn can_prevent_older_options_builder_results_from_replacing_the_new_ones() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let delay: Rc<Cell<Option<Duration>>> = Rc::default();
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(
                    options_view(&captured, &options_key),
                    delayed_options(&delay),
                )
                .field_view_builder(field_view(&captured, &field_key)),
            ));

            let long_request_delay = Duration::from_millis(5000);
            let short_request_delay = Duration::from_millis(1000);
            let focus_node = captured.focus_node();
            act(tester, |app| {
                focus_node.request_focus(app, None);

                // Enter the first letter.
                delay.set(Some(long_request_delay));
            });
            enter_text(tester, by_key(&field_key), "c");
            tester.pump();
            assert_eq!(last_options(&captured), None);

            // Enter the second letter which resolves faster.
            delay.set(Some(short_request_delay));
            enter_text(tester, by_key(&field_key), "ch");
            tester.pump();
            assert_eq!(last_options(&captured), None);

            // Wait for the short request to resolve.
            tester.pump_by(short_request_delay);

            // lastOptions must contain results from the last request.
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(last_options(&captured), Some(strings(&["chameleon"])));

            // Wait for the last timer to finish.
            tester.pump_by(long_request_delay - short_request_delay);
            assert_eq!(last_options(&captured), Some(strings(&["chameleon"])));
        });
    }

    #[test]
    fn updates_result_only_from_the_last_call_made_to_options_builder() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let delay: Rc<Cell<Option<Duration>>> = Rc::default();
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(
                    options_view(&captured, &options_key),
                    delayed_options(&delay),
                )
                .field_view_builder(field_view(&captured, &field_key)),
            ));

            let focus_node = captured.focus_node();
            let first_request_delay = Duration::from_millis(1000);
            let second_request_delay = Duration::from_millis(2000);
            let third_request_delay = Duration::from_millis(3000);
            act(tester, |app| {
                focus_node.request_focus(app, None);

                // Enter the first letter.
                delay.set(Some(first_request_delay));
            });
            enter_text(tester, by_key(&field_key), "l");
            tester.pump();
            assert_eq!(last_options(&captured), None);

            // Enter the second letter which resolves slower.
            delay.set(Some(second_request_delay));
            enter_text(tester, by_key(&field_key), "le");
            tester.pump();
            assert_eq!(last_options(&captured), None);

            // Enter the third letter which resolves the slowest.
            delay.set(Some(third_request_delay));
            enter_text(tester, by_key(&field_key), "lem");
            tester.pump();
            assert_eq!(last_options(&captured), None);

            // lastOptions should get updated only from the last request.
            tester.pump_by(first_request_delay);
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(last_options(&captured), None);

            tester.pump_by(second_request_delay - first_request_delay);
            expect(tester, &by_key(&options_key), finds_nothing());
            assert_eq!(last_options(&captured), None);

            tester.pump_by(third_request_delay - second_request_delay);
            expect(tester, &by_key(&options_key), finds_one_widget());
            assert_eq!(last_options(&captured), Some(strings(&["lemur"])));
        });
    }

    #[test]
    fn update_options_view_when_field_input_changes_return_to_the_starting_keyword() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let delay = Duration::from_millis(100);
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(
                    options_view(&captured, &options_key),
                    move |app, value: TextEditingValue| delayed(app, delay, matching(&value.text)),
                )
                .field_view_builder(field_view(&captured, &field_key)),
            ));

            // Setup starting point.
            enter_text(tester, by_key(&field_key), "ele");
            tester.pump_by(delay);
            assert_eq!(
                last_options(&captured),
                Some(strings(&["chameleon", "elephant"]))
            );
            expect(tester, &by_key(&options_key), finds_one_widget());

            // Hide options.
            escape(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            // Enter another letter and then immediately erase it.
            enter_text(tester, by_key(&field_key), "eleo");
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            enter_text(tester, by_key(&field_key), "ele");
            tester.pump_by(delay);
            assert_eq!(
                last_options(&captured),
                Some(strings(&["chameleon", "elephant"]))
            );

            // Options dropdown should be visible after the last optionsBuilder call is
            // resolved.
            expect(tester, &by_key(&options_key), finds_one_widget());
        });
    }

    #[test]
    fn options_builder_does_not_have_to_be_a_pure_function() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            let delay = Duration::from_millis(100);

            // This is used to tell optionsBuilder to return something different after being
            // called with "ele" the second time. I.e. it is not a pure function.
            let times_options_builder_called_with_ele = Rc::new(Cell::new(0));
            let alt_ele_options = strings(&["something new and crazy for ele!"]);

            let (times, alt) = (
                Rc::clone(&times_options_builder_called_with_ele),
                alt_ele_options.clone(),
            );
            tester.pump_widget(test_widgets_app(
                RawAutocomplete::new(
                    options_view(&captured, &options_key),
                    move |app, value: TextEditingValue| {
                        if value.text == "ele" {
                            times.set(times.get() + 1);
                            if times.get() > 1 {
                                return delayed(app, delay, alt.clone());
                            }
                        }
                        delayed(app, delay, matching(&value.text))
                    },
                )
                .field_view_builder(field_view(&captured, &field_key)),
            ));

            enter_text(tester, by_key(&field_key), "ele");
            tester.pump_by(delay);
            assert_eq!(
                last_options(&captured),
                Some(strings(&["chameleon", "elephant"]))
            );
            expect(tester, &by_key(&options_key), finds_one_widget());

            escape(tester);
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            enter_text(tester, by_key(&field_key), "eleo");
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            enter_text(tester, by_key(&field_key), "ele");
            tester.pump_by(delay);
            assert_eq!(last_options(&captured), Some(alt_ele_options));
            expect(tester, &by_key(&options_key), finds_one_widget());
        });
    }

    #[test]
    fn raw_autocomplete_renders_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(test_widgets_app(
                Center::new().child(
                    SizedBox::shrink().child(
                        RawAutocomplete::new(
                            |_app, _context, _on_selected, _options: &[String]| {
                                Container::new().into_widget()
                            },
                            |_app, _value| Task::ready(strings(&["Y"])),
                        )
                        .initial_value(TextEditingValue::new().text("X"))
                        .field_view_builder(
                            |_app, _context, controller, _focus_node, _on_submitted| {
                                TestTextField::new().controller(controller).into_widget()
                            },
                        ),
                    ),
                ),
            ));
            let x_text = find::text("X");
            assert!(tester.get_size(x_text).is_empty());
        });
    }

    #[test]
    fn floating_menu_is_displayed_when_the_field_is_focused_and_already_has_a_selected_value() {
        // Regression test for https://github.com/flutter/flutter/issues/177429.
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let captured = Captured::<String>::new();
            tester.pump_widget(test_widgets_app(string_autocomplete(
                &captured,
                &field_key,
                &options_key,
            )));

            // The field is always rendered, but the options are not unless needed.
            expect(tester, &by_key(&field_key), finds_one_widget());
            expect(tester, &by_key(&options_key), finds_nothing());

            enter_text(tester, by_key(&field_key), K_OPTIONS[0]);
            tester.pump_and_settle();
            expect(tester, &by_key(&options_key), finds_one_widget());

            let on_selected = captured.on_selected();
            act(tester, |app| on_selected(app, K_OPTIONS[0].to_string()));
            tester.pump();
            expect(tester, &by_key(&options_key), finds_nothing());

            let focus_node = captured.focus_node();
            act(tester, |app| {
                focus_node.unfocus(app, UnfocusDisposition::Scope)
            });
            tester.pump();
            act(tester, |app| focus_node.request_focus(app, None));
            tester.pump_and_settle();

            expect(tester, &by_key(&options_key), finds_one_widget());
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/170403.
    #[test]
    fn does_not_crash_when_disposed_during_async_options_builder() {
        test_widgets(|tester| {
            let options_completer: Completer<Vec<String>> = Completer::new();
            let setter: CapturedSetter = Rc::default();
            let show_autocomplete = Rc::new(Cell::new(true));

            let (stored, shown, completer) = (
                Rc::clone(&setter),
                Rc::clone(&show_autocomplete),
                options_completer.clone(),
            );
            tester.pump_widget(test_widgets_app(StatefulBuilder::new(
                move |_app, _context, set_state| {
                    *stored.borrow_mut() = Some(set_state);
                    if !shown.get() {
                        return SizedBox::shrink().into_widget();
                    }
                    let completer = completer.clone();
                    RawAutocomplete::new(
                        |_app, _context, _on_selected, _options: &[String]| {
                            Container::new().into_widget()
                        },
                        move |app, _value| {
                            let future = completer.future();
                            app.spawn(async move |_cx| future.await)
                        },
                    )
                    .field_view_builder(|_app, _context, controller, focus_node, _on_submitted| {
                        TestTextField::new()
                            .focus_node(focus_node)
                            .controller(controller)
                            .into_widget()
                    })
                    .into_widget()
                },
            )));

            // Focus the field to trigger optionsBuilder.
            tester.tap(find::by_type::<TestTextField>());
            tester.pump();

            // Dispose the widget while optionsBuilder is still pending.
            set_state(tester, &setter, || show_autocomplete.set(false));
            tester.pump();

            act(tester, |app| {
                options_completer.complete(app, strings(&["option1", "option2"]))
            });
            tester.pump();
        });
    }

    #[test]
    fn options_view_is_constrained_by_media_query_bottom_padding_and_view_insets() {
        test_widgets(|tester| {
            const PADDING: f64 = 50.0;
            const BOTTOM_INSETS: f64 = 70.0;
            tester.view().set_device_pixel_ratio(1.0);
            tester.pump_widget(test_widgets_app(
                Column::new().children([
                    SizedBox::new().height(200.0).into_widget(),
                    RawAutocomplete::new(
                        |_app, _context, _on_selected, _options: &[String]| {
                            Placeholder::new().into_widget()
                        },
                        |_app, _value| Task::ready(strings(&["abcd"])),
                    )
                    .field_view_builder(|_app, _context, controller, focus_node, _on_submitted| {
                        TestTextField::new()
                            .controller(controller)
                            .focus_node(focus_node)
                            .into_widget()
                    })
                    .into_widget(),
                    SizedBox::new().height(200.0).into_widget(),
                ]),
            ));

            tester.tap(find::by_type::<TestTextField>());
            enter_text(tester, find::by_type::<TestTextField>(), "a");
            tester.pump();

            let initial_size = tester.get_size(find::by_type::<Placeholder>()).height();
            tester.view().set_padding(ViewPadding {
                bottom: PADDING,
                ..ViewPadding::ZERO
            });
            tester.pump();

            // The options view has shrunk to the available height.
            assert_more_or_less_equals_within(
                tester.get_size(find::by_type::<Placeholder>()).height(),
                initial_size - PADDING,
                0.1,
            );

            tester.view().set_view_insets(ViewPadding {
                bottom: BOTTOM_INSETS,
                ..ViewPadding::ZERO
            });
            tester.pump();

            tester.tap(find::by_type::<TestTextField>());
            enter_text(tester, find::by_type::<TestTextField>(), "abc");
            tester.pump();

            // The options view has shrunk to the available height.
            assert_more_or_less_equals_within(
                tester.get_size(find::by_type::<Placeholder>()).height(),
                initial_size - BOTTOM_INSETS - PADDING,
                0.1,
            );
        });
    }

    #[test]
    fn options_view_is_constrained_by_media_query_top_padding() {
        test_widgets(|tester| {
            const PADDING: f64 = 100.0;
            tester.view().set_device_pixel_ratio(1.0);
            tester.pump_widget(test_widgets_app(
                Column::new().children([
                    SizedBox::new().height(200.0).into_widget(),
                    RawAutocomplete::new(
                        |_app, _context, _on_selected, _options: &[String]| {
                            Placeholder::new().into_widget()
                        },
                        |_app, _value| Task::ready(strings(&["abcd"])),
                    )
                    .options_view_open_direction(OptionsViewOpenDirection::Up)
                    .field_view_builder(|_app, _context, controller, focus_node, _on_submitted| {
                        TestTextField::new()
                            .controller(controller)
                            .focus_node(focus_node)
                            .into_widget()
                    })
                    .into_widget(),
                    SizedBox::new().height(200.0).into_widget(),
                ]),
            ));

            tester.tap(find::by_type::<TestTextField>());
            enter_text(tester, find::by_type::<TestTextField>(), "a");
            tester.pump();

            let initial_size = tester.get_size(find::by_type::<Placeholder>()).height();
            tester.view().set_padding(ViewPadding {
                top: PADDING,
                ..ViewPadding::ZERO
            });
            tester.pump();

            // The options view has shrunk to the available height.
            assert_more_or_less_equals_within(
                tester.get_size(find::by_type::<Placeholder>()).height(),
                initial_size - PADDING,
                0.1,
            );
        });
    }

    #[test]
    fn raw_autocomplete_does_not_crash_when_hiding_non_displayed_overlay() {
        test_widgets(|tester| {
            let (controller, focus_node) = own_field(tester);
            let autocomplete = |key: &'static str| {
                RawAutocomplete::new(
                    |_app, _context, _on_selected, _options: &[String]| {
                        SizedBox::new().into_widget()
                    },
                    |_app, _value| Task::ready(Vec::new()),
                )
                .key(waft_foundation::ValueKey::new(key).into())
                .text_editing_controller(controller.erase())
                .focus_node(focus_node.erase())
                .field_view_builder(
                    |_app, _context, controller, focus_node, _on_submitted| {
                        TestTextField::new()
                            .controller(controller)
                            .focus_node(focus_node)
                            .into_widget()
                    },
                )
            };

            tester.pump_widget(test_widgets_app(autocomplete("initial")));

            // Change the key to force the RawAutocomplete to dispose and re-initialize.
            tester.pump_widget(test_widgets_app(autocomplete("new-key")));

            enter_text(tester, find::by_type::<TestTextField>(), "trigger");
            tester.pump();
        });
    }

    #[test]
    fn same_option_in_raw_autocomplete_should_be_selectable_again_after_text_is_cleared() {
        test_widgets(|tester| {
            let (text_ctrl, text_focus) = own_field(tester);
            let list_item = strings(&["test", "abc", "dexter"]);

            let cleared: TextEditingControllerRef = text_ctrl.erase();
            tester.pump_widget(test_widgets_app(
                Row::new().children([
                    Expanded::new(
                        RawAutocomplete::new(
                            |_app,
                             _context,
                             on_selected: AutocompleteOnSelected<String>,
                             options: &[String]| {
                                let options = options.to_vec();
                                let count = options.len() as i32;
                                ListView::builder(move |_app, _context, index| {
                                    let option = options[index as usize].clone();
                                    let on_selected = Rc::clone(&on_selected);
                                    let tapped = option.clone();
                                    Some(
                                        GestureDetector::new()
                                            .on_tap(Listener::new(move |app| {
                                                on_selected(app, tapped.clone())
                                            }))
                                            .child(Text::new(option))
                                            .into_widget(),
                                    )
                                })
                                .item_count(count)
                                .build()
                                .into_widget()
                            },
                            move |_app, value: TextEditingValue| {
                                Task::ready(
                                    list_item
                                        .iter()
                                        .filter(|e| e.contains(&value.text))
                                        .cloned()
                                        .collect(),
                                )
                            },
                        )
                        .text_editing_controller(text_ctrl.erase())
                        .focus_node(text_focus.erase())
                        .field_view_builder(
                            |_app, _context, controller, focus_node, _on_submitted| {
                                TestTextField::new()
                                    .controller(controller)
                                    .focus_node(focus_node)
                                    .into_widget()
                            },
                        ),
                    )
                    .into_widget(),
                    GestureDetector::new()
                        .on_tap(Listener::new(move |app| cleared.handle().clear(app)))
                        .child(Text::new("Clear"))
                        .into_widget(),
                ]),
            ));

            // Open the popup menu.
            enter_text(tester, find::by_type::<TestTextField>(), "");
            tester.pump_and_settle();
            expect(tester, &find::text("test"), finds_one_widget());

            // Select option 'test'.
            tester.tap(find::text("test"));
            tester.pump_and_settle();
            let controller: TextEditingControllerRef = text_ctrl.erase();
            assert_eq!(text_of(tester, &controller), "test");

            // Clear text.
            tester.tap(find::text("Clear"));
            act(tester, |app| {
                focus_of(&text_focus).unfocus(app, UnfocusDisposition::Scope)
            });
            tester.pump_and_settle();
            assert_eq!(text_of(tester, &controller), "");

            // Select 'test' again.
            tester.tap(find::by_type::<TestTextField>());
            tester.pump_and_settle();
            expect(tester, &find::text("test"), finds_widgets());
            tester.tap(find::text("test").last());
            tester.pump_and_settle();

            // The text field should be updated to 'test'.
            assert_eq!(text_of(tester, &controller), "test");
        });
    }

    #[test]
    fn tab_from_field_skips_the_options_overlay_and_advances_to_the_next_focusable_sibling() {
        // Regression test for https://github.com/flutter/flutter/issues/183456.
        test_widgets(|tester| {
            let target_focus = tester.update(FocusNode::new);
            tester.pump_widget(test_widgets_app(
                Column::new().children([
                    RawAutocomplete::new(
                        |_app,
                         _context,
                         on_selected: AutocompleteOnSelected<String>,
                         options: &[String]| {
                            let options = options.to_vec();
                            let count = options.len() as i32;
                            ListView::builder(move |_app, _context, index| {
                                let option = options[index as usize].clone();
                                let on_selected = Rc::clone(&on_selected);
                                let tapped = option.clone();
                                Some(
                                    GestureDetector::new()
                                        .on_tap(Listener::new(move |app| {
                                            on_selected(app, tapped.clone())
                                        }))
                                        .child(Focus::new(Text::new(option)))
                                        .into_widget(),
                                )
                            })
                            .item_count(count)
                            .build()
                            .shrink_wrap(true)
                            .into_widget()
                        },
                        string_options,
                    )
                    .field_view_builder(|_app, _context, controller, focus_node, _on_submitted| {
                        TestTextField::new()
                            .controller(controller)
                            .focus_node(focus_node)
                            .into_widget()
                    })
                    .into_widget(),
                    ExcludeFocus::new(TestTextField::new()).into_widget(),
                    TestTextField::new()
                        .focus_node(target_focus.erase())
                        .into_widget(),
                ]),
            ));

            // Focus the autocomplete field by tapping it, then TAB.
            tester.tap(find::by_type::<TestTextField>().first());
            tester.pump_and_settle();
            send_key(tester, PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();

            assert!(tester.update(|app| focus_of(&target_focus).has_focus(app)));
        });
    }
}
