//! Flutter counterpart: material_ui's `dropdown_menu.dart`.
//!
//! [`DropdownMenu`] is a [`TextField`] that a [`MenuAnchor`] opens a menu of
//! [`MenuItemButton`]s from. Its state keeps the entries the field filters and searches, the
//! highlighted entry that the arrow keys move, and the widths the private
//! `_RenderDropdownMenuBody` measures from an invisible copy of the menu.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use crate::button_style::ButtonStyle;
use crate::dropdown_menu_theme::{
    DropdownMenuTheme, DropdownMenuThemeData, DropdownMenuThemeDataGetters,
};
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::input_border::OutlineInputBorder;
use crate::input_decorator::{InputDecoration, InputDecorationThemeData};
use crate::material_localizations::MaterialLocalizations;
use crate::menu_anchor::{MenuAnchor, MenuItemButton};
use crate::menu_button_theme::MenuButtonTheme;
use crate::menu_style::MenuStyle;
use crate::text_field::TextField;
use crate::theme::Theme;
use crate::theme_data::VisualDensity;
use crate::theme_data_getters::DefaultsContext;
use waft_animation::Curves;
use waft_embedder::{
    Color, Offset, Size, TargetPlatform, TextAffinity, TextEditingValue, TextInputAction,
    TextInputType, TextSelection,
};
use waft_foundation::{App, Handle, K_IS_WEB, Listener, OptionRetained, Ref, ValueChanged};
use waft_painting::{
    AlignmentGeometry, EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry, TextAlign,
    TextAlignVertical, TextDirection, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData,
    ContainerBoxParentData, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, PaintingContext, ParentData,
    PipelineOwner, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData, RenderHandle,
    RenderObject, RenderObjectData,
};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_services::{
    LogicalKeyboardKey, MouseCursorRef, SystemMouseCursors, TextInputFormatterRef,
};
use waft_widgets::{
    Action, ActionRef, Actions, Align, AnyFocusNode, AnyMenuController, AnyTextEditingController,
    BuildContext, CallbackAction, ConstrainedBox, DefaultTextStyle, Directionality, DismissIntent,
    DismissMenuAction, ExcludeFocus, ExcludeSemantics, ExtendSelectionByCharacterIntent, Focus,
    FocusNode, FocusNodeLeaf, FocusNodeRef, GlobalKey, Icon, Intent, IntentRef, IntoWidget, KeyRef,
    MenuController, MenuControllerRef, MultiChildRenderObjectWidget, Padding, RenderObjectWidget,
    ScrollPositionAlignmentPolicy, Scrollable, Semantics, ShortcutActivatorRef, ShortcutMap,
    Shortcuts, SingleActivator, SizedBox, Stack, State, StateData, StatefulWidget, Text,
    TextEditingController, TextEditingControllerRef, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyAll, WidgetStatePropertyRef, WidgetStates, WidgetStatesController,
};

/// A callback function that returns the list of the items that matches the current applied
/// filter.
///
/// Used by [`DropdownMenu::filter_callback`].
pub type FilterCallback<T> =
    Rc<dyn Fn(&mut App, &[DropdownMenuEntry<T>], &str) -> Vec<DropdownMenuEntry<T>>>;

/// A callback function that returns the index of the item that matches the current contents
/// of a text field.
///
/// If a match doesn't exist then `None` must be returned.
///
/// Used by [`DropdownMenu::search_callback`].
pub type SearchCallback<T> = Rc<dyn Fn(&mut App, &[DropdownMenuEntry<T>], &str) -> Option<usize>>;

/// The type of builder function used by [`DropdownMenu::decoration_builder`] to build the
/// [`InputDecoration`] passed to the inner text field.
///
/// The `context` is the context that the decoration is being built in.
///
/// The `controller` is the [`MenuController`] that can be used to open and close the menu
/// with and query the current state.
pub type DropdownMenuDecorationBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnyMenuController) -> InputDecoration>;

const K_MINIMUM_WIDTH: f64 = 112.0;

const K_DEFAULT_HORIZONTAL_PADDING: f64 = 12.0;

const K_INPUT_START_GAP: f64 = 4.0;

/// Defines a [`DropdownMenu`] menu button that represents one item view in the menu.
///
/// See also:
///
/// * [`DropdownMenu`]
#[derive(Clone)]
pub struct DropdownMenuEntry<T> {
    /// the value used to identify the entry.
    ///
    /// This value must be unique across all entries in a [`DropdownMenu`].
    pub value: T,

    /// The label displayed in the center of the menu item.
    pub label: String,

    /// Overrides the default label widget which is `Text(label)`.
    ///
    /// This widget is only displayed in the open dropdown menu. When an item is selected, the
    /// menu closes and the text field displays the plain text of the [`label`](Self::label).
    ///
    /// The dropdown menu's closed state is a text field or a read-only text field on mobile,
    /// which can only display text. While custom widgets like icons or images can be shown
    /// in [`label_widget`](Self::label_widget) when the menu is open, the text field will only
    /// show the [`label`](Self::label) string upon selection.
    ///
    /// To control the text that appears in the text field for a selected item, set the
    /// [`label`](Self::label) property to a descriptive string.
    pub label_widget: Option<WidgetRef>,

    /// An optional icon to display before the label.
    pub leading_icon: Option<WidgetRef>,

    /// An optional icon to display after the label.
    pub trailing_icon: Option<WidgetRef>,

    /// Whether the menu item is enabled or disabled.
    ///
    /// The default value is true. If true, the [`label`](Self::label) will be filled out in
    /// the text field of the [`DropdownMenu`] when this entry is clicked; otherwise, this
    /// entry is disabled.
    pub enabled: bool,

    /// Customizes this menu item's appearance.
    ///
    /// Null by default.
    pub style: Option<ButtonStyle>,
}

impl<T> DropdownMenuEntry<T> {
    /// Creates an entry that is used with [`DropdownMenu::dropdown_menu_entries`].
    pub fn new(value: T, label: impl Into<String>) -> DropdownMenuEntry<T> {
        DropdownMenuEntry {
            value,
            label: label.into(),
            label_widget: None,
            leading_icon: None,
            trailing_icon: None,
            enabled: true,
            style: None,
        }
    }

    /// Dart `DropdownMenuEntry(labelWidget:)`.
    pub fn label_widget<K>(mut self, label_widget: impl IntoWidget<K>) -> DropdownMenuEntry<T> {
        self.label_widget = Some(label_widget.into_widget());
        self
    }

    /// Dart `DropdownMenuEntry(leadingIcon:)`.
    pub fn leading_icon<K>(mut self, leading_icon: impl IntoWidget<K>) -> DropdownMenuEntry<T> {
        self.leading_icon = Some(leading_icon.into_widget());
        self
    }

    /// Dart `DropdownMenuEntry(trailingIcon:)`.
    pub fn trailing_icon<K>(mut self, trailing_icon: impl IntoWidget<K>) -> DropdownMenuEntry<T> {
        self.trailing_icon = Some(trailing_icon.into_widget());
        self
    }

    /// Dart `DropdownMenuEntry(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> DropdownMenuEntry<T> {
        self.enabled = enabled;
        self
    }

    /// Dart `DropdownMenuEntry(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> DropdownMenuEntry<T> {
        self.style = Some(style);
        self
    }
}

impl<T: Debug> Debug for DropdownMenuEntry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuEntry")
            .field("value", &self.value)
            .field("label", &self.label)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

/// Defines the behavior for closing the dropdown menu when an item is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropdownMenuCloseBehavior {
    /// Closes all open menus in the widget tree.
    All,

    /// Closes only the current dropdown menu: Dart's `self`, a Rust keyword.
    SelfOnly,

    /// Does not close any menus.
    None,
}

/// A dropdown menu that can be opened from a [`TextField`]. The selected menu item is
/// displayed in that field.
///
/// This widget is used to help people make a choice from a menu and put the selected item
/// into the text input field. People can also filter the list based on the text input or
/// search one item in the menu list.
///
/// The menu is composed of a list of [`DropdownMenuEntry`]s. People can provide information,
/// such as: label, leading icon or trailing icon for each entry. The [`TextField`] will be
/// updated based on the selection from the menu entries. The text field will stay empty if
/// the selected entry is disabled.
///
/// When the dropdown menu has focus, it can be traversed by pressing the up or down key.
/// During the process, the corresponding item will be highlighted and the text field will be
/// updated. Disabled items will be skipped during traversal.
///
/// The menu can be scrollable if not all items in the list are displayed at once.
///
/// See also:
///
/// * [`MenuAnchor`], which is a widget used to mark the "anchor" for a set of submenus. The
///   [`DropdownMenu`] uses a [`TextField`] as the "anchor".
/// * [`TextField`], which is a text input widget that uses an [`InputDecoration`].
/// * [`DropdownMenuEntry`], which is used to build the [`MenuItemButton`] in the
///   [`DropdownMenu`] list.
#[derive(Clone)]
pub struct DropdownMenu<T: 'static> {
    pub key: Option<KeyRef>,

    /// Determine if the [`DropdownMenu`] is enabled.
    ///
    /// Defaults to true.
    pub enabled: bool,

    /// Determine the width of the [`DropdownMenu`].
    ///
    /// If this is null, the width of the [`DropdownMenu`] will be the same as the width of the
    /// widest menu item plus the width of the leading/trailing icon.
    pub width: Option<f64>,

    /// Determine the height of the menu.
    ///
    /// If this is null, the menu will display as many items as possible on the screen.
    pub menu_height: Option<f64>,

    /// An optional Icon at the front of the text input field.
    ///
    /// Defaults to null. If this is not null, the menu items will have extra paddings to be
    /// aligned with the text in the text field.
    pub leading_icon: Option<WidgetRef>,

    /// An optional icon at the end of the text field.
    ///
    /// Defaults to an `Icon` with `Icons::arrow_drop_down`.
    ///
    /// If [`show_trailing_icon`](Self::show_trailing_icon) is false, the trailing icon will
    /// not be shown.
    pub trailing_icon: Option<WidgetRef>,

    /// Specifies if the [`DropdownMenu`] should show the
    /// [`trailing_icon`](Self::trailing_icon).
    ///
    /// If [`trailing_icon`](Self::trailing_icon) is set, [`DropdownMenu`] will use that
    /// trailing icon, otherwise a default trailing icon will be created.
    ///
    /// If [`show_trailing_icon`](Self::show_trailing_icon) is false,
    /// [`trailing_icon_focus_node`](Self::trailing_icon_focus_node) must be null.
    ///
    /// If a value is provided for [`decoration_builder`](Self::decoration_builder) and the
    /// resulting `InputDecoration::suffix_icon` is not null,
    /// [`show_trailing_icon`](Self::show_trailing_icon) has no effect.
    ///
    /// Defaults to true.
    pub show_trailing_icon: bool,

    /// Defines the FocusNode for the trailing icon.
    ///
    /// If [`show_trailing_icon`](Self::show_trailing_icon) is false,
    /// [`trailing_icon_focus_node`](Self::trailing_icon_focus_node) must be null.
    ///
    /// If null, this widget will create its own [`FocusNode`].
    pub trailing_icon_focus_node: Option<FocusNodeRef>,

    /// Optional widget that describes the input field.
    ///
    /// When the input field is empty and unfocused, the label is displayed on top of the
    /// input field (i.e., at the same location on the screen where text may be entered in the
    /// input field). When the input field receives focus (or if the field is non-empty), the
    /// label moves above, either vertically adjacent to, or to the center of the input field.
    ///
    /// Defaults to null.
    pub label: Option<WidgetRef>,

    /// Text that suggests what sort of input the field accepts.
    ///
    /// Defaults to null;
    pub hint_text: Option<String>,

    /// Text that provides context about the [`DropdownMenu`]'s value, such as how the value
    /// will be used.
    ///
    /// If non-null, the text is displayed below the input field, in the same location as
    /// [`error_text`](Self::error_text). If a non-null [`error_text`](Self::error_text) value
    /// is specified then the helper text is not shown.
    ///
    /// Defaults to null;
    ///
    /// See also:
    ///
    /// * `InputDecoration::helper_text`, which is the text that provides context about the
    ///   `InputDecorator::child`'s value.
    pub helper_text: Option<String>,

    /// Text that appears below the input field and the border to show the error message.
    ///
    /// If non-null, the border's color animates to red and the
    /// [`helper_text`](Self::helper_text) is not shown.
    ///
    /// Defaults to null;
    ///
    /// See also:
    ///
    /// * `InputDecoration::error_text`, which is the text that appears below the
    ///   `InputDecorator::child` and the border.
    pub error_text: Option<String>,

    /// An optional icon at the end of the text field to indicate that the text field is
    /// pressed.
    ///
    /// Defaults to an `Icon` with `Icons::arrow_drop_up`.
    pub selected_trailing_icon: Option<WidgetRef>,

    /// Determine if the menu list can be filtered by the text input.
    ///
    /// Defaults to false.
    pub enable_filter: bool,

    /// Determine if the first item that matches the text input can be highlighted.
    ///
    /// Defaults to true as the search function could be commonly used.
    pub enable_search: bool,

    /// The type of keyboard to use for editing the text.
    ///
    /// Defaults to `TextInputType::text`.
    pub keyboard_type: Option<TextInputType>,

    /// The text style for the [`TextField`] of the [`DropdownMenu`];
    ///
    /// Defaults to the overall theme's `TextTheme::body_large` if the dropdown menu theme's
    /// value is null.
    pub text_style: Option<TextStyle>,

    /// The text align for the [`TextField`] of the [`DropdownMenu`].
    ///
    /// Defaults to `TextAlign::Start`.
    pub text_align: TextAlign,

    /// Defines the default appearance of [`InputDecoration`] to show around the text field.
    ///
    /// By default, shows a outlined text field.
    pub input_decoration_theme: Option<InputDecorationThemeData>,

    /// The builder function used to create the [`InputDecoration`] passed to the text field.
    ///
    /// If a value is provided for this property and the resulting
    /// `InputDecoration::suffix_icon` is null, a default [`IconButton`] is assigned as the
    /// suffix icon. This button's icon will use [`trailing_icon`](Self::trailing_icon) and
    /// [`selected_trailing_icon`](Self::selected_trailing_icon) if those are explicitly
    /// defined; otherwise, it defaults to `Icons::arrow_drop_down` for the collapsed state
    /// and `Icons::arrow_drop_up` for the expanded state.
    ///
    /// If null, the default builder creates a decoration where:
    /// - `InputDecoration::label` is set to [`label`](Self::label).
    /// - `InputDecoration::hint_text` is set to [`hint_text`](Self::hint_text).
    /// - `InputDecoration::helper_text` is set to [`helper_text`](Self::helper_text).
    /// - `InputDecoration::error_text` is set to [`error_text`](Self::error_text).
    /// - `InputDecoration::prefix_icon` is set to [`leading_icon`](Self::leading_icon).
    /// - `InputDecoration::suffix_icon` is set to an [`IconButton`] which uses
    ///   [`trailing_icon`](Self::trailing_icon) and
    ///   [`selected_trailing_icon`](Self::selected_trailing_icon) if defined, or
    ///   `Icons::arrow_drop_down` and `Icons::arrow_drop_up` otherwise.
    pub decoration_builder: Option<DropdownMenuDecorationBuilder>,

    /// The [`MenuStyle`] that defines the visual attributes of the menu.
    ///
    /// The default width of the menu is set to the width of the text field.
    pub menu_style: Option<MenuStyle>,

    /// Controls the text being edited or selected in the menu.
    ///
    /// If null, this widget will create its own [`TextEditingController`].
    ///
    /// A [`TextEditingController`] can be provided to programmatically clear or modify the
    /// displayed selection. For example, calling `TextEditingController::clear` clears the
    /// text displayed in the dropdown menu.
    ///
    /// When tracking the selected value in a state variable, calling
    /// `TextEditingController::clear` alongside updating the state variable to null is
    /// required to clear the displayed selection, as setting
    /// [`initial_selection`](Self::initial_selection) to null does not clear the text field.
    ///
    /// See also:
    ///
    ///  * [`initial_selection`](Self::initial_selection), which sets the initial selection
    ///    when the widget is first created.
    pub controller: Option<TextEditingControllerRef>,

    /// The value used for an initial selection.
    ///
    /// This property sets the initial value of the dropdown menu when the widget is first
    /// created. If the value matches one of the
    /// [`dropdown_menu_entries`](Self::dropdown_menu_entries), the corresponding label will be
    /// displayed in the text field.
    ///
    /// Setting this to null on a rebuild does not clear the text field, because
    /// [`DropdownMenu`] only updates the text field when
    /// [`initial_selection`](Self::initial_selection) changes to a value that matches one of
    /// the [`dropdown_menu_entries`](Self::dropdown_menu_entries).
    ///
    /// To programmatically clear or reset the selection, provide a
    /// [`controller`](Self::controller) and call `TextEditingController::clear` on it
    /// alongside resetting the state variable to null.
    ///
    /// Defaults to null.
    pub initial_selection: Option<T>,

    /// The callback is called when a selection is made.
    ///
    /// The callback receives the selected entry's value of type `T` when the user chooses an
    /// item. It may also be invoked with `None` to indicate that the selection was cleared /
    /// that no item was chosen.
    ///
    /// Defaults to null. If this callback itself is null, the widget still updates the text
    /// field with the selected label.
    pub on_selected: Option<ValueChanged<Option<T>>>,

    /// Defines the keyboard focus for this widget.
    ///
    /// If null, this widget will create its own [`FocusNode`].
    ///
    /// If this is non-null, the behaviour of
    /// [`request_focus_on_tap`](Self::request_focus_on_tap) is overridden by the
    /// `FocusNode::can_request_focus` property.
    pub focus_node: Option<FocusNodeRef>,

    /// Determine if the dropdown menu requests focus and the on-screen virtual keyboard is
    /// shown in response to a touch event.
    ///
    /// Ignored if a [`focus_node`](Self::focus_node) is explicitly provided (in which case,
    /// `FocusNode::can_request_focus` controls the behavior).
    ///
    /// Defaults to null, which enables platform-specific behavior:
    ///
    ///  * On mobile platforms, acts as if set to false; tapping on the text field and opening
    ///    the menu will not cause a focus request and the virtual keyboard will not appear.
    ///
    ///  * On desktop platforms, acts as if set to true; the dropdown takes the focus when
    ///    activated.
    ///
    /// Set this to true or false explicitly to override the default behavior.
    pub request_focus_on_tap: Option<bool>,

    /// Determines if the dropdown menu behaves as a 'select' component.
    ///
    /// This is useful for mobile platforms where a dropdown menu is commonly used as a
    /// 'select' widget (i.e., the user can only select from the list, not edit the text field
    /// to search or filter).
    ///
    /// When true, the inner text field is read-only.
    ///
    /// If the text field is also focusable (see
    /// [`request_focus_on_tap`](Self::request_focus_on_tap)), the following behaviors are also
    /// activated:
    ///
    ///  * Pressing Enter when the menu is closed opens it.
    ///  * The decoration reflects the focus state.
    ///
    /// Defaults to false.
    pub select_only: bool,

    /// Descriptions of the menu items in the [`DropdownMenu`].
    ///
    /// This is a required parameter. It is recommended that at least one
    /// [`DropdownMenuEntry`] is provided. If this is an empty list, the menu will be empty and
    /// only contain space for padding.
    ///
    /// Dart compares the list by identity when the widget updates; so does this `Rc`.
    pub dropdown_menu_entries: Rc<[DropdownMenuEntry<T>]>,

    /// Defines the menu text field's width to be equal to its parent's width plus the
    /// horizontal width of the specified insets.
    ///
    /// If this property is null, the width of the text field will be determined by the width
    /// of menu items or [`width`](Self::width). If this property is not null, the text field's
    /// width will match the parent's width plus the specified insets. If the value of this
    /// property is `EdgeInsets::ZERO`, the width of the text field will be the same as its
    /// parent's width.
    ///
    /// The insets' top and bottom are ignored, only its left and right properties are used.
    ///
    /// Defaults to null.
    pub expanded_insets: Option<EdgeInsetsGeometry>,

    /// When [`enable_filter`](Self::enable_filter) is true, this callback is used to compute
    /// the list of filtered items.
    ///
    /// Defaults to null. If this parameter is null and the
    /// [`enable_filter`](Self::enable_filter) property is set to true, the default behavior
    /// will return a filtered list. The filtered list will contain items that match the text
    /// provided by the input field, with a case-insensitive comparison. When this is not
    /// null, `enable_filter` must be set to true.
    pub filter_callback: Option<FilterCallback<T>>,

    /// When [`enable_search`](Self::enable_search) is true, this callback is used to compute
    /// the index of the search result to be highlighted.
    ///
    /// Defaults to null. If this is null and [`enable_search`](Self::enable_search) is true,
    /// the default function will return the index of the first matching result which
    /// contains the contents of the text input field.
    pub search_callback: Option<SearchCallback<T>>,

    /// Optional input validation and formatting overrides.
    ///
    /// Formatters are run in the provided order when the user changes the text this widget
    /// contains. When this parameter changes, the new formatters will not be applied until
    /// the next time the user inserts or deletes text. Formatters don't run when the text is
    /// changed programmatically via [`controller`](Self::controller).
    pub input_formatters: Option<Vec<TextInputFormatterRef>>,

    /// {@macro material_ui.MenuAnchor.alignmentOffset}
    pub alignment_offset: Option<Offset>,

    /// Defines the behavior for closing the dropdown menu when an item is selected.
    ///
    /// The close behavior can be set to:
    /// * [`DropdownMenuCloseBehavior::All`]: Closes all open menus in the widget tree.
    /// * [`DropdownMenuCloseBehavior::SelfOnly`]: Closes only the current dropdown menu.
    /// * [`DropdownMenuCloseBehavior::None`]: Does not close any menus.
    ///
    /// This property allows fine-grained control over the menu's closing behavior, which can
    /// be useful for creating nested or complex menu structures.
    ///
    /// Defaults to [`DropdownMenuCloseBehavior::All`].
    pub close_behavior: DropdownMenuCloseBehavior,

    /// Specifies the maximum number of lines the selected value can display in the
    /// [`DropdownMenu`].
    ///
    /// If the provided value is 1, then the text will not wrap, but will scroll horizontally
    /// instead. Defaults to 1.
    ///
    /// If this is null, there is no limit to the number of lines, and the text container
    /// will start with enough vertical space for one line and automatically grow to
    /// accommodate additional lines as they are entered, up to the height of its
    /// constraints.
    ///
    /// If this is not null, the provided value must be greater than zero. The text field will
    /// restrict the input to the given number of lines and take up enough horizontal space
    /// to accommodate that number of lines.
    ///
    /// See also:
    ///  * `TextField::max_lines`, which specifies the maximum number of lines the
    ///    [`TextField`] can display.
    pub max_lines: Option<i32>,

    /// {@macro material_ui.TextField.textInputAction}
    pub text_input_action: Option<TextInputAction>,

    /// {@macro flutter.widgets.editableText.cursorHeight}
    pub cursor_height: Option<f64>,

    /// {@macro cupertino_ui.textfield.restorationId}
    pub restoration_id: Option<String>,

    /// An optional controller that allows opening and closing of the menu from other widgets.
    pub menu_controller: Option<MenuControllerRef>,

    /// {@macro flutter.widgets.editableText.scrollPadding}
    pub scroll_padding: EdgeInsets,
}

impl<T: Clone + PartialEq + 'static> DropdownMenu<T> {
    /// Creates a [`DropdownMenu`].
    ///
    /// The leading and trailing icons in the text field can be customized by using
    /// [`leading_icon`](Self::leading_icon), [`trailing_icon`](Self::trailing_icon) and
    /// [`selected_trailing_icon`](Self::selected_trailing_icon) properties. They are passed
    /// down to the [`InputDecoration`] properties, and will override values in the
    /// `InputDecoration::prefix_icon` and `InputDecoration::suffix_icon`.
    ///
    /// Except leading and trailing icons, the text field can be configured by the
    /// [`input_decoration_theme`](Self::input_decoration_theme) property. The menu can be
    /// configured by the [`menu_style`](Self::menu_style).
    pub fn new(dropdown_menu_entries: impl Into<Rc<[DropdownMenuEntry<T>]>>) -> DropdownMenu<T> {
        DropdownMenu {
            key: None,
            enabled: true,
            width: None,
            menu_height: None,
            leading_icon: None,
            trailing_icon: None,
            show_trailing_icon: true,
            trailing_icon_focus_node: None,
            label: None,
            hint_text: None,
            helper_text: None,
            error_text: None,
            selected_trailing_icon: None,
            enable_filter: false,
            enable_search: true,
            keyboard_type: None,
            text_style: None,
            text_align: TextAlign::Start,
            input_decoration_theme: None,
            decoration_builder: None,
            menu_style: None,
            controller: None,
            initial_selection: None,
            on_selected: None,
            focus_node: None,
            request_focus_on_tap: None,
            select_only: false,
            dropdown_menu_entries: dropdown_menu_entries.into(),
            expanded_insets: None,
            filter_callback: None,
            search_callback: None,
            input_formatters: None,
            alignment_offset: None,
            close_behavior: DropdownMenuCloseBehavior::All,
            max_lines: Some(1),
            text_input_action: None,
            cursor_height: None,
            restoration_id: None,
            menu_controller: None,
            scroll_padding: EdgeInsets::all(20.0),
        }
    }

    /// Dart `DropdownMenu(key:)`.
    pub fn key(mut self, key: KeyRef) -> DropdownMenu<T> {
        self.key = Some(key);
        self
    }

    /// Dart `DropdownMenu(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> DropdownMenu<T> {
        self.enabled = enabled;
        self
    }

    /// Dart `DropdownMenu(width:)`.
    pub fn width(mut self, width: f64) -> DropdownMenu<T> {
        self.width = Some(width);
        self
    }

    /// Dart `DropdownMenu(menuHeight:)`.
    pub fn menu_height(mut self, menu_height: f64) -> DropdownMenu<T> {
        self.menu_height = Some(menu_height);
        self
    }

    /// Dart `DropdownMenu(leadingIcon:)`.
    pub fn leading_icon<K>(mut self, leading_icon: impl IntoWidget<K>) -> DropdownMenu<T> {
        self.leading_icon = Some(leading_icon.into_widget());
        self
    }

    /// Dart `DropdownMenu(trailingIcon:)`.
    pub fn trailing_icon<K>(mut self, trailing_icon: impl IntoWidget<K>) -> DropdownMenu<T> {
        self.trailing_icon = Some(trailing_icon.into_widget());
        self
    }

    /// Dart `DropdownMenu(showTrailingIcon:)`.
    pub fn show_trailing_icon(mut self, show_trailing_icon: bool) -> DropdownMenu<T> {
        self.show_trailing_icon = show_trailing_icon;
        self
    }

    /// Dart `DropdownMenu(trailingIconFocusNode:)`.
    pub fn trailing_icon_focus_node(
        mut self,
        trailing_icon_focus_node: FocusNodeRef,
    ) -> DropdownMenu<T> {
        self.trailing_icon_focus_node = Some(trailing_icon_focus_node);
        self
    }

    /// Dart `DropdownMenu(label:)`.
    pub fn label<K>(mut self, label: impl IntoWidget<K>) -> DropdownMenu<T> {
        self.label = Some(label.into_widget());
        self
    }

    /// Dart `DropdownMenu(hintText:)`.
    pub fn hint_text(mut self, hint_text: impl Into<String>) -> DropdownMenu<T> {
        self.hint_text = Some(hint_text.into());
        self
    }

    /// Dart `DropdownMenu(helperText:)`.
    pub fn helper_text(mut self, helper_text: impl Into<String>) -> DropdownMenu<T> {
        self.helper_text = Some(helper_text.into());
        self
    }

    /// Dart `DropdownMenu(errorText:)`.
    pub fn error_text(mut self, error_text: impl Into<String>) -> DropdownMenu<T> {
        self.error_text = Some(error_text.into());
        self
    }

    /// Dart `DropdownMenu(selectedTrailingIcon:)`.
    pub fn selected_trailing_icon<K>(
        mut self,
        selected_trailing_icon: impl IntoWidget<K>,
    ) -> DropdownMenu<T> {
        self.selected_trailing_icon = Some(selected_trailing_icon.into_widget());
        self
    }

    /// Dart `DropdownMenu(enableFilter:)`.
    pub fn enable_filter(mut self, enable_filter: bool) -> DropdownMenu<T> {
        self.enable_filter = enable_filter;
        self
    }

    /// Dart `DropdownMenu(enableSearch:)`.
    pub fn enable_search(mut self, enable_search: bool) -> DropdownMenu<T> {
        self.enable_search = enable_search;
        self
    }

    /// Dart `DropdownMenu(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> DropdownMenu<T> {
        self.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `DropdownMenu(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> DropdownMenu<T> {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `DropdownMenu(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> DropdownMenu<T> {
        self.text_align = text_align;
        self
    }

    /// Dart `DropdownMenu(inputDecorationTheme:)`.
    pub fn input_decoration_theme(
        mut self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> DropdownMenu<T> {
        self.input_decoration_theme = Some(input_decoration_theme.into());
        self
    }

    /// Dart `DropdownMenu(decorationBuilder:)`.
    pub fn decoration_builder(
        mut self,
        decoration_builder: impl Fn(&mut App, BuildContext, AnyMenuController) -> InputDecoration
        + 'static,
    ) -> DropdownMenu<T> {
        self.decoration_builder = Some(Rc::new(decoration_builder));
        self
    }

    /// Dart `DropdownMenu(menuStyle:)`.
    pub fn menu_style(mut self, menu_style: MenuStyle) -> DropdownMenu<T> {
        self.menu_style = Some(menu_style);
        self
    }

    /// Dart `DropdownMenu(controller:)`.
    pub fn controller(mut self, controller: TextEditingControllerRef) -> DropdownMenu<T> {
        self.controller = Some(controller);
        self
    }

    /// Dart `DropdownMenu(initialSelection:)`.
    pub fn initial_selection(mut self, initial_selection: T) -> DropdownMenu<T> {
        self.initial_selection = Some(initial_selection);
        self
    }

    /// Dart `DropdownMenu(onSelected:)`.
    pub fn on_selected(
        mut self,
        on_selected: impl Fn(&mut App, Option<T>) + 'static,
    ) -> DropdownMenu<T> {
        self.on_selected = Some(Rc::new(on_selected));
        self
    }

    /// Dart `DropdownMenu(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> DropdownMenu<T> {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `DropdownMenu(requestFocusOnTap:)`.
    pub fn request_focus_on_tap(mut self, request_focus_on_tap: bool) -> DropdownMenu<T> {
        self.request_focus_on_tap = Some(request_focus_on_tap);
        self
    }

    /// Dart `DropdownMenu(selectOnly:)`.
    pub fn select_only(mut self, select_only: bool) -> DropdownMenu<T> {
        self.select_only = select_only;
        self
    }

    /// Dart `DropdownMenu(expandedInsets:)`.
    pub fn expanded_insets(
        mut self,
        expanded_insets: impl Into<EdgeInsetsGeometry>,
    ) -> DropdownMenu<T> {
        self.expanded_insets = Some(expanded_insets.into());
        self
    }

    /// Dart `DropdownMenu(filterCallback:)`.
    pub fn filter_callback(
        mut self,
        filter_callback: impl Fn(&mut App, &[DropdownMenuEntry<T>], &str) -> Vec<DropdownMenuEntry<T>>
        + 'static,
    ) -> DropdownMenu<T> {
        self.filter_callback = Some(Rc::new(filter_callback));
        self
    }

    /// Dart `DropdownMenu(searchCallback:)`.
    pub fn search_callback(
        mut self,
        search_callback: impl Fn(&mut App, &[DropdownMenuEntry<T>], &str) -> Option<usize> + 'static,
    ) -> DropdownMenu<T> {
        self.search_callback = Some(Rc::new(search_callback));
        self
    }

    /// Dart `DropdownMenu(inputFormatters:)`.
    pub fn input_formatters(
        mut self,
        input_formatters: Vec<TextInputFormatterRef>,
    ) -> DropdownMenu<T> {
        self.input_formatters = Some(input_formatters);
        self
    }

    /// Dart `DropdownMenu(alignmentOffset:)`.
    pub fn alignment_offset(mut self, alignment_offset: Offset) -> DropdownMenu<T> {
        self.alignment_offset = Some(alignment_offset);
        self
    }

    /// Dart `DropdownMenu(closeBehavior:)`.
    pub fn close_behavior(mut self, close_behavior: DropdownMenuCloseBehavior) -> DropdownMenu<T> {
        self.close_behavior = close_behavior;
        self
    }

    /// Dart `DropdownMenu(maxLines:)`.
    pub fn max_lines(mut self, max_lines: Option<i32>) -> DropdownMenu<T> {
        self.max_lines = max_lines;
        self
    }

    /// Dart `DropdownMenu(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> DropdownMenu<T> {
        self.text_input_action = Some(text_input_action);
        self
    }

    /// Dart `DropdownMenu(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> DropdownMenu<T> {
        self.cursor_height = Some(cursor_height);
        self
    }

    /// Dart `DropdownMenu(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> DropdownMenu<T> {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `DropdownMenu(menuController:)`.
    pub fn menu_controller(mut self, menu_controller: MenuControllerRef) -> DropdownMenu<T> {
        self.menu_controller = Some(menu_controller);
        self
    }

    /// Dart `DropdownMenu(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> DropdownMenu<T> {
        self.scroll_padding = scroll_padding;
        self
    }
}

