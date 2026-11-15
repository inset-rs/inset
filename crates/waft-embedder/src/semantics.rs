//! Flutter counterpart: `dart:ui`'s `semantics.dart`: the value types the semantics widgets
//! name (`SemanticsAction`, `SemanticsRole`, `SemanticsInputType`, `SemanticsHitTestBehavior`,
//! `SemanticsValidationResult`, and the `StringAttribute`s).
//!
//! `SemanticsFlag`, `SemanticsFlags`, `CheckedState`, `Tristate`, `SemanticsUpdateBuilder` and
//! `SemanticsUpdate` wait with the semantics tree, which is what reads them.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;

use crate::{Locale, TextRange};

/// The possible actions that can be conveyed from the operating system
/// accessibility APIs to a semantics node.
///
/// Dart's class with one constant per action: `index` is the action's bit.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticsAction {
    /// The numerical value for this action.
    ///
    /// Each action has one bit set in this bit field.
    pub index: i32,

    /// A human-readable name for this action, used for debugging purposes.
    pub name: &'static str,
}

impl SemanticsAction {
    const fn new(index: i32, name: &'static str) -> SemanticsAction {
        SemanticsAction { index, name }
    }

    const K_TAP_INDEX: i32 = 1 << 0;
    const K_LONG_PRESS_INDEX: i32 = 1 << 1;
    const K_SCROLL_LEFT_INDEX: i32 = 1 << 2;
    const K_SCROLL_RIGHT_INDEX: i32 = 1 << 3;
    const K_SCROLL_UP_INDEX: i32 = 1 << 4;
    const K_SCROLL_DOWN_INDEX: i32 = 1 << 5;
    const K_INCREASE_INDEX: i32 = 1 << 6;
    const K_DECREASE_INDEX: i32 = 1 << 7;
    const K_SHOW_ON_SCREEN_INDEX: i32 = 1 << 8;
    const K_MOVE_CURSOR_FORWARD_BY_CHARACTER_INDEX: i32 = 1 << 9;
    const K_MOVE_CURSOR_BACKWARD_BY_CHARACTER_INDEX: i32 = 1 << 10;
    const K_SET_SELECTION_INDEX: i32 = 1 << 11;
    const K_COPY_INDEX: i32 = 1 << 12;
    const K_CUT_INDEX: i32 = 1 << 13;
    const K_PASTE_INDEX: i32 = 1 << 14;
    const K_DID_GAIN_ACCESSIBILITY_FOCUS_INDEX: i32 = 1 << 15;
    const K_DID_LOSE_ACCESSIBILITY_FOCUS_INDEX: i32 = 1 << 16;
    const K_CUSTOM_ACTION_INDEX: i32 = 1 << 17;
    const K_DISMISS_INDEX: i32 = 1 << 18;
    const K_MOVE_CURSOR_FORWARD_BY_WORD_INDEX: i32 = 1 << 19;
    const K_MOVE_CURSOR_BACKWARD_BY_WORD_INDEX: i32 = 1 << 20;
    const K_SET_TEXT_INDEX: i32 = 1 << 21;
    const K_FOCUS_INDEX: i32 = 1 << 22;
    const K_SCROLL_TO_OFFSET_INDEX: i32 = 1 << 23;
    const K_EXPAND_INDEX: i32 = 1 << 24;
    const K_COLLAPSE_INDEX: i32 = 1 << 25;

    /// The equivalent of a user briefly tapping the screen with the finger
    /// without moving it.
    pub const TAP: SemanticsAction = SemanticsAction::new(Self::K_TAP_INDEX, "tap");

    /// The equivalent of a user pressing and holding the screen with the finger
    /// for a few seconds without moving it.
    pub const LONG_PRESS: SemanticsAction =
        SemanticsAction::new(Self::K_LONG_PRESS_INDEX, "longPress");

    /// The equivalent of a user moving their finger across the screen from right
    /// to left.
    pub const SCROLL_LEFT: SemanticsAction =
        SemanticsAction::new(Self::K_SCROLL_LEFT_INDEX, "scrollLeft");

    /// The equivalent of a user moving their finger across the screen from left
    /// to right.
    pub const SCROLL_RIGHT: SemanticsAction =
        SemanticsAction::new(Self::K_SCROLL_RIGHT_INDEX, "scrollRight");

    /// The equivalent of a user moving their finger across the screen from
    /// bottom to top.
    pub const SCROLL_UP: SemanticsAction =
        SemanticsAction::new(Self::K_SCROLL_UP_INDEX, "scrollUp");

    /// The equivalent of a user moving their finger across the screen from top
    /// to bottom.
    pub const SCROLL_DOWN: SemanticsAction =
        SemanticsAction::new(Self::K_SCROLL_DOWN_INDEX, "scrollDown");

