//! Flutter counterpart: `semantics/semantics.dart`'s values that annotate a subtree:
//! the handler typedefs, `AccessibilityFocusBlockType`, `SemanticsTag`,
//! `CustomSemanticsAction`, `AttributedString`, `SemanticsHintOverrides`,
//! `SemanticsProperties`, `SemanticsSortKey` and `OrdinalSortKey`.
//!
//! Flutter's `semantics` library sits between painting and rendering; Waft has no crate for
//! it, so its values live in the lowest crate above painting. `SemanticsNode`,
//! `SemanticsOwner`, `SemanticsConfiguration` and `SemanticsData` wait with the semantics
//! tree: until it lands nothing reads these values, and they are kept as the widgets give them.

use std::any::Any;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::ops::Add;
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use waft_embedder::{
    SemanticsAction, SemanticsHitTestBehavior, SemanticsInputType, SemanticsRole,
    SemanticsValidationResult, StringAttributeRef, TextDirection, TextRange, TextSelection,
};
use waft_foundation::{App, VoidCallback};

/// Signature for `SemanticsAction`s that move the cursor.
///
/// If `extendSelection` is set to true the cursor movement should extend the
/// current selection or (if nothing is currently selected) start a selection.
pub type MoveCursorHandler = Rc<dyn Fn(&mut App, bool)>;

/// Signature for the `SemanticsAction.setSelection` handlers to change the
/// text selection (or re-position the cursor) to `selection`.
pub type SetSelectionHandler = Rc<dyn Fn(&mut App, TextSelection)>;

/// Signature for the `SemanticsAction.setText` handlers to replace the
/// current text with the input `text`.
pub type SetTextHandler = Rc<dyn Fn(&mut App, String)>;

/// Controls how accessibility focus is blocked.
///
/// This is typically used to prevent screen readers from focusing on parts
/// of the UI.
///
/// Setting this property also blocks the reporting of keyboard focusability
/// for the semantics node, but it does not affect the actual keyboard focus
/// handled by `FocusNode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessibilityFocusBlockType {
    /// Accessibility focus is **not blocked**.
    None,

    /// Blocks accessibility focus for the entire subtree.
    BlockSubtree,

    /// Blocks accessibility focus for the **current node only**. Its descendants
    /// may still be focusable.
    BlockNode,
}

/// A tag for a `SemanticsNode`.
///
/// Tags can be interpreted by the parent of a `SemanticsNode`
/// and depending on the presence of a tag the parent can for example decide
/// how to add the tagged node as a child. Tags are not sent to the engine.
///
/// The provided [`name`](Self::name) is only used for debugging. Two tags are identical only
/// when they are the same object: held as `Rc<SemanticsTag>`, the `Rc`'s identity.
pub struct SemanticsTag {
    /// A human-readable name for this tag used for debugging.
    ///
    /// This string is not used to determine if two tags are identical.
    pub name: String,
}

impl SemanticsTag {
    /// Creates a [`SemanticsTag`].
    pub fn new(name: impl Into<String>) -> Rc<SemanticsTag> {
        Rc::new(SemanticsTag { name: name.into() })
    }
}

impl Debug for SemanticsTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SemanticsTag({})", self.name)
    }
}

/// An identifier of a custom semantics action.
///
/// Custom semantics actions can be provided to make complex user
/// interactions more accessible. For instance, if an application has a
/// drag-and-drop list that requires the user to press and hold an item
/// to move it, users interacting with the application using a hardware
/// switch may have difficulty. This can be made accessible by creating custom
/// actions and pairing them with handlers that move a list item up or down in
/// the list.
///
/// In Android, these actions are presented in the local context menu. In iOS,
/// these are presented in the radial context menu.
///
/// Localization and text direction do not automatically apply to the provided
/// label or hint.
///
/// Dart's identifier registry (`getIdentifier`, `getAction`, `resetForTests`) waits with the
/// semantics tree, which is what sends the identifiers.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CustomSemanticsAction {
    /// The user readable name of this custom semantics action.
    pub label: Option<String>,

    /// The hint description of this custom semantics action.
    pub hint: Option<String>,

    /// The standard semantics action this action replaces.
    pub action: Option<SemanticsAction>,
}

impl CustomSemanticsAction {
    /// Creates a new [`CustomSemanticsAction`].
    ///
    /// The `label` must not be empty.
    pub fn new(label: impl Into<String>) -> CustomSemanticsAction {
        let label = label.into();
        debug_assert!(!label.is_empty());
        CustomSemanticsAction {
            label: Some(label),
            hint: None,
            action: None,
        }
    }

    /// Creates a new [`CustomSemanticsAction`] that overrides a standard semantics
    /// action.
    ///
    /// The `hint` must not be empty.
    pub fn overriding_action(
        hint: impl Into<String>,
        action: SemanticsAction,
    ) -> CustomSemanticsAction {
        let hint = hint.into();
        debug_assert!(!hint.is_empty());
        CustomSemanticsAction {
            label: None,
            hint: Some(hint),
            action: Some(action),
        }
    }
}

impl Debug for CustomSemanticsAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CustomSemanticsAction(label:{:?}, hint:{:?}, action:{:?})",
            self.label, self.hint, self.action
        )
    }
}

/// A string that carries a list of `StringAttribute`s.
#[derive(Clone)]
pub struct AttributedString {
    /// The plain string stored in the attributed string.
    pub string: String,

    /// The attributes this string carries.
    ///
    /// The list must not be modified after this string is created.
    pub attributes: Vec<StringAttributeRef>,
}

impl AttributedString {
    /// Creates a attributed string.
    pub fn new(string: impl Into<String>) -> AttributedString {
        AttributedString {
            string: string.into(),
            attributes: Vec::new(),
        }
    }

    /// Dart `AttributedString(attributes:)`.
    ///
    /// The `TextRange` in the `attributes` must be inside the length of the
    /// `string`.
    ///
    /// The `attributes` must not be changed after the attributed string is
    /// created.
    pub fn attributes(mut self, attributes: Vec<StringAttributeRef>) -> AttributedString {
        debug_assert!(!self.string.is_empty() || attributes.is_empty());
        if cfg!(debug_assertions) {
            let length = utf16_length(&self.string);
            for attribute in &attributes {
                debug_assert!(
                    length >= attribute.range().start && length >= attribute.range().end,
                    "The range in {attribute:?} is outside of the string {}",
                    self.string
                );
            }
        }
        self.attributes = attributes;
        self
    }
}

/// Dart's `String.length`: UTF-16 code units, the unit of a `TextRange`.
fn utf16_length(string: &str) -> i32 {
    string.encode_utf16().count() as i32
}

impl Add for AttributedString {
    type Output = AttributedString;

    /// Returns a new [`AttributedString`] by concatenate the operands
    ///
    /// The string attribute list of the returned [`AttributedString`] will contains
    /// the string attributes from both operands with updated text ranges.
    fn add(self, other: AttributedString) -> AttributedString {
        if self.string.is_empty() {
            return other;
        }
        if other.string.is_empty() {
            return self;
        }

        // None of the strings is empty.
        let new_string = self.string.clone() + &other.string;
        let mut new_attributes = self.attributes.clone();
        if !other.attributes.is_empty() {
            let offset = utf16_length(&self.string);
            for attribute in &other.attributes {
                let new_range = TextRange::new(
                    attribute.range().start + offset,
                    attribute.range().end + offset,
                );
                let adjusted_attribute = attribute.copy(new_range);
                new_attributes.push(adjusted_attribute);
            }
        }
        AttributedString::new(new_string).attributes(new_attributes)
    }
}

/// Two [`AttributedString`]s are equal if their string and attributes are; an attribute equals
/// only itself.
impl PartialEq for AttributedString {
    fn eq(&self, other: &AttributedString) -> bool {
        other.string == self.string
            && other.attributes.len() == self.attributes.len()
            && other
                .attributes
                .iter()
                .zip(&self.attributes)
                .all(|(a, b)| Rc::ptr_eq(a, b))
    }
}

impl Debug for AttributedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttributedString('{}', attributes: {:?})",
            self.string, self.attributes
        )
    }
}

/// Provides hint values which override the default hints on supported
/// platforms.
///
/// On iOS, these values are always ignored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SemanticsHintOverrides {
    /// The hint text for a tap action.
    ///
    /// If null, the standard hint is used instead.
    ///
    /// The hint should describe what happens when a tap occurs, not the
    /// manner in which a tap is accomplished.
    ///
    /// Bad: 'Double tap to show movies'.
    /// Good: 'show movies'.
    pub on_tap_hint: Option<String>,

    /// The hint text for a long press action.
    ///
    /// If null, the standard hint is used instead.
    ///
    /// The hint should describe what happens when a long press occurs, not
    /// the manner in which the long press is accomplished.
    ///
    /// Bad: 'Double tap and hold to favorite'.
    /// Good: 'favorite'.
    pub on_long_press_hint: Option<String>,
}

impl SemanticsHintOverrides {
    /// Creates a semantics hint overrides.
    pub fn new() -> SemanticsHintOverrides {
        SemanticsHintOverrides::default()
    }