impl<T: 'static> Debug for DropdownMenu<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenu")
            .field("enabled", &self.enabled)
            .field("width", &self.width)
            .field("menu_height", &self.menu_height)
            .field("entries", &self.dropdown_menu_entries.len())
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for DropdownMenu<T> {
    type State = DropdownMenuState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.filter_callback.is_none() || self.enable_filter);
        debug_assert!(self.trailing_icon_focus_node.is_none() || self.show_trailing_icon);
        debug_assert!(
            self.decoration_builder.is_none()
                || (self.label.is_none()
                    && self.hint_text.is_none()
                    && self.helper_text.is_none()
                    && self.error_text.is_none())
        );
    }

    fn create_state(&self) -> DropdownMenuState<T> {
        DropdownMenuState {
            state: StateData::new(),
            anchor_key: GlobalKey::new(),
            leading_key: GlobalKey::new(),
            button_item_keys: Vec::new(),
            controller: None,
            internal_menu_controller: None,
            enable_filter: false,
            enable_search: true,
            filtered_entries: Rc::from(Vec::new()),
            initial_menu: None,
            current_highlight: None,
            leading_padding: None,
            menu_has_enabled_item: false,
            local_text_editing_controller: None,
            internal_focus_node: None,
            focus_node_can_request_focus: None,
            highlighted_item_states_controller: None,
            local_trailing_icon_button_focus_node: None,
        }
    }
}

/// Dart's `_DropdownMenuState`.
pub struct DropdownMenuState<T: Clone + PartialEq + 'static> {
    state: StateData<DropdownMenu<T>>,
    anchor_key: GlobalKey,
    leading_key: GlobalKey,
    button_item_keys: Vec<GlobalKey>,
    controller: Option<MenuControllerRef>,
    /// The `MenuController` Dart's `widget.menuController ?? MenuController()` makes, which
    /// this state holds.
    internal_menu_controller: Option<Ref<MenuController>>,
    enable_filter: bool,
    enable_search: bool,
    filtered_entries: Rc<[DropdownMenuEntry<T>]>,
    initial_menu: Option<Vec<WidgetRef>>,
    current_highlight: Option<usize>,
    leading_padding: Option<f64>,
    menu_has_enabled_item: bool,
    local_text_editing_controller: Option<Ref<TextEditingController>>,
    internal_focus_node: Option<Ref<FocusNode>>,
    focus_node_can_request_focus: Option<bool>,
    highlighted_item_states_controller: Option<Ref<WidgetStatesController>>,
    local_trailing_icon_button_focus_node: Option<Ref<FocusNode>>,
}

/// Dart's `_editableShortcuts`.
fn editable_shortcuts() -> ShortcutMap {
    vec![
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_LEFT),
            ExtendSelectionByCharacterIntent::new(false, true),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_RIGHT),
            ExtendSelectionByCharacterIntent::new(true, true),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_UP),
            ArrowUpIntent,
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN),
            ArrowDownIntent,
        ),
    ]
}

/// Dart's `_selectOnlyShortcuts`.
fn select_only_shortcuts() -> ShortcutMap {
    vec![
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_UP),
            ArrowUpIntent,
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN),
            ArrowDownIntent,
        ),
        // When selectOnly is true, a shortcut for the enter key is needed because
        // the text field won't provide one.
        shortcut(SingleActivator::new(LogicalKeyboardKey::ENTER), EnterIntent),
    ]
}

/// The shortcuts of the focus node that handles keyboard navigation when the text field has
/// no focus.
fn unfocused_text_field_shortcuts() -> ShortcutMap {
    vec![
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_UP),
            ArrowUpIntent,
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN),
            ArrowDownIntent,
        ),
        shortcut(SingleActivator::new(LogicalKeyboardKey::ENTER), EnterIntent),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ESCAPE),
            DismissIntent::new(),
        ),
    ]
}

fn shortcut(activator: SingleActivator, intent: impl Intent) -> (ShortcutActivatorRef, IntentRef) {
    (Rc::new(activator), Rc::new(intent))
}

/// The value a [`TextEditingController`] holds for `label`, with the caret after it.
fn text_editing_value_of(label: &str) -> TextEditingValue {
    TextEditingValue::new()
        .text(label)
        .selection(TextSelection::collapsed(
            label.encode_utf16().count() as i32,
            TextAffinity::Downstream,
        ))
}

/// Dart's `==` on two optional widgets, which compares them by identity.
fn same_widget(a: &Option<WidgetRef>, b: &Option<WidgetRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl<T: Clone + PartialEq + 'static> DropdownMenuState<T> {
    fn effective_text_editing_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> TextEditingControllerRef {
        if let Some(controller) = self.widget(app).controller.clone() {
            return controller;
        }
        if let Some(controller) = app.get(self).local_text_editing_controller.as_ref() {
            return controller.erase();
        }
        let controller = TextEditingController::new(app);
        app.get_mut(self).local_text_editing_controller = Some(controller.clone());
        controller.into_erased()
    }

    fn trailing_icon_button_focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = self.widget(app).trailing_icon_focus_node.clone() {
            return focus_node;
        }
        if let Some(focus_node) = app.get(self).local_trailing_icon_button_focus_node.as_ref() {
            return focus_node.erase();
        }
        let focus_node = FocusNode::new(app);
        app.get_mut(self).local_trailing_icon_button_focus_node = Some(focus_node.clone());
        focus_node.into_erased()
    }

    fn menu_controller(self: Handle<Self>, app: &App) -> &MenuControllerRef {
        app.get(self)
            .controller
            .as_ref()
            .expect("init_state sets the menu controller")
    }

    fn internal_focus_node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self)
            .internal_focus_node
            .handle()
            .expect("init_state makes the internal focus node")
    }

    /// Dart's `_controller = widget.menuController ?? MenuController()`.
    fn set_menu_controller(self: Handle<Self>, app: &mut App) {
        let controller = match self.widget(app).menu_controller.clone() {
            Some(controller) => {
                app.get_mut(self).internal_menu_controller = None;
                controller
            }
            None => {
                let internal = MenuController::new(app);
                app.get_mut(self).internal_menu_controller = Some(internal.clone());
                internal.into_erased()
            }
        };
        app.get_mut(self).controller = Some(controller);
    }

    /// Dart's `setState` around nothing: the body ran before the call.
    fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    fn set_button_item_keys(self: Handle<Self>, app: &mut App) {
        let count = app.get(self).filtered_entries.len();
        app.get_mut(self).button_item_keys = (0..count).map(|_| GlobalKey::new()).collect();
    }

    fn update_menu_has_enabled_item(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        state.menu_has_enabled_item = state.filtered_entries.iter().any(|entry| entry.enabled);
    }

    /// Sets the text field to the label of the entry whose value is the widget's
    /// `initial_selection`, if one is.
    fn show_initial_selection(self: Handle<Self>, app: &mut App) {
        let initial_selection = self.widget(app).initial_selection.clone();
        let label = app
            .get(self)
            .filtered_entries
            .iter()
            .find(|entry| Some(&entry.value) == initial_selection.as_ref())
            .map(|entry| entry.label.clone());
        if let Some(label) = label {
            self.effective_text_editing_controller(app)
                .set_value(app, text_editing_value_of(&label));
        }
    }

    fn can_request_focus(self: Handle<Self>, app: &mut App) -> bool {
        if let Some(focus_node) = self.widget(app).focus_node.clone() {
            return focus_node.can_request_focus(app);
        }
        if let Some(request_focus_on_tap) = self.widget(app).request_focus_on_tap {
            return request_focus_on_tap;
        }
        let context = self.context(app);
        match Theme::of(app, context).platform {
            TargetPlatform::IOS | TargetPlatform::Android | TargetPlatform::Fuchsia => false,
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => true,
        }
    }

    fn select_only(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).select_only
    }

    fn is_button(self: Handle<Self>, app: &mut App) -> bool {
        !self.can_request_focus(app) || self.select_only(app)
    }

    fn refresh_leading_padding(self: Handle<Self>, app: &mut App) {
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                if !self.mounted(app) {
                    return;
                }
                let leading_key = app.get(self).leading_key.clone();
                let leading_padding = Self::get_width(app, &leading_key);
                self.set_state(app, |state| {
                    state.leading_padding = leading_padding;
                });
            }),
            None,
        );
    }

    fn scroll_to_highlight(self: Handle<Self>, app: &mut App) {
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                // Dart's callback reads the state's fields after a dispose too, and then finds
                // no context; this one cannot read a freed state.
                if !app.contains(self) {
                    return;
                }
                let state = app.get(self);
                let current_highlight = state
                    .current_highlight
                    .expect("a highlight when its scroll was scheduled");
                let key = state.button_item_keys[current_highlight].clone();
                if let Some(highlight_context) = key.current_context(app) {
                    let render_object = highlight_context
                        .find_render_object(app)
                        .expect("a mounted menu item has a render object");
                    let position = Scrollable::of(app, highlight_context, None).position(app);
                    drop(position.ensure_visible(
                        app,
                        render_object,
                        0.0,
                        Duration::ZERO,
                        Curves::ease(),
                        ScrollPositionAlignmentPolicy::Explicit,
                        None,
                    ));
                }
            }),
            None,
        );
    }

    fn get_width(app: &App, key: &GlobalKey) -> Option<f64> {
        let context = key.current_context(app)?;
        let render_box = context
            .find_render_object(app)
            .and_then(AnyRenderObject::as_box)
            .expect("the key's widget builds a render box");
        if render_box.has_size(app) {
            Some(render_box.size(app).width())
        } else {
            None
        }
    }

    fn filter(
        entries: &[DropdownMenuEntry<T>],
        text_editing_controller: AnyTextEditingController,
        app: &App,
    ) -> Vec<DropdownMenuEntry<T>> {
        let filter_text = text_editing_controller.text_value(app).to_lowercase();
        entries
            .iter()
            .filter(|entry| entry.label.to_lowercase().contains(&filter_text))
            .cloned()
            .collect()
    }

    fn should_update_current_highlight(
        self: Handle<Self>,
        app: &mut App,
        entries: &[DropdownMenuEntry<T>],
    ) -> bool {
        let search_text = self
            .effective_text_editing_controller(app)
            .value(app)
            .text
            .to_lowercase();
        if search_text.is_empty() {
            return true;
        }

        // When `entries` are filtered by filter algorithm, currentHighlight may exceed the valid range of `entries` and should be updated.
        let Some(current_highlight) = app.get(self).current_highlight else {
            return true;
        };
        if current_highlight >= entries.len() {
            return true;
        }

        if entries[current_highlight]
            .label
            .to_lowercase()
            .contains(&search_text)
        {
            return false;
        }

        true
    }

    fn search(
        entries: &[DropdownMenuEntry<T>],
        text_editing_controller: AnyTextEditingController,
        app: &App,
    ) -> Option<usize> {
        let search_text = text_editing_controller.value(app).text.to_lowercase();
        if search_text.is_empty() {
            return None;
        }

        entries
            .iter()
            .position(|entry| entry.label.to_lowercase().contains(&search_text))
    }

    fn build_buttons(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        filtered_entries: &[DropdownMenuEntry<T>],
        options: BuildButtonsOptions,
    ) -> Vec<WidgetRef> {
        let effective_input_start_gap = if options.use_material3.unwrap_or(false) {
            K_INPUT_START_GAP
        } else {
            0.0
        };
        let mut result = Vec::new();
        for (i, entry) in filtered_entries.iter().enumerate() {
            // By default, when the text field has a leading icon but a menu entry doesn't
            // have one, the label of the entry should have extra padding to be aligned
            // with the text in the text input field. When both the text field and the
            // menu entry have leading icons, the menu entry should remove the extra
            // paddings so its leading icon will be aligned with the leading icon of
            // the text field.
            let padding = if entry.leading_icon.is_none() {
                app.get(self)
                    .leading_padding
                    .unwrap_or(K_DEFAULT_HORIZONTAL_PADDING)
            } else {
                K_DEFAULT_HORIZONTAL_PADDING
            };
            let mut effective_style = entry.style.clone().unwrap_or_else(|| {
                MenuItemButton::style_from()
                    .padding(
                        EdgeInsetsDirectional::only(
                            padding,
                            0.0,
                            K_DEFAULT_HORIZONTAL_PADDING,
                            0.0,
                        )
                        .into(),
                    )
                    .build()
            });

            let theme_style = MenuButtonTheme::of(app, context).style;

            let entry_style = entry.style.as_ref();
            let effective_foreground_color = entry_style
                .and_then(|style| style.foreground_color.clone())
                .or_else(|| {
                    theme_style
                        .as_ref()
                        .and_then(|style| style.foreground_color.clone())
                });
            let effective_icon_color = entry_style
                .and_then(|style| style.icon_color.clone())
                .or_else(|| {
                    theme_style
                        .as_ref()
                        .and_then(|style| style.icon_color.clone())
                });
            let effective_overlay_color = entry_style
                .and_then(|style| style.overlay_color.clone())
                .or_else(|| {
                    theme_style
                        .as_ref()
                        .and_then(|style| style.overlay_color.clone())
                });
            let effective_background_color = entry_style
                .and_then(|style| style.background_color.clone())
                .or_else(|| {
                    theme_style
                        .as_ref()
                        .and_then(|style| style.background_color.clone())
                });

            // Simulate the focused state because the text field should always be focused
            // during traversal. Include potential MenuItemButton theme in the focus
            // simulation for all colors in the theme.
            let entry_is_selected = entry.enabled && Some(i) == options.focused_index;
            if entry_is_selected {
                if let Some(replaced) = app.get_mut(self).highlighted_item_states_controller.take()
                {
                    replaced.dispose(app);
                }
                let focused: WidgetStates = [WidgetState::Focused].into_iter().collect();
                let states_controller = WidgetStatesController::new(app, Some(focused.clone()));
                app.get_mut(self).highlighted_item_states_controller = Some(states_controller);

                // Query the Material 3 default style.
                // TODO(bleroux): replace once a standard way for accessing defaults will be defined.
                // See: https://github.com/flutter/flutter/issues/130135.
                let default_style = MenuItemButton::new().default_style_of(app, context);

                let resolve_focused_color =
                    |color_state_property: Option<&WidgetStatePropertyRef<Option<Color>>>| {
                        color_state_property.and_then(|property| property.resolve(&focused))
                    };

                let focused_foreground_color = resolve_focused_color(Some(
                    effective_foreground_color
                        .clone()
                        .or_else(|| default_style.foreground_color(app))
                        .as_ref()
                        .expect("the default style has a foreground color"),
                ))
                .expect("a focused foreground color");
                let focused_icon_color = resolve_focused_color(Some(
                    effective_icon_color
                        .clone()
                        .or_else(|| default_style.icon_color(app))
                        .as_ref()
                        .expect("the default style has an icon color"),
                ))
                .expect("a focused icon color");
                let focused_overlay_color = resolve_focused_color(Some(
                    effective_overlay_color
                        .clone()
                        .or_else(|| default_style.overlay_color(app))
                        .as_ref()
                        .expect("the default style has an overlay color"),
                ))
                .expect("a focused overlay color");
                // For the background color we can't rely on the default style which is transparent.
                // Defaults to onSurface.withOpacity(0.12).
                #[expect(deprecated, reason = "Dart calls the deprecated `withOpacity`")]
                let focused_background_color = resolve_focused_color(
                    effective_background_color.as_ref(),
                )
                .unwrap_or_else(|| {
                    Theme::of(app, context)
                        .color_scheme
                        .on_surface
                        .with_opacity(0.12)
                });

                effective_style.background_color = Some(Rc::new(WidgetStatePropertyAll::new(
                    Some(focused_background_color),
                )));
                effective_style.foreground_color = Some(Rc::new(WidgetStatePropertyAll::new(
                    Some(focused_foreground_color),
                )));
                effective_style.icon_color = Some(Rc::new(WidgetStatePropertyAll::new(Some(
                    focused_icon_color,
                ))));
                effective_style.overlay_color = Some(Rc::new(WidgetStatePropertyAll::new(Some(
                    focused_overlay_color,
                ))));
            } else {
                copy_with_color(
                    &mut effective_style.background_color,
                    effective_background_color,
                );
                copy_with_color(
                    &mut effective_style.foreground_color,
                    effective_foreground_color,
                );
                copy_with_color(&mut effective_style.icon_color, effective_icon_color);
                copy_with_color(&mut effective_style.overlay_color, effective_overlay_color);
            }

            let mut label = entry
                .label_widget
                .clone()
                .unwrap_or_else(|| Text::new(entry.label.clone()).into_widget());
            let widget = self.widget(app);
            if let Some(width) = widget.width {
                let horizontal_padding =
                    padding + K_DEFAULT_HORIZONTAL_PADDING + effective_input_start_gap;
                label = ConstrainedBox::new(
                    BoxConstraints::new().max_width(width - horizontal_padding),
                )
                .child(label)
                .into_widget();
            }

            let on_pressed = (entry.enabled && widget.enabled).then(|| {
                let entry = entry.clone();
                let controller = widget.controller.clone();
                let on_selected = widget.on_selected.clone();
                Listener::new(move |app| {
                    self.handle_entry_pressed(app, &entry, i, controller.handle(), &on_selected)
                })
            });
            let mut menu_item_button = MenuItemButton::new()
                .style(effective_style)
                .close_on_activate(widget.close_behavior == DropdownMenuCloseBehavior::All)
                .request_focus_on_hover(false)
                // MenuItemButton implementation is based on M3 spec for menu which specifies a
                // horizontal padding of 12 pixels.
                // In the context of DropdownMenu the M3 spec specifies that the menu item and the text
                // field content should be aligned. The text field has a horizontal padding of 16 pixels.
                // To conform with the 16 pixels padding, a 4 pixels padding is added in front of the item label.
                .child(
                    Padding::new(EdgeInsetsDirectional::only(
                        effective_input_start_gap,
                        0.0,
                        0.0,
                        0.0,
                    ))
                    .child(label),
                );
            if options.enable_scroll_to_highlight {
                menu_item_button.key = Some(app.get(self).button_item_keys[i].clone().into());
            }
            if entry_is_selected {
                menu_item_button.states_controller =
                    app.get(self).highlighted_item_states_controller.clone();
            }
            menu_item_button.leading_icon = entry.leading_icon.clone();
            menu_item_button.trailing_icon = entry.trailing_icon.clone();
            menu_item_button.on_pressed = on_pressed;

            let menu_item_button = ExcludeFocus::new(
                ExcludeSemantics::new()
                    .excluding(options.exclude_semantics)
                    .child(menu_item_button),
            )
            .into_widget();
            result.push(menu_item_button);
        }

        result
    }

    /// The body of the `onPressed` callback `_buildButtons` gives an enabled entry's button.
    fn handle_entry_pressed(
        self: Handle<Self>,
        app: &mut App,
        entry: &DropdownMenuEntry<T>,
        index: usize,
        controller: Option<AnyTextEditingController>,
        on_selected: &Option<ValueChanged<Option<T>>>,
    ) {
        if !self.mounted(app) {
            // In some cases (e.g., nested menus), calling onSelected from MenuAnchor inside a postFrameCallback
            // can result in the MenuItemButton's onPressed callback being triggered after the state has been disposed.
            // TODO(ahmedrasar): MenuAnchor should avoid calling onSelected inside a postFrameCallback.
            if let Some(controller) = controller {
                controller.set_value(app, text_editing_value_of(&entry.label));
            }
            if let Some(on_selected) = on_selected {
                on_selected(app, Some(entry.value.clone()));
            }
            return;
        }
        self.effective_text_editing_controller(app)
            .set_value(app, text_editing_value_of(&entry.label));
        let enable_search = self.widget(app).enable_search;
        app.get_mut(self).current_highlight = if enable_search { Some(index) } else { None };
        if let Some(on_selected) = self.widget(app).on_selected.clone() {
            on_selected(app, Some(entry.value.clone()));
        }
        app.get_mut(self).enable_filter = false;
        if self.widget(app).close_behavior == DropdownMenuCloseBehavior::SelfOnly {
            self.menu_controller(app).handle().close(app);
        }
    }

    fn handle_up_key(self: Handle<Self>, app: &mut App) {
        self.move_highlight(app, false);
    }

    fn handle_down_key(self: Handle<Self>, app: &mut App) {
        self.move_highlight(app, true);
    }

    /// The `setState` body of Dart's `handleUpKey` and `handleDownKey`.
    fn move_highlight(self: Handle<Self>, app: &mut App, forward: bool) {
        let enabled = self.widget(app).enabled;
        let is_open = self.menu_controller(app).handle().is_open(app);
        if enabled && app.get(self).menu_has_enabled_item && is_open {
            let state = app.get_mut(self);
            state.enable_filter = false;
            state.enable_search = false;
            let length = state.filtered_entries.len() as i64;
            let step = if forward { 1 } else { -1 };
            let start = state
                .current_highlight
                .map_or(if forward { -1 } else { 0 }, |highlight| highlight as i64);
            let mut current_highlight = (start + step).rem_euclid(length);
            while !state.filtered_entries[current_highlight as usize].enabled {
                current_highlight = (current_highlight + step).rem_euclid(length);
            }
            state.current_highlight = Some(current_highlight as usize);
            let current_label = state.filtered_entries[current_highlight as usize]
                .label
                .clone();
            self.effective_text_editing_controller(app)
                .set_value(app, text_editing_value_of(&current_label));
        }
        self.mark_needs_build(app);
    }

    fn handle_enter_key(self: Handle<Self>, app: &mut App) {
        let controller = self.menu_controller(app).handle();
        if self.select_only(app) && !controller.is_open(app) {
            controller.open(app, None);
            return;
        }
        self.handle_submitted(app);
    }

    fn handle_pressed(
        self: Handle<Self>,
        app: &mut App,
        controller: AnyMenuController,
        focus_for_keyboard: bool,
    ) {
        if controller.is_open(app) {
            app.get_mut(self).current_highlight = None;
            controller.close(app);
        } else {
            let entries = Rc::clone(&self.widget(app).dropdown_menu_entries);
            app.get_mut(self).filtered_entries = entries;
            // close to open
            if !self
                .effective_text_editing_controller(app)
                .text_value(app)
                .is_empty()
            {
                app.get_mut(self).enable_filter = false;
            }
            controller.open(app, None);
            if focus_for_keyboard {
                let internal_focus_node = FocusNodeLeaf::as_node(self.internal_focus_node(app));
                internal_focus_node.request_focus(app, None);
            }
        }
        self.mark_needs_build(app);
    }

    fn handle_submitted(self: Handle<Self>, app: &mut App) {
        let controller = self.menu_controller(app).handle();
        if let Some(current_highlight) = app.get(self).current_highlight {
            let entry = app.get(self).filtered_entries[current_highlight].clone();
            if entry.enabled {
                self.effective_text_editing_controller(app)
                    .set_value(app, text_editing_value_of(&entry.label));
                if let Some(on_selected) = self.widget(app).on_selected.clone() {
                    on_selected(app, Some(entry.value));
                }
            }
        } else if controller.is_open(app)
            && let Some(on_selected) = self.widget(app).on_selected.clone()
        {
            on_selected(app, None);
        }
        if !self.widget(app).enable_search {
            app.get_mut(self).current_highlight = None;
        }
        controller.close(app);
    }

    fn build_default_decoration(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        controller: AnyMenuController,
    ) -> InputDecoration {
        let widget = self.widget(app).clone();
        let mut decoration = InputDecoration::new();
        decoration.label = widget.label;
        decoration.hint_text = widget.hint_text;
        decoration.helper_text = widget.helper_text;
        decoration.error_text = widget.error_text;
        decoration.prefix_icon = widget.leading_icon;
        decoration.suffix_icon = self.build_default_suffix_icon(app, context, controller);
        decoration
    }

    fn build_default_suffix_icon(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        controller: AnyMenuController,
    ) -> Option<WidgetRef> {
        let widget = self.widget(app).clone();
        let is_collapsed = widget
            .input_decoration_theme
            .as_ref()
            .is_some_and(|theme| theme.is_collapsed);
        if !widget.show_trailing_icon {
            return None;
        }
        let on_pressed = widget
            .enabled
            .then(|| Listener::new(move |app| self.handle_pressed(app, controller, true)));
        let mut icon_button = IconButton::new(
            on_pressed,
            widget
                .trailing_icon
                .clone()
                .unwrap_or_else(|| Icon::new(Some(Icons::arrow_drop_down())).into_widget()),
        )
        .focus_node(self.trailing_icon_button_focus_node(app))
        .is_selected(controller.is_open(app))
        .selected_icon(
            widget
                .selected_trailing_icon
                .clone()
                .unwrap_or_else(|| Icon::new(Some(Icons::arrow_drop_up())).into_widget()),
        );
        icon_button.constraints = widget
            .input_decoration_theme
            .as_ref()
            .and_then(|theme| theme.suffix_icon_constraints);
        if is_collapsed {
            icon_button.padding = Some(EdgeInsetsGeometry::ZERO);
        }
        let is_button = self.is_button(app);
        Some(
            Padding::new(if is_collapsed {
                EdgeInsetsGeometry::ZERO
            } else {
                EdgeInsetsGeometry::all(4.0)
            })
            .child(
                ExcludeSemantics::new()
                    // When the text field is treated as a button (i.e., it can
                    // not be focused), the trailing button should become part of
                    // the text field button by excluding semantics. Otherwise,
                    // it will inappropriately announce whether this icon button
                    // is selected or not.
                    .excluding(is_button)
                    .child(icon_button),
            )
            .into_widget(),
        )
    }

    /// The `builder` Dart's `build` hands the [`MenuAnchor`].
    fn build_anchor(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        controller: AnyMenuController,
        anchor: &AnchorBuildArguments,
    ) -> WidgetRef {
        debug_assert!(app.get(self).initial_menu.is_some());
        let widget = self.widget(app).clone();
        let mut decoration = match &widget.decoration_builder {
            Some(decoration_builder) => decoration_builder(app, context, controller),
            None => self.build_default_decoration(app, context, controller),
        };
        // If no suffixIcon is provided, the default IconButton is used for convenience.
        if decoration.suffix_icon.is_none() {
            decoration.suffix_icon = self.build_default_suffix_icon(app, context, controller);
        }
        let effective_decoration =
            decoration.apply_defaults(anchor.effective_input_decoration_theme.clone());
        let text_field_decoration = match &effective_decoration.prefix_icon {
            None => effective_decoration.clone(),
            Some(prefix_icon) => {
                let mut decoration = effective_decoration.copy_with();
                decoration.prefix_icon = Some(
                    SizedBox::new()
                        // Used to query the width in refreshLeadingPadding.
                        .key(app.get(self).leading_key.clone().into())
                        .child(prefix_icon.clone())
                        .into_widget(),
                );
                decoration
            }
        };

        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let is_button = self.is_button(app);
        let can_request_focus = self.can_request_focus(app);
        let menu_controller = self.menu_controller(app).handle();
        let is_open = menu_controller.is_open(app);
        let mut semantics = Semantics::new().button(is_button);
        // This is set specifically for iOS because iOS does not have any native
        // APIs to show whether the menu is expanded or collapsed.
        if Theme::of(app, context).platform == TargetPlatform::IOS {
            semantics = semantics.hint(if is_open {
                localizations.collapsed_hint()
            } else {
                localizations.expanded_hint()
            });
        }
        semantics = semantics.expanded(is_open);
        if !is_open {
            semantics = semantics.on_expand(Rc::new(move |app: &mut App| {
                menu_controller.open(app, None);
            }));
        } else {
            semantics = semantics.on_collapse(Rc::new(move |app: &mut App| {
                menu_controller.close(app);
            }));
        }

        let mut text_field = TextField::new()
            .key(app.get(self).anchor_key.clone().into())
            .enabled(widget.enabled)
            .can_request_focus(can_request_focus)
            .enable_interactive_selection(!is_button)
            .read_only(is_button)
            .text_align(widget.text_align)
            .text_align_vertical(TextAlignVertical::CENTER)
            .max_lines(widget.max_lines)
            .controller(self.effective_text_editing_controller(app))
            .on_submitted(move |app, _| self.handle_submitted(app))
            .on_changed(move |app, _text| {
                controller.open(app, None);
                let widget = self.widget(app);
                let entries = Rc::clone(&widget.dropdown_menu_entries);
                let enable_filter = widget.enable_filter;
                let enable_search = widget.enable_search;
                self.set_state(app, |state| {
                    state.filtered_entries = entries;
                    state.enable_filter = enable_filter;
                    state.enable_search = enable_search;
                });
            })
            .decoration(Some(text_field_decoration))
            .scroll_padding(widget.scroll_padding);
        text_field.mouse_cursor = anchor.effective_mouse_cursor.clone();
        text_field.focus_node = widget.focus_node.clone();
        text_field.keyboard_type = widget.keyboard_type;
        text_field.text_input_action = widget.text_input_action;
        text_field.cursor_height = widget.cursor_height;
        text_field.style = anchor.effective_text_style.clone().map(Into::into);
        text_field.on_tap = widget.enabled.then(|| {
            Listener::new(move |app| {
                let focus_for_keyboard = !self.can_request_focus(app);
                self.handle_pressed(app, controller, focus_for_keyboard);
            })
        });
        text_field.input_formatters = widget.input_formatters.clone();
        text_field.restoration_id = widget.restoration_id.clone();

        let text_field = semantics
            .child(
                ExcludeSemantics::new()
                    // When both `isTextField` and `isButton` are true, this widget will
                    // still be treated as a text field on web. So excluding the semantics
                    // of the `TextField` on web is needed.
                    .excluding(is_button && K_IS_WEB)
                    .child(text_field),
            )
            .into_widget();

        // The label used in _DropdownMenuBody to compute the preferred width.
        let effective_label = effective_decoration.label.clone().or_else(|| {
            effective_decoration
                .label_text
                .clone()
                .map(|label_text| Text::new(label_text).into_widget())
        });

        // If [expandedInsets] is not null, the width of the text field should depend
        // on its parent width. So we don't need to use `_DropdownMenuBody` to
        // calculate the children's width.
        let body = if widget.expanded_insets.is_some() {
            text_field
        } else {
            let mut children = vec![text_field];
            children.extend(
                app.get(self)
                    .initial_menu
                    .clone()
                    .expect("build made the initial menu"),
            );
            if let Some(effective_label) = effective_label {
                children.push(
                    ExcludeSemantics::new()
                        .child(
                            Padding::new(
                                // See RenderEditable.floatingCursorAddedMargin for the default horizontal padding.
                                EdgeInsetsGeometry::symmetric(0.0, 4.0),
                            )
                            .child(DefaultTextStyle::new(
                                anchor
                                    .effective_text_style
                                    .clone()
                                    .expect("an effective text style with a label"),
                                effective_label,
                            )),
                        )
                        .into_widget(),
                );
            }
            children.push(
                effective_decoration
                    .suffix_icon
                    .clone()
                    .unwrap_or_else(|| SizedBox::shrink().into_widget()),
            );
            children.push(
                Padding::new(
                    // TODO(bleroux): find a more accurate way to get the correct width.
                    // This padding is used to mimic default input decorator padding.
                    // It won't be correct if non default values are used.
                    EdgeInsetsGeometry::all(8.0),
                )
                .child(
                    effective_decoration
                        .prefix_icon
                        .clone()
                        .unwrap_or_else(|| SizedBox::shrink().into_widget()),
                )
                .into_widget(),
            );
            DropdownMenuBody {
                width: widget.width,
                // The children, except the text field, are used to compute the preferred width,
                // which is the width of the longest children, plus the width of trailingButton
                // and leadingButton.
                //
                // See _RenderDropdownMenuBody layout logic.
                //
                // TODO(bleroux): find a more accurate way to measure the text field minimum width.
                // The text field width computation is not accurate as it is based only on label,
                // prefixIcon and suffixIcon. Other InputDecoration parameters can have an
                // impact on the total width.
                children,
            }
            .into_widget()
        };

        Shortcuts::new(
            if self.select_only(app) {
                select_only_shortcuts()
            } else {
                editable_shortcuts()
            },
            body,
        )
        .into_widget()
    }

    /// The `Actions` of `build`.
    fn build_actions(self: Handle<Self>, app: &mut App) -> HashMap<TypeId, ActionRef> {
        let up = CallbackAction::<ArrowUpIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ArrowUpIntent| {
                self.handle_up_key(app);
                None
            }),
        );
        let down = CallbackAction::<ArrowDownIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ArrowDownIntent| {
                self.handle_down_key(app);
                None
            }),
        );
        let enter = CallbackAction::<EnterIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &EnterIntent| {
                self.handle_enter_key(app);
                None
            }),
        );
        let controller = self.menu_controller(app).clone();
        let dismiss = DismissMenuAction::new(app, controller);
        let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
        actions.insert(TypeId::of::<ArrowUpIntent>(), up.into_erased());
        actions.insert(TypeId::of::<ArrowDownIntent>(), down.into_erased());
        actions.insert(TypeId::of::<EnterIntent>(), enter.into_erased());
        actions.insert(
            <DismissMenuAction as Action>::intent_type(),
            dismiss.into_erased(),
        );
        actions
    }
}

