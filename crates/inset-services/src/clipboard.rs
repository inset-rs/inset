//! Flutter counterpart: `services/clipboard.dart`.

use std::future::Future;

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

    /// Stores the given clipboard data on the clipboard, without waiting for it to be stored.
    pub fn set_data(app: &App, data: ClipboardData) {
        let Some(text) = data.text.as_deref() else {
            return;
        };
        if let Some(clipboard) = app.platform().clipboard() {
            clipboard.set_text(text);
        }
    }

    /// Retrieves data from the clipboard that matches the given format.
    ///
    /// The `format` argument specifies the media type, such as `text/plain`, of
    /// the data to obtain.
    ///
    /// Completes with [`None`] if the data could not be obtained.
    pub fn get_data(
        app: &App,
        format: &str,
    ) -> impl Future<Output = Option<ClipboardData>> + use<> {
        let text = app
            .platform()
            .clipboard()
            .filter(|_| format == Self::K_TEXT_PLAIN)
            .map(|clipboard| clipboard.text());
        async move { text?.await.map(ClipboardData::new) }
    }

    /// Completes with true if (and only if) the clipboard contains string data.
    pub fn has_strings(app: &App) -> impl Future<Output = bool> + use<> {
        let has_strings = app
            .platform()
            .clipboard()
            .map(|clipboard| clipboard.has_strings());
        async move {
            match has_strings {
                Some(has_strings) => has_strings.await,
                None => false,
            }
        }
    }
}