    /// Dart `SemanticsHintOverrides(onTapHint:)`.
    pub fn on_tap_hint(mut self, on_tap_hint: impl Into<String>) -> SemanticsHintOverrides {
        let on_tap_hint = on_tap_hint.into();
        debug_assert!(!on_tap_hint.is_empty());
        self.on_tap_hint = Some(on_tap_hint);
        self
    }

    /// Dart `SemanticsHintOverrides(onLongPressHint:)`.
    pub fn on_long_press_hint(
        mut self,
        on_long_press_hint: impl Into<String>,
    ) -> SemanticsHintOverrides {
        let on_long_press_hint = on_long_press_hint.into();
        debug_assert!(!on_long_press_hint.is_empty());
        self.on_long_press_hint = Some(on_long_press_hint);
        self
    }

    /// Whether there are any non-null hint values.
    pub fn is_not_empty(&self) -> bool {
        self.on_tap_hint.is_some() || self.on_long_press_hint.is_some()
    }
}

/// Contains properties used by assistive technologies to make the application
/// more accessible.
///
/// The properties of this class are used to generate a `SemanticsNode`s in the
/// semantics tree.
///
/// Dart's class does not override `==`: a render object compares two by identity, so they are
/// held as `Rc<SemanticsProperties>` and compared with `Rc::ptr_eq`.
#[derive(Clone, Default)]
pub struct SemanticsProperties {
    /// If non-null, indicates that this subtree represents something that can be
    /// in an enabled or disabled state.
    ///
    /// For example, a button that a user can currently interact with would set
    /// this field to true. A button that currently does not respond to user
    /// interactions would set this field to false.
    pub enabled: Option<bool>,

    /// If non-null, indicates that this subtree represents a checkbox
    /// or similar widget with a "checked" state, and what its current
    /// state is.
    ///
    /// When the `Checkbox.value` of a tristate Checkbox is null,
    /// indicating a mixed-state, this value shall be false, in which
    /// case, `mixed` will be true.
    ///
    /// This is mutually exclusive with `toggled` and `mixed`.
    pub checked: Option<bool>,

    /// If non-null, indicates that this subtree represents a checkbox
    /// or similar widget with a "half-checked" state or similar, and
    /// whether it is currently in this half-checked state.
    ///
    /// This must be null when `Checkbox.tristate` is false, or
    /// when the widget is not a checkbox. When a tristate
    /// checkbox is fully unchecked/checked, this value shall
    /// be false.
    ///
    /// This is mutually exclusive with `checked` and `toggled`.
    pub mixed: Option<bool>,

    /// If non-null, indicates that this subtree represents something
    /// that can be in an "expanded" or "collapsed" state.
    ///
    /// For example, if a `SubmenuButton` is opened, this property
    /// should be set to true; otherwise, this property should be
    /// false.
    pub expanded: Option<bool>,

    /// If non-null, indicates that this subtree represents a toggle switch
    /// or similar widget with an "on" state, and what its current
    /// state is.
    ///
    /// This is mutually exclusive with `checked` and `mixed`.
    pub toggled: Option<bool>,

    /// If non-null indicates that this subtree represents something that can be
    /// in a selected or unselected state, and what its current state is.
    ///
    /// The active tab in a tab bar for example is considered "selected", whereas
    /// all other tabs are unselected.
    pub selected: Option<bool>,

    /// If non-null, indicates that this subtree represents a button.
    ///
    /// TalkBack/VoiceOver provides users with the hint "button" when a button
    /// is focused.
    pub button: Option<bool>,

    /// If non-null, indicates that this subtree represents a link.
    ///
    /// iOS's VoiceOver provides users with a unique hint when a link is focused.
    /// Android's Talkback will announce a link hint the same way it does a
    /// button.
    pub link: Option<bool>,

    /// If non-null, indicates that this subtree represents a header.
    ///
    /// A header is typically the top element of a page or section, such as a
    /// page title or app bar title.
    pub header: Option<bool>,

    /// If non-null, indicates that this subtree represents a text field.
    ///
    /// TalkBack/VoiceOver provide special affordances to enter text into a
    /// text field.
    pub text_field: Option<bool>,

    /// If non-null, indicates that this subtree represents a slider.
    ///
    /// Talkback/\VoiceOver provides users with the hint "slider" when a
    /// slider is focused.
    pub slider: Option<bool>,

    /// If non-null, indicates that this subtree represents a keyboard key.
    pub keyboard_key: Option<bool>,

    /// If non-null, indicates that this subtree is read only.
    ///
    /// Only applicable when `textField` is true.
    ///
    /// TalkBack/VoiceOver will treat it as non-editable text field.
    pub read_only: Option<bool>,

    /// If non-null, whether the node is able to hold input focus.
    ///
    /// If `focusable` is set to false, then `focused` must not be true.
    ///
    /// Input focus indicates that the node will receive keyboard events. It is not
    /// to be confused with accessibility focus. Accessibility focus is the
    /// green/black rectangular highlight that TalkBack/VoiceOver draws around the
    /// element it is reading, and is separate from input focus.
    ///
    /// Deprecated: use `focused` instead; setting `focused` automatically sets `focusable`.
    pub focusable: Option<bool>,

    /// If non-null, whether the node currently holds input focus.
    ///
    /// If null, the node is not focusable.
    ///
    /// At most one node in the tree should hold input focus at any point in time,
    /// and it should not be set to true if `focusable` is false.
    ///
    /// Input focus indicates that the node will receive keyboard events. It is not
    /// to be confused with accessibility focus. Accessibility focus is the
    /// green/black rectangular highlight that TalkBack/VoiceOver draws around the
    /// element it is reading, and is separate from input focus.
    pub focused: Option<bool>,

    /// If non-null, indicates if this subtree or current node is blocked in a11y focus.
    ///
    /// This is for accessibility focus, which is the focus used by screen readers
    /// like TalkBack and VoiceOver. It is different from input focus, which is
    /// usually held by the element that currently responds to keyboard inputs.
    pub accessibility_focus_block_type: Option<AccessibilityFocusBlockType>,

    /// If non-null, whether a semantic node is in a mutually exclusive group.
    ///
    /// For example, a radio button is in a mutually exclusive group because only
    /// one radio button in that group can be marked as `checked`.
    pub in_mutually_exclusive_group: Option<bool>,

    /// If non-null, whether the node is considered hidden.
    ///
    /// Hidden elements are currently not visible on screen. They may be covered
    /// by other elements or positioned outside of the visible area of a viewport.
    ///
    /// Hidden elements cannot gain accessibility focus though regular touch. The
    /// only way they can be focused is by moving the focus to them via linear
    /// navigation.
    ///
    /// Platforms are free to completely ignore hidden elements and new platforms
    /// are encouraged to do so.
    ///
    /// Instead of marking an element as hidden it should usually be excluded from
    /// the semantics tree altogether. Hidden elements are only included in the
    /// semantics tree to work around platform limitations and they are mainly
    /// used to implement accessibility scrolling on iOS.
    pub hidden: Option<bool>,

    /// If non-null, whether `value` should be obscured.
    ///
    /// This option is usually set in combination with `textField` to indicate
    /// that the text field contains a password (or other sensitive information).
    /// Doing so instructs screen readers to not read out the `value`.
    pub obscured: Option<bool>,

    /// Whether the `value` is coming from a field that supports multiline text
    /// editing.
    ///
    /// This option is only meaningful when `textField` is true to indicate
    /// whether it's a single-line or multiline text field.
    ///
    /// This option is null when `textField` is false.
    pub multiline: Option<bool>,

    /// If non-null, whether the node corresponds to the root of a subtree for
    /// which a route name should be announced.
    ///
    /// Generally, this is set in combination with
    /// `SemanticsConfiguration.explicitChildNodes`, since nodes with this flag
    /// are not considered focusable by Android or iOS.
    pub scopes_route: Option<bool>,

    /// If non-null, whether the node contains the semantic label for a route.
    pub names_route: Option<bool>,

    /// If non-null, whether the node represents an image.
    pub image: Option<bool>,

    /// If non-null, whether the node should be considered a live region.
    ///
    /// A live region indicates that updates to semantics node are important.
    /// Platforms may use this information to make polite announcements to the
    /// user to inform them of updates to this node.
    ///
    /// An example of a live region is a `SnackBar` widget. On Android and iOS,
    /// live region causes a polite announcement to be generated automatically,
    /// even if the widget does not have accessibility focus. This announcement
    /// may not be spoken if the OS accessibility services are already
    /// announcing something else, such as reading the label of a focused widget
    /// or providing a system announcement.
    pub live_region: Option<bool>,

    /// If non-null, whether the node should be considered required.
    ///
    /// If true, user input is required on the semantics node before a form can
    /// be submitted. If false, the node is optional before a form can be
    /// submitted. If null, the node does not have a required semantics.
    ///
    /// For example, a login form requires its email text field to be non-empty.
    ///
    /// On web, this will set a `aria-required` attribute on the DOM element
    /// that corresponds to the semantics node.
    pub is_required: Option<bool>,

