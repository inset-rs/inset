//! Flutter counterpart: `widgets/context_menu_button_item.dart`.

use std::fmt::{self, Debug};

use inset_foundation::Listener;

/// The buttons that can appear in a context menu by default.
///
/// See also:
///
///  * [`ContextMenuButtonItem`], which uses this enum to describe a button in a
///    context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextMenuButtonType {
    /// A button that cuts the current text selection.
    Cut,
    /// A button that copies the current text selection.
    Copy,
    /// A button that pastes the clipboard contents into the focused text field.
    Paste,
    /// A button that selects all the contents of the focused text field.
    SelectAll,
    /// A button that deletes the current text selection.
    Delete,
    /// A button that looks up the current text selection.
    LookUp,
    /// A button that launches a web search for the current text selection.
    SearchWeb,
    /// A button that displays the share screen for the current text selection.
    Share,
    /// A button for starting Live Text input.
    LiveTextInput,
    /// Anything other than the default button types.
    Custom,
}

/// The type and callback for a context menu button.
///
/// See also:
///
///  * [`crate::SystemContextMenu`], which can take system items instead.
#[derive(Clone)]
pub struct ContextMenuButtonItem {
    /// The callback to be called when the button is pressed.
    pub on_pressed: Option<Listener>,
    /// The type of button this represents.
    pub r#type: ContextMenuButtonType,
    /// The label to display on the button.
    ///
    /// If a [`r#type`](Self::r#type) other than [`ContextMenuButtonType::Custom`] is given
    /// and a label is not provided, then the default label for that type for the
    /// platform will be looked up.
    pub label: Option<String>,
}

impl ContextMenuButtonItem {
    /// Creates a [`ContextMenuButtonItem`].
    pub fn new(on_pressed: Option<Listener>) -> ContextMenuButtonItem {
        ContextMenuButtonItem {
            on_pressed,
            r#type: ContextMenuButtonType::Custom,
            label: None,
        }
    }

    /// Dart `ContextMenuButtonItem(type:)`.
    pub fn r#type(mut self, r#type: ContextMenuButtonType) -> ContextMenuButtonItem {
        self.r#type = r#type;
        self
    }

    /// Dart `ContextMenuButtonItem(label:)`.
    pub fn label(mut self, label: impl Into<String>) -> ContextMenuButtonItem {
        self.label = Some(label.into());
        self
    }

    /// Creates a new [`ContextMenuButtonItem`] with the provided parameters
    /// overridden.
    pub fn copy_with(&self) -> ContextMenuButtonItem {
        self.clone()
    }
}

impl PartialEq for ContextMenuButtonItem {
    fn eq(&self, other: &ContextMenuButtonItem) -> bool {
        self.label == other.label
            && self.on_pressed == other.on_pressed
            && self.r#type == other.r#type
    }
}

impl Debug for ContextMenuButtonItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ContextMenuButtonItem {:?}, {:?}",
            self.r#type, self.label
        )
    }
}
