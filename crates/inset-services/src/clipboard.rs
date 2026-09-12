//! Flutter counterpart: `services/clipboard.dart`.

use inset_foundation::App;

/// Data stored on the system clipboard.
///
/// The system clipboard can contain data of various media types. This data
/// structure currently supports only plain text data, in the [`text`](Self::text) property.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardData {
    /// Plain text variant of this clipboard data.
    ///
    /// This is nullable as other clipboard data variants, like images, may be
    /// added in the future. Currently, plain text is the only supported variant
    /// and this is guaranteed to be non-null.
    pub text: Option<String>,
}

impl ClipboardData {
    /// Creates data for the system clipboard.
    pub fn new(text: impl Into<String>) -> ClipboardData {
        ClipboardData {
            text: Some(text.into()),
        }
    }
}

/// Utility methods for interacting with the system's clipboard.
pub struct Clipboard;

impl Clipboard {
    /// Plain text data format string.
    ///
    /// Used with [`get_data`](Self::get_data).
    pub const K_TEXT_PLAIN: &'static str = "text/plain";

    /// Stores the given clipboard data on the clipboard.
    pub fn set_data(app: &App, data: ClipboardData) {
        if let Some(text) = data.text.as_deref() {
            app.platform().clipboard_set_data(text);
        }
    }

    /// Retrieves data from the clipboard that matches the given format.
    ///
    /// The `format` argument specifies the media type, such as `text/plain`, of
    /// the data to obtain.
    ///
    /// Returns [`None`] if the data could not be obtained.
    pub fn get_data(app: &App, format: &str) -> Option<ClipboardData> {
        if format != Self::K_TEXT_PLAIN {
            return None;
        }
        app.platform().clipboard_get_data().map(ClipboardData::new)
    }

    /// Returns true if (and only if) the clipboard contains string data.
    pub fn has_strings(app: &App) -> bool {
        app.platform().clipboard_has_strings()
    }
}
