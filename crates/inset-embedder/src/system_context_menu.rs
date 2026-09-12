//! Flutter counterpart: the `ContextMenu.showSystemContextMenu` /
//! `hideSystemContextMenu` payloads on `SystemChannels.platform`.
//!
//! Lives here because [`crate::Platform`] names these types.

/// A button to show in the system-rendered context menu.
///
/// Flutter serializes this as the `items` list of `ContextMenu.showSystemContextMenu`.
/// Built-in kinds have no callback: the platform performs the action (on iOS, against
/// the active text input). [`Custom`](Self::Custom) is invoked through
/// [`crate::EmbedderClient::custom_context_menu_action`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemContextMenuItem {
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
    /// The system's built-in Live Text (OCR) button.
    LiveText,
    /// A developer-defined button.
    Custom {
        /// Id the host sends back on [`crate::EmbedderClient::custom_context_menu_action`].
        id: String,
        /// Title shown in the menu.
        title: String,
    },
}