    /// The maximum number of characters that can be entered into an editable
    /// text field.
    ///
    /// For the purpose of this function a character is defined as one Unicode
    /// scalar value.
    ///
    /// This should only be set when `textField` is true. Defaults to null,
    /// which means no limit is imposed on the text field.
    pub max_value_length: Option<i32>,

    /// The current number of characters that have been entered into an editable
    /// text field.
    ///
    /// For the purpose of this function a character is defined as one Unicode
    /// scalar value.
    ///
    /// This should only be set when `textField` is true. Must be set when
    /// `maxValueLength` is set.
    pub current_value_length: Option<i32>,

    /// Provides an identifier for the semantics node in native accessibility hierarchy.
    ///
    /// This value is not exposed to the users of the app.
    ///
    /// It's usually used for UI testing with tools that work by querying the
    /// native accessibility, like UIAutomator, XCUITest, or Appium. It can be
    /// matched with `CommonFinders.bySemanticsIdentifier`.
    ///
    /// When set, this property implicitly forces the creation of a new
    /// `SemanticsNode` (equivalent to setting `container` to true in
    /// `Semantics`). This ensures the identifier is always attached to its
    /// own node and is not merged into an ancestor.
    ///
    /// On Android, this is used for `AccessibilityNodeInfo.setViewIdResourceName`.
    /// It'll be appear in accessibility hierarchy as `resource-id`.
    ///
    /// On iOS, this will set `UIAccessibilityElement.accessibilityIdentifier`.
    ///
    /// On web, this will set a `flt-semantics-identifier` attribute on the DOM element
    /// that corresponds to the semantics node.
    pub identifier: Option<String>,

    /// Provides an identifier for establishing parent-child relationships in the semantics
    /// traversal tree.
    ///
    /// This property is used to create a logical parent-child relationship between
    /// semantics nodes that may not be directly connected in the widget tree. It's
    /// primarily used with `OverlayPortal` to ensure proper accessibility traversal
    /// order when overlay content needs to be semantically connected to its parent
    /// widget.
    ///
    /// When a semantics node has a `traversalParentIdentifier`, it indicates that
    /// this node can act as a parent for other nodes that reference this identifier
    /// in their `traversalChildIdentifier`. This allows assistive technologies
    /// to navigate the UI in the correct logical order.
    ///
    /// The `traversalParentIdentifier` must be unique in the semantics. No two
    /// semantics node can have the same `traversalParentIdentifier`. This unique
    /// identifier serves as the only reference for its traversal children. To
    /// graft other nodes as the traversal children of this node, assign this same
    /// value to their `traversalChildIdentifier`.
    pub traversal_parent_identifier: Option<Rc<dyn Any>>,

    /// Provides an identifier for establishing parent-child relationships in the semantics
    /// traversal tree.
    ///
    /// This property is used to create a logical parent-child relationship between
    /// semantics nodes that may not be directly connected in the widget tree. It's
    /// primarily used with `OverlayPortal` to ensure proper accessibility traversal
    /// order when overlay content needs to be semantically connected to its parent
    /// widget.
    ///
    /// When a semantics node has a `traversalChildIdentifier`, it indicates that
    /// this node should be treated as a child of another node that has this same
    /// identifier as its `traversalParentIdentifier`. This allows assistive technologies
    /// to navigate the UI in the correct logical order.
    ///
    /// The `traversalChildIdentifier` value may be duplicated across multiple
    /// semantics nodes. To establish one or more nodes as the traversal children
    /// of a parent node, assign this identifier the same value as the parent's
    /// `traversalParentIdentifier`.
    pub traversal_child_identifier: Option<Rc<dyn Any>>,

    /// Provides a textual description of the widget.
    ///
    /// If a label is provided, there must either by an ambient `Directionality`
    /// or an explicit `textDirection` should be provided.
    ///
    /// Callers must not provide both `label` and `attributedLabel`. One or both
    /// must be null.
    pub label: Option<String>,

    /// Provides an `AttributedString` version of textual description of the widget.
    ///
    /// If a `attributedLabel` is provided, there must either by an ambient
    /// `Directionality` or an explicit `textDirection` should be provided.
    ///
    /// Callers must not provide both `label` and `attributedLabel`. One or both
    /// must be null.
    pub attributed_label: Option<AttributedString>,

    /// Provides a textual description of the value of the widget.
    ///
    /// If a value is provided, there must either by an ambient `Directionality`
    /// or an explicit `textDirection` should be provided.
    ///
    /// Callers must not provide both `value` and `attributedValue`, One or both
    /// must be null.
    pub value: Option<String>,

    /// Provides an `AttributedString` version of textual description of the value
    /// of the widget.
    ///
    /// If a `attributedValue` is provided, there must either by an ambient
    /// `Directionality` or an explicit `textDirection` should be provided.
    ///
    /// Callers must not provide both `value` and `attributedValue`, One or both
    /// must be null.
    pub attributed_value: Option<AttributedString>,

    /// The value that `value` or `attributedValue` will become after a
    /// `SemanticsAction.increase` action has been performed on this widget.
    ///
    /// If a value is provided, `onIncrease` must also be set and there must
    /// either be an ambient `Directionality` or an explicit `textDirection`
    /// must be provided.
    ///
    /// Callers must not provide both `increasedValue` and
    /// `attributedIncreasedValue`, One or both must be null.
    pub increased_value: Option<String>,

    /// The `AttributedString` that `value` or `attributedValue` will become after
    /// a `SemanticsAction.increase` action has been performed on this widget.
    ///
    /// If a `attributedIncreasedValue` is provided, `onIncrease` must also be set
    /// and there must either be an ambient `Directionality` or an explicit
    /// `textDirection` must be provided.
    ///
    /// Callers must not provide both `increasedValue` and
    /// `attributedIncreasedValue`, One or both must be null.
    pub attributed_increased_value: Option<AttributedString>,

    /// The value that `value` or `attributedValue` will become after a
    /// `SemanticsAction.decrease` action has been performed on this widget.
    ///
    /// If a value is provided, `onDecrease` must also be set and there must
    /// either be an ambient `Directionality` or an explicit `textDirection`
    /// must be provided.
    ///
    /// Callers must not provide both `decreasedValue` and
    /// `attributedDecreasedValue`, One or both must be null.
    pub decreased_value: Option<String>,

    /// The `AttributedString` that `value` or `attributedValue` will become after
    /// a `SemanticsAction.decrease` action has been performed on this widget.
    ///
    /// If a `attributedDecreasedValue` is provided, `onDecrease` must also be set
    /// and there must either be an ambient `Directionality` or an explicit
    /// `textDirection` must be provided.
    ///
    /// Callers must not provide both `decreasedValue` and
    /// `attributedDecreasedValue`, One or both must be null/// provided.
    pub attributed_decreased_value: Option<AttributedString>,

    /// Provides a brief textual description of the result of an action performed
    /// on the widget.
    ///
    /// If a hint is provided, there must either be an ambient `Directionality`
    /// or an explicit `textDirection` should be provided.
    ///
    /// Callers must not provide both `hint` and `attributedHint`, One or both
    /// must be null.
    pub hint: Option<String>,

    /// Provides an `AttributedString` version of brief textual description of the
    /// result of an action performed on the widget.
    ///
    /// If a `attributedHint` is provided, there must either by an ambient
    /// `Directionality` or an explicit `textDirection` should be provided.
    ///
    /// Callers must not provide both `hint` and `attributedHint`, One or both
    /// must be null.
    pub attributed_hint: Option<AttributedString>,

    /// Provides a textual description of the widget's tooltip.
    ///
    /// In Android, this property sets the `AccessibilityNodeInfo.setTooltipText`.
    /// In iOS, this property is appended to the end of the
    /// `UIAccessibilityElement.accessibilityLabel`.
    ///
    /// If a `tooltip` is provided, there must either by an ambient
    /// `Directionality` or an explicit `textDirection` should be provided.
    pub tooltip: Option<String>,

    /// The heading level in the document structure.
    ///
    /// A heading divides content into sections. For example, an address book
    /// application might define headings A, B, C, etc. to divide the list of
    /// alphabetically sorted contacts into sections.
    ///
    /// Screen readers use this value to determine which part of the page
    /// structure this heading represents. A level 1 heading usually indicates
    /// the main heading of a page, a level 2 heading the first subsection,
    /// a level 3 is a subsection of that, and so on.
    ///
    /// On web, this sets the `aria-level` attribute (e.g., `aria-level="1"`).
    /// On Android, this sets the `isHeading` property for accessibility.
    pub heading_level: Option<i32>,

    /// Overrides the default accessibility hints provided by the platform.
    ///
    /// This `hintOverrides` property does not affect how the platform processes hints;
    /// it only sets the custom text that will be read by assistive technology.
    ///
    /// On Android, these overrides replace the default hints for semantics nodes
    /// with tap or long-press actions. For example, if `SemanticsHintOverrides.onTapHint`
    /// is provided, instead of saying `Double tap to activate`, the screen reader
    /// will say `Double tap to <onTapHint>`.
    ///
    /// On iOS, this property is ignored, and default platform behavior applies.
    ///
    /// Example usage:
    pub hint_overrides: Option<SemanticsHintOverrides>,

