//! Flutter counterpart: `services/clipboard.dart`.

use inset_foundation::{App, Task};

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
    pub fn set_data(app: &App, data: ClipboardData) -> Task<()> {
        let written = data.text.as_deref().and_then(|text| {
            app.platform()
                .clipboard()
                .map(|clipboard| clipboard.set_text(text))
        });
        match written {
            Some(written) => app.spawn(async move |_cx| written.await),
            None => Task::ready(()),
        }
    }

    /// Retrieves data from the clipboard that matches the given format.
    ///
    /// The `format` argument specifies the media type, such as `text/plain`, of
    /// the data to obtain.
    ///
    /// Completes with [`None`] if the data could not be obtained.
    pub fn get_data(app: &App, format: &str) -> Task<Option<ClipboardData>> {
        let text = app
            .platform()
            .clipboard()
            .filter(|_| format == Self::K_TEXT_PLAIN)
            .map(|clipboard| clipboard.text());
        match text {
            Some(text) => app.spawn(async move |_cx| text.await.map(ClipboardData::new)),
            None => Task::ready(None),
        }
    }

    /// Completes with true if (and only if) the clipboard contains string data.
    pub fn has_strings(app: &App) -> Task<bool> {
        let has_strings = app
            .platform()
            .clipboard()
            .map(|clipboard| clipboard.has_strings());
        match has_strings {
            Some(has_strings) => app.spawn(async move |_cx| has_strings.await),
            None => Task::ready(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use inset_foundation::AppCell;
    use inset_test::{MockClipboard, TestPlatform};

    use super::*;

    fn app_with(clipboard: &Rc<MockClipboard>) -> Rc<AppCell> {
        let clipboard: Rc<dyn inset_embedder::Clipboard> = Rc::clone(clipboard) as _;
        AppCell::with_platform(Rc::new(TestPlatform::new().with_clipboard(clipboard)))
    }

    /// Runs the checkpoint `task` needs; its answer, or `None` while it is still on its way.
    fn awaited<T: 'static>(cell: &Rc<AppCell>, task: Task<T>) -> Option<T> {
        let answer = Rc::new(RefCell::new(None));
        let out = Rc::clone(&answer);
        cell.borrow()
            .spawn(async move |_cx| *out.borrow_mut() = Some(task.await));
        cell.checkpoint();
        answer.take()
    }

    // clipboard_test.dart 'Clipboard.getData returns text'.
    #[test]
    fn get_data_returns_text() {
        let mock_clipboard = Rc::new(MockClipboard::new());
        let cell = app_with(&mock_clipboard);
        mock_clipboard.set_clipboard_data(Some("Hello world".to_owned()));

        let data = Clipboard::get_data(&cell.borrow(), Clipboard::K_TEXT_PLAIN);
        let data = awaited(&cell, data).expect("the read completed");

        assert_eq!(data, Some(ClipboardData::new("Hello world")));
    }

    // clipboard_test.dart 'Clipboard.getData returns null'.
    #[test]
    fn get_data_returns_none() {
        let mock_clipboard = Rc::new(MockClipboard::new());
        let cell = app_with(&mock_clipboard);
        mock_clipboard.set_clipboard_data(None);

        let data = Clipboard::get_data(&cell.borrow(), Clipboard::K_TEXT_PLAIN);
        let data = awaited(&cell, data).expect("the read completed");

        assert_eq!(data, None);
    }

    // clipboard_test.dart 'Clipboard.setData sets text'.
    #[test]
    fn set_data_sets_text() {
        let mock_clipboard = Rc::new(MockClipboard::new());
        let cell = app_with(&mock_clipboard);

        let written = Clipboard::set_data(&cell.borrow(), ClipboardData::new("Hello world"));
        awaited(&cell, written).expect("the write completed");

        assert_eq!(
            mock_clipboard.clipboard_data().as_deref(),
            Some("Hello world")
        );
    }
}
