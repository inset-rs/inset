//! Flutter counterpart: material_ui's `autocomplete.dart`.

use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::Curves;
use waft_foundation::{App, Handle, Listener, Ref, Task, ValueKey, VoidCallback};
use waft_painting::EdgeInsetsGeometry;
use waft_rendering::BoxConstraints;
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_services::TextEditingValue;
use waft_widgets::{
    AutocompleteFieldViewBuilder, AutocompleteHighlightedOption, AutocompleteOnSelected,
    AutocompleteOptionToString, AutocompleteOptionsBuilder, AutocompleteOptionsViewBuilder,
    BuildContext, Builder, ConstrainedBox, Container, FocusNodeRef, GlobalKey, IntoWidget, KeyRef,
    ListView, OptionsViewOpenDirection, RawAutocomplete, ScrollController, ScrollControllerLeaf,
    ScrollPositionAlignmentPolicy, Scrollable, Semantics, State, StateData, StatefulWidget,
    StatelessWidget, Text, TextEditingControllerRef, WidgetRef,
};

use crate::ink_well::InkWell;
use crate::material::Material;
use crate::text_form_field::TextFormField;
use crate::theme::Theme;

/// Creates a Material-styled autocomplete widget: a text field the user enters some text in,
/// and a list of options to choose from.
///
/// The user's text input is received in a field built with
/// [`field_view_builder`](Self::field_view_builder). The options to be displayed are
/// determined using [`options_builder`](Self::options_builder) and rendered with
/// [`options_view_builder`](Self::options_view_builder), as [`RawAutocomplete`] does, which is
/// what this widget is built upon and which contains more detailed documentation.
///
/// The type parameter `T` represents the type of the options. Most commonly this is a
/// `String`. However, it's also possible to use another type with a `Display` form, or a
/// custom [`display_string_for_option`](Self::display_string_for_option). Options will be
/// compared using `==`.
///
/// See also:
///
///  * [`RawAutocomplete`], which is what Autocomplete is built upon, and which contains more
///    detailed examples.
pub struct Autocomplete<T: Clone + PartialEq + Hash + Debug + 'static> {
    pub key: Option<KeyRef>,

    /// Returns the string to display in the field when the option is selected.
    ///
    /// This is useful when using a custom T type and the string to display is different than
    /// the string to search by.
    ///
    /// If not provided, will use the option's `Display` form.
    pub display_string_for_option: AutocompleteOptionToString<T>,

    /// Builds the field whose input is used to get the options.
    ///
    /// Pass the provided [`TextEditingControllerRef`] to the field built here so that
    /// RawAutocomplete can listen for changes.
    ///
    /// If not provided, will build a standard Material-style text field by default.
    pub field_view_builder: AutocompleteFieldViewBuilder,

    /// The [`FocusNodeRef`] that is used for the text field.
    ///
    /// For more information, see [`RawAutocomplete::focus_node`].
    ///
    /// If this parameter is not `None`, then
    /// [`text_editing_controller`](Self::text_editing_controller) must also be non-`None`.
    pub focus_node: Option<FocusNodeRef>,

    /// Called when an option is selected by the user.
    pub on_selected: Option<AutocompleteOnSelected<T>>,

    /// A function that returns the current selectable options objects given the current
    /// TextEditingValue.
    pub options_builder: AutocompleteOptionsBuilder<T>,

    /// Builds the selectable options widgets from a list of options objects.
    ///
    /// The options are displayed floating below or above the field inside of an `Overlay`, not
    /// at the same place in the widget tree as [`Autocomplete`]. To control whether it opens
    /// upward or downward, use
    /// [`options_view_open_direction`](Self::options_view_open_direction).
    ///
    /// If not provided, will build a standard Material-style list of results by default.
    pub options_view_builder: Option<AutocompleteOptionsViewBuilder<T>>,

    /// Determines the direction in which to open the options view.
    ///
    /// Defaults to [`OptionsViewOpenDirection::Down`].
    pub options_view_open_direction: OptionsViewOpenDirection,

    /// The maximum height used for the default Material options list widget.
    ///
    /// When [`options_view_builder`](Self::options_view_builder) is `None`, this property sets
    /// the maximum height that the options widget can occupy.
    ///
    /// The default value is set to 200.
    pub options_max_height: f64,

    /// The [`TextEditingControllerRef`] that is used for the text field.
    ///
    /// For more information, see [`RawAutocomplete::text_editing_controller`].
    ///
    /// If this parameter is not `None`, then [`focus_node`](Self::focus_node) must also be
    /// non-`None`.
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