/// Dart's `ButtonStyle.copyWith` of one colour: a null argument keeps the field.
fn copy_with_color(
    field: &mut Option<WidgetStatePropertyRef<Option<Color>>>,
    value: Option<WidgetStatePropertyRef<Option<Color>>>,
) {
    if value.is_some() {
        *field = value;
    }
}

/// The named arguments of Dart's `_buildButtons`.
struct BuildButtonsOptions {
    #[expect(
        dead_code,
        reason = "Dart's argument, which `_buildButtons` does not read"
    )]
    text_direction: TextDirection,
    focused_index: Option<usize>,
    enable_scroll_to_highlight: bool,
    exclude_semantics: bool,
    use_material3: Option<bool>,
}

/// What Dart's `build` computes before it makes the [`MenuAnchor`] and its builder reads.
struct AnchorBuildArguments {
    effective_input_decoration_theme: InputDecorationThemeData,
    effective_text_style: Option<TextStyle>,
    effective_mouse_cursor: Option<MouseCursorRef>,
}

impl<T: Clone + PartialEq + 'static> State for DropdownMenuState<T> {
    type Widget = DropdownMenu<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let internal_focus_node = FocusNode::new(app);
        app.get_mut(self).internal_focus_node = Some(internal_focus_node);
        let widget_focus_node = self.widget(app).focus_node.clone();
        let focus_node_can_request_focus =
            widget_focus_node.map(|focus_node| focus_node.can_request_focus(app));
        let widget = self.widget(app);
        let enable_search = widget.enable_search;
        let entries = Rc::clone(&widget.dropdown_menu_entries);
        let state = app.get_mut(self);
        state.focus_node_can_request_focus = focus_node_can_request_focus;
        state.enable_search = enable_search;
        state.filtered_entries = entries;
        self.set_button_item_keys(app);
        self.update_menu_has_enabled_item(app);
        self.show_initial_selection(app);
        self.refresh_leading_padding(app);
        self.set_menu_controller(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get_mut(self).local_text_editing_controller.take() {
            controller.erase::<AnyTextEditingController>().dispose(app);
        }
        let internal_focus_node = self.internal_focus_node(app);
        FocusNodeLeaf::as_node(internal_focus_node).dispose(app);
        if let Some(focus_node) = app
            .get_mut(self)
            .local_trailing_icon_button_focus_node
            .take()
        {
            focus_node.as_node().dispose(app);
        }
        if let Some(states_controller) = app.get_mut(self).highlighted_item_states_controller.take()
        {
            states_controller.dispose(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &DropdownMenu<T>) {
        let widget = self.widget(app).clone();
        if old_widget.controller != widget.controller
            && let Some(controller) = app.get_mut(self).local_text_editing_controller.take()
        {
            controller.erase::<AnyTextEditingController>().dispose(app);
        }
        if old_widget.enable_filter != widget.enable_filter && !widget.enable_filter {
            app.get_mut(self).enable_filter = false;
        }
        if old_widget.enable_search != widget.enable_search && !widget.enable_search {
            let state = app.get_mut(self);
            state.enable_search = widget.enable_search;
            state.current_highlight = None;
        }
        if !Rc::ptr_eq(
            &old_widget.dropdown_menu_entries,
            &widget.dropdown_menu_entries,
        ) {
            let state = app.get_mut(self);
            state.current_highlight = None;
            state.filtered_entries = Rc::clone(&widget.dropdown_menu_entries);
            self.set_button_item_keys(app);
            self.update_menu_has_enabled_item(app);
        }
        if !same_widget(&old_widget.leading_icon, &widget.leading_icon) {
            self.refresh_leading_padding(app);
        }
        if old_widget.initial_selection != widget.initial_selection {
            self.show_initial_selection(app);
        }
        if old_widget.menu_controller != widget.menu_controller {
            self.set_menu_controller(app);
        }
        if widget.focus_node != old_widget.focus_node {
            app.get_mut(self).focus_node_can_request_focus = widget
                .focus_node
                .as_ref()
                .map(|focus_node| focus_node.can_request_focus(app));
        }
        if old_widget.enabled && !widget.enabled {
            app.get_mut(self).focus_node_can_request_focus = widget
                .focus_node
                .as_ref()
                .map(|focus_node| focus_node.can_request_focus(app));
        }
        if !old_widget.enabled
            && widget.enabled
            && let Some(focus_node) = widget.focus_node.as_ref()
            && let Some(can_request_focus) = app.get(self).focus_node_can_request_focus
        {
            focus_node.set_can_request_focus(app, can_request_focus);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let use_material3 = Theme::of(app, context).use_material3;
        let text_direction = Directionality::of(app, context);
        if app.get(self).initial_menu.is_none() {
            let entries = Rc::clone(&self.widget(app).dropdown_menu_entries);
            let initial_menu = self.build_buttons(
                app,
                context,
                &entries,
                BuildButtonsOptions {
                    text_direction,
                    focused_index: None,
                    enable_scroll_to_highlight: false,
                    // The _initialMenu is invisible, we should not add semantics nodes to it
                    exclude_semantics: true,
                    use_material3: Some(use_material3),
                },
            );
            app.get_mut(self).initial_menu = Some(initial_menu);
        }
        let theme = DropdownMenuTheme::of(app, context);
        let defaults = DropdownMenuDefaultsM3::new(app, context);
        let widget = self.widget(app).clone();

        if app.get(self).enable_filter {
            let controller = self.effective_text_editing_controller(app);
            let filtered_entries = match &widget.filter_callback {
                Some(filter_callback) => {
                    let entries = Rc::clone(&app.get(self).filtered_entries);
                    let text = controller.text_value(app).to_owned();
                    filter_callback(app, &entries, &text)
                }
                None => Self::filter(&widget.dropdown_menu_entries, controller.handle(), app),
            };
            app.get_mut(self).filtered_entries = Rc::from(filtered_entries);
        }
        self.update_menu_has_enabled_item(app);

        if app.get(self).enable_search {
            if let Some(search_callback) = &widget.search_callback {
                let entries = Rc::clone(&app.get(self).filtered_entries);
                let text = self
                    .effective_text_editing_controller(app)
                    .text_value(app)
                    .to_owned();
                app.get_mut(self).current_highlight = search_callback(app, &entries, &text);
            } else {
                let entries = Rc::clone(&app.get(self).filtered_entries);
                let should_update_current_highlight =
                    self.should_update_current_highlight(app, &entries);
                if should_update_current_highlight {
                    let controller = self.effective_text_editing_controller(app);
                    app.get_mut(self).current_highlight =
                        Self::search(&entries, controller.handle(), app);
                }
            }
            if app.get(self).current_highlight.is_some() {
                self.scroll_to_highlight(app);
            }
        }

        let filtered_entries = Rc::clone(&app.get(self).filtered_entries);
        let current_highlight = app.get(self).current_highlight;
        let menu = self.build_buttons(
            app,
            context,
            &filtered_entries,
            BuildButtonsOptions {
                text_direction,
                focused_index: current_highlight,
                enable_scroll_to_highlight: true,
                exclude_semantics: false,
                use_material3: Some(use_material3),
            },
        );

        let base_text_style = widget
            .text_style
            .clone()
            .or_else(|| theme.text_style.clone())
            .or_else(|| defaults.text_style(app));
        let disabled_color = theme
            .disabled_color
            .or_else(|| defaults.disabled_color(app));
        let effective_text_style = if widget.enabled {
            base_text_style
        } else {
            Some(match base_text_style {
                Some(base_text_style) => {
                    let base_text_style = base_text_style.copy_with();
                    match disabled_color {
                        Some(disabled_color) => base_text_style.color(disabled_color),
                        None => base_text_style,
                    }
                }
                None => match disabled_color {
                    Some(disabled_color) => TextStyle::new().color(disabled_color),
                    None => TextStyle::new(),
                },
            })
        };

        let mut effective_menu_style = widget
            .menu_style
            .clone()
            .or_else(|| theme.menu_style.clone())
            .unwrap_or_else(|| {
                defaults
                    .menu_style(app)
                    .expect("the defaults have a menu style")
            });

        // Dart's `minimumSize` closures read `effectiveMenuStyle.maximumSize` when they are
        // resolved, after `menuHeight` has replaced it below.
        let final_maximum_size: Option<WidgetStatePropertyRef<Option<Size>>> =
            match widget.menu_height {
                Some(menu_height) => Some(Rc::new(WidgetStatePropertyAll::new(Some(Size::new(
                    f64::INFINITY,
                    menu_height,
                ))))),
                None => effective_menu_style.maximum_size.clone(),
            };
        let anchor_key = app.get(self).anchor_key.clone();
        let anchor_width = Self::get_width(app, &anchor_key);
        let minimum_width = widget.width.or(anchor_width);
        if let Some(minimum_width) = minimum_width {
            let maximum_size = final_maximum_size.clone();
            effective_menu_style.minimum_size = Some(
                <dyn WidgetStateProperty<Option<Size>>>::resolve_with(move |states| {
                    let effective_maximum_width = maximum_size
                        .as_ref()
                        .and_then(|maximum_size| maximum_size.resolve(states))
                        .map(|size| size.width());
                    Some(Size::new(
                        minimum_width.min(effective_maximum_width.unwrap_or(minimum_width)),
                        0.0,
                    ))
                }),
            );
        }

        if widget.menu_height.is_some() {
            effective_menu_style.maximum_size = final_maximum_size;
        }
        let effective_input_decoration_theme = widget
            .input_decoration_theme
            .clone()
            .or_else(|| theme.input_decoration_theme.clone())
            .unwrap_or_else(|| {
                defaults
                    .input_decoration_theme(app)
                    .expect("the defaults have an input decoration theme")
            });

        let effective_mouse_cursor: Option<MouseCursorRef> = match widget.enabled {
            true => Some(if self.is_button(app) {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::TEXT.into()
            }),
            false => None,
        };

        let controller = self.menu_controller(app).clone();
        let anchor_arguments = Rc::new(AnchorBuildArguments {
            effective_input_decoration_theme,
            effective_text_style,
            effective_mouse_cursor,
        });
        let mut menu_anchor = MenuAnchor::new(menu)
            .style(effective_menu_style)
            .reserved_padding(EdgeInsetsGeometry::ZERO)
            .controller(controller)
            .cross_axis_unconstrained(false)
            .builder(move |app, context, controller, _child| {
                self.build_anchor(app, context, controller, &anchor_arguments)
            });
        menu_anchor.alignment_offset = widget.alignment_offset;
        let mut menu_anchor = menu_anchor.into_widget();

        if let Some(padding) = widget.expanded_insets {
            menu_anchor = Padding::new(
                // Clamp the top and bottom padding to 0.
                padding.clamp(
                    &EdgeInsetsGeometry::ZERO,
                    &EdgeInsetsGeometry::only(f64::INFINITY, 0.0, f64::INFINITY, 0.0).add(
                        EdgeInsetsDirectional::only(f64::INFINITY, 0.0, f64::INFINITY, 0.0).into(),
                    ),
                ),
            )
            .child(menu_anchor)
            .into_widget();
        }

        // Wrap the menu anchor with an Align to narrow down the constraints.
        // Without this Align, when tight constraints are applied to DropdownMenu,
        // the menu will appear below these constraints instead of below the
        // text field.
        let menu_anchor = Align::new()
            .alignment(AlignmentGeometry::TOP_START)
            .width_factor(1.0)
            .height_factor(1.0)
            .child(menu_anchor);

        let actions = self.build_actions(app);
        let internal_focus_node = app
            .get(self)
            .internal_focus_node
            .as_ref()
            .expect("init_state makes the internal focus node")
            .erase::<AnyFocusNode>();
        Actions::new(
            actions,
            Stack::new().children([
                // Handling keyboard navigation when the Textfield has no focus.
                Shortcuts::new(
                    unfocused_text_field_shortcuts(),
                    Focus::new(SizedBox::shrink())
                        .focus_node(internal_focus_node)
                        .skip_traversal(true),
                )
                .into_widget(),
                menu_anchor.into_widget(),
            ]),
        )
        .into_widget()
    }
}

impl<T: Clone + PartialEq + 'static> Debug for DropdownMenuState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuState")
            .field("current_highlight", &self.current_highlight)
            .field("enable_filter", &self.enable_filter)
            .field("enable_search", &self.enable_search)
            .finish_non_exhaustive()
    }
}

// `DropdownMenu` dispatches these private intents on arrow up/down keys.
// They are needed instead of the typical `DirectionalFocusIntent`s because
// `DropdownMenu` does not really navigate the focus tree upon arrow up/down
// keys: the focus stays on the text field and the menu items are given fake
// highlights as if they are focused. Using `DirectionalFocusIntent`s will cause
// the action to be processed by `EditableText`.

/// Dart's `_ArrowUpIntent`.
#[derive(Clone, Copy, Debug)]
struct ArrowUpIntent;

impl Intent for ArrowUpIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `_ArrowDownIntent`.
#[derive(Clone, Copy, Debug)]
struct ArrowDownIntent;

impl Intent for ArrowDownIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `_EnterIntent`.
#[derive(Clone, Copy, Debug)]
struct EnterIntent;

impl Intent for EnterIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `_DropdownMenuBody`.
#[derive(Debug)]
struct DropdownMenuBody {
    children: Vec<WidgetRef>,
    width: Option<f64>,
}

impl RenderObjectWidget for DropdownMenuBody {
    type RenderObject = RenderDropdownMenuBody;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderDropdownMenuBody::new(app, self.width).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderDropdownMenuBody>,
    ) {
        render_object.set_width(app, self.width);
    }
}

impl MultiChildRenderObjectWidget for DropdownMenuBody {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_DropdownMenuBodyParentData`.
#[derive(Debug, Default)]
struct DropdownMenuBodyParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
}

impl DropdownMenuBodyParentData {
    const fn new() -> DropdownMenuBodyParentData {
        DropdownMenuBodyParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
        }
    }
}

impl ParentData for DropdownMenuBodyParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<DropdownMenuBodyParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<DropdownMenuBodyParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for DropdownMenuBodyParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for DropdownMenuBodyParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for DropdownMenuBodyParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.box_parent_data, f)
    }
}

/// Dart's `_RenderDropdownMenuBody`.
struct RenderDropdownMenuBody {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    width: Option<f64>,
}

impl RenderDropdownMenuBody {
    fn new(app: &mut App, width: Option<f64>) -> RenderHandle<RenderDropdownMenuBody> {
        RenderHandle::new_box(
            app,
            RenderDropdownMenuBody {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                width,
            },
        )
    }

    fn width(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).width
    }

    fn set_width(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).width == value {
            return;
        }
        self.get_mut(app).width = value;
        self.mark_needs_layout(app);
    }

    /// The constraints Dart's `performLayout` and `computeDryLayout` lay every child out in.
    fn inner_constraints(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        let intrinsic_width = match self.width(app) {
            Some(width) => width,
            None => self.get_max_intrinsic_width(app, constraints.max_height),
        };
        let width_constraint = intrinsic_width.min(constraints.max_width);
        BoxConstraints::new()
            .max_width(width_constraint)
            .max_height(self.get_max_intrinsic_height(app, width_constraint))
    }

    /// Dart's `computeMinIntrinsicWidth` and `computeMaxIntrinsicWidth` bodies, which differ
    /// only in the child intrinsic they read.
    fn compute_intrinsic_width(
        self: RenderHandle<Self>,
        app: &mut App,
        height: f64,
        child_intrinsic_width: fn(AnyRenderBox, &mut App, f64) -> f64,
    ) -> f64 {
        let first_child = self.first_child(app);
        let last_child = self.last_child(app);
        let mut child = first_child;
        let mut width: f64 = 0.0;
        while let Some(current) = child {
            if Some(current) == first_child {
                child = self.child_after(app, current);
                continue;
            }
            let intrinsic_width = child_intrinsic_width(current, app, height);
            // Add the width of leading icon.
            if Some(current) == last_child {
                width += intrinsic_width;
            }
            // Add the width of trailing icon.
            if Some(current) == self.child_before(app, last_child.expect("a last child")) {
                width += intrinsic_width;
            }
            width = width.max(intrinsic_width);
            child = self.child_after(app, current);
        }

        width.max(K_MINIMUM_WIDTH)
    }
}

impl ContainerRenderObjectMixin for RenderDropdownMenuBody {
    type ChildType = AnyRenderBox;
    type ParentDataType = DropdownMenuBodyParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderDropdownMenuBody {}

impl RenderObject for RenderDropdownMenuBody {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let mut max_width: f64 = 0.0;
        let mut max_height: Option<f64> = None;
        let first_child = self.first_child(app);
        let mut child = first_child;

        let inner_constraints = self.inner_constraints(app, constraints);
        while let Some(current) = child {
            if Some(current) == first_child {
                current.layout(app, inner_constraints, true);
                max_height.get_or_insert(current.size(app).height());
                child = self.child_after(app, current);
                continue;
            }
            current.layout(app, inner_constraints, true);
            current
                .as_object()
                .parent_data_of_mut::<DropdownMenuBodyParentData>(app)
                .set_offset(Offset::ZERO);
            max_width = max_width.max(current.size(app).width());
            max_height.get_or_insert(current.size(app).height());
            child = self.child_after(app, current);
        }

        let max_height = max_height.expect("a dropdown menu body has children");
        max_width = max_width.max(K_MINIMUM_WIDTH);
        let width = self.width(app).unwrap_or(max_width);
        self.set_size(app, constraints.constrain(Size::new(width, max_height)));
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.first_child(app) {
            let child_offset = child
                .as_object()
                .parent_data_of::<DropdownMenuBodyParentData>(app)
                .offset();
            context.paint_child(app, child.as_object(), offset + child_offset);
        }
    }
}

impl RenderBox for RenderDropdownMenuBody {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<DropdownMenuBodyParentData>(app) {
            child.set_parent_data(app, DropdownMenuBodyParentData::new());
        }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let mut max_width: f64 = 0.0;
        let mut max_height: Option<f64> = None;
        let first_child = self.first_child(app);
        let mut child = first_child;
        let inner_constraints = self.inner_constraints(app, constraints);

        while let Some(current) = child {
            let child_size = current.get_dry_layout(app, inner_constraints);

            // The first child is the TextField, which doesn't contribute to the
            // menu's width calculation.
            if Some(current) != first_child {
                max_width = max_width.max(child_size.width());
            }

            max_height.get_or_insert(child_size.height());
            child = self.child_after(app, current);
        }

        let max_height = max_height.expect("a dropdown menu body has children");
        max_width = max_width.max(K_MINIMUM_WIDTH);
        let width = self.width(app).unwrap_or(max_width);
        constraints.constrain(Size::new(width, max_height))
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.compute_intrinsic_width(app, height, |child, app, height| {
            child.get_min_intrinsic_width(app, height)
        })
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.compute_intrinsic_width(app, height, |child, app, height| {
            child.get_max_intrinsic_width(app, height)
        })
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let mut height: f64 = 0.0;
        if let Some(child) = self.first_child(app) {
            height = height.max(child.get_min_intrinsic_height(app, width));
        }
        height
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let mut height: f64 = 0.0;
        if let Some(child) = self.first_child(app) {
            height = height.max(child.get_max_intrinsic_height(app, width));
        }
        height
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if let Some(child) = self.first_child(app) {
            let offset = child
                .as_object()
                .parent_data_of::<DropdownMenuBodyParentData>(app)
                .offset();
            let is_hit =
                result.add_with_paint_offset(Some(offset), position, |result, transformed| {
                    debug_assert_eq!(transformed, position - offset);
                    child.hit_test(app, result, transformed)
                });
            if is_hit {
                return true;
            }
        }
        false
    }
}

/// Dart's `_DropdownMenuDefaultsM3`: hand coded defaults. These will be updated once we have
/// tokens/spec.
struct DropdownMenuDefaultsM3 {
    /// Dart's `super(..)` arguments.
    dropdown_menu_theme_data: DropdownMenuThemeData,
    context: DefaultsContext,
}

impl DropdownMenuDefaultsM3 {
    #[expect(deprecated, reason = "Dart calls the deprecated `withOpacity`")]
    fn new(app: &mut App, context: BuildContext) -> DropdownMenuDefaultsM3 {
        let disabled_color = Theme::of(app, context)
            .color_scheme
            .on_surface
            .with_opacity(0.38);
        DropdownMenuDefaultsM3 {
            dropdown_menu_theme_data: DropdownMenuThemeData::new().disabled_color(disabled_color),
            context: DefaultsContext::new(context),
        }
    }
}

impl DropdownMenuThemeDataGetters for DropdownMenuDefaultsM3 {
    fn dropdown_menu_theme_data(&self) -> &DropdownMenuThemeData {
        &self.dropdown_menu_theme_data
    }

    fn text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_large.clone()
    }

    fn menu_style(&self, _app: &mut App) -> Option<MenuStyle> {
        Some(
            MenuStyle::new()
                .minimum_size(Rc::new(WidgetStatePropertyAll::new(Some(Size::new(
                    K_MINIMUM_WIDTH,
                    0.0,
                )))))
                .maximum_size(Rc::new(WidgetStatePropertyAll::new(Some(Size::INFINITE))))
                .visual_density(VisualDensity::STANDARD),
        )
    }

    fn input_decoration_theme(&self, _app: &mut App) -> Option<InputDecorationThemeData> {
        Some(InputDecorationThemeData::new().border(OutlineInputBorder::new()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/dropdown_menu_test.dart`; the goldens and the semantics-tree cases are
    //! not ported.

    use waft_embedder::Rect;
    use waft_embedder::{Matrix4, TextBaseline};
    use waft_rendering::{
        RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
    };
    use waft_services::PhysicalKeyboardKey;
    use waft_services::TextInputClient;
    use waft_widgets::SingleChildRenderObjectWidget;
    use waft_widgets::{AnyElement, EditableText, EditableTextState};

    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, expect, find, finds_n_widgets,
        finds_nothing, finds_one_widget, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::material::Material;
    use crate::menu_anchor::MenuPanel;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::m3;
    use waft_widgets::FocusNodeLeaf;

    // ---- Dart's `TestMenu`, `ShortMenu` and the entry lists ----

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum TestMenu {
        MainMenu0,
        MainMenu1,
        MainMenu2,
        MainMenu3,
        MainMenu4,
        MainMenu5,
    }

    impl TestMenu {
        pub(crate) const VALUES: [TestMenu; 6] = [
            TestMenu::MainMenu0,
            TestMenu::MainMenu1,
            TestMenu::MainMenu2,
            TestMenu::MainMenu3,
            TestMenu::MainMenu4,
            TestMenu::MainMenu5,
        ];

        pub(crate) fn label(self) -> &'static str {
            match self {
                TestMenu::MainMenu0 => "Item 0",
                TestMenu::MainMenu1 => "Menu 1",
                TestMenu::MainMenu2 => "Item 2",
                TestMenu::MainMenu3 => "Item 3",
                TestMenu::MainMenu4 => "Item 4",
                TestMenu::MainMenu5 => "Item 5",
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ShortMenu {
        Item0,
        Item1,
        Item2,
    }

    impl ShortMenu {
        fn label(self) -> &'static str {
            match self {
                ShortMenu::Item0 => "I0",
                ShortMenu::Item1 => "I1",
                ShortMenu::Item2 => "I2",
            }
        }
    }

    /// Dart's `menuChildren`.
    pub(crate) fn menu_children() -> Rc<[DropdownMenuEntry<TestMenu>]> {
        TestMenu::VALUES
            .iter()
            .map(|value| DropdownMenuEntry::new(*value, value.label()))
            .collect()
    }

    fn short_menu_items() -> Rc<[DropdownMenuEntry<ShortMenu>]> {
        [ShortMenu::Item0, ShortMenu::Item1, ShortMenu::Item2]
            .iter()
            .map(|value| DropdownMenuEntry::new(*value, value.label()))
            .collect()
    }

    /// Dart's `buildTest` arguments.
    #[derive(Clone, Default)]
    struct BuildTest {
        width: Option<f64>,
        menu_height: Option<f64>,
        leading_icon: Option<WidgetRef>,
        label: Option<WidgetRef>,
        decoration_theme: Option<InputDecorationThemeData>,
    }

    impl BuildTest {
        /// Dart's `buildTest(themeData, entries, ...)`.
        fn build<T: Clone + PartialEq + 'static>(
            &self,
            theme: ThemeData,
            entries: Rc<[DropdownMenuEntry<T>]>,
        ) -> WidgetRef {
            let mut menu = DropdownMenu::new(entries);
            menu.label = self.label.clone();
            menu.leading_icon = self.leading_icon.clone();
            menu.width = self.width;
            menu.menu_height = self.menu_height;
            menu.input_decoration_theme = self.decoration_theme.clone();
            material_app(theme, Scaffold::new().body(menu).into_widget())
        }
    }