    /// The reading direction of the `label`, `value`, `increasedValue`,
    /// `decreasedValue`, and `hint`.
    ///
    /// Defaults to the ambient `Directionality`.
    pub text_direction: Option<TextDirection>,

    /// Determines the position of this node among its siblings in the traversal
    /// sort order.
    ///
    /// This is used to describe the order in which the semantic node should be
    /// traversed by the accessibility services on the platform (e.g. VoiceOver
    /// on iOS and TalkBack on Android).
    pub sort_key: Option<SemanticsSortKeyRef>,

    /// A tag to be applied to the child `SemanticsNode`s of this widget.
    ///
    /// The tag is added to all child `SemanticsNode`s that pass through the
    /// `RenderObject` corresponding to this widget while looking to be attached
    /// to a parent SemanticsNode.
    ///
    /// Tags are used to communicate to a parent SemanticsNode that a child
    /// SemanticsNode was passed through a particular RenderObject. The parent can
    /// use this information to determine the shape of the semantics tree.
    pub tag_for_children: Option<Rc<SemanticsTag>>,

    /// The URL that this node links to.
    ///
    /// On the web, this is used to set the `href` attribute of the DOM element.
    pub link_url: Option<String>,

    /// The handler for `SemanticsAction.tap`.
    ///
    /// This is the semantic equivalent of a user briefly tapping the screen with
    /// the finger without moving it. For example, a button should implement this
    /// action.
    ///
    /// VoiceOver users on iOS and TalkBack users on Android *may* trigger this
    /// action by double-tapping the screen while an element is focused.
    ///
    /// Note: different OSes or assistive technologies may decide to interpret
    /// user inputs differently. Some may simulate real screen taps, while others
    /// may call semantics tap. One way to handle taps properly is to provide the
    /// same handler to both gesture tap and semantics tap.
    pub on_tap: Option<VoidCallback>,

    /// The handler for `SemanticsAction.longPress`.
    ///
    /// This is the semantic equivalent of a user pressing and holding the screen
    /// with the finger for a few seconds without moving it.
    ///
    /// VoiceOver users on iOS and TalkBack users on Android *may* trigger this
    /// action by double-tapping the screen without lifting the finger after the
    /// second tap.
    ///
    /// Note: different OSes or assistive technologies may decide to interpret
    /// user inputs differently. Some may simulate real long presses, while others
    /// may call semantics long press. One way to handle long press properly is to
    /// provide the same handler to both gesture long press and semantics long
    /// press.
    pub on_long_press: Option<VoidCallback>,

    /// The handler for `SemanticsAction.scrollLeft`.
    ///
    /// This is the semantic equivalent of a user moving their finger across the
    /// screen from right to left. It should be recognized by controls that are
    /// horizontally scrollable.
    ///
    /// VoiceOver users on iOS can trigger this action by swiping left with three
    /// fingers. TalkBack users on Android can trigger this action by swiping
    /// right and then left in one motion path. On Android, `onScrollUp` and
    /// `onScrollLeft` share the same gesture. Therefore, only on of them should
    /// be provided.
    pub on_scroll_left: Option<VoidCallback>,

    /// The handler for `SemanticsAction.scrollRight`.
    ///
    /// This is the semantic equivalent of a user moving their finger across the
    /// screen from left to right. It should be recognized by controls that are
    /// horizontally scrollable.
    ///
    /// VoiceOver users on iOS can trigger this action by swiping right with three
    /// fingers. TalkBack users on Android can trigger this action by swiping
    /// left and then right in one motion path. On Android, `onScrollDown` and
    /// `onScrollRight` share the same gesture. Therefore, only on of them should
    /// be provided.
    pub on_scroll_right: Option<VoidCallback>,

    /// The handler for `SemanticsAction.scrollUp`.
    ///
    /// This is the semantic equivalent of a user moving their finger across the
    /// screen from bottom to top. It should be recognized by controls that are
    /// vertically scrollable.
    ///
    /// VoiceOver users on iOS can trigger this action by swiping up with three
    /// fingers. TalkBack users on Android can trigger this action by swiping
    /// right and then left in one motion path. On Android, `onScrollUp` and
    /// `onScrollLeft` share the same gesture. Therefore, only on of them should
    /// be provided.
    pub on_scroll_up: Option<VoidCallback>,

    /// The handler for `SemanticsAction.scrollDown`.
    ///
    /// This is the semantic equivalent of a user moving their finger across the
    /// screen from top to bottom. It should be recognized by controls that are
    /// vertically scrollable.
    ///
    /// VoiceOver users on iOS can trigger this action by swiping down with three
    /// fingers. TalkBack users on Android can trigger this action by swiping
    /// left and then right in one motion path. On Android, `onScrollDown` and
    /// `onScrollRight` share the same gesture. Therefore, only on of them should
    /// be provided.
    pub on_scroll_down: Option<VoidCallback>,

    /// The handler for `SemanticsAction.increase`.
    ///
    /// This is a request to increase the value represented by the widget. For
    /// example, this action might be recognized by a slider control.
    ///
    /// If a `value` is set, `increasedValue` must also be provided and
    /// `onIncrease` must ensure that `value` will be set to `increasedValue`.
    ///
    /// VoiceOver users on iOS can trigger this action by swiping up with one
    /// finger. TalkBack users on Android can trigger this action by pressing the
    /// volume up button.
    pub on_increase: Option<VoidCallback>,

    /// The handler for `SemanticsAction.decrease`.
    ///
    /// This is a request to decrease the value represented by the widget. For
    /// example, this action might be recognized by a slider control.
    ///
    /// If a `value` is set, `decreasedValue` must also be provided and
    /// `onDecrease` must ensure that `value` will be set to `decreasedValue`.
    ///
    /// VoiceOver users on iOS can trigger this action by swiping down with one
    /// finger. TalkBack users on Android can trigger this action by pressing the
    /// volume down button.
    pub on_decrease: Option<VoidCallback>,

    /// The handler for `SemanticsAction.copy`.
    ///
    /// This is a request to copy the current selection to the clipboard.
    ///
    /// TalkBack users on Android can trigger this action from the local context
    /// menu of a text field, for example.
    pub on_copy: Option<VoidCallback>,

    /// The handler for `SemanticsAction.cut`.
    ///
    /// This is a request to cut the current selection and place it in the
    /// clipboard.
    ///
    /// TalkBack users on Android can trigger this action from the local context
    /// menu of a text field, for example.
    pub on_cut: Option<VoidCallback>,

    /// The handler for `SemanticsAction.paste`.
    ///
    /// This is a request to paste the current content of the clipboard.
    ///
    /// TalkBack users on Android can trigger this action from the local context
    /// menu of a text field, for example.
    pub on_paste: Option<VoidCallback>,

    /// The handler for `SemanticsAction.moveCursorForwardByCharacter`.
    ///
    /// This handler is invoked when the user wants to move the cursor in a
    /// text field forward by one character.
    ///
    /// TalkBack users can trigger this by pressing the volume up key while the
    /// input focus is in a text field.
    pub on_move_cursor_forward_by_character: Option<MoveCursorHandler>,

    /// The handler for `SemanticsAction.moveCursorBackwardByCharacter`.
    ///
    /// This handler is invoked when the user wants to move the cursor in a
    /// text field backward by one character.
    ///
    /// TalkBack users can trigger this by pressing the volume down key while the
    /// input focus is in a text field.
    pub on_move_cursor_backward_by_character: Option<MoveCursorHandler>,

    /// The handler for `SemanticsAction.moveCursorForwardByWord`.
    ///
    /// This handler is invoked when the user wants to move the cursor in a
    /// text field backward by one word.
    ///
    /// TalkBack users can trigger this by pressing the volume down key while the
    /// input focus is in a text field.
    pub on_move_cursor_forward_by_word: Option<MoveCursorHandler>,

    /// The handler for `SemanticsAction.moveCursorBackwardByWord`.
    ///
    /// This handler is invoked when the user wants to move the cursor in a
    /// text field backward by one word.
    ///
    /// TalkBack users can trigger this by pressing the volume down key while the
    /// input focus is in a text field.
    pub on_move_cursor_backward_by_word: Option<MoveCursorHandler>,

    /// The handler for `SemanticsAction.setSelection`.
    ///
    /// This handler is invoked when the user either wants to change the currently
    /// selected text in a text field or change the position of the cursor.
    ///
    /// TalkBack users can trigger this handler by selecting "Move cursor to
    /// beginning/end" or "Select all" from the local context menu.
    pub on_set_selection: Option<SetSelectionHandler>,

    /// The handler for `SemanticsAction.setText`.
    ///
    /// This handler is invoked when the user wants to replace the current text in
    /// the text field with a new text.
    ///
    /// Voice access users can trigger this handler by speaking `type <text>` to
    /// their Android devices.
    pub on_set_text: Option<SetTextHandler>,