    /// A request to scroll the scrollable container to a given scroll offset.
    pub const SCROLL_TO_OFFSET: SemanticsAction =
        SemanticsAction::new(Self::K_SCROLL_TO_OFFSET_INDEX, "scrollToOffset");

    /// A request to increase the value represented by the semantics node.
    pub const INCREASE: SemanticsAction = SemanticsAction::new(Self::K_INCREASE_INDEX, "increase");

    /// A request to decrease the value represented by the semantics node.
    pub const DECREASE: SemanticsAction = SemanticsAction::new(Self::K_DECREASE_INDEX, "decrease");

    /// A request to fully show the semantics node on screen.
    pub const SHOW_ON_SCREEN: SemanticsAction =
        SemanticsAction::new(Self::K_SHOW_ON_SCREEN_INDEX, "showOnScreen");

    /// Move the cursor forward by one character.
    pub const MOVE_CURSOR_FORWARD_BY_CHARACTER: SemanticsAction = SemanticsAction::new(
        Self::K_MOVE_CURSOR_FORWARD_BY_CHARACTER_INDEX,
        "moveCursorForwardByCharacter",
    );

    /// Move the cursor backward by one character.
    pub const MOVE_CURSOR_BACKWARD_BY_CHARACTER: SemanticsAction = SemanticsAction::new(
        Self::K_MOVE_CURSOR_BACKWARD_BY_CHARACTER_INDEX,
        "moveCursorBackwardByCharacter",
    );

    /// Replaces the current text in the text field.
    pub const SET_TEXT: SemanticsAction = SemanticsAction::new(Self::K_SET_TEXT_INDEX, "setText");

    /// Set the text selection to the given range.
    pub const SET_SELECTION: SemanticsAction =
        SemanticsAction::new(Self::K_SET_SELECTION_INDEX, "setSelection");

    /// Copy the current selection to the clipboard.
    pub const COPY: SemanticsAction = SemanticsAction::new(Self::K_COPY_INDEX, "copy");

    /// Cut the current selection and place it in the clipboard.
    pub const CUT: SemanticsAction = SemanticsAction::new(Self::K_CUT_INDEX, "cut");

    /// Paste the current content of the clipboard.
    pub const PASTE: SemanticsAction = SemanticsAction::new(Self::K_PASTE_INDEX, "paste");

    /// Indicates that the node has gained accessibility focus.
    pub const DID_GAIN_ACCESSIBILITY_FOCUS: SemanticsAction = SemanticsAction::new(
        Self::K_DID_GAIN_ACCESSIBILITY_FOCUS_INDEX,
        "didGainAccessibilityFocus",
    );

    /// Indicates that the node has lost accessibility focus.
    pub const DID_LOSE_ACCESSIBILITY_FOCUS: SemanticsAction = SemanticsAction::new(
        Self::K_DID_LOSE_ACCESSIBILITY_FOCUS_INDEX,
        "didLoseAccessibilityFocus",
    );

    /// Indicates that the user has invoked a custom accessibility action.
    pub const CUSTOM_ACTION: SemanticsAction =
        SemanticsAction::new(Self::K_CUSTOM_ACTION_INDEX, "customAction");

    /// A request that the node should be dismissed.
    pub const DISMISS: SemanticsAction = SemanticsAction::new(Self::K_DISMISS_INDEX, "dismiss");

    /// Move the cursor forward by one word.
    pub const MOVE_CURSOR_FORWARD_BY_WORD: SemanticsAction = SemanticsAction::new(
        Self::K_MOVE_CURSOR_FORWARD_BY_WORD_INDEX,
        "moveCursorForwardByWord",
    );

    /// Move the cursor backward by one word.
    pub const MOVE_CURSOR_BACKWARD_BY_WORD: SemanticsAction = SemanticsAction::new(
        Self::K_MOVE_CURSOR_BACKWARD_BY_WORD_INDEX,
        "moveCursorBackwardByWord",
    );

    /// Move the input focus to the respective widget.
    pub const FOCUS: SemanticsAction = SemanticsAction::new(Self::K_FOCUS_INDEX, "focus");

    /// A request that the node should be expanded.
    pub const EXPAND: SemanticsAction = SemanticsAction::new(Self::K_EXPAND_INDEX, "expand");

    /// A request that the node should be collapsed.
    pub const COLLAPSE: SemanticsAction = SemanticsAction::new(Self::K_COLLAPSE_INDEX, "collapse");