impl<T: Clone + PartialEq + Hash + Debug + Display + 'static> Autocomplete<T> {
    /// Creates an instance of [`Autocomplete`].
    ///
    /// [`display_string_for_option`](Self::display_string_for_option) defaults to the option's
    /// `Display` form; an option type without one is built with
    /// [`new_with_display_string`](Self::new_with_display_string).
    pub fn new(
        options_builder: impl Fn(&mut App, TextEditingValue) -> Task<Vec<T>> + 'static,
    ) -> Autocomplete<T> {
        Autocomplete::new_with_display_string(options_builder, |option: &T| {
            RawAutocomplete::<T>::default_string_for_option(option)
        })
    }
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> Autocomplete<T> {
    /// Creates an instance of [`Autocomplete`] whose options are shown as
    /// `display_string_for_option` gives them, for an option type with or without a `Display`
    /// form.
    pub fn new_with_display_string(
        options_builder: impl Fn(&mut App, TextEditingValue) -> Task<Vec<T>> + 'static,
        display_string_for_option: impl Fn(&T) -> String + 'static,
    ) -> Autocomplete<T> {
        Autocomplete {
            key: None,
            display_string_for_option: Rc::new(display_string_for_option),
            field_view_builder: Rc::new(Self::default_field_view_builder),
            focus_node: None,
            on_selected: None,
            options_builder: Rc::new(options_builder),
            options_view_builder: None,
            options_view_open_direction: OptionsViewOpenDirection::Down,
            options_max_height: 200.0,
            text_editing_controller: None,
            initial_value: None,
        }
    }

    /// Dart `Autocomplete(key:)`.
    pub fn key(mut self, key: KeyRef) -> Autocomplete<T> {
        self.key = Some(key);
        self
    }

    /// Dart `Autocomplete(displayStringForOption:)`.
    pub fn display_string_for_option(
        mut self,
        display_string_for_option: impl Fn(&T) -> String + 'static,
    ) -> Autocomplete<T> {
        self.display_string_for_option = Rc::new(display_string_for_option);
        self
    }

    /// Dart `Autocomplete(fieldViewBuilder:)`.
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
    ) -> Autocomplete<T> {
        self.field_view_builder = Rc::new(field_view_builder);
        self
    }

    /// Dart `Autocomplete(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> Autocomplete<T> {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `Autocomplete(onSelected:)`.
    pub fn on_selected(mut self, on_selected: impl Fn(&mut App, T) + 'static) -> Autocomplete<T> {
        self.on_selected = Some(Rc::new(on_selected));
        self
    }

    /// Dart `Autocomplete(optionsViewBuilder:)`.
    pub fn options_view_builder(
        mut self,
        options_view_builder: impl Fn(
            &mut App,
            BuildContext,
            AutocompleteOnSelected<T>,
            &[T],
        ) -> WidgetRef
        + 'static,
    ) -> Autocomplete<T> {
        self.options_view_builder = Some(Rc::new(options_view_builder));
        self
    }

    /// Dart `Autocomplete(optionsViewOpenDirection:)`.
    pub fn options_view_open_direction(
        mut self,
        options_view_open_direction: OptionsViewOpenDirection,
    ) -> Autocomplete<T> {
        self.options_view_open_direction = options_view_open_direction;
        self
    }

    /// Dart `Autocomplete(optionsMaxHeight:)`.
    pub fn options_max_height(mut self, options_max_height: f64) -> Autocomplete<T> {
        self.options_max_height = options_max_height;
        self
    }

    /// Dart `Autocomplete(textEditingController:)`.
    pub fn text_editing_controller(
        mut self,
        text_editing_controller: TextEditingControllerRef,
    ) -> Autocomplete<T> {
        self.text_editing_controller = Some(text_editing_controller);
        self
    }

    /// Dart `Autocomplete(initialValue:)`.
    pub fn initial_value(mut self, initial_value: TextEditingValue) -> Autocomplete<T> {
        self.initial_value = Some(initial_value);
        self
    }

    fn default_field_view_builder(
        _app: &mut App,
        _context: BuildContext,
        text_editing_controller: TextEditingControllerRef,
        focus_node: FocusNodeRef,
        on_field_submitted: VoidCallback,
    ) -> WidgetRef {
        AutocompleteField {
            focus_node,
            text_editing_controller,
            on_field_submitted,
        }
        .into_widget()
    }
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> Debug for Autocomplete<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Autocomplete")
            .field(
                "options_view_open_direction",
                &self.options_view_open_direction,
            )
            .field("options_max_height", &self.options_max_height)
            .field("focus_node", &self.focus_node)
            .field("text_editing_controller", &self.text_editing_controller)
            .field("initial_value", &self.initial_value)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> StatelessWidget for Autocomplete<T> {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let display_string_for_option = Rc::clone(&self.display_string_for_option);
        let field_view_builder = Rc::clone(&self.field_view_builder);
        let options_builder = Rc::clone(&self.options_builder);
        let options_view_open_direction = self.options_view_open_direction;
        let options_max_height = self.options_max_height;
        let options_view_builder: AutocompleteOptionsViewBuilder<T> =
            match self.options_view_builder.clone() {
                Some(options_view_builder) => options_view_builder,
                None => {
                    let display_string_for_option = Rc::clone(&display_string_for_option);
                    Rc::new(move |_app, _context, on_selected, options| {
                        AutocompleteOptions {
                            display_string_for_option: Rc::clone(&display_string_for_option),
                            on_selected,
                            open_direction: options_view_open_direction,
                            options: options.to_vec(),
                            options_max_height,
                        }
                        .into_widget()
                    })
                }
            };
        let mut raw_autocomplete = RawAutocomplete::<T>::new_with_display_string(
            move |app, context, on_selected, options| {
                options_view_builder(app, context, on_selected, options)
            },
            move |app, text_editing_value| options_builder(app, text_editing_value),
            move |option| display_string_for_option(option),
        )
        .field_view_builder(
            move |app, context, text_editing_controller, focus_node, on_field_submitted| {
                field_view_builder(
                    app,
                    context,
                    text_editing_controller,
                    focus_node,
                    on_field_submitted,
                )
            },
        )
        .options_view_open_direction(options_view_open_direction);
        if let Some(focus_node) = self.focus_node.clone() {
            raw_autocomplete = raw_autocomplete.focus_node(focus_node);
        }
        if let Some(text_editing_controller) = self.text_editing_controller.clone() {
            raw_autocomplete = raw_autocomplete.text_editing_controller(text_editing_controller);
        }
        if let Some(initial_value) = self.initial_value.clone() {
            raw_autocomplete = raw_autocomplete.initial_value(initial_value);
        }
        if let Some(on_selected) = self.on_selected.clone() {
            raw_autocomplete = raw_autocomplete.on_selected(move |app, option| {
                on_selected(app, option);
            });
        }
        raw_autocomplete.into_widget()
    }
}

/// Dart's `_AutocompleteField`: the default Material-style Autocomplete text field.
struct AutocompleteField {
    focus_node: FocusNodeRef,
    on_field_submitted: VoidCallback,
    text_editing_controller: TextEditingControllerRef,
}

impl Debug for AutocompleteField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutocompleteField")
            .field("focus_node", &self.focus_node)
            .field("text_editing_controller", &self.text_editing_controller)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for AutocompleteField {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let on_field_submitted = Rc::clone(&self.on_field_submitted);
        TextFormField::new()
            .controller(self.text_editing_controller.clone())
            .focus_node(self.focus_node.clone())
            .on_field_submitted(move |app, _value| {
                on_field_submitted(app);
            })
            .into_widget()
    }
}