    /// The handler for `SemanticsAction.didGainAccessibilityFocus`.
    ///
    /// This handler is invoked when the node annotated with this handler gains
    /// the accessibility focus. The accessibility focus is the
    /// green (on Android with TalkBack) or black (on iOS with VoiceOver)
    /// rectangle shown on screen to indicate what element an accessibility
    /// user is currently interacting with.
    ///
    /// The accessibility focus is different from the input focus. The input focus
    /// is usually held by the element that currently responds to keyboard inputs.
    /// Accessibility focus and input focus can be held by two different nodes!
    pub on_did_gain_accessibility_focus: Option<VoidCallback>,

    /// The handler for `SemanticsAction.didLoseAccessibilityFocus`.
    ///
    /// This handler is invoked when the node annotated with this handler
    /// loses the accessibility focus. The accessibility focus is
    /// the green (on Android with TalkBack) or black (on iOS with VoiceOver)
    /// rectangle shown on screen to indicate what element an accessibility
    /// user is currently interacting with.
    ///
    /// The accessibility focus is different from the input focus. The input focus
    /// is usually held by the element that currently responds to keyboard inputs.
    /// Accessibility focus and input focus can be held by two different nodes!
    pub on_did_lose_accessibility_focus: Option<VoidCallback>,

    /// The handler for `SemanticsAction.focus`.
    ///
    /// This handler is invoked when the assistive technology requests that the
    /// focusable widget corresponding to this semantics node gain input focus.
    /// The `FocusNode` that manages the focus of the widget must gain focus. The
    /// widget must begin responding to relevant key events. For example:
    ///
    /// * Buttons must respond to tap/click events produced via keyboard shortcuts.
    /// * Text fields must become focused and editable, showing an on-screen
    ///   keyboard, if necessary.
    /// * Checkboxes, switches, and radio buttons must become toggleable using
    ///   keyboard shortcuts.
    ///
    /// Focus behavior is specific to the platform and to the assistive technology
    /// used. See the documentation of `SemanticsAction.focus` for more detail.
    pub on_focus: Option<VoidCallback>,

    /// The handler for `SemanticsAction.dismiss`.
    ///
    /// This is a request to dismiss the currently focused node.
    ///
    /// TalkBack users on Android can trigger this action in the local context
    /// menu, and VoiceOver users on iOS can trigger this action with a standard
    /// gesture or menu option.
    pub on_dismiss: Option<VoidCallback>,

    /// The handler for `SemanticsAction.expand`.
    ///
    /// This is a request to expand the currently focused node. For example, this
    /// action might be recognized by a dropdown.
    ///
    /// This handler should only be set when the node is in a collapsed state
    /// (i.e., `expanded` is false).
    pub on_expand: Option<VoidCallback>,

    /// The handler for `SemanticsAction.collapse`.
    ///
    /// This is a request to collapse the currently focused node. For example,
    /// this action might be recognized by a dropdown.
    ///
    /// This handler should only be set when the node is in an expanded state
    /// (i.e., `expanded` is true).
    pub on_collapse: Option<VoidCallback>,

    /// A map from each supported `CustomSemanticsAction` to a provided handler.
    ///
    /// The handler associated with each custom action is called whenever a
    /// semantics action of type `SemanticsAction.customAction` is received. The
    /// provided argument will be an identifier used to retrieve an instance of
    /// a custom action which can then retrieve the correct handler from this map.
    pub custom_semantics_actions: Option<IndexMap<CustomSemanticsAction, VoidCallback>>,

    /// A enum to describe what role the subtree represents.
    ///
    /// Setting the role for a widget subtree helps assistive technologies, such
    /// as screen readers, to understand and interact with the UI correctly.
    ///
    /// Defaults to `SemanticsRole.none` if not set, which means the subtree does
    /// not represent any complex ui or controls.
    ///
    /// For a list of available roles, see `SemanticsRole`.
    pub role: Option<SemanticsRole>,

    /// The `SemanticsNode.identifier`s of widgets controlled by this subtree.
    ///
    /// If a widget is controlling the visibility or content of another widget,
    /// for example, `Tab`s control child visibilities of `TabBarView` or
    /// `ExpansionTile` controls visibility of its expanded content, one must
    /// assign a `SemanticsNode.identifier` to the content and also provide a set
    /// of identifiers including the content's identifier to this property.
    pub controls_nodes: Option<IndexSet<String>>,

    /// Describes the validation result for a form field represented by this
    /// widget.
    ///
    /// Providing a validation result helps assistive technologies, such as screen
    /// readers, to communicate to the user whether they provided correct
    /// information in a form.
    ///
    /// Defaults to `SemanticsValidationResult.none` if not set, which means no
    /// validation information is available for the respective semantics node.
    ///
    /// For a list of available validation results, see `SemanticsValidationResult`.
    pub validation_result: SemanticsValidationResult,

    /// Describes how the semantic node should behave during hit testing.
    ///
    /// See `ui.SemanticsHitTestBehavior` for more details.
    pub hit_test_behavior: Option<SemanticsHitTestBehavior>,

    /// The input type for of a editable widget.
    ///
    /// This property is only used when the subtree represents a text field.
    ///
    /// Assistive technologies use this property to provide better information to
    /// users. For example, screen reader reads out the input type of text field
    /// when focused.
    pub input_type: Option<SemanticsInputType>,

    /// The maximum value of the node.
    ///
    /// Used in conjunction with `value` to define the current value and range
    /// of a node. A typical usage is for progress indicators, where `value`
    /// represents the current progress and `maxValue` defines the maximum
    /// possible value.
    pub max_value: Option<String>,

    /// The minimum value of the node.
    ///
    /// Used in conjunction with `value` to define the current value and range
    /// of a node. A typical usage is for progress indicators, where `value`
    /// represents the current progress and `minValue` defines the minimum
    /// possible value.
    pub min_value: Option<String>,
}

impl SemanticsProperties {
    /// Creates a semantic annotation.
    pub fn new() -> SemanticsProperties {
        SemanticsProperties::default()
    }

