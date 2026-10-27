//! MenuFlyoutItemBase's item family expressed as immutable widget configuration.

use inset_foundation::{App, Listener};
use inset_widgets::{IntoWidget, KeyRef, WidgetRef};
use std::{fmt, rc::Rc};

/// Requested checked value after activating a toggle menu item.
pub type MenuCheckedChanged = Rc<dyn Fn(&mut App, bool)>;

/// The source item classes share text, icon and enabled properties.
#[derive(Clone)]
pub enum MenuFlyoutItemKind {
    /// MenuFlyoutItem invokes its Click callback.
    Item,

    /// ToggleMenuFlyoutItem requests the opposite checked value before Click.
    Toggle {
        /// Checked state supplied by the owner.
        is_checked: bool,

        /// Updates the owner's checked value.
        changed: MenuCheckedChanged,
    },

    /// RadioMenuFlyoutItem can request selection but cannot uncheck itself.
    Radio {
        /// Whether this option is selected.
        is_checked: bool,

        /// Updates the owner's selected option, as with RadioButton.
        checked: Listener,
    },

    /// MenuFlyoutSubItem opens a cascading collection.
    SubItem {
        /// Items displayed in the child menu.
        items: Vec<MenuFlyoutItem>,
    },

    /// MenuFlyoutSeparator contributes no focus stop or activation.
    Separator,
}

impl fmt::Debug for MenuFlyoutItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Item => f.write_str("Item"),
            Self::Toggle { is_checked, .. } => f.debug_tuple("Toggle").field(is_checked).finish(),
            Self::Radio { is_checked, .. } => f.debug_tuple("Radio").field(is_checked).finish(),
            Self::SubItem { items } => f.debug_tuple("SubItem").field(items).finish(),
            Self::Separator => f.write_str("Separator"),
        }
    }
}

/// Configuration for one member of the MenuFlyoutItemBase family.
#[derive(Clone)]
pub struct MenuFlyoutItem {
    /// Text presented in the item's main column.
    pub text: String,

    /// Optional icon widget fitted into the shared icon column.
    pub icon: Option<WidgetRef>,

    /// Whether this item can receive focus or be invoked.
    pub is_enabled: bool,

    /// Explicit shortcut label in the right-hand column.
    pub keyboard_accelerator_text_override: String,

    /// Called after the checked-value callback and before closing the root menu.
    pub click: Option<Listener>,

    /// Leaves the menu open after invocation, following PreventDismissOnPointer.
    pub prevent_dismiss_on_pointer: bool,

    /// Chooses the source item behavior and template.
    pub kind: MenuFlyoutItemKind,

    /// Identity used to retain this item's state when the collection changes.
    pub key: Option<KeyRef>,
}

impl fmt::Debug for MenuFlyoutItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuFlyoutItem")
            .field("text", &self.text)
            .field("kind", &self.kind)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl MenuFlyoutItem {
    /// Creates an ordinary command item.
    pub fn new(text: impl Into<String>, click: Listener) -> Self {
        Self {
            text: text.into(),
            icon: None,
            is_enabled: true,
            keyboard_accelerator_text_override: String::new(),
            click: Some(click),
            prevent_dismiss_on_pointer: false,
            kind: MenuFlyoutItemKind::Item,
            key: None,
        }
    }

    /// Creates the ToggleMenuFlyoutItem variant with an owner-supplied value.
    pub fn toggle(
        text: impl Into<String>,
        is_checked: bool,
        changed: impl Fn(&mut App, bool) + 'static,
    ) -> Self {
        let mut item = Self::new(text, Listener::new(|_| {}));
        item.click = None;
        item.kind = MenuFlyoutItemKind::Toggle {
            is_checked,
            changed: Rc::new(changed),
        };
        item
    }

    /// Creates the RadioMenuFlyoutItem variant; the owner updates the selected option.
    pub fn radio(text: impl Into<String>, is_checked: bool, checked: Listener) -> Self {
        let mut item = Self::new(text, Listener::new(|_| {}));
        item.click = None;
        item.kind = MenuFlyoutItemKind::Radio {
            is_checked,
            checked,
        };
        item
    }

    /// Creates the MenuFlyoutSubItem variant.
    pub fn sub_item(text: impl Into<String>, items: Vec<MenuFlyoutItem>) -> Self {
        let mut item = Self::new(text, Listener::new(|_| {}));
        item.click = None;
        item.kind = MenuFlyoutItemKind::SubItem { items };
        item
    }

    /// Creates a non-interactive MenuFlyoutSeparator.
    pub fn separator() -> Self {
        let mut item = Self::new("", Listener::new(|_| {}));
        item.click = None;
        item.kind = MenuFlyoutItemKind::Separator;
        item
    }

    /// Sets the widget displayed by the template’s icon presenter.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> Self {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Enables or disables activation and focus.
    pub fn is_enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }

    /// Sets the displayed shortcut label; key bindings are supplied separately.
    pub fn keyboard_accelerator_text_override(mut self, text: impl Into<String>) -> Self {
        self.keyboard_accelerator_text_override = text.into();
        self
    }

    /// Sets Click for checked variants as well as ordinary items.
    pub fn click(mut self, click: Listener) -> Self {
        self.click = Some(click);
        self
    }

    /// Keeps the root menu open after invocation.
    pub fn prevent_dismiss_on_pointer(mut self, value: bool) -> Self {
        self.prevent_dismiss_on_pointer = value;
        self
    }

    /// Sets collection identity independently of item position.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}