/// Dart's `_AutocompleteOptions`: the default Material-style Autocomplete options.
struct AutocompleteOptions<T: Clone + PartialEq + Hash + Debug + 'static> {
    display_string_for_option: AutocompleteOptionToString<T>,
    on_selected: AutocompleteOnSelected<T>,
    open_direction: OptionsViewOpenDirection,
    options: Vec<T>,
    options_max_height: f64,
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> Debug for AutocompleteOptions<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutocompleteOptions")
            .field("open_direction", &self.open_direction)
            .field("options", &self.options)
            .field("options_max_height", &self.options_max_height)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> StatelessWidget for AutocompleteOptions<T> {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let highlighted_index = AutocompleteHighlightedOption::of(app, context);

        Material::new()
            .elevation(4.0)
            .child(
                ConstrainedBox::new(BoxConstraints::new().max_height(self.options_max_height))
                    .child(AutocompleteOptionsList {
                        display_string_for_option: Rc::clone(&self.display_string_for_option),
                        highlighted_index,
                        on_selected: Rc::clone(&self.on_selected),
                        options: self.options.clone(),
                    }),
            )
            .into_widget()
    }
}

/// Dart's `_AutocompleteOptionsList`.
struct AutocompleteOptionsList<T: Clone + PartialEq + Hash + Debug + 'static> {
    display_string_for_option: AutocompleteOptionToString<T>,
    highlighted_index: usize,
    on_selected: AutocompleteOnSelected<T>,
    options: Vec<T>,
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> Debug for AutocompleteOptionsList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutocompleteOptionsList")
            .field("highlighted_index", &self.highlighted_index)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> StatefulWidget for AutocompleteOptionsList<T> {
    type State = AutocompleteOptionsListState<T>;

    fn create_state(&self) -> AutocompleteOptionsListState<T> {
        AutocompleteOptionsListState {
            state: StateData::new(),
            scroll_controller: None,
        }
    }
}

/// Dart's `_AutocompleteOptionsListState`.
struct AutocompleteOptionsListState<T: Clone + PartialEq + Hash + Debug + 'static> {
    state: StateData<AutocompleteOptionsList<T>>,
    /// Dart's `final ScrollController _scrollController = ScrollController()`, made in
    /// `init_state`, which has the `App` a constructor needs.
    scroll_controller: Option<Ref<ScrollController>>,
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> AutocompleteOptionsListState<T> {
    fn scroll_controller(self: Handle<Self>, app: &App) -> Ref<ScrollController> {
        app.get(self)
            .scroll_controller
            .clone()
            .expect("made in init_state")
    }

    /// Dart's `GlobalObjectKey(option)`: Rust has no identity for a value, so the key
    /// compares and hashes as the option does.
    fn option_key(option: &T) -> GlobalKey {
        GlobalKey::wrapping(Rc::new(ValueKey::new(option.clone())))
    }
}