    /// Dart `SemanticsProperties(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> SemanticsProperties {
        self.enabled = Some(enabled);
        self
    }

    /// Dart `SemanticsProperties(checked:)`.
    pub fn checked(mut self, checked: bool) -> SemanticsProperties {
        self.checked = Some(checked);
        self
    }

    /// Dart `SemanticsProperties(mixed:)`.
    pub fn mixed(mut self, mixed: bool) -> SemanticsProperties {
        self.mixed = Some(mixed);
        self
    }

    /// Dart `SemanticsProperties(expanded:)`.
    pub fn expanded(mut self, expanded: bool) -> SemanticsProperties {
        self.expanded = Some(expanded);
        self
    }

    /// Dart `SemanticsProperties(toggled:)`.
    pub fn toggled(mut self, toggled: bool) -> SemanticsProperties {
        self.toggled = Some(toggled);
        self
    }

    /// Dart `SemanticsProperties(selected:)`.
    pub fn selected(mut self, selected: bool) -> SemanticsProperties {
        self.selected = Some(selected);
        self
    }

    /// Dart `SemanticsProperties(button:)`.
    pub fn button(mut self, button: bool) -> SemanticsProperties {
        self.button = Some(button);
        self
    }

    /// Dart `SemanticsProperties(link:)`.
    pub fn link(mut self, link: bool) -> SemanticsProperties {
        self.link = Some(link);
        self
    }

    /// Dart `SemanticsProperties(header:)`.
    pub fn header(mut self, header: bool) -> SemanticsProperties {
        self.header = Some(header);
        self
    }

    /// Dart `SemanticsProperties(textField:)`.
    pub fn text_field(mut self, text_field: bool) -> SemanticsProperties {
        self.text_field = Some(text_field);
        self
    }

    /// Dart `SemanticsProperties(slider:)`.
    pub fn slider(mut self, slider: bool) -> SemanticsProperties {
        self.slider = Some(slider);
        self
    }

    /// Dart `SemanticsProperties(keyboardKey:)`.
    pub fn keyboard_key(mut self, keyboard_key: bool) -> SemanticsProperties {
        self.keyboard_key = Some(keyboard_key);
        self
    }

    /// Dart `SemanticsProperties(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> SemanticsProperties {
        self.read_only = Some(read_only);
        self
    }

    /// Dart `SemanticsProperties(focusable:)`.
    pub fn focusable(mut self, focusable: bool) -> SemanticsProperties {
        self.focusable = Some(focusable);
        self
    }

    /// Dart `SemanticsProperties(focused:)`.
    pub fn focused(mut self, focused: bool) -> SemanticsProperties {
        self.focused = Some(focused);
        self
    }

    /// Dart `SemanticsProperties(accessibilityFocusBlockType:)`.
    pub fn accessibility_focus_block_type(
        mut self,
        accessibility_focus_block_type: AccessibilityFocusBlockType,
    ) -> SemanticsProperties {
        self.accessibility_focus_block_type = Some(accessibility_focus_block_type);
        self
    }

    /// Dart `SemanticsProperties(inMutuallyExclusiveGroup:)`.
    pub fn in_mutually_exclusive_group(
        mut self,
        in_mutually_exclusive_group: bool,
    ) -> SemanticsProperties {
        self.in_mutually_exclusive_group = Some(in_mutually_exclusive_group);
        self
    }

    /// Dart `SemanticsProperties(hidden:)`.
    pub fn hidden(mut self, hidden: bool) -> SemanticsProperties {
        self.hidden = Some(hidden);
        self
    }

    /// Dart `SemanticsProperties(obscured:)`.
    pub fn obscured(mut self, obscured: bool) -> SemanticsProperties {
        self.obscured = Some(obscured);
        self
    }

    /// Dart `SemanticsProperties(multiline:)`.
    pub fn multiline(mut self, multiline: bool) -> SemanticsProperties {
        self.multiline = Some(multiline);
        self
    }

    /// Dart `SemanticsProperties(scopesRoute:)`.
    pub fn scopes_route(mut self, scopes_route: bool) -> SemanticsProperties {
        self.scopes_route = Some(scopes_route);
        self
    }

    /// Dart `SemanticsProperties(namesRoute:)`.
    pub fn names_route(mut self, names_route: bool) -> SemanticsProperties {
        self.names_route = Some(names_route);
        self
    }

    /// Dart `SemanticsProperties(image:)`.
    pub fn image(mut self, image: bool) -> SemanticsProperties {
        self.image = Some(image);
        self
    }

    /// Dart `SemanticsProperties(liveRegion:)`.
    pub fn live_region(mut self, live_region: bool) -> SemanticsProperties {
        self.live_region = Some(live_region);
        self
    }

    /// Dart `SemanticsProperties(isRequired:)`.
    pub fn is_required(mut self, is_required: bool) -> SemanticsProperties {
        self.is_required = Some(is_required);
        self
    }

    /// Dart `SemanticsProperties(maxValueLength:)`.
    pub fn max_value_length(mut self, max_value_length: i32) -> SemanticsProperties {
        self.max_value_length = Some(max_value_length);
        self
    }

    /// Dart `SemanticsProperties(currentValueLength:)`.
    pub fn current_value_length(mut self, current_value_length: i32) -> SemanticsProperties {
        self.current_value_length = Some(current_value_length);
        self
    }

    /// Dart `SemanticsProperties(identifier:)`.
    pub fn identifier(mut self, identifier: impl Into<String>) -> SemanticsProperties {
        self.identifier = Some(identifier.into());
        self
    }

    /// Dart `SemanticsProperties(traversalParentIdentifier:)`.
    pub fn traversal_parent_identifier(
        mut self,
        traversal_parent_identifier: Rc<dyn Any>,
    ) -> SemanticsProperties {
        self.traversal_parent_identifier = Some(traversal_parent_identifier);
        self
    }

    /// Dart `SemanticsProperties(traversalChildIdentifier:)`.
    pub fn traversal_child_identifier(
        mut self,
        traversal_child_identifier: Rc<dyn Any>,
    ) -> SemanticsProperties {
        self.traversal_child_identifier = Some(traversal_child_identifier);
        self
    }

    /// Dart `SemanticsProperties(label:)`.
    pub fn label(mut self, label: impl Into<String>) -> SemanticsProperties {
        self.label = Some(label.into());
        self
    }

    /// Dart `SemanticsProperties(attributedLabel:)`.
    pub fn attributed_label(mut self, attributed_label: AttributedString) -> SemanticsProperties {
        self.attributed_label = Some(attributed_label);
        self
    }

    /// Dart `SemanticsProperties(value:)`.
    pub fn value(mut self, value: impl Into<String>) -> SemanticsProperties {
        self.value = Some(value.into());
        self
    }

    /// Dart `SemanticsProperties(attributedValue:)`.
    pub fn attributed_value(mut self, attributed_value: AttributedString) -> SemanticsProperties {
        self.attributed_value = Some(attributed_value);
        self
    }

    /// Dart `SemanticsProperties(increasedValue:)`.
    pub fn increased_value(mut self, increased_value: impl Into<String>) -> SemanticsProperties {
        self.increased_value = Some(increased_value.into());
        self
    }

    /// Dart `SemanticsProperties(attributedIncreasedValue:)`.
    pub fn attributed_increased_value(
        mut self,
        attributed_increased_value: AttributedString,
    ) -> SemanticsProperties {
        self.attributed_increased_value = Some(attributed_increased_value);
        self
    }

    /// Dart `SemanticsProperties(decreasedValue:)`.
    pub fn decreased_value(mut self, decreased_value: impl Into<String>) -> SemanticsProperties {
        self.decreased_value = Some(decreased_value.into());
        self
    }

    /// Dart `SemanticsProperties(attributedDecreasedValue:)`.
    pub fn attributed_decreased_value(
        mut self,
        attributed_decreased_value: AttributedString,
    ) -> SemanticsProperties {
        self.attributed_decreased_value = Some(attributed_decreased_value);
        self
    }

    /// Dart `SemanticsProperties(hint:)`.
    pub fn hint(mut self, hint: impl Into<String>) -> SemanticsProperties {
        self.hint = Some(hint.into());
        self
    }

    /// Dart `SemanticsProperties(attributedHint:)`.
    pub fn attributed_hint(mut self, attributed_hint: AttributedString) -> SemanticsProperties {
        self.attributed_hint = Some(attributed_hint);
        self
    }

    /// Dart `SemanticsProperties(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> SemanticsProperties {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Dart `SemanticsProperties(headingLevel:)`.
    pub fn heading_level(mut self, heading_level: i32) -> SemanticsProperties {
        debug_assert!(
            heading_level > 0 && heading_level <= 6,
            "Heading level must be between 1 and 6"
        );
        self.heading_level = Some(heading_level);
        self
    }

    /// Dart `SemanticsProperties(hintOverrides:)`.
    pub fn hint_overrides(mut self, hint_overrides: SemanticsHintOverrides) -> SemanticsProperties {
        self.hint_overrides = Some(hint_overrides);
        self
    }

    /// Dart `SemanticsProperties(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> SemanticsProperties {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `SemanticsProperties(sortKey:)`.
    pub fn sort_key(mut self, sort_key: SemanticsSortKeyRef) -> SemanticsProperties {
        self.sort_key = Some(sort_key);
        self
    }

    /// Dart `SemanticsProperties(tagForChildren:)`.
    pub fn tag_for_children(mut self, tag_for_children: Rc<SemanticsTag>) -> SemanticsProperties {
        self.tag_for_children = Some(tag_for_children);
        self
    }

    /// Dart `SemanticsProperties(linkUrl:)`.
    pub fn link_url(mut self, link_url: impl Into<String>) -> SemanticsProperties {
        self.link_url = Some(link_url.into());
        self
    }

    /// Dart `SemanticsProperties(onTap:)`.
    pub fn on_tap(mut self, on_tap: VoidCallback) -> SemanticsProperties {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `SemanticsProperties(onLongPress:)`.
    pub fn on_long_press(mut self, on_long_press: VoidCallback) -> SemanticsProperties {
        self.on_long_press = Some(on_long_press);
        self
    }

    /// Dart `SemanticsProperties(onScrollLeft:)`.
    pub fn on_scroll_left(mut self, on_scroll_left: VoidCallback) -> SemanticsProperties {
        self.on_scroll_left = Some(on_scroll_left);
        self
    }

    /// Dart `SemanticsProperties(onScrollRight:)`.
    pub fn on_scroll_right(mut self, on_scroll_right: VoidCallback) -> SemanticsProperties {
        self.on_scroll_right = Some(on_scroll_right);
        self
    }

    /// Dart `SemanticsProperties(onScrollUp:)`.
    pub fn on_scroll_up(mut self, on_scroll_up: VoidCallback) -> SemanticsProperties {
        self.on_scroll_up = Some(on_scroll_up);
        self
    }

    /// Dart `SemanticsProperties(onScrollDown:)`.
    pub fn on_scroll_down(mut self, on_scroll_down: VoidCallback) -> SemanticsProperties {
        self.on_scroll_down = Some(on_scroll_down);
        self
    }

    /// Dart `SemanticsProperties(onIncrease:)`.
    pub fn on_increase(mut self, on_increase: VoidCallback) -> SemanticsProperties {
        self.on_increase = Some(on_increase);
        self
    }

    /// Dart `SemanticsProperties(onDecrease:)`.
    pub fn on_decrease(mut self, on_decrease: VoidCallback) -> SemanticsProperties {
        self.on_decrease = Some(on_decrease);
        self
    }

    /// Dart `SemanticsProperties(onCopy:)`.
    pub fn on_copy(mut self, on_copy: VoidCallback) -> SemanticsProperties {
        self.on_copy = Some(on_copy);
        self
    }

    /// Dart `SemanticsProperties(onCut:)`.
    pub fn on_cut(mut self, on_cut: VoidCallback) -> SemanticsProperties {
        self.on_cut = Some(on_cut);
        self
    }

    /// Dart `SemanticsProperties(onPaste:)`.
    pub fn on_paste(mut self, on_paste: VoidCallback) -> SemanticsProperties {
        self.on_paste = Some(on_paste);
        self
    }

    /// Dart `SemanticsProperties(onMoveCursorForwardByCharacter:)`.
    pub fn on_move_cursor_forward_by_character(
        mut self,
        on_move_cursor_forward_by_character: MoveCursorHandler,
    ) -> SemanticsProperties {
        self.on_move_cursor_forward_by_character = Some(on_move_cursor_forward_by_character);
        self
    }

    /// Dart `SemanticsProperties(onMoveCursorBackwardByCharacter:)`.
    pub fn on_move_cursor_backward_by_character(
        mut self,
        on_move_cursor_backward_by_character: MoveCursorHandler,
    ) -> SemanticsProperties {
        self.on_move_cursor_backward_by_character = Some(on_move_cursor_backward_by_character);
        self
    }

    /// Dart `SemanticsProperties(onMoveCursorForwardByWord:)`.
    pub fn on_move_cursor_forward_by_word(
        mut self,
        on_move_cursor_forward_by_word: MoveCursorHandler,
    ) -> SemanticsProperties {
        self.on_move_cursor_forward_by_word = Some(on_move_cursor_forward_by_word);
        self
    }

    /// Dart `SemanticsProperties(onMoveCursorBackwardByWord:)`.
    pub fn on_move_cursor_backward_by_word(
        mut self,
        on_move_cursor_backward_by_word: MoveCursorHandler,
    ) -> SemanticsProperties {
        self.on_move_cursor_backward_by_word = Some(on_move_cursor_backward_by_word);
        self
    }

    /// Dart `SemanticsProperties(onSetSelection:)`.
    pub fn on_set_selection(
        mut self,
        on_set_selection: SetSelectionHandler,
    ) -> SemanticsProperties {
        self.on_set_selection = Some(on_set_selection);
        self
    }

    /// Dart `SemanticsProperties(onSetText:)`.
    pub fn on_set_text(mut self, on_set_text: SetTextHandler) -> SemanticsProperties {
        self.on_set_text = Some(on_set_text);
        self
    }

    /// Dart `SemanticsProperties(onDidGainAccessibilityFocus:)`.
    pub fn on_did_gain_accessibility_focus(
        mut self,
        on_did_gain_accessibility_focus: VoidCallback,
    ) -> SemanticsProperties {
        self.on_did_gain_accessibility_focus = Some(on_did_gain_accessibility_focus);
        self
    }

    /// Dart `SemanticsProperties(onDidLoseAccessibilityFocus:)`.
    pub fn on_did_lose_accessibility_focus(
        mut self,
        on_did_lose_accessibility_focus: VoidCallback,
    ) -> SemanticsProperties {
        self.on_did_lose_accessibility_focus = Some(on_did_lose_accessibility_focus);
        self
    }

    /// Dart `SemanticsProperties(onFocus:)`.
    pub fn on_focus(mut self, on_focus: VoidCallback) -> SemanticsProperties {
        self.on_focus = Some(on_focus);
        self
    }

    /// Dart `SemanticsProperties(onDismiss:)`.
    pub fn on_dismiss(mut self, on_dismiss: VoidCallback) -> SemanticsProperties {
        self.on_dismiss = Some(on_dismiss);
        self
    }

    /// Dart `SemanticsProperties(onExpand:)`.
    pub fn on_expand(mut self, on_expand: VoidCallback) -> SemanticsProperties {
        self.on_expand = Some(on_expand);
        self
    }

    /// Dart `SemanticsProperties(onCollapse:)`.
    pub fn on_collapse(mut self, on_collapse: VoidCallback) -> SemanticsProperties {
        self.on_collapse = Some(on_collapse);
        self
    }

    /// Dart `SemanticsProperties(customSemanticsActions:)`.
    pub fn custom_semantics_actions(
        mut self,
        custom_semantics_actions: IndexMap<CustomSemanticsAction, VoidCallback>,
    ) -> SemanticsProperties {
        self.custom_semantics_actions = Some(custom_semantics_actions);
        self
    }

    /// Dart `SemanticsProperties(role:)`.
    pub fn role(mut self, role: SemanticsRole) -> SemanticsProperties {
        self.role = Some(role);
        self
    }

    /// Dart `SemanticsProperties(controlsNodes:)`.
    pub fn controls_nodes(mut self, controls_nodes: IndexSet<String>) -> SemanticsProperties {
        self.controls_nodes = Some(controls_nodes);
        self
    }

    /// Dart `SemanticsProperties(validationResult:)`.
    pub fn validation_result(
        mut self,
        validation_result: SemanticsValidationResult,
    ) -> SemanticsProperties {
        self.validation_result = validation_result;
        self
    }

    /// Dart `SemanticsProperties(hitTestBehavior:)`.
    pub fn hit_test_behavior(
        mut self,
        hit_test_behavior: SemanticsHitTestBehavior,
    ) -> SemanticsProperties {
        self.hit_test_behavior = Some(hit_test_behavior);
        self
    }

    /// Dart `SemanticsProperties(inputType:)`.
    pub fn input_type(mut self, input_type: SemanticsInputType) -> SemanticsProperties {
        self.input_type = Some(input_type);
        self
    }

    /// Dart `SemanticsProperties(maxValue:)`.
    pub fn max_value(mut self, max_value: impl Into<String>) -> SemanticsProperties {
        self.max_value = Some(max_value.into());
        self
    }

    /// Dart `SemanticsProperties(minValue:)`.
    pub fn min_value(mut self, min_value: impl Into<String>) -> SemanticsProperties {
        self.min_value = Some(min_value.into());
        self
    }

    /// Dart's constructor asserts that relate two fields. The widgets built on these properties
    /// run it from their `debug_check_constructor`; an assert on one field stays in its setter.
    pub fn debug_check_constructor(&self) {
        debug_assert!(
            self.label.is_none() || self.attributed_label.is_none(),
            "Only one of label or attributedLabel should be provided"
        );
        debug_assert!(
            self.value.is_none() || self.attributed_value.is_none(),
            "Only one of value or attributedValue should be provided"
        );
        debug_assert!(
            self.increased_value.is_none() || self.attributed_increased_value.is_none(),
            "Only one of increasedValue or attributedIncreasedValue should be provided"
        );
        debug_assert!(
            self.decreased_value.is_none() || self.attributed_decreased_value.is_none(),
            "Only one of decreasedValue or attributedDecreasedValue should be provided"
        );
        debug_assert!(
            self.hint.is_none() || self.attributed_hint.is_none(),
            "Only one of hint or attributedHint should be provided"
        );
        debug_assert!(
            self.link_url.is_none() || self.link.unwrap_or(false),
            "If linkUrl is set then link must be true"
        );
    }
}

