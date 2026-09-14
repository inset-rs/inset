//! Flutter counterpart: `SystemContextMenuController` and
//! `IOSSystemContextMenuItemData*` in `services/text_input.dart`.

use std::collections::HashMap;
use std::fmt::{self, Debug};

use inset_embedder::{Rect, SystemContextMenuItem};
use inset_foundation::{App, Handle, Listener};

use crate::text_input::TextInput;

/// Describes a context menu button that will be rendered in the system context
/// menu.
///
/// Dart's sealed `IOSSystemContextMenuItemData` subclasses are the variants.
#[derive(Clone, Debug)]
pub enum IOSSystemContextMenuItemData {
    /// The system's built-in copy button.
    Copy,
    /// The system's built-in cut button.
    Cut,
    /// The system's built-in paste button.
    Paste,
    /// The system's built-in select-all button.
    SelectAll,
    /// The system's built-in look-up button.
    LookUp {
        /// Localized title.
        title: String,
    },
    /// The system's built-in search-web button.
    SearchWeb {
        /// Localized title.
        title: String,
    },
    /// The system's built-in share button.
    Share {
        /// Localized title.
        title: String,
    },
    /// The system's built-in Live Text button.
    LiveText,
    /// A developer-defined button.
    Custom {
        /// Title shown in the menu.
        title: String,
        /// Callback when the item is selected.
        on_pressed: Listener,
        /// Id sent to the host and back on custom action.
        callback_id: String,
    },
}

impl IOSSystemContextMenuItemData {
    /// The text to display to the user.
    ///
    /// Not exposed for some built-in menu items whose title is always set by the
    /// platform.
    pub fn title(&self) -> Option<&str> {
        match self {
            Self::LookUp { title }
            | Self::SearchWeb { title }
            | Self::Share { title }
            | Self::Custom { title, .. } => Some(title.as_str()),
            _ => None,
        }
    }

    fn to_host(&self) -> SystemContextMenuItem {
        match self {
            Self::Copy => SystemContextMenuItem::Copy,
            Self::Cut => SystemContextMenuItem::Cut,
            Self::Paste => SystemContextMenuItem::Paste,
            Self::SelectAll => SystemContextMenuItem::SelectAll,
            Self::LookUp { title } => SystemContextMenuItem::LookUp {
                title: title.clone(),
            },
            Self::SearchWeb { title } => SystemContextMenuItem::SearchWeb {
                title: title.clone(),
            },
            Self::Share { title } => SystemContextMenuItem::Share {
                title: title.clone(),
            },
            Self::LiveText => SystemContextMenuItem::LiveText,
            Self::Custom {
                title, callback_id, ..
            } => SystemContextMenuItem::Custom {
                id: callback_id.clone(),
                title: title.clone(),
            },
        }
    }
}

impl PartialEq for IOSSystemContextMenuItemData {
    fn eq(&self, other: &IOSSystemContextMenuItemData) -> bool {
        match (self, other) {
            (Self::Copy, Self::Copy)
            | (Self::Cut, Self::Cut)
            | (Self::Paste, Self::Paste)
            | (Self::SelectAll, Self::SelectAll)
            | (Self::LiveText, Self::LiveText) => true,
            (Self::LookUp { title: a }, Self::LookUp { title: b })
            | (Self::SearchWeb { title: a }, Self::SearchWeb { title: b })
            | (Self::Share { title: a }, Self::Share { title: b }) => a == b,
            (
                Self::Custom {
                    title: a,
                    on_pressed: a_cb,
                    callback_id: a_id,
                },
                Self::Custom {
                    title: b,
                    on_pressed: b_cb,
                    callback_id: b_id,
                },
            ) => a == b && a_cb == b_cb && a_id == b_id,
            _ => false,
        }
    }
}

/// Dart's `SystemContextMenuController._lastShown` and
/// `ServicesBinding.systemContextMenuClient`.
#[derive(Default)]
struct SystemContextMenuRegistry {
    last_shown: Option<Handle<SystemContextMenuController>>,
}

impl SystemContextMenuRegistry {
    fn instance(app: &mut App) -> Handle<SystemContextMenuRegistry> {
        app.singleton::<SystemContextMenuRegistry>()
    }
}

/// Allows access to the system context menu.
///
/// The context menu is the menu that appears, for example, when doing text
/// selection. Flutter typically draws this menu itself, but on some platforms
/// this class calls a platform method to show the system-drawn context menu
/// instead.
///
/// There can only be one system context menu visible at a time. Calling [show]
/// on one instance of this class while another instance is already shown will
/// hide the old one.
///
/// Call [dispose] when no longer needed.
pub struct SystemContextMenuController {
    /// Called when the system has hidden the context menu.
    pub on_system_hide: Option<Listener>,
    last_target_rect: Option<Rect>,
    last_items: Option<Vec<IOSSystemContextMenuItemData>>,
    hidden_by_system: bool,
    disposed: bool,
    custom_action_callbacks: HashMap<String, Listener>,
}

impl Debug for SystemContextMenuController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemContextMenuController")
            .field("hidden_by_system", &self.hidden_by_system)
            .field("disposed", &self.disposed)
            .finish_non_exhaustive()
    }
}

impl SystemContextMenuController {
    /// Creates an instance of [`SystemContextMenuController`].
    ///
    /// Not shown until [`show`](Self::show) is called.
    pub fn new(app: &mut App, on_system_hide: Option<Listener>) -> Handle<Self> {
        app.create(SystemContextMenuController {
            on_system_hide,
            last_target_rect: None,
            last_items: None,
            hidden_by_system: false,
            disposed: false,
            custom_action_callbacks: HashMap::new(),
        })
    }