impl<T: Clone + PartialEq + Hash + Debug + 'static> State for AutocompleteOptionsListState<T> {
    type Widget = AutocompleteOptionsList<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let scroll_controller = ScrollController::default(app);
        app.get_mut(self).scroll_controller = Some(scroll_controller);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &AutocompleteOptionsList<T>,
    ) {
        if self.widget(app).highlighted_index != old_widget.highlighted_index {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _time_stamp| {
                    if !self.mounted(app) {
                        return;
                    }
                    let widget = self.widget(app);
                    let highlighted_index = widget.highlighted_index;
                    let highlighted_key = Self::option_key(&widget.options[highlighted_index]);
                    match highlighted_key.current_context(app) {
                        None => {
                            let scroll_controller = self.scroll_controller(app).handle();
                            let offset = if highlighted_index == 0 {
                                0.0
                            } else {
                                scroll_controller.position(app).max_scroll_extent(app)
                            };
                            scroll_controller.jump_to(app, offset);
                        }
                        Some(highlighted_context) => {
                            drop(Scrollable::ensure_visible(
                                app,
                                highlighted_context,
                                0.5,
                                Duration::ZERO,
                                Curves::ease(),
                                ScrollPositionAlignmentPolicy::Explicit,
                            ));
                        }
                    }
                }),
                None,
            );
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.scroll_controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let highlighted_index = AutocompleteHighlightedOption::of(app, context);
        let item_count = self.widget(app).options.len();
        let scroll_controller = self.scroll_controller(app).erase();

        ListView::builder(move |app, _context, index| {
            let option = self.widget(app).options[index as usize].clone();
            let highlight = highlighted_index == index as usize;
            Some(
                Semantics::new()
                    .button(true)
                    .selected(highlight)
                    .child(
                        InkWell::new()
                            .key(Self::option_key(&option).into())
                            .on_tap({
                                let option = option.clone();
                                Listener::new(move |app| {
                                    let on_selected = Rc::clone(&self.widget(app).on_selected);
                                    on_selected(app, option.clone());
                                })
                            })
                            .child(Builder::new(move |app, context| {
                                let display_string_for_option =
                                    Rc::clone(&self.widget(app).display_string_for_option);
                                let mut container = Container::new()
                                    .padding(EdgeInsetsGeometry::all(16.0))
                                    .child(Text::new(display_string_for_option(&option)));
                                if highlight {
                                    container =
                                        container.color(Theme::of(app, context).focus_color);
                                }
                                container.into_widget()
                            })),
                    )
                    .into_widget(),
            )
        })
        .item_count(item_count as i32)
        .build()
        .padding(EdgeInsetsGeometry::ZERO)
        .shrink_wrap(true)
        .controller(scroll_controller)
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{Color, KeyEventType, Size, TextAffinity, TextSelection};
    use waft_rendering::testing::{CanvasMethod, PaintCall, assert_paints, count_calls};
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey, TextInputClient};
    use waft_widgets::testing::{
        WidgetTester, assert_f32_equals, assert_more_or_less_equals, expect, find, finds_nothing,
        finds_one_widget, test_widgets,
    };
    use waft_widgets::{
        Align, AlignmentGeometry, AnyElement, Center, Column, EditableText, EditableTextState,
        Expanded, FocusNode, Icon, ScrollView, SizedBox, StatefulBuilder, TextEditingController,
        UnfocusDisposition,
    };

    use super::*;
    use crate::icon_button::IconButton;
    use crate::icons::Icons;
    use crate::list_tile::ListTile;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};
    use crate::text_field::TextField;
    use crate::theme_data::ThemeData;

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

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct User {
        email: &'static str,
        name: &'static str,
    }

    impl Display for User {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}, {}", self.name, self.email)
        }
    }

    const K_OPTIONS_USERS: [User; 3] = [
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
    ];

    const HIGHLIGHT_COLOR: Color = Color::new(0xFF112233);

    /// Dart's `kOptions.where((option) => option.contains(text.toLowerCase()))`.
    fn matching(text: &str) -> Vec<String> {
        let query = text.to_lowercase();
        K_OPTIONS
            .iter()
            .filter(|option| option.contains(&query))
            .map(|option| option.to_string())
            .collect()
    }

    fn string_options(_app: &mut App, value: TextEditingValue) -> Task<Vec<String>> {
        Task::ready(matching(&value.text))
    }

    fn user_options(_app: &mut App, value: TextEditingValue) -> Task<Vec<User>> {
        let query = value.text.to_lowercase();
        Task::ready(
            K_OPTIONS_USERS
                .iter()
                .filter(|option| option.to_string().contains(&query))
                .cloned()
                .collect(),
        )
    }

    fn string_autocomplete() -> Autocomplete<String> {
        Autocomplete::new(string_options)
    }

    /// Dart's `MaterialApp(theme: theme, home: Scaffold(body: body))`.
    fn scaffold_app<K>(
        tester: &WidgetTester,
        theme: ThemeData,
        body: impl IntoWidget<K>,
    ) -> WidgetRef {
        let _ = tester;
        material_app(theme, Scaffold::new().body(body).into_widget())
    }

    fn default_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    fn highlight_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .focus_color(HIGHLIGHT_COLOR)
            .build(&tester.cell.borrow())
    }

    fn pump_in_scaffold<K>(tester: &mut WidgetTester, body: impl IntoWidget<K>) {
        let app = scaffold_app(tester, default_theme(tester), body);
        tester.pump_widget(app);
    }

    fn pump_in_highlight_scaffold<K>(tester: &mut WidgetTester, body: impl IntoWidget<K>) {
        let app = scaffold_app(tester, highlight_theme(tester), body);
        tester.pump_widget(app);
    }

    /// The end of one event of the case: the checkpoint runs what it left.
    fn act<R>(tester: &mut WidgetTester, body: impl FnOnce(&mut App) -> R) -> R {
        let result = tester.update(body);
        tester.cell.checkpoint();
        result
    }

    fn editable_state(tester: &WidgetTester) -> Handle<EditableTextState> {
        let element: AnyElement = tester.element(find::descendant(
            find::by_type::<TextFormField>(),
            find::by_type::<EditableText>(),
        ));
        element
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state")
    }

    /// Dart's `tester.enterText(find.byType(TextFormField), text)`.
    fn enter_text(tester: &mut WidgetTester, text: &str) {
        let state = editable_state(tester);
        state.request_keyboard(&mut tester.cell.borrow_mut());
        tester.pump();
        let length = text.encode_utf16().count() as i32;
        let value = TextEditingValue::new()
            .text(text)
            .selection(TextSelection::collapsed(length, TextAffinity::Downstream));
        TextInputClient::update_editing_value(state, &mut tester.cell.borrow_mut(), value);
        tester.cell.checkpoint();
    }

    /// Dart's `tester.sendKeyDownEvent(key)` or `sendKeyUpEvent(key)`.
    fn send_key_down_or_up(
        tester: &mut WidgetTester,
        physical: PhysicalKeyboardKey,
        logical: LogicalKeyboardKey,
        event_type: KeyEventType,
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

    fn arrow_down(tester: &mut WidgetTester) {
        tester.send_key_event(
            PhysicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ARROW_DOWN,
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
            KeyEventType::Down,
        );
        tester.send_key_event(physical, logical);
        send_key_down_or_up(
            tester,
            PhysicalKeyboardKey::CONTROL_LEFT,
            LogicalKeyboardKey::CONTROL_LEFT,
            KeyEventType::Up,
        );
    }

    /// Dart's `find.byType(TextFormField).evaluate().first.widget as TextFormField`'s
    /// `controller!`.
    fn field_controller(tester: &WidgetTester) -> TextEditingControllerRef {
        tester.widget(find::by_type::<TextFormField>(), |field: &TextFormField| {
            field
                .controller
                .clone()
                .expect("the field has a controller")
        })
    }

    /// Dart's `field.controller!.text`.
    fn field_text(tester: &WidgetTester) -> String {
        controller_text(tester, &field_controller(tester))
    }

    fn controller_text(tester: &WidgetTester, controller: &TextEditingControllerRef) -> String {
        controller
            .handle()
            .text_value(&tester.cell.borrow())
            .to_string()
    }

    /// Dart's `list.semanticChildCount` of the one `ListView`.
    fn semantic_child_count(tester: &WidgetTester) -> i32 {
        tester.widget(find::by_type::<ListView>(), |list: &ListView| {
            list.scroll_view_data()
                .semantic_child_count
                .expect("a builder list has a semantic child count")
        })
    }

    /// Dart's `list.height` of `tester.getSize(find.byType(ListView))`.
    fn options_height(tester: &WidgetTester) -> f64 {
        tester.get_size(find::by_type::<ListView>()).height()
    }

    /// Dart's `checkOptionHighlight`: the option's `Container` paints its background `color`,
    /// or only the paragraph when `color` is `None`.
    fn check_option_highlight(tester: &WidgetTester, label: &str, color: Option<Color>) {
        let container = find::ancestor(find::text(label), find::by_type::<Container>());
        let ops = tester.painted_ops(tester.render_object(container));
        match color {
            Some(color) => assert_paints(&ops, &[PaintCall::rect(Some(color))]),
            None => {
                assert_eq!(count_calls(&ops, CanvasMethod::DrawRect), 0);
                assert_paints(&ops, &[PaintCall::Paragraph]);
            }
        }
    }

    /// Dart's `finder.hitTestable()` for the one widget `finder` finds: a hit test at its centre
    /// reaches its render box.
    fn hit_testable(tester: &WidgetTester, finder: waft_widgets::testing::Finder) -> bool {
        let render = tester.render_object(finder.clone());
        let render = render.as_box().expect("a render box");
        let center = tester.get_rect(finder).center();
        let mut app = tester.cell.borrow_mut();
        let view_id = app.platform().views()[0].id();
        let mut result = waft_gestures::HitTestResult::new();
        waft_gestures::GestureBinding::instance(&mut app).hit_test_in_view(
            &mut app,
            &mut result,
            center,
            view_id,
        );
        result.path().iter().any(|entry| {
            (entry.target() as &dyn std::any::Any)
                .downcast_ref::<waft_rendering::BoxHitTestEntry>()
                .is_some_and(|entry| entry.target() == render)
        })
    }

    #[test]
    fn can_filter_and_select_a_list_of_string_options() {
        test_widgets(|tester| {
            let last_selection = Rc::new(RefCell::new(String::new()));
            let selected = Rc::clone(&last_selection);
            pump_in_scaffold(
                tester,
                string_autocomplete().on_selected(move |_app, selection| {
                    *selected.borrow_mut() = selection;
                }),
            );

            // The field is always rendered, but the options are not unless needed.
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());

            // Focus the empty field. All the options are displayed.
            tester.tap(find::by_type::<TextFormField>());
            tester.pump();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            assert_eq!(semantic_child_count(tester), K_OPTIONS.len() as i32);

            // Enter text. The options are filtered by the text.
            enter_text(tester, "ele");
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            // 'chameleon' and 'elephant' are displayed.
            assert_eq!(semantic_child_count(tester), 2);

            // Select a option. The options hide and the field updates to show the selection.
            tester.tap(find::by_type::<InkWell>().first());
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());
            assert_eq!(field_text(tester), "chameleon");
            assert_eq!(*last_selection.borrow(), "chameleon");

            // Modify the field text. The options appear again and are filtered.
            enter_text(tester, "e");
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            // 'chameleon', 'elephant', 'goose', 'lemur', 'mouse', and 'northern white
            // rhinoceros' are displayed.
            assert_eq!(semantic_child_count(tester), 6);
        });
    }

    #[test]
    fn can_filter_and_select_a_list_of_custom_user_options() {
        test_widgets(|tester| {
            pump_in_scaffold(tester, Autocomplete::<User>::new(user_options));

            // The field is always rendered, but the options are not unless needed.
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());

            // Focus the empty field. All the options are displayed.
            tester.tap(find::by_type::<TextFormField>());
            tester.pump();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            assert_eq!(semantic_child_count(tester), K_OPTIONS_USERS.len() as i32);

            // Enter text. The options are filtered by the text.
            enter_text(tester, "example");
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            // 'Alice' and 'Bob' are displayed because they have "example.com" emails.
            assert_eq!(semantic_child_count(tester), 2);

            // Select a option. The options hide and the field updates to show the selection.
            tester.tap(find::by_type::<InkWell>().first());
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());
            assert_eq!(field_text(tester), "Alice, alice@example.com");

            // Modify the field text. The options appear again and are filtered.
            enter_text(tester, "B");
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            // 'Bob' is displayed.
            assert_eq!(semantic_child_count(tester), 1);
        });
    }

    #[test]
    fn display_string_for_option_is_displayed_in_the_options() {
        test_widgets(|tester| {
            pump_in_scaffold(
                tester,
                Autocomplete::<User>::new(user_options)
                    .display_string_for_option(|option: &User| option.name.to_string()),
            );

            // The field is always rendered, but the options are not unless needed.
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());

            // Focus the empty field. All the options are displayed, and the string that is used
            // comes from displayStringForOption.
            tester.tap(find::by_type::<TextFormField>());
            tester.pump();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            assert_eq!(semantic_child_count(tester), K_OPTIONS_USERS.len() as i32);
            for user in &K_OPTIONS_USERS {
                expect(tester, &find::text(user.name), finds_one_widget());
            }

            // Select a option. The options hide and the field updates to show the selection.
            // The text in the field is given by displayStringForOption.
            tester.tap(find::by_type::<InkWell>().first());
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());
            assert_eq!(field_text(tester), K_OPTIONS_USERS[0].name);
        });
    }

    /// Not a Dart case: an option type with no `Display` form is shown through the function the
    /// second constructor takes.
    #[test]
    fn an_option_type_without_display_is_shown_by_its_display_string() {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        struct Animal(&'static str);

        test_widgets(|tester| {
            pump_in_scaffold(
                tester,
                Autocomplete::new_with_display_string(
                    |_app, _value| Task::ready(vec![Animal("aardvark"), Animal("bobcat")]),
                    |animal: &Animal| animal.0.to_uppercase(),
                ),
            );

            tester.tap(find::by_type::<TextFormField>());
            tester.pump();
            expect(tester, &find::text("AARDVARK"), finds_one_widget());
            expect(tester, &find::text("BOBCAT"), finds_one_widget());

            tester.tap(find::by_type::<InkWell>().first());
            tester.pump();
            assert_eq!(field_text(tester), "AARDVARK");
        });
    }

    #[test]
    fn can_build_a_custom_field() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let key = field_key.clone();
            pump_in_scaffold(
                tester,
                string_autocomplete().field_view_builder(move |_app, _context, _c, _f, _s| {
                    Container::new().key(key.clone().into()).into_widget()
                }),
            );

            // The custom field is rendered and not the default TextFormField.
            expect(tester, &find::by_global_key(&field_key), finds_one_widget());
            expect(tester, &find::by_type::<TextFormField>(), finds_nothing());
        });
    }

    #[test]
    fn can_build_custom_options() {
        test_widgets(|tester| {
            let options_key = GlobalKey::new();
            let key = options_key.clone();
            pump_in_scaffold(
                tester,
                string_autocomplete().options_view_builder(
                    move |_app, _context, _on_selected, _options| {
                        Container::new().key(key.clone().into()).into_widget()
                    },
                ),
            );

            // The default field is rendered but not the options, yet.
            expect(tester, &find::by_global_key(&options_key), finds_nothing());
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );

            // Focus the empty field. The custom options is displayed.
            tester.tap(find::by_type::<TextFormField>());
            tester.pump();
            expect(
                tester,
                &find::by_global_key(&options_key),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn the_default_autocomplete_options_widget_has_a_maximum_height_of_200() {
        test_widgets(|tester| {
            pump_in_scaffold(tester, string_autocomplete());

            tester.tap(find::by_type::<TextFormField>());
            enter_text(tester, "");
            tester.pump();
            assert_f32_equals(options_height(tester), 200.0);
        });
    }

    #[test]
    fn the_options_height_restricts_to_max_desired_height() {
        test_widgets(|tester| {
            const DESIRED_HEIGHT: f64 = 150.0;
            pump_in_scaffold(
                tester,
                string_autocomplete().options_max_height(DESIRED_HEIGHT),
            );

            // Entering "a" returns 9 items from kOptions so basically the height of 9 options
            // would be beyond `desiredHeight=150`, so height gets restricted to desiredHeight.
            tester.tap(find::by_type::<TextFormField>());
            enter_text(tester, "a");
            tester.pump();
            assert_f32_equals(options_height(tester), DESIRED_HEIGHT);
        });
    }

    #[test]
    fn the_height_of_options_shrinks_to_height_of_resulting_items_if_less_than_max_height() {
        test_widgets(|tester| {
            // Dart's `getDefaultOptionsHeight`: the height of the default options after the
            // provided text had been entered into the field.
            fn default_options_height(tester: &mut WidgetTester, entered_text: &str) -> f64 {
                let controller = field_controller(tester);
                act(tester, |app| controller.handle().clear(app));
                tester.tap(find::by_type::<TextFormField>());
                enter_text(tester, entered_text);
                tester.pump();
                options_height(tester)
            }

            const MAX_OPTIONS_HEIGHT: f64 = 250.0;
            pump_in_scaffold(
                tester,
                string_autocomplete().options_max_height(MAX_OPTIONS_HEIGHT),
            );

            expect(tester, &find::by_type::<ListView>(), finds_nothing());

            // Entering `a` returns 9 items(height > `maxOptionsHeight`) from the kOptions so
            // height gets restricted to `maxOptionsHeight =250`.
            let nine_items_height = default_options_height(tester, "a");
            assert_f32_equals(nine_items_height, MAX_OPTIONS_HEIGHT);

            // Returns 2 Items (height < `maxOptionsHeight`) so options height shrinks to 2 Items
            // combined height.
            let two_items_height = default_options_height(tester, "el");
            assert!(two_items_height < MAX_OPTIONS_HEIGHT);

            // Returns 1 item (height < `maxOptionsHeight`) from `kOptions` so options height
            // shrinks to 1 items height.
            let one_items_height = default_options_height(tester, "elep");
            assert!(one_items_height < two_items_height);
        });
    }

    #[test]
    fn initial_value_sets_initial_text_field_value() {
        test_widgets(|tester| {
            let last_selection = Rc::new(RefCell::new(String::new()));
            let selected = Rc::clone(&last_selection);
            pump_in_scaffold(
                tester,
                string_autocomplete()
                    .initial_value(TextEditingValue::new().text("lem"))
                    .on_selected(move |_app, selection| {
                        *selected.borrow_mut() = selection;
                    }),
            );

            // The field is always rendered, but the options are not unless needed.
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());
            assert_eq!(field_text(tester), "lem");

            // Focus the empty field. All the options are displayed.
            tester.tap(find::by_type::<TextFormField>());
            tester.pump();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            // Displays just one option ('lemur').
            assert_eq!(semantic_child_count(tester), 1);

            // Select a option. The options hide and the field updates to show the selection.
            tester.tap(find::by_type::<InkWell>().first());
            tester.pump();
            expect(
                tester,
                &find::by_type::<TextFormField>(),
                finds_one_widget(),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());
            assert_eq!(field_text(tester), "lemur");
            assert_eq!(*last_selection.borrow(), "lemur");
        });
    }

    #[test]
    fn keyboard_navigation_of_the_options_properly_highlights_the_option() {
        test_widgets(|tester| {
            pump_in_highlight_scaffold(tester, string_autocomplete());

            tester.tap(find::by_type::<TextFormField>());
            enter_text(tester, "el");
            tester.pump();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            assert_eq!(semantic_child_count(tester), 2);

            // Initially the first option should be highlighted.
            check_option_highlight(tester, "chameleon", Some(HIGHLIGHT_COLOR));
            check_option_highlight(tester, "elephant", None);

            // Move the selection down.
            arrow_down(tester);
            tester.pump();

            // Highlight should be moved to the second item.
            check_option_highlight(tester, "chameleon", None);
            check_option_highlight(tester, "elephant", Some(HIGHLIGHT_COLOR));
        });
    }

    #[test]
    fn keyboard_navigation_keeps_the_highlighted_option_scrolled_into_view() {
        test_widgets(|tester| {
            pump_in_highlight_scaffold(tester, string_autocomplete());

            tester.tap(find::by_type::<TextFormField>());
            enter_text(tester, "e");
            tester.pump();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            assert_eq!(semantic_child_count(tester), 6);

            let options_group_rect = tester.get_rect(find::by_type::<ListView>());
            const OPTIONS_GROUP_PADDING: f64 = 16.0;

            // Highlighted item should be at the top.
            check_option_highlight(tester, "chameleon", Some(HIGHLIGHT_COLOR));
            assert_f32_equals(
                tester.get_top_left(find::text("chameleon")).dy(),
                options_group_rect.top + OPTIONS_GROUP_PADDING,
            );

            // Move down the list of options.
            arrow_down(tester); // Select 'elephant'.
            tester.pump_and_settle();
            arrow_down(tester); // Select 'goose'.
            tester.pump_and_settle();
            arrow_down(tester); // Select 'lemur'.
            tester.pump_and_settle();

            // Highlighted item 'lemur' should be centered in the options popup.
            check_option_highlight(tester, "lemur", Some(HIGHLIGHT_COLOR));
            assert_f32_equals(
                tester.get_center(find::text("lemur")).dy(),
                options_group_rect.center().dy(),
            );

            arrow_down(tester); // Select 'mouse'.
            tester.pump_and_settle();

            check_option_highlight(tester, "mouse", Some(HIGHLIGHT_COLOR));

            // First item should have scrolled off the top, and not be selected.
            expect(tester, &find::text("chameleon"), finds_nothing());

            // The other items on screen should not be selected.
            check_option_highlight(tester, "goose", None);
            check_option_highlight(tester, "lemur", None);
            check_option_highlight(tester, "northern white rhinoceros", None);
        });
    }

    fn one_option_autocomplete(option: &'static str) -> Autocomplete<String> {
        Autocomplete::new(move |_app, _value| Task::ready(vec![option.to_string()]))
    }

    fn raw_open_direction(tester: &WidgetTester) -> OptionsViewOpenDirection {
        tester.widget(
            find::by_type::<RawAutocomplete<String>>(),
            |raw: &RawAutocomplete<String>| raw.options_view_open_direction,
        )
    }

    #[test]
    fn options_view_open_direction_default_down() {
        test_widgets(|tester| {
            pump_in_scaffold(tester, one_option_autocomplete("a"));
            assert_eq!(raw_open_direction(tester), OptionsViewOpenDirection::Down);
        });
    }

    #[test]
    fn options_view_open_direction_down() {
        test_widgets(|tester| {
            pump_in_scaffold(
                tester,
                one_option_autocomplete("a")
                    .options_view_open_direction(OptionsViewOpenDirection::Down),
            );
            assert_eq!(raw_open_direction(tester), OptionsViewOpenDirection::Down);
        });
    }

    #[test]
    fn options_view_open_direction_up() {
        test_widgets(|tester| {
            pump_in_scaffold(
                tester,
                Center::new().child(
                    one_option_autocomplete("aa")
                        .options_view_open_direction(OptionsViewOpenDirection::Up),
                ),
            );
            assert_eq!(raw_open_direction(tester), OptionsViewOpenDirection::Up);

            tester.tap(find::by_type::<RawAutocomplete<String>>());
            enter_text(tester, "a");
            tester.pump();
            expect(tester, &find::text("aa"), finds_one_widget());
            assert!(hit_testable(tester, find::text("aa")));
        });
    }

    #[test]
    fn options_view_open_direction_automatic_opens_in_the_direction_with_more_space() {
        test_widgets(|tester| {
            let field_key = GlobalKey::new();
            let options_key = GlobalKey::new();
            let set_state: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let alignment = Rc::new(Cell::new(AlignmentGeometry::TOP_CENTER));

            let (stored, aligned) = (Rc::clone(&set_state), Rc::clone(&alignment));
            let (field, options) = (field_key.clone(), options_key.clone());
            pump_in_scaffold(
                tester,
                StatefulBuilder::new(move |_app, _context, setter| {
                    *stored.borrow_mut() = Some(setter);
                    let (field, options) = (field.clone(), options.clone());
                    Align::new()
                        .alignment(aligned.get())
                        .child(
                            Autocomplete::<String>::new(|_app, _value| {
                                Task::ready(vec!["a".to_string(), "b".to_string(), "c".to_string()])
                            })
                            .options_view_open_direction(OptionsViewOpenDirection::MostSpace)
                            .field_view_builder(
                                move |_app, _context, controller, focus_node, _on_submitted| {
                                    TextField::new()
                                        .key(field.clone().into())
                                        .controller(controller)
                                        .focus_node(focus_node)
                                        .into_widget()
                                },
                            )
                            .options_view_builder(
                                move |_app, _context, _on_selected, options_list: &[String]| {
                                    Material::new()
                                        .child(
                                            ListView::new().key(options.clone().into()).children(
                                                options_list.iter().map(|option| {
                                                    Text::new(option.clone()).into_widget()
                                                }),
                                            ),
                                        )
                                        .into_widget()
                                },
                            ),
                        )
                        .into_widget()
                }),
            );
            let change_alignment = |tester: &mut WidgetTester, to: AlignmentGeometry| {
                let setter = set_state.borrow().clone().expect("the builder ran");
                act(tester, |app| setter(app, &mut |_app| alignment.set(to)));
                tester.pump();
            };

            // Show the options. It should open downwards since there is more space.
            tester.tap(find::by_global_key(&field_key));
            tester.pump();

            assert_more_or_less_equals(
                tester.get_bottom_left(find::by_global_key(&field_key)),
                tester.get_top_left(find::by_global_key(&options_key)),
            );

            // Move the field to the bottom.
            change_alignment(tester, AlignmentGeometry::BOTTOM_CENTER);

            // The options should now open upwards, since there is more space above.
            assert_more_or_less_equals(
                tester.get_top_left(find::by_global_key(&field_key)),
                tester.get_bottom_left(find::by_global_key(&options_key)),
            );

            // Move the field to the center.
            change_alignment(tester, AlignmentGeometry::CENTER);

            // Show the options. It should open downwards since there is more space.
            assert_more_or_less_equals(
                tester.get_bottom_left(find::by_global_key(&field_key)),
                tester.get_top_left(find::by_global_key(&options_key)),
            );
        });
    }

    #[test]
    fn can_jump_to_options_that_are_not_yet_built() {
        test_widgets(|tester| {
            pump_in_highlight_scaffold(tester, string_autocomplete());

            tester.tap(find::by_type::<TextFormField>());
            tester.pump();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            assert_eq!(semantic_child_count(tester), K_OPTIONS.len() as i32);

            let option_finder = |index: usize| {
                find::ancestor(find::text(K_OPTIONS[index]), find::by_type::<Container>())
            };

            expect(tester, &option_finder(0), finds_one_widget());
            expect(tester, &option_finder(K_OPTIONS.len() - 1), finds_nothing());

            // Jump to the bottom.
            with_control(
                tester,
                PhysicalKeyboardKey::ARROW_DOWN,
                LogicalKeyboardKey::ARROW_DOWN,
            );
            tester.pump_and_settle();
            expect(tester, &option_finder(0), finds_nothing());
            expect(
                tester,
                &option_finder(K_OPTIONS.len() - 1),
                finds_one_widget(),
            );
            check_option_highlight(
                tester,
                K_OPTIONS[K_OPTIONS.len() - 1],
                Some(HIGHLIGHT_COLOR),
            );

            // Jump to the top.
            with_control(
                tester,
                PhysicalKeyboardKey::ARROW_UP,
                LogicalKeyboardKey::ARROW_UP,
            );
            tester.pump_and_settle();
            expect(tester, &option_finder(0), finds_one_widget());
            expect(tester, &option_finder(K_OPTIONS.len() - 1), finds_nothing());
            check_option_highlight(tester, K_OPTIONS[0], Some(HIGHLIGHT_COLOR));
        });
    }

    #[test]
    fn passes_text_editing_controller_focus_node_to_raw_autocomplete() {
        test_widgets(|tester| {
            let (text_editing_controller, focus_node) =
                tester.update(|app| (TextEditingController::new(app), FocusNode::new(app)));
            let controller: TextEditingControllerRef = text_editing_controller.erase();
            let node: FocusNodeRef = focus_node.erase();

            let home = Material::new()
                .child(
                    Center::new().child(
                        Autocomplete::<String>::new(|_app, _value| Task::ready(vec!["a".into()]))
                            .focus_node(node.clone())
                            .text_editing_controller(controller.clone()),
                    ),
                )
                .into_widget();
            let app = material_app(default_theme(tester), home);
            tester.pump_widget(app);

            let (raw_controller, raw_focus_node) = tester.widget(
                find::by_type::<RawAutocomplete<String>>(),
                |raw: &RawAutocomplete<String>| {
                    (raw.text_editing_controller.clone(), raw.focus_node.clone())
                },
            );
            assert_eq!(raw_controller, Some(controller));
            assert_eq!(raw_focus_node, Some(node));
        });
    }

    #[test]
    fn when_field_scrolled_offscreen_reshown_selected_value_when_scrolled_back() {
        test_widgets(|tester| {
            let (scroll_controller, text_editing_controller, focus_node) = tester.update(|app| {
                (
                    ScrollController::default(app),
                    TextEditingController::new(app),
                    FocusNode::new(app),
                )
            });
            let node: FocusNodeRef = focus_node.erase();
            let controller: TextEditingControllerRef = text_editing_controller.erase();

            pump_in_scaffold(
                tester,
                ListView::new()
                    .controller(scroll_controller.erase())
                    .children([
                        string_autocomplete()
                            .focus_node(node.clone())
                            .text_editing_controller(controller.clone())
                            .into_widget(),
                        SizedBox::new().height(1000.0).into_widget(),
                    ]),
            );

            // Select an option.
            tester.tap(find::by_type::<TextField>());
            tester.pump();
            const TEXT_SELECTION: &str = "chameleon";
            tester.tap(find::text(TEXT_SELECTION));

            // Unfocus and scroll to deconstruct the widget.
            act(tester, |app| {
                node.handle().unfocus(app, UnfocusDisposition::Scope);
                scroll_controller.handle().jump_to(app, 2000.0);
            });
            tester.pump_and_settle();

            // Scroll to go back to the widget.
            act(tester, |app| scroll_controller.handle().jump_to(app, 0.0));
            tester.pump_and_settle();

            // Checks that the option selected is still present.
            assert_eq!(controller_text(tester, &controller), TEXT_SELECTION);
        });
    }

    #[test]
    fn suggestions_are_hit_tested_before_list_tiles() {
        test_widgets(|tester| {
            let tiles: Vec<WidgetRef> = (0..3)
                .map(|i| {
                    ListTile::new()
                        .title(Text::new(format!("Item {i}")))
                        .on_tap(Listener::new(|_app| {}))
                        .into_widget()
                })
                .collect();
            let mut children: Vec<WidgetRef> = vec![
                Autocomplete::<String>::new(|_app, value| {
                    Task::ready(
                        ["Apple", "Banana", "Cherry"]
                            .iter()
                            .filter(|option| option.to_lowercase().contains(&value.text))
                            .map(|option| option.to_string())
                            .collect(),
                    )
                })
                .into_widget(),
            ];
            children.extend(tiles);
            pump_in_scaffold(tester, Column::new().children(children));

            tester.tap(find::by_type::<TextField>());
            tester.pump();

            let cherry_finder = find::text("Cherry");
            expect(tester, &cherry_finder, finds_one_widget());

            tester.tap(cherry_finder);
            tester.pump();

            expect(
                tester,
                &find::widget_with_text::<TextField>("Cherry"),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn renders_at_zero_area() {
        test_widgets(|tester| {
            let app = material_app(
                default_theme(tester),
                Center::new()
                    .child(
                        SizedBox::shrink().child(
                            Scaffold::new().body(
                                Autocomplete::<String>::new(|_app, _value| {
                                    Task::ready(vec!["Y".to_string()])
                                })
                                .initial_value(TextEditingValue::new().text("X")),
                            ),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(app);
            let x_text = find::text("X");
            assert_eq!(tester.get_size(x_text), Size::ZERO);
        });
    }

    #[test]
    fn same_option_should_be_selectable_again_after_text_is_cleared() {
        test_widgets(|tester| {
            let (text_ctrl, text_focus) =
                tester.update(|app| (TextEditingController::new(app), FocusNode::new(app)));
            let controller: TextEditingControllerRef = text_ctrl.erase();
            let focus: FocusNodeRef = text_focus.erase();
            let list_item = ["test", "abc", "dexter"];
            let clear_key: KeyRef = ValueKey::new("clear".to_string()).into();

            let clearing = controller.clone();
            pump_in_scaffold(
                tester,
                waft_widgets::Row::new().children([
                    Expanded::new(
                        Autocomplete::<String>::new(move |_app, value| {
                            Task::ready(
                                list_item
                                    .iter()
                                    .filter(|e| {
                                        e.to_lowercase().contains(&value.text.to_lowercase())
                                    })
                                    .map(|e| e.to_string())
                                    .collect(),
                            )
                        })
                        .text_editing_controller(controller.clone())
                        .focus_node(focus.clone()),
                    )
                    .into_widget(),
                    IconButton::new(
                        Some(Listener::new(move |app| clearing.handle().clear(app))),
                        Icon::new(Some(Icons::add())),
                    )
                    .key(clear_key.clone())
                    .into_widget(),
                ]),
            );

            // Open the popup menu.
            let state = tester.element(find::by_type::<EditableText>());
            let state = state
                .state_handle::<EditableTextState>(&tester.cell.borrow())
                .expect("an EditableText state");
            state.request_keyboard(&mut tester.cell.borrow_mut());
            tester.pump();
            TextInputClient::update_editing_value(
                state,
                &mut tester.cell.borrow_mut(),
                TextEditingValue::new()
                    .selection(TextSelection::collapsed(0, TextAffinity::Downstream)),
            );
            tester.cell.checkpoint();
            tester.pump_and_settle();
            expect(tester, &find::text("test"), finds_one_widget());

            // Select option 'test'.
            tester.tap(find::text("test"));
            tester.pump_and_settle();
            assert_eq!(controller_text(tester, &controller), "test");

            // Clear text using the icon button.
            tester.tap(find::by_key(clear_key));
            act(tester, |app| {
                focus.handle().unfocus(app, UnfocusDisposition::Scope)
            });
            tester.pump_and_settle();
            assert_eq!(controller_text(tester, &controller), "");

            // Select 'test' again.
            tester.tap(find::by_type::<TextField>());
            tester.pump_and_settle();
            expect(
                tester,
                &find::text("test"),
                waft_widgets::testing::finds_widgets(),
            );
            tester.tap(find::text("test").last());
            tester.pump_and_settle();

            // The text field should be updated to 'test'.
            assert_eq!(controller_text(tester, &controller), "test");
        });
    }
}