/// Dart's `debugFillProperties`, as it lists them.
impl Debug for SemanticsProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticsProperties")
            .field("checked", &self.checked)
            .field("mixed", &self.mixed)
            .field("expanded", &self.expanded)
            .field("selected", &self.selected)
            .field("isRequired", &self.is_required)
            .field("identifier", &self.identifier)
            .field("label", &self.label)
            .field("attributedLabel", &self.attributed_label)
            .field("value", &self.value)
            .field("attributedValue", &self.attributed_value)
            .field("increasedValue", &self.value)
            .field("attributedIncreasedValue", &self.attributed_increased_value)
            .field("decreasedValue", &self.value)
            .field("attributedDecreasedValue", &self.attributed_decreased_value)
            .field("hint", &self.hint)
            .field("attributedHint", &self.attributed_hint)
            .field("tooltip", &self.tooltip)
            .field("textDirection", &self.text_direction)
            .field("role", &self.role)
            .field("validationResult", &self.validation_result)
            .field("sortKey", &self.sort_key)
            .field("hintOverrides", &self.hint_overrides)
            .finish_non_exhaustive()
    }
}

/// Base class for all sort keys for [`SemanticsProperties::sort_key`] accessibility
/// traversal order sorting.
///
/// Sort keys are sorted by [`name`](Self::name), then by the comparison that the subclass
/// implements. If [`SemanticsProperties::sort_key`] is specified, sort keys within
/// the same semantic group must all be of the same type.
///
/// Keys with no [`name`](Self::name) are compared to other keys with no
/// [`name`](Self::name), and will be traversed before those with a [`name`](Self::name).
///
/// If no sort key is applied to a semantics node, then it will be ordered using
/// a platform dependent default algorithm.
pub trait SemanticsSortKey: Debug + Any {
    /// An optional name that will group this sort key with other sort keys of the
    /// same [`name`](Self::name).
    ///
    /// Sort keys must have the same type when compared.
    fn name(&self) -> Option<&str>;

    /// The implementation of [`compare_to`](dyn SemanticsSortKey::compare_to).
    ///
    /// The argument is guaranteed to be of the same type as this object and have
    /// the same [`name`](Self::name).
    ///
    /// The method should return [`Ordering::Less`] if this object comes earlier in
    /// the sort order than the argument; and [`Ordering::Greater`] if it comes later
    /// in the sort order. Returning [`Ordering::Equal`] causes the system to use default sort
    /// order.
    fn do_compare(&self, other: &dyn SemanticsSortKey) -> Ordering;
}