    /// Dart's `_kActionById`, in its order.
    const K_ACTION_BY_ID: [SemanticsAction; 26] = [
        Self::TAP,
        Self::LONG_PRESS,
        Self::SCROLL_LEFT,
        Self::SCROLL_RIGHT,
        Self::SCROLL_UP,
        Self::SCROLL_DOWN,
        Self::SCROLL_TO_OFFSET,
        Self::INCREASE,
        Self::DECREASE,
        Self::SHOW_ON_SCREEN,
        Self::MOVE_CURSOR_FORWARD_BY_CHARACTER,
        Self::MOVE_CURSOR_BACKWARD_BY_CHARACTER,
        Self::SET_SELECTION,
        Self::COPY,
        Self::CUT,
        Self::PASTE,
        Self::DID_GAIN_ACCESSIBILITY_FOCUS,
        Self::DID_LOSE_ACCESSIBILITY_FOCUS,
        Self::CUSTOM_ACTION,
        Self::DISMISS,
        Self::MOVE_CURSOR_FORWARD_BY_WORD,
        Self::MOVE_CURSOR_BACKWARD_BY_WORD,
        Self::SET_TEXT,
        Self::FOCUS,
        Self::EXPAND,
        Self::COLLAPSE,
    ];

    /// Every action, in Dart's `_kActionById` order.
    pub fn values() -> Vec<SemanticsAction> {
        Self::K_ACTION_BY_ID.to_vec()
    }

    /// The action whose bit is `index`, if there is one.
    pub fn from_index(index: i32) -> Option<SemanticsAction> {
        Self::K_ACTION_BY_ID
            .into_iter()
            .find(|action| action.index == index)
    }
}

impl Debug for SemanticsAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SemanticsAction.{}", self.name)
    }
}

/// An enum to describe the role for a semantics node.
///
/// The roles are translated into native accessibility roles in each platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticsRole {
    /// Does not represent any role.
    None,
    /// A tab button.
    Tab,
    /// Contains tab buttons.
    TabBar,
    /// The main display for a tab.
    TabPanel,
    /// A pop up dialog.
    Dialog,
    /// An alert dialog.
    AlertDialog,
    /// A table structure containing data arranged in rows and columns.
    Table,
    /// A cell in a [`Table`](Self::Table) that does not contain column or row header
    /// information.
    Cell,
    /// A row of [`Cell`](Self::Cell)s or [`ColumnHeader`](Self::ColumnHeader)s in a
    /// [`Table`](Self::Table).
    Row,
    /// A cell in a [`Table`](Self::Table) contains header information for a column.
    ColumnHeader,
    /// A control used for dragging across content.
    DragHandle,
    /// A control to cycle through content on tap.
    SpinButton,
    /// A input field with a dropdown list box attached.
    ComboBox,
    /// A presentation of [`Menu`](Self::Menu) that usually remains visible and is usually
    /// presented horizontally.
    MenuBar,
    /// A permanently visible list of controls or a widget that can be made to open and close.
    Menu,
    /// An item in a dropdown created by [`Menu`](Self::Menu) or [`MenuBar`](Self::MenuBar).
    MenuItem,
    /// An item with a checkbox in a dropdown created by [`Menu`](Self::Menu) or
    /// [`MenuBar`](Self::MenuBar).
    MenuItemCheckbox,
    /// An item with a radio button in a dropdown created by [`Menu`](Self::Menu) or
    /// [`MenuBar`](Self::MenuBar).
    MenuItemRadio,
    /// A container to display multiple [`ListItem`](Self::ListItem)s in vertical or horizontal
    /// layout.
    List,
    /// An item in a [`List`](Self::List).
    ListItem,
    /// An area that represents a form.
    Form,
    /// A pop up displayed when hovering over a component to provide contextual explanation.
    Tooltip,
    /// A graphic object that spins to indicate the application is busy.
    LoadingSpinner,
    /// A graphic object that shows progress with a numeric number.
    ProgressBar,
    /// A keyboard shortcut field that allows the user to enter a combination or sequence of
    /// keystrokes.
    HotKey,
    /// A group of radio buttons.
    RadioGroup,
    /// A component to provide advisory information that is not important to justify an
    /// [`Alert`](Self::Alert).
    Status,
    /// A component to provide important and usually time-sensitive information.
    Alert,
    /// A supporting section that relates to the main content.
    Complementary,
    /// A section for a footer, containing identifying information such as copyright
    /// information, navigation links and privacy statements.
    ContentInfo,
    /// The primary content of a document.
    Main,
    /// A region of a web page that contains navigation links.
    Navigation,
    /// A section of content sufficiently important but cannot be described by one of the other
    /// landmark roles, such as main, contentinfo, complementary, or navigation.
    Region,
}

/// Describe the type of data for an input field.
///
/// This is typically used to complement text fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticsInputType {
    /// The default for non text field.
    None,
    /// Describes a generic text field.
    Text,
    /// Describes a url text field.
    Url,
    /// Describes a text field for phone input.
    Phone,
    /// Describes a text field that act as a search box.
    Search,
    /// Describes a text field for email input.
    Email,
}

