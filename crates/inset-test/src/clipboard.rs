//! A clipboard a test fills and reads back: the `MockClipboard` of Flutter's widget tests
//! (`test/widgets/clipboard_utils.dart`), as a host's [`Clipboard`] rather than a handler of
//! platform-channel calls.

use std::cell::RefCell;

use inset_embedder::{
    Clipboard, ClipboardHasStringsFuture, ClipboardSetTextFuture, ClipboardTextFuture,
};

/// A clipboard that holds its text in memory and answers every call at once.
#[derive(Default)]
pub struct MockClipboard {
    clipboard_data: RefCell<Option<String>>,
}

impl MockClipboard {
    /// An empty clipboard.
    pub fn new() -> MockClipboard {
        MockClipboard::default()
    }

    /// The text on the clipboard, which the framework's last write left or the test set.
    pub fn clipboard_data(&self) -> Option<String> {
        self.clipboard_data.borrow().clone()
    }

    /// Puts `text` on the clipboard, or empties it with `None`.
    pub fn set_clipboard_data(&self, text: Option<String>) {
        *self.clipboard_data.borrow_mut() = text;
    }
}

impl Clipboard for MockClipboard {
    fn set_text(&self, text: &str) -> ClipboardSetTextFuture {
        self.set_clipboard_data(Some(text.to_owned()));
        Box::pin(std::future::ready(()))
    }

    fn text(&self) -> ClipboardTextFuture {
        Box::pin(std::future::ready(self.clipboard_data()))
    }

    fn has_strings(&self) -> ClipboardHasStringsFuture {
        let has_strings = self
            .clipboard_data
            .borrow()
            .as_deref()
            .is_some_and(|text| !text.is_empty());
        Box::pin(std::future::ready(has_strings))
    }
}