impl dyn SemanticsSortKey {
    /// Dart's `compareTo`: by name first and then the subclass's ordering.
    pub fn compare_to(&self, other: &dyn SemanticsSortKey) -> Ordering {
        debug_assert!(
            (self as &dyn Any).type_id() == (other as &dyn Any).type_id(),
            "Semantics sort keys can only be compared to other sort keys of the same type."
        );

        // Defer to the subclass implementation for ordering only if the names are
        // identical (or both null).
        if self.name() == other.name() {
            return self.do_compare(other);
        }

        // Keys that don't have a name are sorted together and come before those with
        // a name.
        match (self.name(), other.name()) {
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(name), Some(other_name)) => name.cmp(other_name),
            (None, None) => unreachable!("equal names compared above"),
        }
    }
}

/// A shared [`SemanticsSortKey`].
pub type SemanticsSortKeyRef = Rc<dyn SemanticsSortKey>;

/// A [`SemanticsSortKey`] that sorts based on the `f64` value it is
/// given.
///
/// The [`OrdinalSortKey`] compares itself with other [`OrdinalSortKey`]s
/// to sort based on the order it is given.
///
/// [`OrdinalSortKey`]s are sorted by the optional [`name`](SemanticsSortKey::name), then by
/// their [`order`](Self::order). If [`SemanticsProperties::sort_key`] is a [`OrdinalSortKey`],
/// then all the other specified sort keys in the same semantics group must also be
/// [`OrdinalSortKey`]s.
///
/// The ordinal value [`order`](Self::order) is typically a whole number, though it can be
/// fractional, e.g. in order to fit between two other consecutive whole
/// numbers. The value must be finite (it cannot be NaN or infinite).
#[derive(Clone, Debug)]
pub struct OrdinalSortKey {
    name: Option<String>,

    /// Determines the placement of this key in a sequence of keys that defines
    /// the order in which this node is traversed by the platform's accessibility
    /// services.
    ///
    /// Lower values will be traversed first. Keys with the same name will be
    /// grouped together and sorted by name first, and then sorted by order.
    pub order: f64,
}

impl OrdinalSortKey {
    /// Creates a semantics sort key that uses a `f64` as its key value.
    ///
    /// The `order` must be a finite number.
    pub fn new(order: f64) -> OrdinalSortKey {
        debug_assert!(order > f64::NEG_INFINITY);
        debug_assert!(order < f64::INFINITY);
        OrdinalSortKey { name: None, order }
    }

    /// Dart `OrdinalSortKey(name:)`.
    pub fn name(mut self, name: impl Into<String>) -> OrdinalSortKey {
        self.name = Some(name.into());
        self
    }
}

impl SemanticsSortKey for OrdinalSortKey {
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn do_compare(&self, other: &dyn SemanticsSortKey) -> Ordering {
        let other = (other as &dyn Any)
            .downcast_ref::<OrdinalSortKey>()
            .expect("compare_to checks the type");
        if other.order == self.order {
            return Ordering::Equal;
        }
        self.order.total_cmp(&other.order)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Locale, LocaleStringAttribute, SpellOutStringAttribute};

    use super::*;

    fn key(order: f64) -> SemanticsSortKeyRef {
        Rc::new(OrdinalSortKey::new(order))
    }

    fn named_key(order: f64, name: &str) -> SemanticsSortKeyRef {
        Rc::new(OrdinalSortKey::new(order).name(name))
    }

    // semantics_test.dart: 'OrdinalSortKey compares correctly when names are the same'
    #[test]
    fn ordinal_sort_keys_with_the_same_name_compare_by_order() {
        let tests = [
            (key(0.0), key(0.0), Ordering::Equal),
            (key(0.0), key(1.0), Ordering::Less),
            (key(1.0), key(0.0), Ordering::Greater),
            (key(1.0), key(1.0), Ordering::Equal),
            (named_key(0.0, "a"), named_key(0.0, "a"), Ordering::Equal),
            (named_key(0.0, "a"), named_key(1.0, "a"), Ordering::Less),
            (named_key(1.0, "a"), named_key(0.0, "a"), Ordering::Greater),
            (named_key(1.0, "a"), named_key(1.0, "a"), Ordering::Equal),
        ];
        for (a, b, expected) in tests {
            assert_eq!(a.compare_to(b.as_ref()), expected, "{a:?} vs {b:?}");
        }
    }

    // semantics_test.dart: 'OrdinalSortKey compares correctly when the names are different'
    #[test]
    fn ordinal_sort_keys_with_different_names_compare_by_name() {
        let tests = [
            (key(0.0), named_key(0.0, "bar"), Ordering::Less),
            (key(0.0), named_key(1.0, "bar"), Ordering::Less),
            (key(1.0), named_key(0.0, "bar"), Ordering::Less),
            (key(1.0), named_key(1.0, "bar"), Ordering::Less),
            (named_key(0.0, "foo"), key(0.0), Ordering::Greater),
            (named_key(0.0, "foo"), key(1.0), Ordering::Greater),
            (named_key(1.0, "foo"), key(0.0), Ordering::Greater),
            (named_key(1.0, "foo"), key(1.0), Ordering::Greater),
            (
                named_key(0.0, "foo"),
                named_key(0.0, "bar"),
                Ordering::Greater,
            ),
            (
                named_key(0.0, "foo"),
                named_key(1.0, "bar"),
                Ordering::Greater,
            ),
            (
                named_key(1.0, "foo"),
                named_key(0.0, "bar"),
                Ordering::Greater,
            ),
            (
                named_key(1.0, "foo"),
                named_key(1.0, "bar"),
                Ordering::Greater,
            ),
            (named_key(0.0, "bar"), named_key(0.0, "foo"), Ordering::Less),
            (named_key(0.0, "bar"), named_key(1.0, "foo"), Ordering::Less),
            (named_key(1.0, "bar"), named_key(0.0, "foo"), Ordering::Less),
            (named_key(1.0, "bar"), named_key(1.0, "foo"), Ordering::Less),
        ];
        for (a, b, expected) in tests {
            assert_eq!(a.compare_to(b.as_ref()), expected, "{a:?} vs {b:?}");
        }
    }

    #[derive(Debug)]
    struct CustomSortKey(f64);

    impl SemanticsSortKey for CustomSortKey {
        fn name(&self) -> Option<&str> {
            None
        }

        fn do_compare(&self, other: &dyn SemanticsSortKey) -> Ordering {
            let other = (other as &dyn Any).downcast_ref::<CustomSortKey>().unwrap();
            self.0.total_cmp(&other.0)
        }
    }

    // semantics_test.dart: 'Incompatible OrdinalSortKey throw AssertionError when compared'
    #[test]
    #[should_panic(expected = "Semantics sort keys can only be compared")]
    fn comparing_sort_keys_of_different_types_asserts() {
        let _ = key(0.0).compare_to(&CustomSortKey(0.0));
    }

    // semantics_test.dart: 'Attributed String can concat'
    #[test]
    fn concatenating_attributed_strings_shifts_the_attributes_of_the_second() {
        let string1 = AttributedString::new("string1")
            .attributes(vec![SpellOutStringAttribute::new(TextRange::new(0, 4))]);
        let string2 =
            AttributedString::new("string2").attributes(vec![LocaleStringAttribute::new(
                TextRange::new(0, 4),
                Locale::new("es").country_code("MX"),
            )]);
        let result = string1 + string2;
        assert_eq!(result.string, "string1string2");
        assert_eq!(result.attributes.len(), 2);
        assert_eq!(result.attributes[0].range(), TextRange::new(0, 4));
        assert!(
            (result.attributes[0].as_ref() as &dyn Any)
                .downcast_ref::<SpellOutStringAttribute>()
                .is_some()
        );
        assert_eq!(
            format!("{:?}", result.attributes[1]),
            "LocaleStringAttribute(TextRange { start: 7, end: 11 }, es-MX)"
        );
    }

    #[test]
    fn attributed_strings_are_equal_only_with_the_same_attributes() {
        let attribute = SpellOutStringAttribute::new(TextRange::new(0, 1));
        let a = AttributedString::new("a").attributes(vec![attribute.clone()]);
        let b = AttributedString::new("a").attributes(vec![attribute]);
        let c = AttributedString::new("a")
            .attributes(vec![SpellOutStringAttribute::new(TextRange::new(0, 1))]);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(AttributedString::new("a"), AttributedString::new("a"));
    }

    #[test]
    #[should_panic(expected = "Only one of label or attributedLabel should be provided")]
    fn a_label_and_an_attributed_label_together_assert() {
        SemanticsProperties::new()
            .label("a")
            .attributed_label(AttributedString::new("a"))
            .debug_check_constructor();
    }

    #[test]
    #[should_panic(expected = "If linkUrl is set then link must be true")]
    fn a_link_url_without_link_asserts() {
        SemanticsProperties::new()
            .link_url("https://flutter.dev")
            .debug_check_constructor();
    }

    #[test]
    #[should_panic(expected = "Heading level must be between 1 and 6")]
    fn a_heading_level_out_of_range_asserts() {
        let _ = SemanticsProperties::new().heading_level(7);
    }
}