/// How a semantics node handles pointer events for the platform's accessibility layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticsHitTestBehavior {
    /// Defer to the platform's default hit test behavior inference.
    ///
    /// When set to defer, the platform will infer the appropriate behavior
    /// based on the semantic node's properties such as interactive behaviors,
    /// route scoping, etc.
    ///
    /// This is the default value and provides backward compatibility.
    Defer,

    /// The semantic element is opaque to hit testing, consuming any pointer
    /// events within its bounds and preventing them from reaching elements
    /// behind it in Z-order (siblings and ancestors).
    ///
    /// Children of this node can still receive pointer events normally.
    Opaque,

    /// The semantic element is transparent to hit testing.
    ///
    /// Transparent nodes do not receive hit test events and allow events to pass
    /// through to elements behind them.
    Transparent,
}

/// The validation result of a form field.
///
/// The type, shape, and correctness of the value is specific to the kind of
/// form field used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SemanticsValidationResult {
    /// The node has no validation information attached to it.
    ///
    /// This is the default value.
    #[default]
    None,

    /// The entered value is valid, and no error should be displayed to the user.
    Valid,

    /// The entered value is invalid, and an error message should be communicated
    /// to the user.
    Invalid,
}

/// An abstract interface for string attributes that affects how assistive
/// technologies, e.g. VoiceOver or TalkBack, treat the text.
///
/// Dart's `abstract base class StringAttribute`, held as a [`StringAttributeRef`]. It does
/// not override `==`: two attributes are equal only when they are the same object.
pub trait StringAttribute: Debug + Any {
    /// The range of the text to which this attribute applies.
    fn range(&self) -> TextRange;

    /// Creates a new attribute with all properties copied except for range, which
    /// is updated to the specified value.
    ///
    /// For example, the [`LocaleStringAttribute`] specifies a [`Locale`] for its
    /// range of characters. Copying it will result in a new
    /// [`LocaleStringAttribute`] that has the same locale but an updated
    /// [`TextRange`].
    fn copy(&self, range: TextRange) -> StringAttributeRef;
}

/// A shared [`StringAttribute`]; Dart's identity is the `Rc`'s.
pub type StringAttributeRef = Rc<dyn StringAttribute>;

/// A string attribute that causes the assistive technologies, e.g. VoiceOver,
/// to spell out the string character by character.
pub struct SpellOutStringAttribute {
    range: TextRange,
}

impl SpellOutStringAttribute {
    /// Creates a string attribute that denotes the text in `range` must be
    /// spell out when the assistive technologies announce the string.
    pub fn new(range: TextRange) -> Rc<SpellOutStringAttribute> {
        Rc::new(SpellOutStringAttribute { range })
    }
}

impl StringAttribute for SpellOutStringAttribute {
    fn range(&self) -> TextRange {
        self.range
    }

    fn copy(&self, range: TextRange) -> StringAttributeRef {
        SpellOutStringAttribute::new(range)
    }
}

impl Debug for SpellOutStringAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpellOutStringAttribute({:?})", self.range)
    }
}

/// A string attribute that causes the assistive technologies, e.g. VoiceOver,
/// to treat string as a certain language.
pub struct LocaleStringAttribute {
    range: TextRange,

    /// The language of this attribute.
    pub locale: Locale,
}

impl LocaleStringAttribute {
    /// Creates a string attribute that denotes the text in `range` must be
    /// treated as the language specified by the `locale` when the assistive
    /// technologies announce the string.
    pub fn new(range: TextRange, locale: Locale) -> Rc<LocaleStringAttribute> {
        Rc::new(LocaleStringAttribute { range, locale })
    }
}

impl StringAttribute for LocaleStringAttribute {
    fn range(&self) -> TextRange {
        self.range
    }

    fn copy(&self, range: TextRange) -> StringAttributeRef {
        LocaleStringAttribute::new(range, self.locale.clone())
    }
}

impl Debug for LocaleStringAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LocaleStringAttribute({:?}, {})",
            self.range,
            self.locale.to_language_tag()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_action_is_found_by_its_bit() {
        assert_eq!(
            SemanticsAction::from_index(1 << 17),
            Some(SemanticsAction::CUSTOM_ACTION)
        );
        assert_eq!(SemanticsAction::from_index(1 << 26), None);
        assert_eq!(SemanticsAction::values().len(), 26);
    }

    #[test]
    fn copying_a_locale_attribute_keeps_its_locale_and_takes_the_new_range() {
        let attribute = LocaleStringAttribute::new(TextRange::new(0, 2), Locale::new("es"));
        let copy = attribute.copy(TextRange::new(3, 5));
        assert_eq!(copy.range(), TextRange::new(3, 5));
        let copy = (copy.as_ref() as &dyn Any)
            .downcast_ref::<LocaleStringAttribute>()
            .unwrap();
        assert_eq!(copy.locale, Locale::new("es"));
    }
}