    fn is_visible(self: Handle<Self>, app: &mut App) -> bool {
        let registry = SystemContextMenuRegistry::instance(app);
        app.get(registry).last_shown == Some(self) && !app.get(self).hidden_by_system
    }

    /// Called when the system has hidden the context menu.
    pub fn handle_system_hide(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).disposed);
        if app.get(self).disposed || !self.is_visible(app) {
            return;
        }
        let registry = SystemContextMenuRegistry::instance(app);
        if app.get(registry).last_shown == Some(self) {
            app.get_mut(registry).last_shown = None;
        }
        app.get_mut(self).hidden_by_system = true;
        app.get_mut(self).custom_action_callbacks.clear();
        if let Some(on_hide) = app.get(self).on_system_hide.clone() {
            on_hide.call(app);
        }
    }

    /// Called when the user selects a custom menu item.
    pub fn handle_custom_context_menu_action(self: Handle<Self>, app: &mut App, callback_id: &str) {
        let callback = app
            .get(self)
            .custom_action_callbacks
            .get(callback_id)
            .cloned();
        debug_assert!(
            callback.is_some(),
            "Custom action callback not found for id: {callback_id}. \
             This may indicate that the menu item was not properly registered."
        );
        if let Some(callback) = callback {
            callback.call(app);
        }
    }

    /// Shows the system context menu anchored on the given [`Rect`].
    ///
    /// Currently this system context menu is bound to text input. Using this
    /// without an active `TextInputConnection` will be a noop.
    #[deprecated(note = "Use `show_with_items` instead.")]
    pub fn show(self: Handle<Self>, app: &mut App, target_rect: Rect) {
        debug_assert!(!app.get(self).disposed);
        debug_assert!(
            TextInput::instance(app).current_connection(app).is_some(),
            "Currently, the system context menu can only be shown for an active text input connection"
        );
        if self.is_visible(app) && app.get(self).last_target_rect == Some(target_rect) {
            return;
        }
        debug_assert!(
            {
                let registry = SystemContextMenuRegistry::instance(app);
                let last = app.get(registry).last_shown;
                last.is_none()
                    || last == Some(self)
                    || last.is_some_and(|last| !last.is_visible(app))
            },
            "Attempted to show while another instance was still visible."
        );
        let registry = SystemContextMenuRegistry::instance(app);
        app.get_mut(self).last_target_rect = Some(target_rect);
        app.get_mut(registry).last_shown = Some(self);
        app.get_mut(self).hidden_by_system = false;
        if let Some(menu) = app.platform().system_context_menu() {
            menu.show(target_rect, None);
        }
    }

    /// Shows the system context menu anchored on the given [`Rect`] with the given
    /// buttons.
    pub fn show_with_items(
        self: Handle<Self>,
        app: &mut App,
        target_rect: Rect,
        items: Vec<IOSSystemContextMenuItemData>,
    ) {
        debug_assert!(!app.get(self).disposed);
        debug_assert!(!items.is_empty());
        debug_assert!(
            TextInput::instance(app).current_connection(app).is_some(),
            "Currently, the system context menu can only be shown for an active text input connection"
        );
        if self.is_visible(app)
            && app.get(self).last_target_rect == Some(target_rect)
            && app.get(self).last_items.as_ref() == Some(&items)
        {
            return;
        }
        debug_assert!(
            {
                let registry = SystemContextMenuRegistry::instance(app);
                let last = app.get(registry).last_shown;
                last.is_none()
                    || last == Some(self)
                    || last.is_some_and(|last| !last.is_visible(app))
            },
            "Attempted to show while another instance was still visible."
        );

        app.get_mut(self).custom_action_callbacks.clear();
        for item in &items {
            if let IOSSystemContextMenuItemData::Custom {
                callback_id,
                on_pressed,
                ..
            } = item
            {
                app.get_mut(self)
                    .custom_action_callbacks
                    .insert(callback_id.clone(), on_pressed.clone());
            }
        }
        let host_items: Vec<SystemContextMenuItem> =
            items.iter().map(|item| item.to_host()).collect();
        app.get_mut(self).last_target_rect = Some(target_rect);
        app.get_mut(self).last_items = Some(items);
        let registry = SystemContextMenuRegistry::instance(app);
        app.get_mut(registry).last_shown = Some(self);
        app.get_mut(self).hidden_by_system = false;
        if let Some(menu) = app.platform().system_context_menu() {
            menu.show(target_rect, Some(&host_items));
        }
    }

    /// Hides this system context menu.
    pub fn hide(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).disposed);
        let registry = SystemContextMenuRegistry::instance(app);
        if app.get(registry).last_shown != Some(self) {
            return;
        }
        app.get_mut(registry).last_shown = None;
        app.get_mut(self).custom_action_callbacks.clear();
        if let Some(menu) = app.platform().system_context_menu() {
            menu.hide();
        }
    }

    /// Used to release resources when this instance will never be used again.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).disposed);
        self.hide(app);
        app.get_mut(self).disposed = true;
    }

    /// Dispatch a system-hide from the host to the currently shown controller.
    pub fn dispatch_system_hide(app: &mut App) {
        let registry = SystemContextMenuRegistry::instance(app);
        if let Some(shown) = app.get(registry).last_shown {
            shown.handle_system_hide(app);
        }
    }

    /// Dispatch a custom action from the host to the currently shown controller.
    pub fn dispatch_custom_action(app: &mut App, callback_id: &str) {
        let registry = SystemContextMenuRegistry::instance(app);
        if let Some(shown) = app.get(registry).last_shown {
            shown.handle_custom_context_menu_action(app, callback_id);
        }
    }
}