    // ---- finders and gestures ----

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    /// Dart's `find.ancestor(of: element, matching: find.byType(W))`, nearest first.
    fn ancestors_of_type<W: 'static>(
        tester: &WidgetTester,
        element: AnyElement,
    ) -> Vec<AnyElement> {
        let ancestors = tester.ancestors(element);
        let app = tester.cell.borrow();
        ancestors
            .into_iter()
            .filter(|ancestor| {
                waft_widgets::downcast_widget::<W>(&**ancestor.widget(&app)).is_some()
            })
            .collect()
    }

    /// Dart's `find.widgetWithText(W, text)`: each `W` with a `Text` of `text` under it.
    fn widgets_with_text<W: 'static>(tester: &WidgetTester, text: &str) -> Vec<AnyElement> {
        let texts = find::text(text).evaluate(tester);
        find::by_type::<W>()
            .evaluate(tester)
            .into_iter()
            .filter(|widget| {
                texts
                    .iter()
                    .any(|text| tester.ancestors(*text).contains(widget))
            })
            .collect()
    }

    /// Dart's `find.widgetWithIcon(W, icon)`.
    fn widgets_with_icon<W: 'static>(
        tester: &WidgetTester,
        icon: &waft_widgets::IconData,
    ) -> Vec<AnyElement> {
        let icons: Vec<AnyElement> = find::by_type::<Icon>()
            .evaluate(tester)
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .collect();
        find::by_type::<W>()
            .evaluate(tester)
            .into_iter()
            .filter(|widget| {
                icons
                    .iter()
                    .any(|icon| tester.ancestors(*icon).contains(widget))
            })
            .collect()
    }

    /// Dart's `findMenuItemButton(label)`: the last `MenuItemButton` with that label; the
    /// first is the invisible copy `_DropdownMenuBody` measures.
    fn find_menu_item_button(tester: &WidgetTester, label: &str) -> AnyElement {
        *widgets_with_text::<MenuItemButton>(tester, label)
            .last()
            .expect("a menu item button with that label")
    }

    /// Dart's `getButtonMaterial`.
    fn button_material<R>(
        tester: &WidgetTester,
        label: &str,
        read: impl FnOnce(&Material) -> R,
    ) -> R {
        let button = find_menu_item_button(tester, label);
        let material = only(tester.descendants_of_type::<Material>(button));
        tester.widget::<Material, R>(material, read)
    }

    /// Dart's `isItemHighlighted`.
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn is_item_highlighted(tester: &WidgetTester, theme: &ThemeData, label: &str) -> bool {
        let color = button_material(tester, label, |material| material.color);
        color == Some(theme.color_scheme.on_surface.with_opacity(0.12))
    }

    /// Dart's `findMenuMaterial()`.
    fn menu_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let panel = tester.element(find::by_type::<MenuPanel>());
        let material = tester.descendants_of_type::<Material>(panel)[0];
        tester.widget::<Material, R>(material, read)
    }

    fn first<W: 'static>(tester: &WidgetTester) -> AnyElement {
        find::by_type::<W>().evaluate(tester)[0]
    }

    /// Dart's `tester.tap(finder)`.
    fn tap(tester: &mut WidgetTester, element: AnyElement) {
        let center = tester.get_rect(element).center();
        tester.tap_at(center, 1);
    }

    /// Dart's `tester.tap(find.byType(DropdownMenu<T>))`.
    fn tap_dropdown_menu<T: 'static>(tester: &mut WidgetTester) {
        let menu = dropdown_menu::<T>(tester);
        tap(tester, menu);
    }

    /// Dart's `tester.tap(find.byType(W).first)`.
    fn tap_first<W: 'static>(tester: &mut WidgetTester) {
        let element = first::<W>(tester);
        tap(tester, element);
    }

    fn pump(tester: &mut WidgetTester) {
        tester.pump();
    }

    /// Dart's `tester.sendKeyEvent(key)`.
    fn send_key(tester: &mut WidgetTester, logical: LogicalKeyboardKey) {
        let physical = match logical {
            LogicalKeyboardKey::ARROW_DOWN => PhysicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ARROW_UP => PhysicalKeyboardKey::ARROW_UP,
            LogicalKeyboardKey::ARROW_LEFT => PhysicalKeyboardKey::ARROW_LEFT,
            LogicalKeyboardKey::ARROW_RIGHT => PhysicalKeyboardKey::ARROW_RIGHT,
            LogicalKeyboardKey::TAB => PhysicalKeyboardKey::TAB,
            LogicalKeyboardKey::ESCAPE => PhysicalKeyboardKey::ESCAPE,
            LogicalKeyboardKey::ENTER => PhysicalKeyboardKey::ENTER,
            LogicalKeyboardKey::SPACE => PhysicalKeyboardKey::SPACE,
            _ => panic!("no physical key for {logical:?} in this kit"),
        };
        tester.send_key_event(physical, logical);
    }

    fn editable_state(tester: &WidgetTester) -> Handle<EditableTextState> {
        first::<EditableText>(tester)
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state")
    }

    fn text_field<R>(tester: &WidgetTester, read: impl FnOnce(&TextField) -> R) -> R {
        tester.widget::<TextField, R>(first::<TextField>(tester), read)
    }

    fn editable<R>(tester: &WidgetTester, read: impl FnOnce(&EditableText) -> R) -> R {
        tester.widget::<EditableText, R>(first::<EditableText>(tester), read)
    }

    fn pump_app(tester: &mut WidgetTester, root: impl FnOnce(&mut App) -> WidgetRef) {
        let root = root(&mut tester.cell.borrow_mut());
        tester.pump_widget(root);
    }

    fn theme(tester: &WidgetTester) -> ThemeData {
        m3(&tester.cell.borrow())
    }

    fn the_menu<T: Clone + PartialEq + 'static>(
        theme: ThemeData,
        menu: DropdownMenu<T>,
    ) -> WidgetRef {
        material_app(theme, Scaffold::new().body(menu).into_widget())
    }

    // ---- cases ----

    // 'DropdownMenu defaults'
    #[test]
    fn a_dropdown_menu_takes_its_defaults_from_the_theme() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(BuildTest::default().build(theme.clone(), menu_children()));

            let body_large = theme.text_theme.body_large.clone().expect("a body large");
            editable(tester, |editable| {
                assert_eq!(editable.style.color, body_large.color);
                assert_eq!(editable.style.background, body_large.background);
                assert_eq!(editable.style.shadows, body_large.shadows);
                assert_eq!(editable.style.decoration, body_large.decoration);
                assert_eq!(editable.style.word_spacing, body_large.word_spacing);
                assert_eq!(editable.style.font_size, Some(16.0));
                assert_eq!(editable.style.height, Some(1.5));
            });
            text_field(tester, |text_field| {
                let decoration = text_field.decoration.as_ref().expect("a decoration");
                assert!(
                    decoration
                        .border
                        .as_ref()
                        .is_some_and(|border| border.eq_shape(&OutlineInputBorder::new()))
                );
                let style = text_field.style.as_ref().expect("a style");
                assert_eq!(style.font_size, Some(16.0));
                assert_eq!(style.height, Some(1.5));
            });

            let icon_button = widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_down())[0];
            tap(tester, icon_button);
            pump(tester);
            expect(tester, &find::by_type::<MenuAnchor>(), finds_one_widget());

            menu_material(tester, |material| {
                assert_eq!(material.color, Some(theme.color_scheme.surface_container()));
                assert_eq!(material.shadow_color, Some(theme.color_scheme.shadow()));
                assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
                assert_eq!(material.elevation, 3.0);
                assert!(material.shape.as_ref().is_some_and(|shape| shape.eq_shape(
                    &waft_painting::RoundedRectangleBorder::new(
                        waft_painting::BorderSide::NONE,
                        waft_painting::BorderRadiusGeometry::all(waft_embedder::Radius::circular(
                            4.0
                        )),
                    )
                )));
            });

            button_material(tester, TestMenu::MainMenu0.label(), |material| {
                assert_eq!(material.color, Some(Colors::TRANSPARENT));
                assert_eq!(material.elevation, 0.0);
                assert!(material.shape.as_ref().is_some_and(|shape| {
                    shape.eq_shape(&waft_painting::RoundedRectangleBorder::default())
                }));
                let text_style = material.text_style.as_ref().expect("a text style");
                assert_eq!(text_style.color, Some(theme.color_scheme.on_surface.into()));
                assert_eq!(text_style.font_size, Some(14.0));
                assert_eq!(text_style.height, Some(1.43));
            });
        });
    }

    // ---- the 'Item style' group ----
    //
    // Each case but the last ends by hovering the two items and expecting each one's ink to
    // paint a rect of its overlay colour at alpha 0. The display list drops a draw with no
    // effect, as Flutter's engine does, and Dart's matcher sees it only through
    // `TestRecordingCanvas`, so those expectations are Skipped and the cases end before the hover.

    const FOCUSED_BACKGROUND_COLOR: Color = Color::new(0xffff0000);
    const FOCUSED_FOREGROUND_COLOR: Color = Color::new(0xff00ff00);
    const FOCUSED_ICON_COLOR: Color = Color::new(0xff0000ff);
    const FOCUSED_OVERLAY_COLOR: Color = Color::new(0xffff00ff);
    const DEFAULT_BACKGROUND_COLOR: Color = Color::new(0xff00ffff);
    const DEFAULT_FOREGROUND_COLOR: Color = Color::new(0xff000000);
    const DEFAULT_ICON_COLOR: Color = Color::new(0xffffffff);
    const DEFAULT_OVERLAY_COLOR: Color = Color::new(0xffffff00);

    /// A property that resolves to `focused` in the focused state and to `default` otherwise.
    fn focused_or(focused: Color, default: Color) -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Focused) {
                return Some(focused);
            }
            Some(default)
        })
    }

    /// The group's `customButtonStyle`.
    fn custom_button_style() -> ButtonStyle {
        ButtonStyle::new()
            .background_color(focused_or(
                FOCUSED_BACKGROUND_COLOR,
                DEFAULT_BACKGROUND_COLOR,
            ))
            .foreground_color(focused_or(
                FOCUSED_FOREGROUND_COLOR,
                DEFAULT_FOREGROUND_COLOR,
            ))
            .icon_color(focused_or(FOCUSED_ICON_COLOR, DEFAULT_ICON_COLOR))
            .overlay_color(focused_or(FOCUSED_OVERLAY_COLOR, DEFAULT_OVERLAY_COLOR))
    }

    /// The entries with icons, each styled with `style`: the group's `styledMenuEntries`.
    fn styled_menu_entries(style: &ButtonStyle) -> Rc<[DropdownMenuEntry<TestMenu>]> {
        menu_children_with_icons()
            .iter()
            .cloned()
            .map(|entry| entry.style(style.clone()))
            .collect()
    }

    /// The group's `iconStyle`: the colour of the glyph of the last widget keyed `key`.
    fn item_icon_color(tester: &WidgetTester, key: &KeyRef) -> Option<Color> {
        let icon = *find_by_key(tester, key).last().expect("an icon keyed so");
        tester.glyph_style(icon).color.as_deref().copied()
    }

    /// The menu with `entries` around `selected_item`, opened.
    fn open_styled_menu(
        tester: &mut WidgetTester,
        theme: ThemeData,
        entries: Rc<[DropdownMenuEntry<TestMenu>]>,
        selected_item: TestMenu,
    ) {
        tester.pump_widget(the_menu(
            theme,
            DropdownMenu::new(entries).initial_selection(selected_item),
        ));

        // Open the menu.
        tap_dropdown_menu::<TestMenu>(tester);
        pump(tester);
    }

    /// The colours of an item: its button's material colour and text colour, and its leading
    /// icon's colour.
    fn item_colors(
        tester: &WidgetTester,
        item: TestMenu,
    ) -> (Option<Color>, Option<Color>, Option<Color>) {
        let (color, text_color) = button_material(tester, item.label(), |material| {
            (
                material.color,
                material
                    .text_style
                    .as_ref()
                    .and_then(|style| style.color.as_deref().copied()),
            )
        });
        (
            color,
            text_color,
            item_icon_color(tester, &leading_icon_key(item)),
        )
    }

    // 'Item style' 'defaults are correct'
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn item_style_defaults_are_correct() {
        test_widgets(|tester| {
            let selected_item = TestMenu::MainMenu3;
            let non_selected_item = TestMenu::MainMenu2;
            let theme = theme(tester);
            open_styled_menu(
                tester,
                theme.clone(),
                menu_children_with_icons(),
                selected_item,
            );

            let on_surface = theme.color_scheme.on_surface;
            let on_surface_variant = theme.color_scheme.on_surface_variant();
            assert_eq!(
                item_colors(tester, selected_item),
                (
                    Some(on_surface.with_opacity(0.12)),
                    Some(on_surface),
                    Some(on_surface_variant)
                )
            );
            assert_eq!(
                item_colors(tester, non_selected_item),
                (
                    Some(Colors::TRANSPARENT),
                    Some(on_surface),
                    Some(on_surface_variant)
                )
            );
        });
    }

    /// The expectations the cases with `customButtonStyle` share.
    fn assert_custom_button_style(tester: &mut WidgetTester) {
        let selected_item = TestMenu::MainMenu3;
        let non_selected_item = TestMenu::MainMenu2;
        assert_eq!(
            item_colors(tester, selected_item),
            (
                Some(FOCUSED_BACKGROUND_COLOR),
                Some(FOCUSED_FOREGROUND_COLOR),
                Some(FOCUSED_ICON_COLOR)
            )
        );
        assert_eq!(
            item_colors(tester, non_selected_item),
            (
                Some(DEFAULT_BACKGROUND_COLOR),
                Some(DEFAULT_FOREGROUND_COLOR),
                Some(DEFAULT_ICON_COLOR)
            )
        );
    }

    // 'Item style' 'can be overridden at application theme level'
    #[test]
    fn item_style_can_be_overridden_at_application_theme_level() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .menu_button_theme(
                    crate::menu_button_theme::MenuButtonThemeData::new()
                        .style(custom_button_style()),
                )
                .build(&tester.cell.borrow());
            open_styled_menu(
                tester,
                theme,
                menu_children_with_icons(),
                TestMenu::MainMenu3,
            );
            assert_custom_button_style(tester);
        });
    }

    // 'Item style' 'can be overridden at menu entry level'
    #[test]
    fn item_style_can_be_overridden_at_menu_entry_level() {
        test_widgets(|tester| {
            let theme = theme(tester);
            open_styled_menu(
                tester,
                theme,
                styled_menu_entries(&custom_button_style()),
                TestMenu::MainMenu3,
            );
            assert_custom_button_style(tester);
        });
    }

    // 'Item style' 'defined at menu entry level takes precedence'
    #[test]
    fn item_style_defined_at_menu_entry_level_takes_precedence() {
        test_widgets(|tester| {
            let lucky_color = Color::new(0xff777777);
            let all = |color: Color| -> WidgetStatePropertyRef<Option<Color>> {
                Rc::new(WidgetStatePropertyAll::new(Some(color)))
            };
            let single_color_button_style = ButtonStyle::new()
                .background_color(all(lucky_color))
                .foreground_color(all(lucky_color))
                .icon_color(all(lucky_color))
                .overlay_color(all(lucky_color));
            let theme = ThemeData::builder()
                .menu_button_theme(
                    crate::menu_button_theme::MenuButtonThemeData::new()
                        .style(single_color_button_style),
                )
                .build(&tester.cell.borrow());
            open_styled_menu(
                tester,
                theme,
                styled_menu_entries(&custom_button_style()),
                TestMenu::MainMenu3,
            );
            assert_custom_button_style(tester);
        });
    }

    // 'Item style' 'defined at menu entry level and application level are merged'
    #[test]
    fn item_style_defined_at_menu_entry_level_and_application_level_are_merged() {
        test_widgets(|tester| {
            let selected_item = TestMenu::MainMenu3;
            let non_selected_item = TestMenu::MainMenu2;
            let lucky_color = Color::new(0xff777777);
            let all = |color: Color| -> WidgetStatePropertyRef<Option<Color>> {
                Rc::new(WidgetStatePropertyAll::new(Some(color)))
            };
            let partial_button_style = ButtonStyle::new()
                .background_color(all(lucky_color))
                .foreground_color(all(lucky_color));
            let theme = ThemeData::builder()
                .menu_button_theme(
                    crate::menu_button_theme::MenuButtonThemeData::new()
                        .style(custom_button_style()),
                )
                .build(&tester.cell.borrow());
            open_styled_menu(
                tester,
                theme,
                styled_menu_entries(&partial_button_style),
                selected_item,
            );

            assert_eq!(
                item_colors(tester, selected_item),
                (
                    Some(lucky_color),
                    Some(lucky_color),
                    Some(FOCUSED_ICON_COLOR)
                )
            );
            assert_eq!(
                item_colors(tester, non_selected_item),
                (
                    Some(lucky_color),
                    Some(lucky_color),
                    Some(DEFAULT_ICON_COLOR)
                )
            );
        });
    }

    // 'Item style' 'textStyle property is resolved when item is highlighted'
    // Regression test for https://github.com/flutter/flutter/issues/177363.
    #[test]
    fn item_style_text_style_is_resolved_when_the_item_is_highlighted() {
        test_widgets(|tester| {
            let selected_item = TestMenu::MainMenu3;
            let non_selected_item = TestMenu::MainMenu2;
            let custom_button_style =
                ButtonStyle::new().text_style(
                    <dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(|states| {
                        Some(TextStyle::new().font_weight(
                            if states.contains(&WidgetState::Focused) {
                                waft_embedder::FontWeight::BOLD
                            } else {
                                waft_embedder::FontWeight::NORMAL
                            },
                        ))
                    }),
                );
            let menu_entries: Rc<[DropdownMenuEntry<TestMenu>]> = menu_children()
                .iter()
                .cloned()
                .map(|entry| entry.style(custom_button_style.clone()))
                .collect();
            let theme = theme(tester);
            open_styled_menu(tester, theme, menu_entries, selected_item);

            let item_label_weight = |tester: &WidgetTester, label: &str| {
                let button = find_menu_item_button(tester, label);
                let text = *find::text(label)
                    .evaluate(tester)
                    .iter()
                    .find(|text| tester.ancestors(**text).contains(&button))
                    .expect("the item's label");
                let render = tester.render_box_of(text);
                let app = tester.cell.borrow();
                let paragraph = render
                    .as_object()
                    .downcast::<waft_rendering::RenderParagraph>(&app)
                    .expect("a Text renders a paragraph");
                paragraph
                    .text(&app)
                    .style()
                    .and_then(|style| style.font_weight)
            };
            assert_eq!(
                item_label_weight(tester, selected_item.label()),
                Some(waft_embedder::FontWeight::BOLD)
            );
            assert_eq!(
                item_label_weight(tester, non_selected_item.label()),
                Some(waft_embedder::FontWeight::NORMAL)
            );
        });
    }

    /// `MaterialApp(theme:, home: Scaffold(body: SafeArea(child: menu)))`.
    fn safe_area_menu<T: Clone + PartialEq + 'static>(
        theme: ThemeData,
        menu: DropdownMenu<T>,
    ) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new()
                .body(waft_widgets::SafeArea::new(menu))
                .into_widget(),
        )
    }

    fn m2(app: &App) -> ThemeData {
        ThemeData::builder().use_material3(false).build(app)
    }

    /// Dart's `find.byType(DropdownMenu<T>)`.
    fn dropdown_menu<T: 'static>(tester: &WidgetTester) -> AnyElement {
        tester.element(find::by_type::<DropdownMenu<T>>())
    }

    /// Dart's `find.ancestor(of: find.byType(SingleChildScrollView), matching: find.byType(W)).first`.
    fn menu_ancestor<W: 'static>(tester: &WidgetTester) -> AnyElement {
        let scroll_view =
            find::by_type::<waft_widgets::SingleChildScrollView>().evaluate(tester)[0];
        ancestors_of_type::<W>(tester, scroll_view)[0]
    }

    fn arrow_drop_down_button(tester: &WidgetTester) -> AnyElement {
        widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_down())[0]
    }

    // 'Inner TextField is disabled when DropdownMenu is disabled'
    #[test]
    fn a_disabled_dropdown_menu_disables_its_text_field_and_opens_no_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(safe_area_menu(
                theme,
                DropdownMenu::new(menu_children()).enabled(false),
            ));

            text_field(tester, |text_field| {
                assert_eq!(text_field.enabled, Some(false))
            });
            assert!(
                find::by_type::<waft_widgets::SingleChildScrollView>()
                    .evaluate(tester)
                    .is_empty()
            );

            tap_first::<TextField>(tester);
            pump(tester);
            assert!(
                find::by_type::<waft_widgets::SingleChildScrollView>()
                    .evaluate(tester)
                    .is_empty()
            );
        });
    }

    // 'Inner IconButton is disabled when DropdownMenu is disabled'
    #[test]
    fn a_disabled_dropdown_menu_disables_its_trailing_icon_button() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(safe_area_menu(
                theme,
                DropdownMenu::new(menu_children()).enabled(false),
            ));

            let button = arrow_drop_down_button(tester);
            tester.widget(button, |button: &IconButton| {
                assert!(button.on_pressed.is_none())
            });
        });
    }

    // 'Material2 - The width of the text field should always be the same as the menu view'
    #[test]
    fn material2_the_text_field_is_as_wide_as_the_menu() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());
            let entries = menu_children();
            tester.pump_widget(safe_area_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries)),
            ));

            assert_eq!(
                tester.get_size(first::<TextField>(tester)),
                Size::new(180.0, 56.0)
            );

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            let menu_material = menu_ancestor::<Material>(tester);
            assert_eq!(tester.get_size(menu_material), Size::new(180.0, 304.0));

            // The text field should have same width as the menu
            // when the width property is not null.
            tester.pump_widget(
                BuildTest {
                    width: Some(200.0),
                    ..BuildTest::default()
                }
                .build(theme, entries),
            );

            let anchor = first::<TextField>(tester);
            assert_eq!(tester.get_size(anchor).width(), 200.0);

            tap(tester, anchor);
            tester.pump_and_settle();

            let updated_menu = menu_ancestor::<Material>(tester);
            assert_eq!(tester.get_size(updated_menu).width(), 200.0);
        });
    }

    // 'Material3 - The width of the text field should always be the same as the menu view'
    #[test]
    fn material3_the_text_field_is_as_wide_as_the_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            tester.pump_widget(safe_area_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries)),
            ));

            let anchor_width = tester.get_size(first::<TextField>(tester)).width();
            assert!((anchor_width - 184.5).abs() < 0.1, "{anchor_width}");

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            let menu_width = tester.get_size(menu_ancestor::<Material>(tester)).width();
            assert!((menu_width - 184.5).abs() < 0.1, "{menu_width}");

            // The text field should have same width as the menu
            // when the width property is not null.
            tester.pump_widget(
                BuildTest {
                    width: Some(200.0),
                    ..BuildTest::default()
                }
                .build(theme, entries),
            );

            let anchor = first::<TextField>(tester);
            assert_eq!(tester.get_size(anchor).width(), 200.0);

            tap(tester, anchor);
            tester.pump_and_settle();

            let updated_menu = menu_ancestor::<Material>(tester);
            assert_eq!(tester.get_size(updated_menu).width(), 200.0);
        });
    }

    // 'The width property can customize the width of the dropdown menu'
    #[test]
    fn the_width_property_sets_the_width_of_the_field_and_its_items() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let short_menu_items = short_menu_items();

            let custom_big_width = 250.0;
            tester.pump_widget(
                BuildTest {
                    width: Some(custom_big_width),
                    ..BuildTest::default()
                }
                .build(theme.clone(), Rc::clone(&short_menu_items)),
            );
            let menu = dropdown_menu::<ShortMenu>(tester);
            assert_eq!(tester.get_size(menu).width(), custom_big_width);

            tap(tester, menu);
            pump(tester);
            expect(
                tester,
                &find::by_type::<MenuItemButton>(),
                finds_n_widgets(6),
            );
            let button = find_menu_item_button(tester, "I0");
            assert_eq!(tester.get_size(button).width(), custom_big_width);

            // reset test
            tester.pump_widget(SizedBox::new().into_widget());
            let custom_small_width = 100.0;
            tester.pump_widget(
                BuildTest {
                    width: Some(custom_small_width),
                    ..BuildTest::default()
                }
                .build(theme, short_menu_items),
            );
            let menu = dropdown_menu::<ShortMenu>(tester);
            assert_eq!(tester.get_size(menu).width(), custom_small_width);

            tap(tester, menu);
            pump(tester);
            expect(
                tester,
                &find::by_type::<MenuItemButton>(),
                finds_n_widgets(6),
            );
            let button = find_menu_item_button(tester, "I0");
            assert_eq!(tester.get_size(button).width(), custom_small_width);
        });
    }

    // 'The width property update test'
    #[test]
    fn a_new_width_resizes_the_dropdown_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let short_menu_items = short_menu_items();

            tester.pump_widget(
                BuildTest {
                    width: Some(250.0),
                    ..BuildTest::default()
                }
                .build(theme.clone(), Rc::clone(&short_menu_items)),
            );
            let menu = dropdown_menu::<ShortMenu>(tester);
            assert_eq!(tester.get_size(menu).width(), 250.0);

            // Update width
            tester.pump_widget(
                BuildTest {
                    width: Some(400.0),
                    ..BuildTest::default()
                }
                .build(theme, short_menu_items),
            );
            let menu = dropdown_menu::<ShortMenu>(tester);
            assert_eq!(tester.get_size(menu).width(), 400.0);
        });
    }

    // 'The width is determined by the menu entries'
    #[test]
    fn without_a_width_the_widest_entry_sets_the_width() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entry_label_width = 100.0;
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![
                    DropdownMenuEntry::new(0, "Flutter")
                        .label_widget(SizedBox::new().width(entry_label_width)),
                ]),
            ));

            let width = tester.get_size(dropdown_menu::<i32>(tester)).width();
            let menu_entry_padding = 24.0; // See _kDefaultHorizontalPadding.
            let decoration_start_gap = 4.0; // See _kInputStartGap.
            let leading_width = 16.0;
            let trailing_width = 56.0;

            assert_eq!(
                width,
                entry_label_width
                    + leading_width
                    + trailing_width
                    + menu_entry_padding
                    + decoration_start_gap
            );
        });
    }

    // 'The width is determined by the label when it is longer than menu entries'
    #[test]
    fn a_label_wider_than_the_entries_sets_the_width() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let label_width = 120.0;
            let entry_label_width = 100.0;
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![
                    DropdownMenuEntry::new(0, "Flutter")
                        .label_widget(SizedBox::new().width(entry_label_width)),
                ])
                .label(SizedBox::new().width(label_width)),
            ));

            let width = tester.get_size(dropdown_menu::<i32>(tester)).width();
            let leading_width = 16.0;
            let trailing_width = 56.0;
            let label_padding = 8.0; // See RenderEditable.floatingCursorAddedMargin.

            assert_eq!(
                width,
                label_width + label_padding + leading_width + trailing_width
            );
        });
    }

    fn sized_short_menu(
        expanded_insets: Option<EdgeInsetsGeometry>,
    ) -> impl FnOnce(&mut App) -> WidgetRef {
        move |app| {
            let mut menu = DropdownMenu::new(short_menu_items_shared());
            menu.expanded_insets = expanded_insets;
            material_app(
                m3(app),
                Scaffold::new()
                    .body(SizedBox::square(Some(500.0)).child(menu))
                    .into_widget(),
            )
        }
    }

    thread_local! {
        static SHORT_MENU_ITEMS: Rc<[DropdownMenuEntry<ShortMenu>]> = short_menu_items();
    }

    /// Dart's one `shortMenuItems` list, which each rebuild of a case hands the menu again.
    fn short_menu_items_shared() -> Rc<[DropdownMenuEntry<ShortMenu>]> {
        SHORT_MENU_ITEMS.with(Rc::clone)
    }

    /// The rest of 'The width of MenuAnchor respects MenuAnchor.expandedInsets' and its
    /// directional twin after the default width: the field and the items are `500 - 35 - 20`
    /// wide and the field is at the top of the sized box.
    fn assert_insets_narrow_the_field(tester: &mut WidgetTester, insets: EdgeInsetsGeometry) {
        let parent_width = 500.0;
        tester.pump_widget(SizedBox::new().into_widget());
        pump_app(tester, sized_short_menu(Some(insets)));
        let field = first::<TextField>(tester);
        assert_eq!(tester.get_size(field).width(), parent_width - 35.0 - 20.0);
        let container_rect = tester.get_rect(first::<SizedBox>(tester));
        let dropdown_menu_rect = tester.get_rect(field);
        assert_eq!(dropdown_menu_rect.top, container_rect.top);

        tap(tester, field);
        tester.pump_and_settle();

        let button = find_menu_item_button(tester, "I0");
        assert_eq!(tester.get_size(button).width(), parent_width - 35.0 - 20.0);
    }

    // 'The width of MenuAnchor respects MenuAnchor.expandedInsets'
    #[test]
    fn expanded_insets_make_the_field_and_its_menu_as_wide_as_the_parent_less_the_insets() {
        test_widgets(|tester| {
            let parent_width = 500.0;

            // By default, the width of the text field is determined by the menu children.
            pump_app(tester, sized_short_menu(None));
            let field = first::<TextField>(tester);
            assert_eq!(tester.get_size(field).width(), 136.0);

            tap(tester, field);
            tester.pump_and_settle();
            let button = find_menu_item_button(tester, "I0");
            assert_eq!(tester.get_size(button).width(), 136.0);

            // If expandedInsets is EdgeInsets.zero, the width should be the same as its parent.
            tester.pump_widget(SizedBox::new().into_widget());
            pump_app(tester, sized_short_menu(Some(EdgeInsetsGeometry::ZERO)));
            let field = first::<TextField>(tester);
            assert_eq!(tester.get_size(field).width(), parent_width);

            tap(tester, field);
            tester.pump_and_settle();
            let button = find_menu_item_button(tester, "I0");
            assert_eq!(tester.get_size(button).width(), parent_width);

            // If expandedInsets is not zero, the width of the text field should be adjusted
            // based on the EdgeInsets.left and EdgeInsets.right. The top and bottom values
            // will be ignored.
            assert_insets_narrow_the_field(tester, EdgeInsetsGeometry::only(35.0, 50.0, 20.0, 0.0));
        });
    }

    // 'expandedInsets can use EdgeInsets or EdgeInsetsDirectional'
    #[test]
    fn expanded_insets_can_be_directional() {
        test_widgets(|tester| {
            // By default, the width of the text field is determined by the menu children.
            pump_app(tester, sized_short_menu(None));
            let field = first::<TextField>(tester);
            assert_eq!(tester.get_size(field).width(), 136.0);
            tap(tester, field);
            tester.pump_and_settle();
            let button = find_menu_item_button(tester, "I0");
            assert_eq!(tester.get_size(button).width(), 136.0);

            assert_insets_narrow_the_field(tester, EdgeInsetsGeometry::only(35.0, 50.0, 20.0, 0.0));
            // Regression test for https://github.com/flutter/flutter/issues/151769.
            assert_insets_narrow_the_field(
                tester,
                EdgeInsetsDirectional::only(35.0, 50.0, 20.0, 0.0).into(),
            );
        });
    }

    // 'Menu panel width can expand to full-screen width'
    #[test]
    fn zero_expanded_insets_make_the_menu_panel_as_wide_as_the_screen() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![DropdownMenuEntry::new(0, "Flutter")])
                    .expanded_insets(EdgeInsetsGeometry::ZERO),
            ));

            let dropdown_width = tester.get_size(dropdown_menu::<i32>(tester)).width();
            assert_eq!(dropdown_width, 800.0);

            tap_dropdown_menu::<i32>(tester);
            pump(tester);

            let menu_width = tester
                .get_size(find_menu_item_button(tester, "Flutter"))
                .width();
            assert_eq!(dropdown_width, menu_width);
        });
    }

    /// The vertical extent of the items 'Item 0' to 'Item 5' in the open menu.
    fn items_extent(tester: &WidgetTester) -> f64 {
        let first_rect = tester.get_rect(find_menu_item_button(tester, "Item 0"));
        let last_rect = tester.get_rect(find_menu_item_button(tester, "Item 5"));
        last_rect.bottom - first_rect.top
    }

    // 'Material2 - The menuHeight property can be used to show a shorter scrollable menu list
    // instead of the complete list'
    #[test]
    fn material2_menu_height_limits_the_menu() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());
            let entries = menu_children();
            tester.pump_widget(BuildTest::default().build(theme.clone(), Rc::clone(&entries)));

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            // height = height of MenuItemButton * 6 = 48 * 6
            assert_eq!(items_extent(tester), 288.0);

            let menu_view = menu_ancestor::<Padding>(tester);
            // 304 = 288 + vertical padding(2 * 8)
            assert_eq!(tester.get_size(menu_view), Size::new(180.0, 304.0));

            // Constrains the menu height.
            tester.pump_widget(SizedBox::new().into_widget());
            tester.pump_widget(
                BuildTest {
                    menu_height: Some(100.0),
                    ..BuildTest::default()
                }
                .build(theme, entries),
            );
            tester.pump_and_settle();

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            let updated_menu = menu_ancestor::<Padding>(tester);
            assert_eq!(tester.get_size(updated_menu), Size::new(180.0, 100.0));
        });
    }

    // 'Material3 - The menuHeight property can be used to show a shorter scrollable menu list
    // instead of the complete list'
    #[test]
    fn material3_menu_height_limits_the_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            tester.pump_widget(BuildTest::default().build(theme.clone(), Rc::clone(&entries)));

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            // height = height of MenuItemButton * 6 = 48 * 6
            assert_eq!(items_extent(tester), 288.0);

            let menu_view = menu_ancestor::<Padding>(tester);
            // 304 = 288 + vertical padding(2 * 8)
            assert_eq!(tester.get_size(menu_view).height(), 304.0);

            // Constrains the menu height.
            tester.pump_widget(SizedBox::new().into_widget());
            tester.pump_widget(
                BuildTest {
                    menu_height: Some(100.0),
                    ..BuildTest::default()
                }
                .build(theme, entries),
            );
            tester.pump_and_settle();

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            let updated_menu = menu_ancestor::<Padding>(tester);
            assert_eq!(tester.get_size(updated_menu).height(), 100.0);
        });
    }

    // 'DropdownMenu has default trailing icon button'
    #[test]
    fn the_default_trailing_icon_button_opens_the_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(BuildTest::default().build(theme, menu_children()));
            pump(tester);

            let icon_button = arrow_drop_down_button(tester);
            tap(tester, icon_button);
            pump(tester);

            let button = find_menu_item_button(tester, TestMenu::MainMenu0.label());
            assert!(!ancestors_of_type::<Material>(tester, button).is_empty());
        });
    }

    // 'Trailing IconButton status test'
    #[test]
    fn the_trailing_icon_shows_whether_the_menu_is_open() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(
                BuildTest {
                    width: Some(100.0),
                    menu_height: Some(100.0),
                    ..BuildTest::default()
                }
                .build(theme, menu_children()),
            );
            pump(tester);

            assert!(widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_up()).is_empty());
            let icon_button = arrow_drop_down_button(tester);

            tap(tester, icon_button);
            pump(tester);

            assert!(!widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_up()).is_empty());
            assert!(widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_down()).is_empty());

            // Tap outside
            tester.tap_at(Offset::new(500.0, 500.0), 1);
            pump(tester);

            assert!(widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_up()).is_empty());
            assert!(!widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_down()).is_empty());
        });
    }

    // 'Trailing IconButton height respects InputDecorationTheme.suffixIconConstraints'
    #[test]
    fn the_trailing_icon_button_takes_the_suffix_icon_constraints() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();

            // Default suffix icon constraints.
            tester.pump_widget(BuildTest::default().build(theme.clone(), Rc::clone(&entries)));
            pump(tester);
            let icon_button = arrow_drop_down_button(tester);
            assert_eq!(tester.get_size(icon_button), Size::new(48.0, 48.0));

            // Custom suffix icon constraints.
            tester.pump_widget(
                BuildTest {
                    decoration_theme: Some(
                        InputDecorationThemeData::new().suffix_icon_constraints(
                            BoxConstraints::new().min_width(66.0).min_height(62.0),
                        ),
                    ),
                    ..BuildTest::default()
                }
                .build(theme, entries),
            );
            pump(tester);
            let icon_button = arrow_drop_down_button(tester);
            assert_eq!(tester.get_size(icon_button), Size::new(66.0, 62.0));
        });
    }

    // 'InputDecorationTheme.isCollapsed reduces height'
    #[test]
    fn a_collapsed_decoration_theme_reduces_the_height() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();

            // Default height.
            tester.pump_widget(BuildTest::default().build(theme.clone(), Rc::clone(&entries)));
            pump(tester);
            assert_eq!(tester.get_size(first::<TextField>(tester)).height(), 56.0);

            // Collapsed height.
            tester.pump_widget(
                BuildTest {
                    decoration_theme: Some(InputDecorationThemeData::new().is_collapsed(true)),
                    ..BuildTest::default()
                }
                .build(theme.clone(), Rc::clone(&entries)),
            );
            pump(tester);
            // IconButton min height.
            assert_eq!(tester.get_size(first::<TextField>(tester)).height(), 48.0);

            // Collapsed height with custom suffix icon constraints.
            tester.pump_widget(
                BuildTest {
                    decoration_theme: Some(
                        InputDecorationThemeData::new()
                            .is_collapsed(true)
                            .suffix_icon_constraints(
                                BoxConstraints::new().max_width(24.0).max_height(24.0),
                            ),
                    ),
                    ..BuildTest::default()
                }
                .build(theme, entries),
            );
            pump(tester);
            assert_eq!(tester.get_size(first::<TextField>(tester)).height(), 24.0);
        });
    }

    // 'Do not crash when resize window during menu opening'
    #[test]
    fn resizing_the_window_while_the_menu_opens_does_not_panic() {
        test_widgets(|tester| {
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new()
                    .body(waft_widgets::StatefulBuilder::new(
                        move |app, context, _set_state| {
                            DropdownMenu::new(menu_children())
                                .width(waft_widgets::MediaQuery::width_of(app, context))
                                .into_widget()
                        },
                    ))
                    .into_widget(),
            );
            tester.pump_widget(root);

            let icon_button = widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_down());
            assert!(!icon_button.is_empty());
            tap(tester, icon_button[0]);
            pump(tester);

            find_menu_item_button(tester, TestMenu::MainMenu0.label());

            // didChangeMetrics.
            tester.view().set_physical_size(Size::new(700.0, 700.0));
            pump(tester);

            // Go without throw.
        });
    }

    /// Not a Dart case: what Dart's collector does with the controllers and focus nodes the
    /// menu's state makes for itself, freed once the state goes.
    #[test]
    fn removing_the_menu_frees_what_its_state_made() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(theme.clone(), DropdownMenu::new(menu_children())));
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);
            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            pump(tester);

            let owned: Vec<waft_foundation::HandleId> = {
                let menu = dropdown_menu::<TestMenu>(tester);
                let app = tester.cell.borrow();
                let state = menu
                    .state_handle::<DropdownMenuState<TestMenu>>(&app)
                    .expect("the menu's state");
                let state = app.get(state);
                [
                    state
                        .local_text_editing_controller
                        .as_ref()
                        .map(|node| node.into()),
                    state.internal_focus_node.as_ref().map(|node| node.into()),
                    state
                        .local_trailing_icon_button_focus_node
                        .as_ref()
                        .map(|node| node.into()),
                    state
                        .internal_menu_controller
                        .as_ref()
                        .map(|node| node.into()),
                ]
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .expect("the state made each of them")
            };

            tester.pump_widget(material_app(theme, SizedBox::shrink().into_widget()));
            pump(tester);

            let app = tester.cell.borrow();
            for handle in owned {
                assert!(!app.contains(handle), "{handle:?} is still in the arena");
            }
        });
    }

    // 'DropdownMenu can customize trailing icon button'
    #[test]
    fn a_custom_trailing_icon_opens_the_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).trailing_icon(Icon::new(Some(Icons::ac_unit()))),
            ));
            pump(tester);

            let icon_button = widgets_with_icon::<IconButton>(tester, &Icons::ac_unit())[0];
            tap(tester, icon_button);
            pump(tester);

            let button = find_menu_item_button(tester, TestMenu::MainMenu0.label());
            assert!(!ancestors_of_type::<Material>(tester, button).is_empty());
        });
    }

    // 'Down key can highlight the menu item while focused'
    #[test]
    fn the_down_key_highlights_the_next_item_while_focused() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .trailing_icon(Icon::new(Some(Icons::ac_unit()))),
            ));

            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, "Item 0"));

            // Press down key one more time, the highlight should move to the next item.
            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, "Menu 1"));

            // The previous item should not be highlighted.
            assert!(!is_item_highlighted(tester, &theme, "Item 0"));
        });
    }

    // 'Up key can highlight the menu item while focused'
    #[test]
    fn the_up_key_highlights_the_previous_item_while_focused() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(menu_children()).request_focus_on_tap(true),
            ));

            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            send_key(tester, LogicalKeyboardKey::ARROW_UP);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, "Item 5"));

            // Press up key one more time, the highlight should move up to the item 4.
            send_key(tester, LogicalKeyboardKey::ARROW_UP);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, "Item 4"));

            // The previous item should not be highlighted.
            assert!(!is_item_highlighted(tester, &theme, "Item 5"));
        });
    }

    fn leading_icon_key(menu_entry: TestMenu) -> KeyRef {
        waft_foundation::ValueKey::new(format!("leading-{}", menu_entry.label())).into()
    }

    fn trailing_icon_key(menu_entry: TestMenu) -> KeyRef {
        waft_foundation::ValueKey::new(format!("trailing-{}", menu_entry.label())).into()
    }

    /// Dart's `menuChildrenWithIcons`.
    fn menu_children_with_icons() -> Rc<[DropdownMenuEntry<TestMenu>]> {
        TestMenu::VALUES
            .iter()
            .map(|value| {
                DropdownMenuEntry::new(*value, value.label())
                    .leading_icon(Icon::new(Some(Icons::alarm())).key(leading_icon_key(*value)))
                    .trailing_icon(Icon::new(Some(Icons::abc())).key(trailing_icon_key(*value)))
            })
            .collect()
    }

    /// Dart's `find.byKey(key)`, offstage included.
    fn find_by_key(tester: &WidgetTester, key: &KeyRef) -> Vec<AnyElement> {
        let found = tester.elements();
        let app = tester.cell.borrow();
        found
            .into_iter()
            .filter(|element| element.widget(&app).key() == Some(key))
            .collect()
    }

    /// `MaterialApp(theme:, home: Scaffold(body: Directionality(textDirection:, child: menu)))`.
    fn directional_menu<T: Clone + PartialEq + 'static>(
        theme: ThemeData,
        text_direction: TextDirection,
        menu: DropdownMenu<T>,
    ) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new()
                .body(Directionality::new(text_direction, menu))
                .into_widget(),
        )
    }

    const LEADING_ICON_TO_INPUT_PADDING: f64 = 4.0;

    /// The width of the last `SizedBox` above the search icon: Dart's
    /// `tester.getSize(find.widgetWithIcon(SizedBox, Icons.search).last).width`.
    fn search_icon_box_width(tester: &WidgetTester) -> f64 {
        let boxes = widgets_with_icon::<SizedBox>(tester, &Icons::search());
        tester.get_size(*boxes.last().expect("a sized box")).width()
    }

    // 'The text in the menu button should be aligned with the text of the text field - LTR'
    #[test]
    fn the_item_text_lines_up_with_the_field_text_in_ltr() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            // Default text field (without leading icon).
            tester.pump_widget(
                BuildTest {
                    label: Some(Text::new("label").into_widget()),
                    ..BuildTest::default()
                }
                .build(theme.clone(), Rc::clone(&entries)),
            );

            let label_top_left = tester.get_rect(find::text("label").first()).top_left();
            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();
            let item_text_top_left = tester.get_rect(find::text("Item 0").last()).top_left();
            assert_eq!(label_top_left.dx(), item_text_top_left.dx());

            // Test when the text field has a leading icon.
            tester.pump_widget(SizedBox::new().into_widget());
            tester.pump_widget(
                BuildTest {
                    leading_icon: Some(Icon::new(Some(Icons::search())).into_widget()),
                    label: Some(Text::new("label").into_widget()),
                    ..BuildTest::default()
                }
                .build(theme.clone(), Rc::clone(&entries)),
            );

            let icon_width = search_icon_box_width(tester);
            let updated_label_top_left = tester.get_rect(find::text("label").first()).top_left();
            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();
            let updated_item_text_top_left =
                tester.get_rect(find::text("Item 0").last()).top_left();
            assert_eq!(updated_label_top_left.dx(), updated_item_text_top_left.dx());
            assert_eq!(
                updated_label_top_left.dx(),
                icon_width + LEADING_ICON_TO_INPUT_PADDING
            );

            // Test when then leading icon is a widget with a bigger size.
            tester.pump_widget(SizedBox::new().into_widget());
            tester.pump_widget(
                BuildTest {
                    leading_icon: Some(
                        SizedBox::new()
                            .width(75.0)
                            .child(Icon::new(Some(Icons::search())))
                            .into_widget(),
                    ),
                    label: Some(Text::new("label").into_widget()),
                    ..BuildTest::default()
                }
                .build(theme, entries),
            );

            let large_icon_width = search_icon_box_width(tester);
            let updated_label_top_left1 = tester.get_rect(find::text("label").first()).top_left();
            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();
            let updated_item_text_top_left1 =
                tester.get_rect(find::text("Item 0").last()).top_left();
            assert_eq!(
                updated_label_top_left1.dx(),
                updated_item_text_top_left1.dx()
            );
            assert_eq!(
                updated_label_top_left1.dx(),
                large_icon_width + LEADING_ICON_TO_INPUT_PADDING
            );
        });
    }

    // 'The text in the menu button should be aligned with the text of the text field - RTL'
    #[test]
    fn the_item_text_lines_up_with_the_field_text_in_rtl() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            // Default text field (without leading icon).
            tester.pump_widget(directional_menu(
                theme.clone(),
                TextDirection::Rtl,
                DropdownMenu::new(Rc::clone(&entries)).label(Text::new("label")),
            ));

            let label_top_right = tester.get_rect(find::text("label").first()).top_right();
            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();
            let item_text_top_right = tester.get_rect(find::text("Item 0").last()).top_right();
            assert_eq!(label_top_right.dx(), item_text_top_right.dx());

            // Test when the text field has a leading icon, then when the leading icon is a
            // widget with a bigger size.
            for leading_icon in [
                Icon::new(Some(Icons::search())).into_widget(),
                SizedBox::new()
                    .width(75.0)
                    .child(Icon::new(Some(Icons::search())))
                    .into_widget(),
            ] {
                tester.pump_widget(SizedBox::new().into_widget());
                tester.pump_widget(directional_menu(
                    theme.clone(),
                    TextDirection::Rtl,
                    DropdownMenu::new(Rc::clone(&entries))
                        .leading_icon(leading_icon)
                        .label(Text::new("label")),
                ));
                pump(tester);

                let icon_width = search_icon_box_width(tester);
                let dropdown_menu_top_right = tester
                    .get_rect(dropdown_menu::<TestMenu>(tester))
                    .top_right();
                let updated_label_top_right =
                    tester.get_rect(find::text("label").first()).top_right();
                tap_dropdown_menu::<TestMenu>(tester);
                tester.pump_and_settle();
                let updated_item_text_top_right =
                    tester.get_rect(find::text("Item 0").last()).top_right();
                assert_eq!(
                    updated_label_top_right.dx(),
                    updated_item_text_top_right.dx()
                );
                assert_eq!(
                    updated_label_top_right.dx(),
                    dropdown_menu_top_right.dx() - icon_width - LEADING_ICON_TO_INPUT_PADDING
                );
            }
        });
    }

    /// The rest of 'The icon in the menu button should be aligned with the icon of the text
    /// field': the rects of the field's search glyph and of the first item's leading icon.
    fn field_and_item_icon_rects(
        tester: &mut WidgetTester,
        text_direction: TextDirection,
    ) -> (Rect, Rect) {
        let theme = m3(&tester.cell.borrow());
        tester.pump_widget(directional_menu(
            theme,
            text_direction,
            DropdownMenu::new(menu_children_with_icons())
                .leading_icon(Icon::new(Some(Icons::search())))
                .label(Text::new("label")),
        ));

        let search_icon = tester.element(find::by_icon(&Icons::search()).first());
        let dropdown_icon = *tester
            .descendants_of_type::<waft_widgets::RichText>(search_icon)
            .last()
            .expect("the glyph of the search icon");
        let dropdown_icon_rect = tester.get_rect(dropdown_icon);

        tap_dropdown_menu::<TestMenu>(tester);
        tester.pump_and_settle();
        let item_leading_icon = *find_by_key(tester, &leading_icon_key(TestMenu::MainMenu0))
            .last()
            .expect("the first item's leading icon");
        (dropdown_icon_rect, tester.get_rect(item_leading_icon))
    }

    // 'The icon in the menu button should be aligned with the icon of the text field - LTR'
    #[test]
    fn the_item_icon_lines_up_with_the_field_icon_in_ltr() {
        test_widgets(|tester| {
            let (field_icon, item_icon) = field_and_item_icon_rects(tester, TextDirection::Ltr);
            assert_eq!(field_icon.left, item_icon.left);
        });
    }

    // 'The icon in the menu button should be aligned with the icon of the text field - RTL'
    #[test]
    fn the_item_icon_lines_up_with_the_field_icon_in_rtl() {
        test_widgets(|tester| {
            let (field_icon, item_icon) = field_and_item_icon_rects(tester, TextDirection::Rtl);
            assert_eq!(field_icon.right, item_icon.right);
        });
    }

    fn controller(tester: &WidgetTester) -> TextEditingControllerRef {
        TextEditingController::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    fn controller_text(tester: &WidgetTester, controller: AnyTextEditingController) -> String {
        controller.text_value(&tester.cell.borrow()).to_owned()
    }

    fn controller_selection(
        tester: &WidgetTester,
        controller: AnyTextEditingController,
    ) -> TextSelection {
        controller.selection(&tester.cell.borrow())
    }

    fn collapsed(offset: i32) -> TextSelection {
        TextSelection::collapsed(offset, TextAffinity::Downstream)
    }

    /// The rest of 'Left and right keys can move text field selection' and its
    /// `expandedInsets` twin, after the menu is pumped.
    fn assert_left_and_right_keys_move_the_caret(
        tester: &mut WidgetTester,
        controller: AnyTextEditingController,
    ) {
        // Open the menu.
        tap_dropdown_menu::<TestMenu>(tester);
        pump(tester);

        tester.enter_text(find::by_type::<TextField>().first(), "example");
        pump(tester);
        assert_eq!(controller_text(tester, controller), "example");
        assert_eq!(controller_selection(tester, controller), collapsed(7));

        // Press left key, the caret should move left.
        send_key(tester, LogicalKeyboardKey::ARROW_LEFT);
        pump(tester);
        assert_eq!(controller_selection(tester, controller), collapsed(6));

        // Press Right key, the caret should move right.
        send_key(tester, LogicalKeyboardKey::ARROW_RIGHT);
        pump(tester);
        assert_eq!(controller_selection(tester, controller), collapsed(7));
    }

    fn case_sensitive_filter(
        _app: &mut App,
        entries: &[DropdownMenuEntry<TestMenu>],
        filter: &str,
    ) -> Vec<DropdownMenuEntry<TestMenu>> {
        entries
            .iter()
            .filter(|element| element.label.contains(filter))
            .cloned()
            .collect()
    }

    // 'Left and right keys can move text field selection'
    #[test]
    fn the_left_and_right_keys_move_the_caret() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .enable_filter(true)
                    .filter_callback(case_sensitive_filter)
                    .controller(controller.clone()),
            ));
            assert_left_and_right_keys_move_the_caret(tester, controller.handle());
        });
    }

    // 'Left and right keys can move text field selection when expandedInsets is set'
    #[test]
    fn the_left_and_right_keys_move_the_caret_with_expanded_insets() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .expanded_insets(EdgeInsetsGeometry::ZERO)
                    .request_focus_on_tap(true)
                    .enable_filter(true)
                    .filter_callback(case_sensitive_filter)
                    .controller(controller.clone()),
            ));
            assert_left_and_right_keys_move_the_caret(tester, controller.handle());
        });
    }

    // 'Up and down keys can highlight the menu item when expandedInsets is set'
    #[test]
    fn the_up_and_down_keys_highlight_items_with_expanded_insets() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(menu_children())
                    .expanded_insets(EdgeInsetsGeometry::ZERO)
                    .request_focus_on_tap(true),
            ));

            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            send_key(tester, LogicalKeyboardKey::ARROW_UP);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, "Item 5"));

            // Press up key one more time, the highlight should move up to the item 4.
            send_key(tester, LogicalKeyboardKey::ARROW_UP);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, "Item 4"));

            // The previous item should not be highlighted.
            assert!(!is_item_highlighted(tester, &theme, "Item 5"));

            // Press down key, the highlight should move back to the item 5.
            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, "Item 5"));
        });
    }

    fn abc_entries() -> Vec<DropdownMenuEntry<i32>> {
        vec![
            DropdownMenuEntry::new(0, "ABC"),
            DropdownMenuEntry::new(1, "AB"),
            DropdownMenuEntry::new(2, "ABCD"),
        ]
    }

    /// The keys and highlights of 'Down key and up key can navigate while focused when a label
    /// text contains another label text' and its custom-search twin.
    fn assert_navigation_over_nested_labels(tester: &mut WidgetTester, theme: &ThemeData) {
        tap_dropdown_menu::<i32>(tester);
        pump(tester);

        // Press down key three times, the highlight should move to the next item each time.
        for label in ["ABC", "AB", "ABCD"] {
            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            pump(tester);
            assert!(is_item_highlighted(tester, theme, label), "{label}");
        }

        // Press up key two times, the highlight should up each time.
        for label in ["AB", "ABC"] {
            send_key(tester, LogicalKeyboardKey::ARROW_UP);
            pump(tester);
            assert!(is_item_highlighted(tester, theme, label), "{label}");
        }
    }

    // 'Down key and up key can navigate while focused when a label text contains another
    // label text'
    #[test]
    fn the_arrow_keys_navigate_labels_that_contain_each_other() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(abc_entries()).request_focus_on_tap(true),
            ));
            assert_navigation_over_nested_labels(tester, &theme);
        });
    }

    // 'Searching for non matching item does not crash'
    #[test]
    fn searching_for_a_non_matching_item_does_not_panic() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .enable_filter(true)
                    .request_focus_on_tap(true),
            ));

            // Open the menu.
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);
            tester.enter_text(find::by_type::<TextField>().first(), "Me");
            pump(tester);
            tester.enter_text(find::by_type::<TextField>().first(), "Meu");
            pump(tester);
        });
    }

    // 'Keyboard navigation does not throw when no entries match the filter'
    #[test]
    fn keyboard_navigation_over_no_matching_entries_does_not_panic() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .enable_filter(true),
            ));
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);
            tester.enter_text(find::by_type::<TextField>().first(), "No match");
            pump(tester);
            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            pump(tester);
            tester.enter_text(find::by_type::<TextField>().first(), "No match 2");
            pump(tester);
        });
    }

    // 'Keyboard navigation only traverses filtered entries'
    #[test]
    fn keyboard_navigation_traverses_only_the_filtered_entries() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![
                    DropdownMenuEntry::new(TestMenu::MainMenu0, "Good Match 1"),
                    DropdownMenuEntry::new(TestMenu::MainMenu1, "Bad Match 1"),
                    DropdownMenuEntry::new(TestMenu::MainMenu2, "Good Match 2"),
                    DropdownMenuEntry::new(TestMenu::MainMenu3, "Bad Match 2"),
                    DropdownMenuEntry::new(TestMenu::MainMenu4, "Good Match 3"),
                    DropdownMenuEntry::new(TestMenu::MainMenu5, "Bad Match 3"),
                ])
                .request_focus_on_tap(true)
                .enable_filter(true)
                .controller(controller.clone()),
            ));

            // Open the menu.
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            // Filter the entries to only show the ones with 'Good Match'.
            tester.enter_text(find::by_type::<TextField>().first(), "Good Match");
            pump(tester);

            // Since the first entry is already highlighted, navigate to the second item, the
            // third, and back to the first.
            for expected in ["Good Match 2", "Good Match 3", "Good Match 1"] {
                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);
                assert_eq!(controller_text(tester, controller.handle()), expected);
            }
        });
    }

    // 'Default search prioritises the current highlight'
    #[test]
    fn the_default_search_keeps_the_current_highlight_when_it_still_matches() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(menu_children()).request_focus_on_tap(true),
            ));

            let item_label = "Item 2";
            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);
            // Highlight the third item by exact search.
            tester.enter_text(find::by_type::<TextField>().first(), item_label);
            pump(tester);
            assert!(is_item_highlighted(tester, &theme, item_label));

            // Search something that matches multiple items.
            tester.enter_text(find::by_type::<TextField>().first(), "Item");
            pump(tester);
            // The third item should still be highlighted.
            assert!(is_item_highlighted(tester, &theme, item_label));
        });
    }

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    const MOBILE_PLATFORMS: [TargetPlatform; 3] = [
        TargetPlatform::Android,
        TargetPlatform::IOS,
        TargetPlatform::Fuchsia,
    ];

    const DESKTOP_PLATFORMS: [TargetPlatform; 3] = [
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    /// Dart's `find.widgetWithText(TextField, text)` finding one widget: the field's text is
    /// `text`.
    fn text_field_shows(tester: &WidgetTester, text: &str) -> bool {
        let fields = find::by_type::<TextField>().evaluate(tester);
        fields.len() == 1
            && editable(tester, |editable| {
                editable.controller.text_value(&tester.cell.borrow()) == text
            })
    }

    /// Whether a hit test at the centre of `element` reaches its render box: Dart's
    /// `hitTestable()` finder.
    fn hit_testable(tester: &WidgetTester, element: AnyElement) -> bool {
        let render = tester.render_box_of(element);
        let center = tester.global_rect(render).center();
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

    fn tap_item(tester: &mut WidgetTester, label: &str) {
        let item = find_menu_item_button(tester, label);
        tap(tester, item);
    }

    // 'Down key and up key can navigate while focused when a label text contains another
    // label text using customized search algorithm'
    #[test]
    fn the_arrow_keys_navigate_labels_that_contain_each_other_with_a_custom_search() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(abc_entries())
                    .request_focus_on_tap(true)
                    .search_callback(|_app, entries, query| {
                        if query.is_empty() {
                            return None;
                        }
                        entries.iter().position(|entry| entry.label.contains(query))
                    }),
            ));
            assert_navigation_over_nested_labels(tester, &theme);
        });
    }

    // 'Searching can highlight entry after keyboard navigation while focused'
    #[test]
    fn searching_highlights_an_entry_after_keyboard_navigation() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries)).request_focus_on_tap(true),
            ));

            // Open the menu and highlight the first item.
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);
            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            pump(tester);
            // Search for the last item.
            let searched_label = entries.last().expect("entries").label.clone();
            tester.enter_text(find::by_type::<TextField>().first(), &searched_label);
            pump(tester);
            // The corresponding menu entry is highlighted.
            assert!(is_item_highlighted(tester, &theme, &searched_label));
        });
    }

    // 'The text input should match the label of the menu item when pressing down key while
    // focused'
    #[test]
    fn the_down_key_puts_the_highlighted_label_in_the_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).request_focus_on_tap(true),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            for label in ["Item 0", "Menu 1", "Item 2"] {
                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);
                assert!(text_field_shows(tester, label), "{label}");
            }
        });
    }

    // 'The text input should match the label of the menu item when pressing up key while
    // focused'
    #[test]
    fn the_up_key_puts_the_highlighted_label_in_the_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).request_focus_on_tap(true),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            for label in ["Item 5", "Item 4", "Item 3"] {
                send_key(tester, LogicalKeyboardKey::ARROW_UP);
                pump(tester);
                assert!(text_field_shows(tester, label), "{label}");
            }
        });
    }

    // 'Disabled button will be skipped while pressing up/down key while focused'
    #[test]
    fn the_arrow_keys_skip_disabled_items() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(vec![
                    DropdownMenuEntry::new(TestMenu::MainMenu0, "Item 0"),
                    DropdownMenuEntry::new(TestMenu::MainMenu1, "Item 1").enabled(false),
                    DropdownMenuEntry::new(TestMenu::MainMenu2, "Item 2").enabled(false),
                    DropdownMenuEntry::new(TestMenu::MainMenu3, "Item 3"),
                    DropdownMenuEntry::new(TestMenu::MainMenu4, "Item 4"),
                    DropdownMenuEntry::new(TestMenu::MainMenu5, "Item 5").enabled(false),
                ])
                .request_focus_on_tap(true),
            ));
            pump(tester);

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            tester.pump_and_settle();

            // First item is highlighted as it's enabled.
            assert!(is_item_highlighted(tester, &theme, "Item 0"));

            // Continue to press down key. Item 3 should be highlighted as Menu 1 and Item 2 are both disabled.
            send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
            tester.pump_and_settle();
            assert!(is_item_highlighted(tester, &theme, "Item 3"));
        });
    }

    // 'Searching is enabled by default if initialSelection is non null'
    #[test]
    fn an_initial_selection_is_highlighted_when_the_menu_opens() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .initial_selection(TestMenu::MainMenu1),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            // Initial selection (Menu 1) button is highlighted.
            assert!(is_item_highlighted(tester, &theme, "Menu 1"));
        });
    }

    // 'Highlight can move up/down starting from the searching result while focused'
    #[test]
    fn the_highlight_moves_from_the_search_result() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(menu_children()).request_focus_on_tap(true),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);
            tester.enter_text(find::by_type::<TextField>().first(), "Menu 1");
            tester.pump_and_settle();
            assert!(is_item_highlighted(tester, &theme, "Menu 1"));

            // Press up to the upper item (Item 0), then move up to the last item (Item 5).
            for label in ["Item 0", "Item 5"] {
                send_key(tester, LogicalKeyboardKey::ARROW_UP);
                tester.pump_and_settle();
                assert!(text_field_shows(tester, label), "{label}");
                assert!(is_item_highlighted(tester, &theme, label), "{label}");
            }
        });
    }

    // 'Filtering is disabled by default'
    #[test]
    fn filtering_is_off_by_default() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).request_focus_on_tap(true),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            tester.enter_text(find::by_type::<TextField>().first(), "Menu 1");
            tester.pump_and_settle();
            for menu in TestMenu::VALUES {
                // One is layout for the _DropdownMenuBody, the other one is the real button item in the menu.
                assert_eq!(
                    widgets_with_text::<MenuItemButton>(tester, menu.label()).len(),
                    2,
                    "{menu:?}"
                );
            }
        });
    }

    // 'Enable filtering'
    #[test]
    fn enabled_filtering_shows_only_the_matching_items() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .enable_filter(true),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            tester.enter_text(find::by_type::<TextField>().first(), "Menu 1");
            tester.pump_and_settle();
            for menu in TestMenu::VALUES {
                // 'Menu 1' should be 2, other items should only find one.
                let expected = if menu == TestMenu::MainMenu1 { 2 } else { 1 };
                assert_eq!(
                    widgets_with_text::<MenuItemButton>(tester, menu.label()).len(),
                    expected,
                    "{menu:?}"
                );
            }
        });
    }

    // 'Enable filtering with custom filter callback that filter text case sensitive'
    #[test]
    fn a_custom_filter_callback_filters_the_items() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .enable_filter(true)
                    .filter_callback(case_sensitive_filter)
                    .controller(controller.clone()),
            ));

            // Open the menu.
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            tester.enter_text(find::by_type::<TextField>().first(), "item");
            assert_eq!(controller_text(tester, controller.handle()), "item");
            tester.pump_and_settle();
            for menu in TestMenu::VALUES {
                let button = find_menu_item_button(tester, menu.label());
                assert!(!hit_testable(tester, button), "{menu:?}");
            }

            tester.enter_text(find::by_type::<TextField>().first(), "Item");
            assert_eq!(controller_text(tester, controller.handle()), "Item");
            tester.pump_and_settle();
            for (label, shown) in [
                ("Item 0", true),
                ("Menu 1", false),
                ("Item 2", true),
                ("Item 3", true),
                ("Item 4", true),
                ("Item 5", true),
            ] {
                let button = find_menu_item_button(tester, label);
                assert_eq!(hit_testable(tester, button), shown, "{label}");
            }
        });
    }

    // 'Throw assertion error when enable filtering with custom filter callback and
    // enableFilter set on False'
    #[test]
    #[should_panic(expected = "self.filter_callback.is_none() || self.enable_filter")]
    fn a_filter_callback_without_enabled_filtering_panics() {
        let menu = DropdownMenu::new(menu_children())
            .request_focus_on_tap(true)
            .filter_callback(case_sensitive_filter);
        let _ = menu.into_widget();
    }

    // 'The controller can access the value in the input field'
    #[test]
    fn the_controller_holds_the_text_of_the_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .enable_filter(true)
                    .controller(controller.clone()),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);
            tap_item(tester, "Item 3");
            tester.pump_and_settle();

            assert_eq!(controller_text(tester, controller.handle()), "Item 3");

            tester.enter_text(find::by_type::<TextField>().first(), "New Item");
            assert_eq!(controller_text(tester, controller.handle()), "New Item");
        });
    }

    fn menu_controller_is_open(tester: &WidgetTester) -> bool {
        let anchor = tester.element(find::by_type::<MenuAnchor>());
        let controller = tester.widget(anchor, |anchor: &MenuAnchor| {
            anchor.controller.clone().expect("a controller")
        });
        controller.is_open(&tester.cell.borrow())
    }

    // 'The menu should be closed after text editing is complete'
    #[test]
    fn completing_the_text_editing_closes_the_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .enable_filter(true)
                    .controller(controller),
            ));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();
            assert!(menu_controller_is_open(tester));

            // Simulate `TextInputAction.done` on textfield
            tester
                .test_text_input()
                .receive_action(TextInputAction::Done);
            tester.pump_and_settle();
            assert!(!menu_controller_is_open(tester));
        });
    }

    // 'The onSelected gets called only when a selection is made' (every platform)
    #[test]
    fn on_selected_is_called_only_when_a_selection_is_made() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = theme(tester);
                let controller = controller(tester);
                let selection_count = Rc::new(std::cell::Cell::new(0));
                let count = Rc::clone(&selection_count);
                tester.pump_widget(the_menu(
                    theme.clone(),
                    DropdownMenu::new(vec![
                        DropdownMenuEntry::new(TestMenu::MainMenu0, "Item 0"),
                        DropdownMenuEntry::new(TestMenu::MainMenu1, "Item 1").enabled(false),
                        DropdownMenuEntry::new(TestMenu::MainMenu2, "Item 2"),
                        DropdownMenuEntry::new(TestMenu::MainMenu3, "Item 3"),
                    ])
                    .controller(controller.clone())
                    .on_selected(move |_app, _value| count.set(count.get() + 1)),
                ));

                // Open the menu
                tap_dropdown_menu::<TestMenu>(tester);
                pump(tester);
                let is_mobile = MOBILE_PLATFORMS.contains(&theme.platform);
                let mut expected_count = 1;

                // Test onSelected on key press
                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                tester.pump_and_settle();

                // On mobile platforms, the TextField cannot gain focus by default; the focus is
                // on a FocusNode specifically used for keyboard navigation. Therefore,
                // LogicalKeyboardKey.enter should be used.
                if is_mobile {
                    send_key(tester, LogicalKeyboardKey::ENTER);
                } else {
                    tester
                        .test_text_input()
                        .receive_action(TextInputAction::Done);
                }
                tester.pump_and_settle();
                assert_eq!(selection_count.get(), expected_count, "{platform:?}");

                // Open the menu
                tap_dropdown_menu::<TestMenu>(tester);
                pump(tester);

                // Disabled item doesn't trigger onSelected callback.
                tap_item(tester, "Item 1");
                tester.pump_and_settle();
                assert_eq!(controller_text(tester, controller.handle()), "Item 0");
                assert_eq!(selection_count.get(), expected_count, "{platform:?}");

                tap_item(tester, "Item 2");
                tester.pump_and_settle();
                assert_eq!(controller_text(tester, controller.handle()), "Item 2");
                expected_count += 1;
                assert_eq!(selection_count.get(), expected_count, "{platform:?}");

                tap_dropdown_menu::<TestMenu>(tester);
                pump(tester);
                tap_item(tester, "Item 3");
                tester.pump_and_settle();
                assert_eq!(controller_text(tester, controller.handle()), "Item 3");
                expected_count += 1;
                assert_eq!(selection_count.get(), expected_count, "{platform:?}");

                // On desktop platforms, when typing something in the text field without selecting any of the options,
                // the onSelected should not be called.
                if !is_mobile {
                    tester.enter_text(find::by_type::<TextField>().first(), "New Item");
                    assert_eq!(controller_text(tester, controller.handle()), "New Item");
                    assert_eq!(selection_count.get(), expected_count, "{platform:?}");
                    assert!(text_field_shows(tester, "New Item"));
                    tester.enter_text(find::by_type::<TextField>().first(), "");
                    assert_eq!(selection_count.get(), expected_count, "{platform:?}");
                    assert!(controller_text(tester, controller.handle()).is_empty());
                }
            });
        }
    }

    // 'The selectedValue gives an initial text and highlights the according item'
    #[test]
    fn the_initial_selection_fills_the_field_and_highlights_its_item() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(menu_children())
                    .initial_selection(TestMenu::MainMenu3)
                    .controller(controller),
            ));

            assert!(text_field_shows(tester, "Item 3"));

            // Open the menu
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            // Validate the item 3 is highlighted.
            assert!(is_item_highlighted(tester, &theme, "Item 3"));
        });
    }

    // 'When the initial selection matches a menu entry, the text field displays the
    // corresponding value'
    #[test]
    fn a_matching_initial_selection_shows_its_label() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .initial_selection(TestMenu::MainMenu3)
                    .controller(controller.clone()),
            ));
            assert_eq!(
                controller_text(tester, controller.handle()),
                TestMenu::MainMenu3.label()
            );
        });
    }

    // 'Text field is empty when the initial selection does not match any menu entries'
    #[test]
    fn an_initial_selection_matching_no_entry_leaves_the_field_empty() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                // Use a menu entries which does not contain TestMenu.mainMenu3.
                DropdownMenu::new(menu_children()[0..1].to_vec())
                    .initial_selection(TestMenu::MainMenu3)
                    .controller(controller.clone()),
            ));
            assert!(controller_text(tester, controller.handle()).is_empty());
        });
    }

    // 'Text field content is not cleared when the initial selection does not match any menu
    // entries'
    #[test]
    fn an_initial_selection_matching_no_entry_keeps_the_controller_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::new(&mut app)
                    .text(&mut app, "Flutter")
                    .into_erased::<waft_widgets::AnyTextEditingController>()
            };
            tester.pump_widget(the_menu(
                theme,
                // Use a menu entries which does not contain TestMenu.mainMenu3.
                DropdownMenu::new(menu_children()[0..1].to_vec())
                    .initial_selection(TestMenu::MainMenu3)
                    .controller(controller.clone()),
            ));
            assert_eq!(controller_text(tester, controller.handle()), "Flutter");
        });
    }

    fn column_menu(theme: ThemeData, menu: DropdownMenu<TestMenu>) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new()
                .body(waft_widgets::Column::new().children(vec![menu.into_widget()]))
                .into_widget(),
        )
    }

    // 'The default text input field should not be focused on mobile platforms when it is
    // tapped' (mobile platforms)
    #[test]
    fn the_text_field_cannot_request_focus_by_default_on_mobile() {
        for platform in MOBILE_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = theme(tester);
                tester.pump_widget(column_menu(theme, DropdownMenu::new(menu_children())));
                pump(tester);
                text_field(tester, |text_field| {
                    assert!(!text_field.can_request_focus, "{platform:?}");
                });
            });
        }
    }

    // 'The text input field should be focused on desktop platforms when it is tapped'
    // (desktop platforms)
    #[test]
    fn the_text_field_can_request_focus_by_default_on_desktop() {
        for platform in DESKTOP_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = theme(tester);
                tester.pump_widget(column_menu(theme, DropdownMenu::new(menu_children())));
                pump(tester);
                text_field(tester, |text_field| {
                    assert!(text_field.can_request_focus, "{platform:?}");
                });
            });
        }
    }

    // 'If requestFocusOnTap is true, the text input field can request focus, otherwise it
    // cannot request focus' (every platform)
    #[test]
    fn request_focus_on_tap_decides_whether_the_field_can_request_focus() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = theme(tester);
                let entries = menu_children();

                // Set requestFocusOnTap to true.
                tester.pump_widget(column_menu(
                    theme.clone(),
                    DropdownMenu::new(Rc::clone(&entries)).request_focus_on_tap(true),
                ));
                pump(tester);
                text_field(tester, |text_field| {
                    assert!(text_field.can_request_focus, "{platform:?}");
                });
                // Open the dropdown menu.
                tap_first::<TextField>(tester);
                pump(tester);
                // Make a selection.
                tap_item(tester, "Item 0");
                pump(tester);
                assert_eq!(
                    widgets_with_text::<MenuItemButton>(tester, "Item 0").len(),
                    1
                );

                // Set requestFocusOnTap to false.
                tester.pump_widget(SizedBox::new().into_widget());
                tester.pump_widget(column_menu(
                    theme,
                    DropdownMenu::new(entries).request_focus_on_tap(false),
                ));
                tester.pump_and_settle();
                text_field(tester, |text_field| {
                    assert!(!text_field.can_request_focus, "{platform:?}");
                });
                // Open the dropdown menu.
                tap_first::<TextField>(tester);
                pump(tester);
                // Make a selection.
                tap_item(tester, "Item 0");
                pump(tester);
                assert!(text_field_shows(tester, "Item 0"), "{platform:?}");
            });
        }
    }

    /// A mouse over the centre of the text field: Dart's `createGesture(kind: mouse,
    /// pointer: 1)` and `moveTo(tester.getCenter(textFieldFinder))`.
    fn mouse_over_the_text_field(tester: &mut WidgetTester) -> TestPointer {
        let center = tester.get_rect(first::<TextField>(tester)).center();
        let mut mouse = TestPointer::mouse(1, 0);
        tester.add_pointer(&mut mouse, center);
        mouse
    }

    fn cursor_is(tester: &WidgetTester, cursor: waft_services::SystemMouseCursor) -> bool {
        let expected: MouseCursorRef = cursor.into();
        *tester.active_cursor(1) == *expected
    }

    // 'If requestFocusOnTap is false, the mouse cursor should be clickable when hovered'
    #[test]
    fn a_field_that_cannot_request_focus_shows_the_click_cursor() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(column_menu(
                theme,
                DropdownMenu::new(menu_children()).request_focus_on_tap(false),
            ));
            tester.pump_and_settle();

            text_field(tester, |text_field| assert!(!text_field.can_request_focus));
            mouse_over_the_text_field(tester);
            assert!(cursor_is(tester, SystemMouseCursors::CLICK));
        });
    }

    // 'If enabled is false, the mouse cursor should be deferred when hovered'
    #[test]
    fn a_disabled_field_defers_the_cursor() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();

            // Check mouse cursor dropdown menu is disabled and requestFocusOnTap is true.
            tester.pump_widget(column_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries))
                    .enabled(false)
                    .request_focus_on_tap(true),
            ));
            tester.pump_and_settle();
            text_field(tester, |text_field| assert!(text_field.can_request_focus));
            let mouse = mouse_over_the_text_field(tester);
            assert!(cursor_is(tester, SystemMouseCursors::BASIC));
            // Remove the pointer.
            tester.send(&mouse, waft_embedder::PointerChange::Remove);

            // Check mouse cursor dropdown menu is disabled and requestFocusOnTap is false.
            tester.pump_widget(column_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries))
                    .enabled(false)
                    .request_focus_on_tap(false),
            ));
            tester.pump_and_settle();
            text_field(tester, |text_field| assert!(!text_field.can_request_focus));
            // Add a new pointer.
            let mouse = mouse_over_the_text_field(tester);
            assert!(cursor_is(tester, SystemMouseCursors::BASIC));
            // Remove the pointer.
            tester.send(&mouse, waft_embedder::PointerChange::Remove);

            // Check enabled dropdown menu updates the mouse cursor when hovered.
            tester.pump_widget(column_menu(
                theme,
                DropdownMenu::new(entries).request_focus_on_tap(true),
            ));
            tester.pump_and_settle();
            text_field(tester, |text_field| assert!(text_field.can_request_focus));
            // Add a new pointer.
            mouse_over_the_text_field(tester);
            assert!(cursor_is(tester, SystemMouseCursors::TEXT));
        });
    }

    fn list_view_menu(menu: DropdownMenu<TestMenu>) -> impl FnOnce(&mut App) -> WidgetRef {
        move |app| {
            material_app(
                m3(app),
                Scaffold::new()
                    .body(waft_widgets::ListView::new().children(vec![menu.into_widget()]))
                    .into_widget(),
            )
        }
    }

    // 'The menu has the same width as the input field in ListView'
    #[test]
    fn in_a_list_view_the_menu_is_as_wide_as_the_field() {
        test_widgets(|tester| {
            let entries = menu_children();
            pump_app(
                tester,
                list_view_menu(DropdownMenu::new(Rc::clone(&entries))),
            );

            let text_input = tester.get_rect(first::<TextField>(tester));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            let menu = tester.get_rect(tester.element(find::by_type::<MenuPanel>()));
            assert_eq!(text_input.width(), menu.width());

            tester.pump_widget(SizedBox::new().into_widget());
            pump_app(
                tester,
                list_view_menu(DropdownMenu::new(entries).width(200.0)),
            );

            let text_input1 = tester.get_rect(first::<TextField>(tester));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            let menu1 = tester.get_rect(tester.element(find::by_type::<MenuPanel>()));
            assert_eq!(text_input1.width(), 200.0);
            assert_eq!(menu1.width(), 200.0);
        });
    }

    fn centered_menu(theme: ThemeData, menu: DropdownMenu<TestMenu>) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new()
                .body(waft_widgets::Center::new().child(menu))
                .into_widget(),
        )
    }

    // 'helperText is not visible when errorText is not null'
    #[test]
    fn an_error_text_hides_the_helper_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            let helper_text = "I am helperText";
            let error_text = "I am errorText";
            let build_frame = |has_error: bool| {
                let mut menu = DropdownMenu::new(Rc::clone(&entries)).helper_text(helper_text);
                if has_error {
                    menu = menu.error_text(error_text);
                }
                centered_menu(theme.clone(), menu)
            };

            tester.pump_widget(build_frame(false));
            expect(tester, &find::text(helper_text), finds_one_widget());
            expect(tester, &find::text(error_text), finds_nothing());

            tester.pump_widget(build_frame(true));
            tester.pump_and_settle();
            expect(tester, &find::text(helper_text), finds_nothing());
            expect(tester, &find::text(error_text), finds_one_widget());
        });
    }

    // 'DropdownMenu can respect helperText when helperText is not null'
    #[test]
    fn the_helper_text_shows_below_the_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let helper_text = "I am helperText";
            tester.pump_widget(centered_menu(
                theme,
                DropdownMenu::new(menu_children()).helper_text(helper_text),
            ));
            expect(tester, &find::text(helper_text), finds_one_widget());
        });
    }

    // 'DropdownMenu can respect errorText when errorText is not null'
    #[test]
    fn the_error_text_shows_below_the_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let error_text = "I am errorText";
            tester.pump_widget(centered_menu(
                theme,
                DropdownMenu::new(menu_children()).error_text(error_text),
            ));
            expect(tester, &find::text(error_text), finds_one_widget());
        });
    }

    /// Dart's `find.text(text).hitTestable()` finding one widget.
    fn text_is_hit_testable(tester: &WidgetTester, text: &str) -> bool {
        find::text(text)
            .evaluate(tester)
            .into_iter()
            .filter(|element| hit_testable(tester, *element))
            .count()
            == 1
    }

    // 'Can scroll to the highlighted item'
    #[test]
    fn the_menu_scrolls_to_the_highlighted_item() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    // Give a small number so the list can only show 2 or 3 items.
                    .menu_height(100.0),
            ));
            tester.pump_and_settle();

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            assert!(!text_is_hit_testable(tester, "Item 5"));
            tester.enter_text(find::by_type::<TextField>().first(), "5");
            tester.pump_and_settle();
            // Item 5 should show up.
            assert!(text_is_hit_testable(tester, "Item 5"));
        });
    }

    // 'Material3 - DropdownMenu uses correct text styles'
    #[test]
    fn material3_the_field_takes_body_large_and_the_items_label_large() {
        test_widgets(|tester| {
            let input_text_theme_style = TextStyle::new()
                .font_size(18.5)
                .font_style(waft_painting::FontStyle::Italic)
                .word_spacing(1.2)
                .decoration(waft_painting::TextDecoration::LINE_THROUGH);
            let menu_item_text_theme_style = TextStyle::new()
                .font_size(20.5)
                .font_style(waft_painting::FontStyle::Italic)
                .word_spacing(2.1)
                .decoration(waft_painting::TextDecoration::UNDERLINE);
            let theme = ThemeData::builder()
                .text_theme(
                    crate::text_theme::TextTheme::new()
                        .body_large(input_text_theme_style.clone())
                        .label_large(menu_item_text_theme_style.clone()),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(BuildTest::default().build(theme, menu_children()));

            // Test input text style uses the TextTheme.bodyLarge.
            editable(tester, |editable| {
                assert_eq!(editable.style.font_size, input_text_theme_style.font_size);
                assert_eq!(editable.style.font_style, input_text_theme_style.font_style);
                assert_eq!(
                    editable.style.word_spacing,
                    input_text_theme_style.word_spacing
                );
                assert_eq!(editable.style.decoration, input_text_theme_style.decoration);
            });

            // Open the menu.
            let icon_button = arrow_drop_down_button(tester);
            tap(tester, icon_button);
            pump(tester);

            // Test menu item text style uses the TextTheme.labelLarge.
            button_material(tester, TestMenu::MainMenu0.label(), |material| {
                let text_style = material.text_style.as_ref().expect("a text style");
                assert_eq!(text_style.font_size, menu_item_text_theme_style.font_size);
                assert_eq!(text_style.font_style, menu_item_text_theme_style.font_style);
                assert_eq!(
                    text_style.word_spacing,
                    menu_item_text_theme_style.word_spacing
                );
                assert_eq!(text_style.decoration, menu_item_text_theme_style.decoration);
            });
        });
    }

    // 'DropdownMenuEntry.leadingIcon default layout'
    #[test]
    fn items_with_and_without_a_leading_icon_line_up_with_the_field_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            // The DropdownMenu should not get extra padding in DropdownMenuEntry items
            // when both text field and DropdownMenuEntry have leading icons.
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![
                    DropdownMenuEntry::new(0, "Item 0")
                        .leading_icon(Icon::new(Some(Icons::alarm()))),
                    DropdownMenuEntry::new(1, "Item 1"),
                ])
                .leading_icon(Icon::new(Some(Icons::search())))
                .hint_text("Hint"),
            ));
            tap_dropdown_menu::<i32>(tester);
            tester.pump_and_settle();

            // Check text location in text field.
            assert_eq!(tester.get_rect(find::text("Hint").first()).left, 52.0);

            // By default, the text of item 0 should be aligned with the text of the text field.
            assert_eq!(tester.get_rect(find::text("Item 0").last()).left, 52.0);

            // By default, the text of item 1 should be aligned with the text of the text field,
            // so there are some extra padding before "Item 1".
            assert_eq!(tester.get_rect(find::text("Item 1").last()).left, 52.0);
        });
    }

    // 'DropdownMenu can have customized search algorithm'
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn a_search_callback_decides_the_highlighted_item() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries: Rc<[DropdownMenuEntry<i32>]> = Rc::from(vec![
                DropdownMenuEntry::new(0, "All"),
                DropdownMenuEntry::new(1, "Unread"),
                DropdownMenuEntry::new(2, "Read"),
            ]);
            let dropdown_menu = |search_callback: Option<SearchCallback<i32>>| {
                let mut menu = DropdownMenu::new(Rc::clone(&entries)).request_focus_on_tap(true);
                menu.search_callback = search_callback;
                the_menu(theme.clone(), menu)
            };
            let check_expected_highlight =
                |tester: &WidgetTester, search_result: Option<&str>, other_items: &[&str]| {
                    if let Some(search_result) = search_result {
                        let color =
                            button_material(tester, search_result, |material| material.color);
                        assert_eq!(
                            color,
                            Some(theme.color_scheme.on_surface.with_opacity(0.12))
                        );
                    }
                    for non_highlight in other_items {
                        let color =
                            button_material(tester, non_highlight, |material| material.color);
                        assert_eq!(color, Some(Colors::TRANSPARENT), "{non_highlight}");
                    }
                };

            // Test default.
            tester.pump_widget(dropdown_menu(None));
            pump(tester);
            tap_dropdown_menu::<i32>(tester);
            tester.pump_and_settle();

            tester.enter_text(find::by_type::<TextField>().first(), "read");
            pump(tester);
            // Because "Unread" contains "read".
            check_expected_highlight(tester, Some("Unread"), &["All", "Read"]);

            // Test custom search algorithm.
            tester.pump_widget(dropdown_menu(Some(Rc::new(|_app, _entries, _query| {
                Some(0)
            }))));
            pump(tester);
            tester.enter_text(find::by_type::<TextField>().first(), "read");
            pump(tester);
            // Because the search result should always be index 0.
            check_expected_highlight(tester, Some("All"), &["Unread", "Read"]);

            // Test custom search algorithm - exact match.
            tester.pump_widget(dropdown_menu(Some(Rc::new(
                |_app, entries: &[DropdownMenuEntry<i32>], query: &str| {
                    if query.is_empty() {
                        return None;
                    }
                    entries.iter().position(|entry| entry.label == query)
                },
            ))));
            pump(tester);

            tester.enter_text(find::by_type::<TextField>().first(), "read");
            pump(tester);
            // Because it's case sensitive.
            check_expected_highlight(tester, None, &["All", "Unread", "Read"]);
            tester.enter_text(find::by_type::<TextField>().first(), "Read");
            pump(tester);
            check_expected_highlight(tester, Some("Read"), &["All", "Unread"]);
        });
    }

    /// The case of 'onSelected gets called when a selection is made in a nested menu' and its
    /// twin: a dropdown menu inside a `MenuAnchor`'s menu, which an `IconButton` opens.
    fn assert_a_nested_selection_calls_on_selected(tester: &mut WidgetTester) {
        let theme = theme(tester);
        let selection_count = Rc::new(std::cell::Cell::new(0));
        let count = Rc::clone(&selection_count);
        let dropdown =
            DropdownMenu::new(vec![DropdownMenuEntry::new(TestMenu::MainMenu0, "Item 0")])
                .on_selected(move |_app, _value| count.set(count.get() + 1));
        tester.pump_widget(material_app(
            theme,
            Scaffold::new()
                .body(MenuAnchor::new(vec![dropdown.into_widget()]).builder(
                    |_app, _context, controller, _child| {
                        IconButton::new(
                            Some(Listener::new(move |app| controller.open(app, None))),
                            Icon::new(Some(Icons::smartphone_rounded())),
                        )
                        .into_widget()
                    },
                ))
                .into_widget(),
        ));

        // Open the first menu
        tap_first::<IconButton>(tester);
        pump(tester);
        // Open the dropdown menu
        tap_dropdown_menu::<TestMenu>(tester);
        pump(tester);

        tap_item(tester, "Item 0");
        tester.pump_and_settle();

        assert_eq!(selection_count.get(), 1);
    }

    // 'onSelected gets called when a selection is made in a nested menu'
    #[test]
    fn a_selection_in_a_nested_menu_calls_on_selected() {
        test_widgets(assert_a_nested_selection_calls_on_selected);
    }

    // 'When onSelected is called and menu is closed, no textEditingController exception is
    // thrown'
    #[test]
    fn a_selection_that_closes_the_nested_menu_does_not_panic() {
        test_widgets(assert_a_nested_selection_calls_on_selected);
    }

    // 'setState is not called through addPostFrameCallback after DropdownMenu is unmounted'
    #[test]
    fn a_menu_scrolled_out_of_a_list_does_not_set_state_after_it_unmounts() {
        test_widgets(|tester| {
            let entries = menu_children();
            pump_app(tester, move |app| {
                material_app(
                    m3(app),
                    Scaffold::new()
                        .body(
                            waft_widgets::ListView::builder(move |_app, _context, index| {
                                Some(if index == 250 {
                                    DropdownMenu::new(Rc::clone(&entries)).into_widget()
                                } else {
                                    SizedBox::new().height(50.0).into_widget()
                                })
                            })
                            .item_count(500),
                        )
                        .into_widget(),
                )
            });

            let list = tester.get_rect(first::<waft_widgets::ListView>(tester));
            tester.fling_from(list.center(), Offset::new(0.0, -20000.0), 200000.0, 1);
            tester.pump_and_settle();
        });
    }

    // 'Menu shows scrollbar when height is limited' (every platform)
    #[test]
    fn a_menu_taller_than_the_screen_shows_a_scrollbar() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = theme(tester);
                tester.pump_widget(the_menu(
                    theme,
                    DropdownMenu::new(vec![
                        DropdownMenuEntry::new(TestMenu::MainMenu0, "Item 0").style(
                            MenuItemButton::style_from()
                                .minimum_size(Size::from_height(1000.0))
                                .build(),
                        ),
                    ]),
                ));

                tap_dropdown_menu::<TestMenu>(tester);
                tester.pump_and_settle();

                expect(
                    tester,
                    &find::by_type::<crate::scrollbar::Scrollbar>(),
                    finds_one_widget(),
                );
            });
        }
    }

    fn primary_focus_next(tester: &mut WidgetTester) {
        let mut app = tester.cell.borrow_mut();
        let focus = waft_widgets::primary_focus(&mut app).expect("a primary focus");
        focus.next_focus(&mut app);
    }

    /// Dart's `paints..rrect(color: color)` on `render`: whether it draws a rounded rect (a
    /// path) in `color`.
    fn paints_rrect_of_color(tester: &WidgetTester, render: AnyRenderBox, color: Color) -> bool {
        use waft_embedder::valo::Op;
        let expected: waft_embedder::valo::Color = color.into();
        let painted: Vec<waft_embedder::valo::Color> = tester
            .painted_ops(render.as_object())
            .into_iter()
            .filter_map(|op| match op {
                Op::DrawPath { paint, .. } => Some(paint.color),
                _ => None,
            })
            .collect();
        let found = painted.contains(&expected);
        if !found {
            eprintln!("paths painted in: {painted:?}");
        }
        found
    }

    // 'DropdownMenu.focusNode can focus text input field'
    #[test]
    fn the_focus_node_can_focus_the_text_input_field() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            let theme = theme(tester);
            let entries: Rc<[DropdownMenuEntry<String>]> = Rc::from(vec![
                DropdownMenuEntry::new("Yolk".to_owned(), "Yolk"),
                DropdownMenuEntry::new("Eggbert".to_owned(), "Eggbert"),
            ]);
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(entries).focus_node(focus_node.erase()),
            ));

            let decorator =
                tester.render_box_of(first::<crate::input_decorator::InputDecorator>(tester));

            // Test input border when not focused.
            assert!(paints_rrect_of_color(
                tester,
                decorator,
                theme.color_scheme.outline()
            ));

            focus_node
                .as_node()
                .request_focus(&mut tester.cell.borrow_mut(), None);
            pump(tester);
            // Advance input decorator animation.
            tester.pump_by(Duration::from_millis(200));

            let decorator =
                tester.render_box_of(first::<crate::input_decorator::InputDecorator>(tester));

            // Test input border when focused.
            assert!(paints_rrect_of_color(
                tester,
                decorator,
                theme.color_scheme.primary
            ));
        });
    }

    // 'Focus traversal ignores non visible entries'
    #[test]
    fn focus_traversal_skips_the_invisible_entries() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let (button_focus_node, text_field_focus_node) = {
                let mut app = tester.cell.borrow_mut();
                (FocusNode::new(&mut app), FocusNode::new(&mut app))
            };
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(waft_widgets::Column::new().children(vec![
                        DropdownMenu::new(menu_children())
                            .focus_node(text_field_focus_node.erase())
                            .into_widget(),
                        crate::elevated_button::ElevatedButton::new(
                            Some(Listener::new(|_app| {})),
                            Some(Text::new("Button")),
                        )
                        .focus_node(button_focus_node.erase())
                        .into_widget(),
                    ]))
                    .into_widget(),
            ));

            // Move the focus to the dropdown trailing icon.
            primary_focus_next(tester);
            pump(tester);
            let icon_button = tester.element(find::by_icon(&Icons::arrow_drop_down()).first());
            {
                let mut app = tester.cell.borrow_mut();
                assert!(Focus::of(&mut app, icon_button, false, true).has_focus(&mut app));
            }

            // Move the focus to the text field.
            primary_focus_next(tester);
            pump(tester);
            assert!(
                text_field_focus_node
                    .as_node()
                    .has_focus(&mut tester.cell.borrow_mut())
            );

            // Move the focus to the elevated button.
            primary_focus_next(tester);
            pump(tester);
            assert!(
                button_focus_node
                    .as_node()
                    .has_focus(&mut tester.cell.borrow_mut())
            );
        });
    }

    // 'DropdownMenu honors inputFormatters'
    #[test]
    fn the_input_formatters_format_the_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let called = Rc::new(std::cell::Cell::new(0));
            let count = Rc::clone(&called);
            let formatter = waft_services::TextInputFormatterRef::with_function(Rc::new(
                move |_old_value: &TextEditingValue, new_value: &TextEditingValue| {
                    count.set(count.get() + 1);
                    new_value.clone()
                },
            ));
            let controller = controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![
                    DropdownMenuEntry::new("Blue".to_owned(), "Blue"),
                    DropdownMenuEntry::new("Green".to_owned(), "Green"),
                ])
                .request_focus_on_tap(true)
                .controller(controller.clone())
                .input_formatters(vec![
                    formatter,
                    waft_services::TextInputFormatterRef(Rc::new(
                        waft_services::FilteringTextInputFormatter::deny(
                            waft_services::FilterPattern::Digits,
                        ),
                    )),
                ]),
            ));

            let state = editable_state(tester);
            for (text, count, expected) in [
                ("Blue", 1, "Blue"),
                ("Green", 2, "Green"),
                ("Green2", 3, "Green"),
            ] {
                TextInputClient::update_editing_value(
                    state,
                    &mut tester.cell.borrow_mut(),
                    TextEditingValue::new().text(text),
                );
                assert_eq!(called.get(), count);
                assert_eq!(controller_text(tester, controller.handle()), expected);
            }
        });
    }

    fn long_label() -> String {
        "This is a long text that is multiplied by 4 so it can overflow. ".repeat(4)
    }

    // 'Long text item does not overflow'
    #[test]
    fn a_long_item_opens_without_panicking() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![DropdownMenuEntry::new(0, long_label())]),
            ));
            pump(tester);
            tap_dropdown_menu::<i32>(tester);
            tester.pump_and_settle();
        });
    }

    // 'Text field does not overflow parent'
    #[test]
    fn the_text_field_keeps_to_its_parent_width() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(SizedBox::new().width(300.0).child(DropdownMenu::new(vec![
                        DropdownMenuEntry::new(0, long_label()),
                    ])))
                    .into_widget(),
            ));
            pump(tester);
            assert_eq!(tester.get_size(first::<TextField>(tester)).width(), 300.0);
        });
    }

    // 'Text field with large helper text can be selected'
    #[test]
    fn a_field_with_a_large_helper_text_opens_on_a_tap() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let label_text = "MenuEntry 1";
            tester.pump_widget(centered_menu_of(
                theme,
                DropdownMenu::new(vec![DropdownMenuEntry::new(0, label_text)])
                    .hint_text("Hint text")
                    .helper_text("Menu Helper text")
                    .input_decoration_theme(
                        InputDecorationThemeData::new()
                            .helper_max_lines(2)
                            .helper_style(TextStyle::new().font_size(30.0)),
                    ),
            ));
            pump(tester);
            let hint = tester.get_rect(find::text("Hint text").first()).center();
            tester.tap_at(hint, 1);
            tester.pump_and_settle();
            // One is layout for the _DropdownMenuBody, the other one is the real button item in the menu.
            assert_eq!(
                widgets_with_text::<MenuItemButton>(tester, label_text).len(),
                2
            );
        });
    }

    fn centered_menu_of<T: Clone + PartialEq + 'static>(
        theme: ThemeData,
        menu: DropdownMenu<T>,
    ) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new()
                .body(waft_widgets::Center::new().child(menu))
                .into_widget(),
        )
    }

    // 'DropdownMenu allows customizing text field text align'
    #[test]
    fn the_text_align_reaches_the_text_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let empty: Vec<DropdownMenuEntry<i32>> = Vec::new();
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(waft_widgets::Column::new().children(vec![
                        DropdownMenu::new(empty.clone()).into_widget(),
                        DropdownMenu::new(empty)
                            .text_align(TextAlign::Center)
                            .into_widget(),
                    ]))
                    .into_widget(),
            ));

            let fields = find::by_type::<TextField>().evaluate(tester);
            tester.widget(fields[0], |field: &TextField| {
                assert_eq!(field.text_align, TextAlign::Start);
            });
            tester.widget(fields[1], |field: &TextField| {
                assert_eq!(field.text_align, TextAlign::Center);
            });
        });
    }

    // 'DropdownMenu correctly sets keyboardType on TextField'
    #[test]
    fn the_keyboard_type_reaches_the_text_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(safe_area_menu(
                theme,
                DropdownMenu::new(menu_children()).keyboard_type(TextInputType::NUMBER),
            ));
            text_field(tester, |text_field| {
                assert_eq!(text_field.effective_keyboard_type(), TextInputType::NUMBER);
            });
        });
    }

    // 'DropdownMenu keyboardType defaults to TextInputType.text'
    #[test]
    fn the_keyboard_type_defaults_to_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(safe_area_menu(theme, DropdownMenu::new(menu_children())));
            text_field(tester, |text_field| {
                assert_eq!(text_field.effective_keyboard_type(), TextInputType::TEXT);
            });
        });
    }

    // 'DropdownMenu passes an alignmentOffset to MenuAnchor'
    #[test]
    fn the_alignment_offset_reaches_the_menu_anchor() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let alignment_offset = Offset::new(0.0, 16.0);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(vec![
                    DropdownMenuEntry::new("1".to_owned(), "One"),
                    DropdownMenuEntry::new("2".to_owned(), "Two"),
                ])
                .alignment_offset(alignment_offset),
            ));

            let anchor = tester.element(find::by_type::<MenuAnchor>());
            tester.widget(anchor, |anchor: &MenuAnchor| {
                assert_eq!(anchor.alignment_offset, Some(alignment_offset));
            });
        });
    }

    fn assert_every_item_is_shown_twice(tester: &WidgetTester) {
        for menu in TestMenu::VALUES {
            assert_eq!(
                widgets_with_text::<MenuItemButton>(tester, menu.label()).len(),
                2,
                "{menu:?}"
            );
        }
    }

    // 'DropdownMenu filter is disabled until text input'
    #[test]
    fn the_filter_applies_only_after_text_input() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .enable_filter(true)
                    .initial_selection(TestMenu::MainMenu0),
            ));

            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();

            // All entries should be available, and two buttons should be found for each entry.
            // One is layout for the _DropdownMenuBody, the other one is the real button item in the menu.
            assert_every_item_is_shown_twice(tester);

            // Text input would enable the filter.
            tester.enter_text(find::by_type::<TextField>().first(), "Menu 1");
            tester.pump_and_settle();
            for menu in TestMenu::VALUES {
                // 'Menu 1' should be 2, other items should only find one.
                let expected = if menu == TestMenu::MainMenu1 { 2 } else { 1 };
                assert_eq!(
                    widgets_with_text::<MenuItemButton>(tester, menu.label()).len(),
                    expected,
                    "{menu:?}"
                );
            }

            // Selecting an item would disable filter again.
            tap_item(tester, "Menu 1");
            tester.pump_and_settle();
            tap_dropdown_menu::<TestMenu>(tester);
            tester.pump_and_settle();
            assert_every_item_is_shown_twice(tester);
        });
    }

    // 'scrollToHighlight does not scroll parent'
    #[test]
    fn scrolling_to_the_highlight_leaves_the_parent_scroll_view_alone() {
        test_widgets(|tester| {
            let controller = waft_widgets::ScrollController::new(
                &mut tester.cell.borrow_mut(),
                0.0,
                true,
                None,
                None,
                None,
            )
            .into_erased::<waft_widgets::AnyScrollController>();
            let entries = menu_children();
            pump_app(tester, {
                let controller = controller.clone();
                move |app| {
                    let last = entries.last().expect("entries").value;
                    material_app(
                        m3(app),
                        Scaffold::new()
                            .body(
                                waft_widgets::ListView::new()
                                    .controller(controller.clone())
                                    .children(vec![
                                        waft_widgets::ListView::new()
                                            .shrink_wrap(true)
                                            .children(vec![
                                                DropdownMenu::new(entries)
                                                    .initial_selection(last)
                                                    .into_widget(),
                                            ])
                                            .into_widget(),
                                        SizedBox::new().height(1000.0).into_widget(),
                                    ]),
                            )
                            .into_widget(),
                    )
                }
            });

            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            assert_eq!(controller.offset(&tester.cell.borrow()), 0.0);
        });
    }

    // 'DropdownMenu with expandedInsets can be aligned'
    #[test]
    fn a_menu_with_expanded_insets_follows_its_alignment() {
        test_widgets(|tester| {
            let entries = menu_children();
            for (alignment, expected) in [
                (waft_painting::Alignment::TOP_CENTER, Offset::new(16.0, 0.0)),
                (waft_painting::Alignment::CENTER, Offset::new(16.0, 272.0)),
                (
                    waft_painting::Alignment::BOTTOM_CENTER,
                    Offset::new(16.0, 544.0),
                ),
            ] {
                let entries = Rc::clone(&entries);
                pump_app(tester, move |app| {
                    material_app(
                        m3(app),
                        Scaffold::new()
                            .body(waft_widgets::Row::new().children(vec![
                                waft_widgets::Expanded::new(
                                    Align::new().alignment(alignment.into()).child(
                                        DropdownMenu::new(entries)
                                            .expanded_insets(EdgeInsetsGeometry::all(16.0)),
                                    ),
                                )
                                .into_widget(),
                            ]))
                            .into_widget(),
                    )
                });
                let text_field_position = tester.get_rect(first::<TextField>(tester)).top_left();
                assert_eq!(text_field_position, expected);
            }
        });
    }

    fn last_menu_anchor_is_open(tester: &WidgetTester) -> bool {
        let anchor = *find::by_type::<MenuAnchor>()
            .evaluate(tester)
            .last()
            .expect("a menu anchor");
        let controller = tester.widget(anchor, |anchor: &MenuAnchor| {
            anchor.controller.clone().expect("a controller")
        });
        controller.is_open(&tester.cell.borrow())
    }

    // 'DropdownMenu.closeBehavior controls menu closing behavior'
    #[test]
    fn the_close_behavior_decides_which_menus_a_selection_closes() {
        test_widgets(|tester| {
            let entries = menu_children();
            let build_dropdown_menu = |close_behavior: DropdownMenuCloseBehavior| {
                let entries = Rc::clone(&entries);
                move |app: &mut App| {
                    material_app(
                        m3(app),
                        Scaffold::new()
                            .body(
                                MenuAnchor::new(vec![
                                    DropdownMenu::new(entries)
                                        .close_behavior(close_behavior)
                                        .into_widget(),
                                ])
                                .child(Text::new("Open Menu"))
                                .builder(
                                    |_app, _context, controller, child| {
                                        crate::elevated_button::ElevatedButton::new(
                                            Some(Listener::new(move |app| {
                                                controller.open(app, None)
                                            })),
                                            child.cloned(),
                                        )
                                        .into_widget()
                                    },
                                ),
                            )
                            .into_widget(),
                    )
                }
            };
            let tap_button = |tester: &mut WidgetTester| {
                tap_first::<crate::elevated_button::ElevatedButton>(tester);
                tester.pump_and_settle();
            };

            // Test closeBehavior set to all.
            pump_app(tester, build_dropdown_menu(DropdownMenuCloseBehavior::All));
            // Tap the button to open the root anchor.
            tap_button(tester);
            // Tap the menu item to open the dropdown menu.
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            expect(
                tester,
                &find::by_type::<DropdownMenu<TestMenu>>(),
                finds_one_widget(),
            );
            assert!(last_menu_anchor_is_open(tester));
            // Tap the dropdown menu item.
            tap_item(tester, TestMenu::MainMenu0.label());
            tester.pump_and_settle();
            // All menus should be closed.
            expect(
                tester,
                &find::by_type::<DropdownMenu<TestMenu>>(),
                finds_nothing(),
            );
            expect(tester, &find::by_type::<MenuAnchor>(), finds_one_widget());

            // Test closeBehavior set to self, then none.
            for (close_behavior, stays_open) in [
                (DropdownMenuCloseBehavior::SelfOnly, false),
                (DropdownMenuCloseBehavior::None, true),
            ] {
                pump_app(tester, build_dropdown_menu(close_behavior));
                // Tap the button to open the root anchor.
                tap_button(tester);
                expect(
                    tester,
                    &find::by_type::<DropdownMenu<TestMenu>>(),
                    finds_one_widget(),
                );
                // Tap the menu item to open the dropdown menu.
                tap_first::<TextField>(tester);
                tester.pump_and_settle();
                let dropdown_menu_anchor = *find::by_type::<MenuAnchor>()
                    .evaluate(tester)
                    .last()
                    .expect("a menu anchor");
                let controller = tester.widget(dropdown_menu_anchor, |anchor: &MenuAnchor| {
                    anchor.controller.clone().expect("a controller")
                });
                assert!(controller.is_open(&tester.cell.borrow()));
                // Tap the dropdown menu item.
                tap_item(tester, TestMenu::MainMenu0.label());
                tester.pump_and_settle();
                assert_eq!(
                    controller.is_open(&tester.cell.borrow()),
                    stays_open,
                    "{close_behavior:?}"
                );
            }
        });
    }

    /// The case of the group 'The menu is attached at the bottom of the TextField':
    /// `TextField`'s visible bottom is 56 pixels.
    fn assert_the_menu_is_attached_at_the_bottom_of_the_text_field(
        tester: &mut WidgetTester,
        tight: bool,
        expanded_insets: bool,
    ) {
        // Define the expected text field bottom instead of querying it using
        // tester.getRect because when tight constraints are applied to the
        // Dropdown the TextField bounds are expanded while the visible size
        // remains 56 pixels.
        let text_field_bottom = 56.0;
        pump_app(tester, move |app| {
            let mut menu = DropdownMenu::new(menu_children_with_icons())
                .initial_selection(TestMenu::MainMenu3);
            if expanded_insets {
                menu = menu.expanded_insets(EdgeInsetsGeometry::ZERO);
            }
            let body = if tight {
                SizedBox::new()
                    .width(200.0)
                    .height(300.0)
                    .child(menu)
                    .into_widget()
            } else {
                menu.into_widget()
            };
            material_app(m3(app), Scaffold::new().body(body).into_widget())
        });

        // Open the menu.
        tap_first::<TextField>(tester);
        pump(tester);

        let panel = tester.element(find::by_type::<MenuPanel>());
        let material = tester.descendants_of_type::<Material>(panel)[0];
        assert_eq!(tester.get_rect(material).top, text_field_bottom);
    }

    // 'The menu is attached at the bottom of the TextField' 'when given loose constraints and
    // expandedInsets is set'
    #[test]
    fn the_menu_is_attached_below_the_field_under_loose_constraints_with_expanded_insets() {
        test_widgets(|tester| {
            assert_the_menu_is_attached_at_the_bottom_of_the_text_field(tester, false, true)
        });
    }

    // '… when given tight constraints and expandedInsets is set'
    #[test]
    fn the_menu_is_attached_below_the_field_under_tight_constraints_with_expanded_insets() {
        test_widgets(|tester| {
            assert_the_menu_is_attached_at_the_bottom_of_the_text_field(tester, true, true)
        });
    }

    // '… when given loose constraints and expandedInsets is not set'
    #[test]
    fn the_menu_is_attached_below_the_field_under_loose_constraints() {
        test_widgets(|tester| {
            assert_the_menu_is_attached_at_the_bottom_of_the_text_field(tester, false, false)
        });
    }

    // '… when given tight constraints and expandedInsets is not set'
    #[test]
    fn the_menu_is_attached_below_the_field_under_tight_constraints() {
        test_widgets(|tester| {
            assert_the_menu_is_attached_at_the_bottom_of_the_text_field(tester, true, false)
        });
    }

    /// The case of 'Using keyboard navigation to select' and its twin without a focus node:
    /// tab to the trailing icon, enter opens the menu, down highlights the first item and enter
    /// selects it.
    fn assert_keyboard_navigation_selects(platform: TargetPlatform, with_focus_node: bool) {
        let mut tester = WidgetTester::on(platform);
        let tester = &mut tester;
        let theme = theme(tester);
        let selected_menu = Rc::new(std::cell::Cell::new(None));
        let selected = Rc::clone(&selected_menu);
        let mut menu =
            DropdownMenu::new(menu_children()).on_selected(move |_app, menu| selected.set(menu));
        if with_focus_node {
            menu = menu.focus_node(FocusNode::new(&mut tester.cell.borrow_mut()).erase());
        }
        tester.pump_widget(material_app(
            theme,
            Material::new()
                .child(waft_widgets::Center::new().child(menu))
                .into_widget(),
        ));

        // Adding FocusNode to IconButton causes the IconButton to receive focus.
        // Thus it does not matter if the TextField has a FocusNode or not.
        send_key(tester, LogicalKeyboardKey::TAB);
        pump(tester);

        // Now the focus is on the icon button.
        let icon_button = tester.element(find::by_icon(&Icons::arrow_drop_down()).first());
        {
            let mut app = tester.cell.borrow_mut();
            assert!(
                Focus::of(&mut app, icon_button, false, true).has_primary_focus(&app),
                "{platform:?}"
            );
        }

        for key in [
            LogicalKeyboardKey::ENTER,
            LogicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ENTER,
        ] {
            send_key(tester, key);
            pump(tester);
        }

        assert_eq!(
            selected_menu.get(),
            Some(TestMenu::MainMenu0),
            "{platform:?}"
        );
    }

    // 'Using keyboard navigation to select' (every platform)
    #[test]
    fn the_keyboard_opens_the_menu_and_selects_an_item() {
        for platform in ALL_PLATFORMS {
            test_widgets(|_| assert_keyboard_navigation_selects(platform, true));
        }
    }

    // 'Using keyboard navigation to select and without setting the FocusNode parameter'
    // (every platform)
    #[test]
    fn the_keyboard_selects_an_item_without_a_focus_node() {
        for platform in ALL_PLATFORMS {
            test_widgets(|_| assert_keyboard_navigation_selects(platform, false));
        }
    }

    fn material_centered_menu(theme: ThemeData, menu: DropdownMenu<TestMenu>) -> WidgetRef {
        material_app(
            theme,
            Material::new()
                .child(waft_widgets::Center::new().child(menu))
                .into_widget(),
        )
    }

    /// The rest of the 'Pressing ESC key closes the menu' cases: a tap opens the menu and
    /// escape closes it.
    fn assert_escape_closes_the_menu(tester: &mut WidgetTester, request_focus_on_tap: bool) {
        let theme = theme(tester);
        tester.pump_widget(material_centered_menu(
            theme,
            DropdownMenu::new(menu_children()).request_focus_on_tap(request_focus_on_tap),
        ));

        // Move focus to the TextField and open the menu.
        tap_first::<TextField>(tester);
        pump(tester);
        expect(tester, &find::by_type::<MenuPanel>(), finds_one_widget());

        // Press ESC to close the menu.
        send_key(tester, LogicalKeyboardKey::ESCAPE);
        pump(tester);
        expect(tester, &find::by_type::<MenuPanel>(), finds_nothing());
    }

    // 'Pressing ESC key closes the menu when requestFocusOnTap is false'
    #[test]
    fn escape_closes_the_menu_of_a_field_that_cannot_request_focus() {
        test_widgets(|tester| assert_escape_closes_the_menu(tester, false));
    }

    // 'Pressing ESC key closes the menu when requestFocusOnTap is true'
    #[test]
    fn escape_closes_the_menu_of_a_focusable_field() {
        test_widgets(|tester| assert_escape_closes_the_menu(tester, true));
    }

    // 'Pressing ESC key after changing the selected item closes the menu' (every platform)
    #[test]
    fn escape_closes_the_menu_after_the_highlight_moves() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = theme(tester);
                let entries = menu_children();
                tester.pump_widget(material_centered_menu(
                    theme.clone(),
                    DropdownMenu::new(Rc::clone(&entries)).initial_selection(entries[2].value),
                ));

                // Move focus to the TextField and open the menu.
                tap_first::<TextField>(tester);
                pump(tester);
                expect(tester, &find::by_type::<MenuPanel>(), finds_one_widget());

                // Move the selection.
                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);
                assert!(
                    is_item_highlighted(tester, &theme, &entries[3].label),
                    "{platform:?}"
                );

                // Press ESC to close the menu.
                send_key(tester, LogicalKeyboardKey::ESCAPE);
                pump(tester);
                assert!(
                    find::by_type::<MenuPanel>().evaluate(tester).is_empty(),
                    "{platform:?}"
                );
            });
        }
    }

    // 'DropdownMenu passes maxLines to TextField'
    #[test]
    fn max_lines_reaches_the_text_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            for (max_lines, expected) in [
                (None, Some(1)),
                (Some(None), None),
                (Some(Some(2)), Some(2)),
            ] {
                let mut menu = DropdownMenu::new(Rc::clone(&entries));
                if let Some(max_lines) = max_lines {
                    menu = menu.max_lines(max_lines);
                }
                tester.pump_widget(the_menu(theme.clone(), menu));
                text_field(tester, |text_field| {
                    assert_eq!(text_field.max_lines, expected)
                });
            }
        });
    }

    // 'DropdownMenu passes textInputAction to TextField'
    #[test]
    fn the_text_input_action_reaches_the_text_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let entries = menu_children();
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries)),
            ));
            // Default behavior.
            text_field(tester, |text_field| {
                assert_eq!(text_field.text_input_action, None)
            });

            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(entries).text_input_action(TextInputAction::Next),
            ));
            text_field(tester, |text_field| {
                assert_eq!(text_field.text_input_action, Some(TextInputAction::Next));
            });
        });
    }

    // 'When requestFocusOnTap is true, the TextField should gain focus after being tapped.'
    #[test]
    fn a_tap_focuses_a_field_that_requests_focus_on_tap() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).request_focus_on_tap(true),
            ));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            let text_field = first::<TextField>(tester);
            let mut app = tester.cell.borrow_mut();
            assert!(Focus::of(&mut app, text_field, false, true).has_focus(&mut app));
        });
    }

    fn maximum_size(width: f64, height: f64) -> MenuStyle {
        MenuStyle::new().maximum_size(Rc::new(WidgetStatePropertyAll::new(Some(Size::new(
            width, height,
        )))))
    }

    // 'items can be constrained to be smaller than the text field with menuStyle'
    #[test]
    fn a_menu_style_maximum_size_narrows_the_items() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let long_label = "This is a long text that it can overflow.";
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(vec![DropdownMenuEntry::new(0, long_label)])
                    .menu_style(maximum_size(150.0, 50.0)),
            ));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            assert_eq!(
                tester
                    .get_size(find_menu_item_button(tester, long_label))
                    .width(),
                150.0
            );

            // The overwrite of menuStyle is different when a width is provided,
            // So it needs to be tested separately.
            let entries = menu_children();
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries))
                    .width(200.0)
                    .menu_style(maximum_size(150.0, 50.0)),
            ));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            let first_label = entries[0].label.clone();
            assert_eq!(
                tester
                    .get_size(find_menu_item_button(tester, &first_label))
                    .width(),
                150.0
            );

            // The overwrite of menuStyle is different when a width is provided but maximumSize is not,
            // So it needs to be tested separately.
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(entries)
                    .width(200.0)
                    .menu_style(MenuStyle::new()),
            ));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            assert_eq!(
                tester
                    .get_size(find_menu_item_button(tester, &first_label))
                    .width(),
                200.0
            );
        });
    }

    // 'ensure items are constrained to intrinsic size of DropdownMenu (width or anchor) when no
    // maximumSize'
    #[test]
    fn without_a_maximum_size_the_items_take_the_width_of_the_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let short_label = "Male";
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(vec![DropdownMenuEntry::new(0, short_label)])
                    .width(200.0)
                    .menu_style(MenuStyle::new()),
            ));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            assert_eq!(
                tester
                    .get_size(find_menu_item_button(tester, short_label))
                    .width(),
                200.0
            );

            // Use expandedInsets to anchor the TextField to the same size as the parent.
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(
                        SizedBox::new().width(f64::INFINITY).child(
                            DropdownMenu::new(vec![DropdownMenuEntry::new(0, short_label)])
                                .expanded_insets(EdgeInsetsGeometry::symmetric(0.0, 20.0))
                                .menu_style(MenuStyle::new()),
                        ),
                    )
                    .into_widget(),
            ));
            tap_first::<TextField>(tester);
            tester.pump_and_settle();
            // Default width is 800, so the expected width is 800 - padding (20 + 20).
            assert_eq!(
                tester
                    .get_size(find_menu_item_button(tester, short_label))
                    .width(),
                760.0
            );
        });
    }

    // 'restorationId is passed to inner TextField'
    #[test]
    fn the_restoration_id_reaches_the_text_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let restoration_id = "dropdown_menu";
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .request_focus_on_tap(true)
                    .restoration_id(restoration_id),
            ));
            expect(tester, &find::by_type::<TextField>(), finds_one_widget());
            text_field(tester, |text_field| {
                assert_eq!(text_field.restoration_id.as_deref(), Some(restoration_id));
            });
        });
    }

    // 'DropdownMenu does not include the default trailing icon when showTrailingIcon is false'
    #[test]
    fn show_trailing_icon_false_hides_the_default_trailing_icon() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).show_trailing_icon(false),
            ));
            pump(tester);
            assert!(widgets_with_icon::<IconButton>(tester, &Icons::arrow_drop_down()).is_empty());
        });
    }

    // 'DropdownMenu does not include the provided trailing icon when showTrailingIcon is false'
    #[test]
    fn show_trailing_icon_false_hides_a_given_trailing_icon() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .trailing_icon(Icon::new(Some(Icons::ac_unit())))
                    .show_trailing_icon(false),
            ));
            pump(tester);
            assert!(widgets_with_icon::<IconButton>(tester, &Icons::ac_unit()).is_empty());
        });
    }

    // 'Explicitly provided controllers should not be disposed when switched out.'
    #[test]
    fn given_controllers_are_not_disposed_when_replaced() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller1 = controller(tester);
            let controller2 = controller(tester);
            let entries = menu_children();
            tester.pump_widget(the_menu(
                theme.clone(),
                DropdownMenu::new(Rc::clone(&entries)).controller(controller1.clone()),
            ));
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(entries).controller(controller2.clone()),
            ));
            let mut app = tester.cell.borrow_mut();
            controller1.dispose(&mut app);
            controller2.dispose(&mut app);
        });
    }

    // 'DropdownMenu disabled state applies proper styling to label and selected value text'
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn a_disabled_menu_shows_its_text_in_the_disabled_color() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let disabled_color = theme.color_scheme.on_surface.with_opacity(0.38);
            let entries: Rc<[DropdownMenuEntry<String>]> =
                Rc::from(vec![DropdownMenuEntry::new("One".to_owned(), "One")]);
            let build_dropdown_menu = |is_enabled: bool| {
                the_menu(
                    theme.clone(),
                    DropdownMenu::new(Rc::clone(&entries))
                        .width(f64::INFINITY)
                        .enabled(is_enabled)
                        .initial_selection("One".to_owned())
                        .label(Text::new("Choose number")),
                )
            };

            tester.pump_widget(build_dropdown_menu(true));
            text_field(tester, |text_field| {
                assert_eq!(text_field.enabled, Some(true))
            });
            editable(tester, |editable| {
                assert_ne!(editable.style.color, Some(disabled_color.into()));
            });

            // Switch to the disabled state by rebuilding the widget.
            tester.pump_widget(build_dropdown_menu(false));
            text_field(tester, |text_field| {
                assert_eq!(text_field.enabled, Some(false))
            });
            // The selected value text has the disabled color.
            editable(tester, |editable| {
                assert_eq!(editable.style.color, Some(disabled_color.into()));
            });
        });
    }

    /// The traversal of 'DropdownMenu trailingIconFocusNode is created when not provided' and
    /// '… is used when provided': the trailing icon, then the field, then the button.
    fn assert_traversal_reaches_the_trailing_icon_first(
        tester: &mut WidgetTester,
        given_trailing_icon_focus_node: bool,
    ) {
        let theme = theme(tester);
        let (text_field_focus_node, trailing_icon_focus_node, button_focus_node) = {
            let mut app = tester.cell.borrow_mut();
            (
                FocusNode::new(&mut app),
                FocusNode::new(&mut app),
                FocusNode::new(&mut app),
            )
        };
        let mut menu = DropdownMenu::new(menu_children()).focus_node(text_field_focus_node.erase());
        if given_trailing_icon_focus_node {
            menu = menu.trailing_icon_focus_node(trailing_icon_focus_node.erase());
        }
        tester.pump_widget(material_app(
            theme,
            Scaffold::new()
                .body(waft_widgets::Column::new().children(vec![
                    menu.into_widget(),
                    crate::elevated_button::ElevatedButton::new(
                        Some(Listener::new(|_app| {})),
                        Some(Text::new("Button")),
                    )
                    .focus_node(button_focus_node.erase())
                    .into_widget(),
                ]))
                .into_widget(),
        ));

        primary_focus_next(tester);
        pump(tester);
        if given_trailing_icon_focus_node {
            // Ensure the trailing icon has focus.
            assert!(
                trailing_icon_focus_node
                    .as_node()
                    .has_focus(&mut tester.cell.borrow_mut())
            );
        } else {
            // Ensure the trailing icon does not have focus.
            // If FocusNode is not created then the TextField will have focus.
            let icon_button = tester.element(find::by_icon(&Icons::arrow_drop_down()).first());
            let mut app = tester.cell.borrow_mut();
            assert!(Focus::of(&mut app, icon_button, false, true).has_focus(&mut app));
        }

        // Ensure the TextField has focus.
        primary_focus_next(tester);
        pump(tester);
        assert!(
            text_field_focus_node
                .as_node()
                .has_focus(&mut tester.cell.borrow_mut())
        );

        // Ensure the button has focus.
        primary_focus_next(tester);
        pump(tester);
        assert!(
            button_focus_node
                .as_node()
                .has_focus(&mut tester.cell.borrow_mut())
        );
    }

    // 'DropdownMenu trailingIconFocusNode is created when not provided'
    #[test]
    fn the_trailing_icon_gets_a_focus_node_of_its_own() {
        test_widgets(|tester| assert_traversal_reaches_the_trailing_icon_first(tester, false));
    }

    // 'DropdownMenu trailingIconFocusNode is used when provided'
    #[test]
    fn the_trailing_icon_uses_a_given_focus_node() {
        test_widgets(|tester| assert_traversal_reaches_the_trailing_icon_first(tester, true));
    }

    // 'Throw assertion error when showTrailingIcon is false and trailingIconFocusNode is
    // provided'
    #[test]
    #[should_panic(expected = "self.trailing_icon_focus_node.is_none() || self.show_trailing_icon")]
    fn a_trailing_icon_focus_node_without_a_trailing_icon_panics() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            let _ = DropdownMenu::new(menu_children())
                .show_trailing_icon(false)
                .trailing_icon_focus_node(focus_node.erase())
                .into_widget();
        });
    }

    // 'DropdownMenu can set cursorHeight'
    #[test]
    fn the_cursor_height_reaches_the_editable_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).cursor_height(4.0),
            ));
            editable(tester, |editable| {
                assert_eq!(editable.cursor_height, Some(4.0))
            });
        });
    }

    // 'DropdownMenu.scrollPadding is passed through to EditableText'
    #[test]
    fn the_scroll_padding_reaches_the_editable_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).scroll_padding(EdgeInsets::all(30.0)),
            ));
            editable(tester, |editable| {
                assert_eq!(editable.scroll_padding, EdgeInsets::all(30.0));
            });
        });
    }

    // 'DropdownMenu.scrollPadding defaults to EdgeInsets.all(20.0)'
    #[test]
    fn the_scroll_padding_defaults_to_twenty() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(theme, DropdownMenu::new(menu_children())));
            editable(tester, |editable| {
                assert_eq!(editable.scroll_padding, EdgeInsets::all(20.0));
            });
        });
    }

    // 'DropdownMenu accepts a MenuController'
    #[test]
    fn a_given_menu_controller_opens_and_closes_the_menu() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let menu_controller = MenuController::new(&mut tester.cell.borrow_mut())
                .into_erased::<waft_widgets::AnyMenuController>();
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).menu_controller(menu_controller.clone()),
            ));
            let item_0_is_hit_testable = |tester: &WidgetTester| {
                let button = find_menu_item_button(tester, "Item 0");
                hit_testable(tester, button)
            };
            assert!(!item_0_is_hit_testable(tester));
            menu_controller.open(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(item_0_is_hit_testable(tester));
            menu_controller.close(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(!item_0_is_hit_testable(tester));
        });
    }

    // ---- the 'DropdownMenu.decorationBuilder' group ----

    const LABEL_TEXT: &str = "labelText";

    fn build_decoration_with_suffix_icon(
        app: &mut App,
        _context: BuildContext,
        controller: AnyMenuController,
    ) -> InputDecoration {
        InputDecoration::new()
            .label_text(LABEL_TEXT)
            .suffix_icon(Icon::new(Some(if controller.is_open(app) {
                Icons::arrow_drop_up()
            } else {
                Icons::arrow_drop_down()
            })))
    }

    fn build_decoration(
        _app: &mut App,
        _context: BuildContext,
        _controller: AnyMenuController,
    ) -> InputDecoration {
        InputDecoration::new().label_text(LABEL_TEXT)
    }

    /// Dart's `find.byIcon(icon)` count.
    fn icon_count(tester: &WidgetTester, icon: &waft_widgets::IconData) -> usize {
        find::by_type::<Icon>()
            .evaluate(tester)
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .count()
    }

    fn new_menu_controller(tester: &WidgetTester) -> MenuControllerRef {
        MenuController::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    // 'DropdownMenu.decorationBuilder' 'Decoration properties set by decorationBuilder are
    // applied'
    #[test]
    fn the_decoration_builder_decorates_the_field() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let menu_controller = new_menu_controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .menu_controller(menu_controller)
                    .decoration_builder(|_app, _context, _controller| {
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .helper_text("helperText")
                            .hint_text("hintText")
                            .filled(true)
                    }),
            ));

            text_field(tester, |text_field| {
                let decoration = text_field.decoration.as_ref().expect("a decoration");
                assert_eq!(decoration.label_text.as_deref(), Some(LABEL_TEXT));
                assert_eq!(decoration.helper_text.as_deref(), Some("helperText"));
                assert_eq!(decoration.hint_text.as_deref(), Some("hintText"));
                assert_eq!(decoration.filled, Some(true));
            });
        });
    }

    // '… Custom decorationBuilder can replace default suffixIcon'
    #[test]
    fn a_decoration_builder_can_replace_the_trailing_icon_button() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let menu_controller = new_menu_controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .menu_controller(menu_controller)
                    .decoration_builder(build_decoration_with_suffix_icon),
            ));

            assert_eq!(icon_count(tester, &Icons::arrow_drop_down()), 2);
            expect(tester, &find::by_type::<IconButton>(), finds_nothing());
        });
    }

    // '… Custom decorationBuilder is called when the menu opens and closes'
    #[test]
    fn the_decoration_builder_runs_when_the_menu_opens() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let menu_controller = new_menu_controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .menu_controller(menu_controller)
                    .decoration_builder(build_decoration_with_suffix_icon),
            ));

            assert_eq!(icon_count(tester, &Icons::arrow_drop_down()), 2);
            assert_eq!(icon_count(tester, &Icons::arrow_drop_up()), 0);

            // Open the menu.
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            // Check that the custom decorationBuilder updated the icon.
            assert_eq!(icon_count(tester, &Icons::arrow_drop_down()), 0);
            assert_eq!(icon_count(tester, &Icons::arrow_drop_up()), 2);
        });
    }

    // '… Default IconButton is used when decorationBuilder does not set
    // InputDecoration.suffixIcon'
    #[test]
    fn a_decoration_without_a_suffix_icon_gets_the_default_icon_button() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let menu_controller = new_menu_controller(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .menu_controller(menu_controller)
                    .decoration_builder(build_decoration),
            ));
            expect(tester, &find::by_type::<IconButton>(), finds_n_widgets(2));
        });
    }

    // '… Passing label and decorationBuilder throws'
    #[test]
    #[should_panic(expected = "self.decoration_builder.is_none()")]
    fn a_label_with_a_decoration_builder_panics() {
        let _ = DropdownMenu::new(menu_children())
            .label(Text::new("Label"))
            .decoration_builder(build_decoration)
            .into_widget();
    }

    // '… Passing hintText and decorationBuilder throws'
    #[test]
    #[should_panic(expected = "self.decoration_builder.is_none()")]
    fn a_hint_text_with_a_decoration_builder_panics() {
        let _ = DropdownMenu::new(menu_children())
            .hint_text("hintText")
            .decoration_builder(build_decoration)
            .into_widget();
    }

    // '… Passing helperText and decorationBuilder throws', which Dart writes with a hint text,
    // as here.
    #[test]
    #[should_panic(expected = "self.decoration_builder.is_none()")]
    fn a_helper_text_with_a_decoration_builder_panics() {
        let _ = DropdownMenu::new(menu_children())
            .hint_text("hintText")
            .decoration_builder(build_decoration)
            .into_widget();
    }

    // '… Passing errorText and decorationBuilder throws'
    #[test]
    #[should_panic(expected = "self.decoration_builder.is_none()")]
    fn an_error_text_with_a_decoration_builder_panics() {
        let _ = DropdownMenu::new(menu_children())
            .error_text("errorText")
            .decoration_builder(build_decoration)
            .into_widget();
    }

    // '… Preferred width takes labelText into account'
    #[test]
    fn the_preferred_width_counts_a_decoration_label_text() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).decoration_builder(
                    |_app, _context, _controller| {
                        InputDecoration::new().label_text("Long label text")
                    },
                ),
            ));
            assert_eq!(tester.get_size(first::<TextField>(tester)).width(), 327.5);
        });
    }

    // '… Preferred width takes label into account'
    #[test]
    fn the_preferred_width_counts_a_decoration_label() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children()).decoration_builder(
                    |_app, _context, _controller| {
                        InputDecoration::new().label(SizedBox::new().width(200.0))
                    },
                ),
            ));
            assert_eq!(tester.get_size(first::<TextField>(tester)).width(), 280.0);
        });
    }

    // ---- the 'DropdownMenu.selectOnly' group ----

    // 'DropdownMenu.selectOnly' 'defaults to false on all platforms'
    #[test]
    fn select_only_defaults_to_false() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = theme(tester);
                tester.pump_widget(the_menu(theme, DropdownMenu::new(menu_children())));
                let menu = dropdown_menu::<TestMenu>(tester);
                tester.widget(menu, |menu: &DropdownMenu<TestMenu>| {
                    assert!(!menu.select_only, "{platform:?}");
                });
            });
        }
    }

    fn select_only_menu(tester: &mut WidgetTester, request_focus_on_tap: bool) {
        let theme = theme(tester);
        tester.pump_widget(the_menu(
            theme,
            DropdownMenu::new(menu_children())
                .select_only(true)
                .request_focus_on_tap(request_focus_on_tap),
        ));
    }

    // '… when true and requestFocusOnTap is false, makes the text field readOnly' and its
    // `requestFocusOnTap: true` twin
    #[test]
    fn select_only_makes_the_field_read_only() {
        for request_focus_on_tap in [false, true] {
            test_widgets(|tester| {
                select_only_menu(tester, request_focus_on_tap);
                text_field(tester, |text_field| assert!(text_field.read_only));
            });
        }
    }

    // '… when true and requestFocusOnTap is false, disables text field interactive selection'
    // and its `requestFocusOnTap: true` twin
    #[test]
    fn select_only_disables_interactive_selection() {
        for request_focus_on_tap in [false, true] {
            test_widgets(|tester| {
                select_only_menu(tester, request_focus_on_tap);
                text_field(tester, |text_field| {
                    assert_eq!(text_field.enable_interactive_selection, Some(false));
                });
            });
        }
    }

    fn editable_has_focus(tester: &WidgetTester) -> bool {
        let focus_node = editable(tester, |editable| editable.focus_node.clone());
        focus_node.has_focus(&mut tester.cell.borrow_mut())
    }

    // '… when true and requestFocusOnTap is false, does not make the text field focusable'
    #[test]
    fn select_only_without_request_focus_on_tap_keeps_the_field_unfocused() {
        test_widgets(|tester| {
            select_only_menu(tester, false);
            assert!(!editable_has_focus(tester));
            // Open the menu.
            tap_first::<TextField>(tester);
            pump(tester);
            assert!(!editable_has_focus(tester));
        });
    }

    // '… when true and requestFocusOnTap is true, makes the text field focusable'
    #[test]
    fn select_only_with_request_focus_on_tap_focuses_the_field() {
        test_widgets(|tester| {
            select_only_menu(tester, true);
            assert!(!editable_has_focus(tester));
            // Open the menu.
            tap_first::<TextField>(tester);
            pump(tester);
            assert!(editable_has_focus(tester));
        });
    }

    // '… when true and the text field is focused, pressing enter opens the menu'
    #[test]
    fn select_only_opens_the_menu_on_enter() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let menu_controller = new_menu_controller(tester);
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            tester.pump_widget(the_menu(
                theme,
                DropdownMenu::new(menu_children())
                    .menu_controller(menu_controller.clone())
                    .focus_node(focus_node.erase())
                    .select_only(true),
            ));

            // Focus the dropdownMenu.
            assert!(!editable_has_focus(tester));
            focus_node
                .as_node()
                .request_focus(&mut tester.cell.borrow_mut(), None);
            pump(tester);
            assert!(editable_has_focus(tester));

            // Pressing enter opens the menu.
            assert!(!menu_controller.is_open(&tester.cell.borrow()));
            send_key(tester, LogicalKeyboardKey::ENTER);
            pump(tester);
            assert!(menu_controller.is_open(&tester.cell.borrow()));
        });
    }

    // '… when true, the mouse cursor should be SystemMouseCursors.click when hovered'
    #[test]
    fn select_only_shows_the_click_cursor() {
        test_widgets(|tester| {
            let theme = theme(tester);
            tester.pump_widget(column_menu(
                theme,
                DropdownMenu::new(menu_children()).select_only(true),
            ));
            tester.pump_and_settle();
            mouse_over_the_text_field(tester);
            assert!(cursor_is(tester, SystemMouseCursors::CLICK));
        });
    }

    // 'DropdownMenu keeps the selected item from filtered list after entries list is updated'
    #[test]
    fn a_selection_from_the_filtered_list_survives_new_entries() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let entries = menu_children();
            pump_app(tester, {
                let controller = controller.clone();
                move |app| {
                    material_app(
                        m3(app),
                        Scaffold::new()
                            .body(waft_widgets::StatefulBuilder::new(
                                move |_app, _context, set_state| {
                                    // A new list on each build, as Dart's `toList()` simulates a
                                    // list update.
                                    let new_entries: Vec<DropdownMenuEntry<TestMenu>> =
                                        entries.to_vec();
                                    DropdownMenu::new(new_entries)
                                        .controller(controller.clone())
                                        .request_focus_on_tap(true)
                                        .enable_filter(true)
                                        .on_selected(move |app, _value| {
                                            set_state(app, &mut |_app| {});
                                        })
                                        .into_widget()
                                },
                            ))
                            .into_widget(),
                    )
                }
            });

            // Open the menu.
            tap_dropdown_menu::<TestMenu>(tester);
            pump(tester);

            // Filter the entries to only show 'Menu 1'.
            tester.enter_text(
                find::by_type::<TextField>().first(),
                TestMenu::MainMenu1.label(),
            );
            pump(tester);

            // Select the 'Menu 1' item.
            tap_item(tester, TestMenu::MainMenu1.label());
            tester.pump_and_settle();

            assert_eq!(
                controller_text(tester, controller.handle()),
                TestMenu::MainMenu1.label()
            );
        });
    }

    // 'DropdownMenu does not crash at zero area'
    #[test]
    fn a_menu_at_zero_area_does_not_panic() {
        test_widgets(|tester| {
            tester.view().set_physical_size(Size::ZERO);
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::new(&mut app)
                    .text(&mut app, "I")
                    .into_erased::<waft_widgets::AnyTextEditingController>()
            };
            let theme = theme(tester);
            tester.pump_widget(centered_menu(
                theme,
                DropdownMenu::new(menu_children()).controller(controller.clone()),
            ));
            assert_eq!(
                tester.get_size(dropdown_menu::<TestMenu>(tester)),
                Size::ZERO
            );
            controller.set_selection(&mut tester.cell.borrow_mut(), collapsed(0));
            pump(tester);
            assert!(
                !find::by_type::<MenuItemButton>()
                    .evaluate(tester)
                    .is_empty()
            );
        });
    }

    // 'Handles focus correctly when TextInputAction is invoked', for each TextInputAction.
    #[test]
    fn each_text_input_action_moves_the_focus_as_the_field_does() {
        // The expectations for each of the types of TextInputAction.
        let action_should_lose_focus = [
            (TextInputAction::None, false),
            (TextInputAction::Unspecified, false),
            (TextInputAction::Done, true),
            (TextInputAction::Go, true),
            (TextInputAction::Search, true),
            (TextInputAction::Send, true),
            (TextInputAction::ContinueAction, false),
            (TextInputAction::Join, false),
            (TextInputAction::Route, false),
            (TextInputAction::EmergencyCall, false),
            (TextInputAction::Newline, true),
            (TextInputAction::Next, true),
            (TextInputAction::Previous, true),
        ];
        for (text_input_action, should_lose_focus) in action_should_lose_focus {
            test_widgets(|tester| {
                let should_focus_next = text_input_action == TextInputAction::Next;
                let should_focus_previous = text_input_action == TextInputAction::Previous;
                let theme = theme(tester);
                let (previous_focus_node, text_field_focus_node, next_focus_node) = {
                    let mut app = tester.cell.borrow_mut();
                    (
                        FocusNode::new(&mut app),
                        FocusNode::new(&mut app),
                        FocusNode::new(&mut app),
                    )
                };
                let text_button = |focus_node: Ref<FocusNode>, label: &str| {
                    crate::text_button::TextButton::new(
                        Some(Listener::new(|_app| {})),
                        Some(Text::new(label)),
                    )
                    .focus_node(focus_node.erase())
                    .into_widget()
                };
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(waft_widgets::Column::new().children(vec![
                            text_button(previous_focus_node.clone(), "Previous"),
                            DropdownMenu::new(menu_children())
                                .focus_node(text_field_focus_node.erase())
                                .text_input_action(text_input_action)
                                .request_focus_on_tap(true)
                                .show_trailing_icon(false)
                                .into_widget(),
                            text_button(next_focus_node.clone(), "Next"),
                        ]))
                        .into_widget(),
                ));

                let has_focus = |tester: &WidgetTester, node: Handle<FocusNode>| {
                    FocusNodeLeaf::as_node(node).has_focus(&mut tester.cell.borrow_mut())
                };
                assert!(!has_focus(tester, text_field_focus_node.handle()));

                // Tap on DropdownMenu to request focus on the TextField.
                tap_dropdown_menu::<TestMenu>(tester);
                tester.pump_and_settle();
                assert!(has_focus(tester, text_field_focus_node.handle()));

                tester.test_text_input().receive_action(text_input_action);
                tester.pump_and_settle();

                assert_eq!(
                    has_focus(tester, previous_focus_node.handle()),
                    should_focus_previous,
                    "{text_input_action:?}"
                );
                assert_eq!(
                    has_focus(tester, text_field_focus_node.handle()),
                    !should_lose_focus,
                    "{text_input_action:?}"
                );
                assert_eq!(
                    has_focus(tester, next_focus_node.handle()),
                    should_focus_next,
                    "{text_input_action:?}"
                );
            });
        }
    }

    // 'Allow null entry to clear selection'
    #[test]
    fn an_entry_with_a_null_value_can_be_selected() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let select_none_label = "Select none";
            let nullable_menu_items: Rc<[DropdownMenuEntry<Option<String>>]> = Rc::from(vec![
                DropdownMenuEntry::new(None, select_none_label),
                DropdownMenuEntry::new(Some("a".to_owned()), "A"),
                DropdownMenuEntry::new(Some("b".to_owned()), "B"),
            ]);
            pump_app(tester, {
                let controller = controller.clone();
                move |app| {
                    material_app(
                        m3(app),
                        Scaffold::new()
                            .body(waft_widgets::StatefulBuilder::new(
                                move |_app, _context, set_state| {
                                    DropdownMenu::new(Rc::clone(&nullable_menu_items))
                                        .controller(controller.clone())
                                        .request_focus_on_tap(true)
                                        .enable_filter(true)
                                        .on_selected(move |app, _value| {
                                            set_state(app, &mut |_app| {});
                                        })
                                        .into_widget()
                                },
                            ))
                            .into_widget(),
                    )
                }
            });

            // Open the menu.
            tap_dropdown_menu::<Option<String>>(tester);
            pump(tester);

            // Select the 'None' item.
            tap_item(tester, select_none_label);
            tester.pump_and_settle();

            assert_eq!(
                controller_text(tester, controller.handle()),
                select_none_label
            );
        });
    }

    /// A column of a dropdown menu of one entry that `menu` configures and an `ElevatedButton`
    /// whose press runs `on_pressed` and rebuilds the column.
    fn menu_with_toggle(
        menu: impl Fn(DropdownMenu<i32>) -> DropdownMenu<i32> + 'static,
        on_pressed: impl Fn(&mut App) + 'static,
    ) -> impl FnOnce(&mut App) -> WidgetRef {
        let menu = Rc::new(menu);
        let on_pressed: Rc<dyn Fn(&mut App)> = Rc::new(on_pressed);
        move |app| {
            material_app(
                m3(app),
                Scaffold::new()
                    .body(waft_widgets::StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            let on_pressed = Rc::clone(&on_pressed);
                            waft_widgets::Column::new()
                                .children(vec![
                                    menu(DropdownMenu::new(vec![DropdownMenuEntry::new(1, "One")]))
                                        .into_widget(),
                                    crate::elevated_button::ElevatedButton::new(
                                        Some(Listener::new(move |app| {
                                            let on_pressed = Rc::clone(&on_pressed);
                                            set_state(app, &mut |app| on_pressed(app));
                                        })),
                                        Some(Text::new("Toggle")),
                                    )
                                    .into_widget(),
                                ])
                                .into_widget()
                        },
                    ))
                    .into_widget(),
            )
        }
    }

    fn press_the_toggle(tester: &mut WidgetTester) {
        tap_first::<crate::elevated_button::ElevatedButton>(tester);
        pump(tester);
    }

    // 'DropdownMenu retains text cursor after disable and enable'
    #[test]
    fn the_field_stays_editable_after_it_is_disabled_and_enabled() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            let is_enabled = Rc::new(std::cell::Cell::new(true));
            let enabled = Rc::clone(&is_enabled);
            let toggled = Rc::clone(&is_enabled);
            pump_app(
                tester,
                menu_with_toggle(
                    move |menu| menu.enabled(enabled.get()).focus_node(focus_node.erase()),
                    move |_app| toggled.set(!toggled.get()),
                ),
            );

            // Initial state: enabled.
            text_field(tester, |text_field| {
                assert_eq!(text_field.enabled, Some(true));
                assert!(!text_field.read_only);
            });

            // Disable.
            press_the_toggle(tester);
            text_field(tester, |text_field| {
                assert_eq!(text_field.enabled, Some(false))
            });

            // Enable again.
            press_the_toggle(tester);
            text_field(tester, |text_field| {
                assert_eq!(text_field.enabled, Some(true));
                assert!(!text_field.read_only);
            });
        });
    }

    // 'DropdownMenu updates internal state when focusNode is replaced'
    #[test]
    fn a_new_focus_node_that_cannot_request_focus_makes_the_field_read_only() {
        test_widgets(|tester| {
            let (focus_node1, focus_node2) = {
                let mut app = tester.cell.borrow_mut();
                let focus_node2 = FocusNode::new(&mut app);
                focus_node2.as_node().set_can_request_focus(&mut app, false);
                (FocusNode::new(&mut app), focus_node2)
            };
            let current_focus_node = Rc::new(std::cell::RefCell::new(
                focus_node1.erase::<waft_widgets::AnyFocusNode>(),
            ));
            let current = Rc::clone(&current_focus_node);
            pump_app(
                tester,
                menu_with_toggle(
                    move |menu| menu.focus_node(current.borrow().clone()),
                    move |_app| *current_focus_node.borrow_mut() = focus_node2.erase(),
                ),
            );

            // Initial focus node is focusable.
            text_field(tester, |text_field| assert!(!text_field.read_only));

            // Swap focus node.
            press_the_toggle(tester);

            // New focus node cannot request focus, so it should be readOnly.
            text_field(tester, |text_field| assert!(text_field.read_only));
        });
    }

    // 'DropdownMenu updates when focusNode.canRequestFocus changes while enabled'
    #[test]
    fn a_focus_node_that_stops_requesting_focus_makes_the_field_read_only() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_app(
                tester,
                menu_with_toggle(
                    {
                        let focus_node = focus_node.clone();
                        move |menu| menu.focus_node(focus_node.erase())
                    },
                    move |app| focus_node.as_node().set_can_request_focus(app, false),
                ),
            );

            // Initially it can request focus.
            text_field(tester, |text_field| assert!(!text_field.read_only));

            // Change canRequestFocus.
            press_the_toggle(tester);

            // Should now be readOnly.
            text_field(tester, |text_field| assert!(text_field.read_only));
        });
    }

    // 'canRequestFocus set on the node survives an internal rebuild'
    #[test]
    fn a_focus_node_can_request_focus_survives_a_rebuild_of_the_menu() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_app(tester, {
                let focus_node = focus_node.clone();
                move |app| {
                    material_app(
                        ThemeData::builder()
                            .platform(TargetPlatform::MacOS)
                            .build(app),
                        Scaffold::new()
                            .body(
                                DropdownMenu::new(vec![DropdownMenuEntry::new(1, "One")])
                                    .focus_node(focus_node.erase()),
                            )
                            .into_widget(),
                    )
                }
            });

            // set canRequestFocus
            focus_node
                .as_node()
                .set_can_request_focus(&mut tester.cell.borrow_mut(), false);
            // Opens the menu.
            tap_first::<IconButton>(tester);
            pump(tester);

            assert!(
                !focus_node
                    .as_node()
                    .can_request_focus(&mut tester.cell.borrow_mut())
            );
            text_field(tester, |text_field| assert!(text_field.read_only));
        });
    }

    const LONG_TEXT: &str = "one two three four five six seven eight nine ten eleven twelve";

    /// Dart's `find.descendant(of: findMenuItemButton('$label $longText'), matching:
    /// find.byType(Text)).last`.
    fn menu_item_text(tester: &WidgetTester, label: &str) -> AnyElement {
        let button = find_menu_item_button(tester, &format!("{label} {LONG_TEXT}"));
        *tester
            .descendants_of_type::<Text>(button)
            .last()
            .expect("the item's text")
    }

    fn long_entries(max_lines: i32) -> Vec<DropdownMenuEntry<TestMenu>> {
        TestMenu::VALUES
            .iter()
            .map(|item| {
                let label = format!("{} {LONG_TEXT}", item.label());
                DropdownMenuEntry::new(*item, label.clone())
                    .label_widget(Text::new(label).max_lines(max_lines))
            })
            .collect()
    }

    // 'DropdownMenuEntry.labelWidget is Text that specifies maxLines 1 or 2'
    #[test]
    fn a_label_widget_with_max_lines_limits_the_item_height() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let controller = controller(tester);
            for (max_lines, expected_height) in [(1, 20.0), (2, 40.0)] {
                tester.pump_widget(the_menu(
                    theme.clone(),
                    DropdownMenu::new(long_entries(max_lines))
                        .key(waft_foundation::ValueKey::new(max_lines).into())
                        .controller(controller.clone())
                        .width(100.0),
                ));
                tap_dropdown_menu::<TestMenu>(tester);

                for label in ["Item 0", "Menu 1", "Item 2", "Item 3"] {
                    let height = tester.get_size(menu_item_text(tester, label)).height();
                    assert!(
                        (height - expected_height).abs() <= 0.05,
                        "{max_lines} {label}: {height}"
                    );
                }

                // Close the menu
                tap_first::<TextField>(tester);
                tester.pump_and_settle();
                assert_eq!(controller_text(tester, controller.handle()), ""); // nothing selected
            }
        });
    }

    // `_TestDryLayout`: a proxy that asks its child for a dry layout, then lays it out, to test
    // that the child's `computeDryLayout` is valid.
    #[derive(Debug)]
    struct TestDryLayout {
        child: WidgetRef,
    }

    impl RenderObjectWidget for TestDryLayout {
        type RenderObject = RenderTestDryLayout;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderTestDryLayout::new(app).as_object()
        }

        fn update_render_object(
            &self,
            _app: &mut App,
            _context: BuildContext,
            _render_object: RenderHandle<RenderTestDryLayout>,
        ) {
        }
    }

    impl SingleChildRenderObjectWidget for TestDryLayout {
        fn child(&self) -> Option<&WidgetRef> {
            Some(&self.child)
        }
    }

    struct RenderTestDryLayout {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        child: RenderObjectWithChildData<AnyRenderBox>,
    }

    impl RenderTestDryLayout {
        fn new(app: &mut App) -> RenderHandle<RenderTestDryLayout> {
            RenderHandle::new_box(
                app,
                RenderTestDryLayout {
                    render_object: RenderObjectData::new(),
                    render_box: RenderBoxData::new(),
                    child: RenderObjectWithChildData::new(),
                },
            )
        }
    }

    impl RenderObjectWithChildMixin for RenderTestDryLayout {
        type ChildType = AnyRenderBox;

        fn child_data(
            self: RenderHandle<Self>,
            app: &App,
        ) -> &RenderObjectWithChildData<AnyRenderBox> {
            &self.get(app).child
        }

        fn child_data_mut(
            self: RenderHandle<Self>,
            app: &mut App,
        ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
            &mut self.get_mut(app).child
        }
    }

    impl RenderProxyBoxMixin for RenderTestDryLayout {}

    impl RenderObject for RenderTestDryLayout {
        waft_rendering::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let constraints = self.constraints(app);
            let Some(child) = self.child(app) else {
                self.set_size(app, constraints.smallest());
                return;
            };
            child.get_dry_layout(app, constraints);
            child.layout(app, constraints, true);
            let size = child.size(app);
            self.set_size(app, size);
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
        ) {
            RenderProxyBoxMixin::paint(self, app, context, offset);
        }

        fn visit_children(
            self: RenderHandle<Self>,
            app: &App,
            visitor: &mut dyn FnMut(AnyRenderObject),
        ) {
            if let Some(child) = self.child(app) {
                visitor(child.as_object());
            }
        }
    }

    impl RenderBox for RenderTestDryLayout {
        waft_rendering::render_box_accessors!();

        fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
            RenderProxyBoxMixin::setup_parent_data(self, app, child);
        }

        fn apply_paint_transform(
            self: RenderHandle<Self>,
            app: &App,
            child: AnyRenderObject,
            transform: &mut Matrix4,
        ) {
            RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
        }

        fn hit_test_children(
            self: RenderHandle<Self>,
            app: &mut App,
            result: &mut BoxHitTestResult<'_>,
            position: Offset,
        ) -> bool {
            RenderProxyBoxMixin::hit_test_children(self, app, result, position)
        }

        fn compute_min_intrinsic_width(
            self: RenderHandle<Self>,
            app: &mut App,
            height: f64,
        ) -> f64 {
            RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
        }

        fn compute_max_intrinsic_width(
            self: RenderHandle<Self>,
            app: &mut App,
            height: f64,
        ) -> f64 {
            RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
        }

        fn compute_min_intrinsic_height(
            self: RenderHandle<Self>,
            app: &mut App,
            width: f64,
        ) -> f64 {
            RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
        }

        fn compute_max_intrinsic_height(
            self: RenderHandle<Self>,
            app: &mut App,
            width: f64,
        ) -> f64 {
            RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
        }

        fn compute_distance_to_actual_baseline(
            self: RenderHandle<Self>,
            app: &mut App,
            baseline: TextBaseline,
        ) -> Option<f64> {
            RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
        }

        fn compute_dry_layout(
            self: RenderHandle<Self>,
            app: &mut App,
            constraints: BoxConstraints,
        ) -> Size {
            RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
        }

        fn compute_dry_baseline(
            self: RenderHandle<Self>,
            app: &mut App,
            constraints: BoxConstraints,
            baseline: TextBaseline,
        ) -> Option<f64> {
            RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
        }
    }

    // '_RenderDropdownMenuBody.computeDryLayout does not access this.constraints'. Dart ends on
    // `takeException()` being null; here an exception during layout panics the test.
    #[test]
    fn the_dropdown_menu_body_dry_layout_does_not_access_its_constraints() {
        test_widgets(|tester| {
            let theme = theme(tester);
            let menu = DropdownMenu::new(vec![
                DropdownMenuEntry::new(1, "One"),
                DropdownMenuEntry::new(2, "Two"),
            ]);
            let body = waft_widgets::Center::new().child(TestDryLayout {
                child: menu.into_widget(),
            });
            tester.pump_widget(material_app(
                theme,
                Scaffold::new().body(body).into_widget(),
            ));

            expect(
                tester,
                &find::by_type::<DropdownMenu<i32>>(),
                finds_one_widget(),
            );
        });
    }
}
