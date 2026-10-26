//! Flutter counterpart: `services/clipboard.dart`.
//!
//! Flutter's text clipboard, as a layer over the host's full one ([`inset_embedder::Clipboard`]):
//! it writes one item in `text/plain`, and reads the first item offered in it. While the
//! framework handles a clipboard event of the host's ([`Clipboard::handle_event`]), it writes
//! through the event's copy request and reads what the user pasted, in place of the host's
//! clipboard.

use std::rc::Rc;

use inset_embedder::{
    ClipboardEvent, ClipboardItem, ClipboardItemsFuture, ClipboardWriteFuture, Content,
    CopyOptions, PastedItem,
};
use inset_foundation::{App, Handle, Task};

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
    pub fn set_data(app: &mut App, data: ClipboardData) -> Task<()> {
        let Some(text) = data.text else {
            return Task::ready(());
        };
        let item = ClipboardItem {
            representations: vec![(
                Self::K_TEXT_PLAIN.to_owned(),
                Content::Data(text.into_bytes()),
            )],
        };
        match Self::write(app, vec![item]) {
            Some(written) => app.spawn(async move |_cx| {
                // Flutter's `setData` completes whether or not the host took the text.
                let _refused = written.await;
            }),
            None => Task::ready(()),
        }
    }

    /// Retrieves data from the clipboard that matches the given format.
    ///
    /// The `format` argument specifies the media type, such as `text/plain`, of
    /// the data to obtain.
    ///
    /// Completes with [`None`] if the data could not be obtained.
    pub fn get_data(app: &mut App, format: &str) -> Task<Option<ClipboardData>> {
        if format != Self::K_TEXT_PLAIN {
            return Task::ready(None);
        }
        match Self::readable(app) {
            Some(readable) => {
                app.spawn(async move |_cx| readable.text().await.map(ClipboardData::new))
            }
            None => Task::ready(None),
        }
    }

    /// Completes with true if (and only if) the clipboard contains string data.
    pub fn has_strings(app: &mut App) -> Task<bool> {
        match Self::readable(app) {
            Some(readable) => app.spawn(async move |_cx| readable.holds_text().await),
            None => Task::ready(false),
        }
    }

    /// Runs `handle` with `event` as the clipboard: while it runs, [`set_data`](Self::set_data)
    /// writes through a copy's or a cut's request, and [`get_data`](Self::get_data) and
    /// [`has_strings`](Self::has_strings) read what a paste pasted. What `handle` starts may
    /// complete later; what is started after it returns reaches the host's clipboard.
    ///
    /// No Flutter counterpart: Flutter's engines paste into a text field through its text input
    /// connection, and reach the clipboard only through the platform channel.
    pub fn handle_event<R>(
        app: &mut App,
        event: ClipboardEvent,
        handle: impl FnOnce(&mut App) -> R,
    ) -> R {
        let in_hand = Self::event_in_hand(app);
        let outer = app.get_mut(in_hand).0.replace(Rc::new(event));
        let result = handle(app);
        app.get_mut(in_hand).0 = outer;
        result
    }

    /// Replaces the clipboard with `items`: through the request of the copy or cut being
    /// handled, or the host's clipboard. `None` where the host has none.
    fn write(app: &mut App, items: Vec<ClipboardItem>) -> Option<ClipboardWriteFuture> {
        let in_hand = Self::event_in_hand(app);
        match app.get(in_hand).0.as_deref() {
            Some(ClipboardEvent::Copy(request) | ClipboardEvent::Cut(request)) => {
                Some(request.write(items, CopyOptions::new()))
            }
            Some(ClipboardEvent::Paste(_)) | None => app
                .platform()
                .clipboard()
                .map(|clipboard| clipboard.write(items, CopyOptions::new())),
        }
    }

    /// What a read reads: the paste being handled, or the host's clipboard. `None` where the
    /// host has none.
    fn readable(app: &mut App) -> Option<Readable> {
        let in_hand = Self::event_in_hand(app);
        match app.get(in_hand).0.clone() {
            Some(event) if matches!(*event, ClipboardEvent::Paste(_)) => {
                Some(Readable::Pasted(event))
            }
            _ => app
                .platform()
                .clipboard()
                .map(|clipboard| Readable::Listing(clipboard.items())),
        }
    }

    fn event_in_hand(app: &mut App) -> Handle<EventInHand> {
        app.singleton()
    }
}

/// The host's clipboard event the framework is handling, if it is handling one.
#[derive(Default)]
struct EventInHand(Option<Rc<ClipboardEvent>>);

/// What a read of the clipboard reads.
enum Readable {
    /// What the user pasted: a paste event.
    Pasted(Rc<ClipboardEvent>),
    /// What is on the host's clipboard, being listed.
    Listing(ClipboardItemsFuture),
}

impl Readable {
    /// The text of the first item offered as text; none when no item is, or the text could not
    /// be read.
    async fn text(self) -> Option<String> {
        match self {
            Readable::Pasted(paste) => text_of(pasted_items(&paste)).await,
            Readable::Listing(items) => text_of(&items.await).await,
        }
    }

    /// Whether any item is offered as text.
    async fn holds_text(self) -> bool {
        match self {
            Readable::Pasted(paste) => holds_text(pasted_items(&paste)),
            Readable::Listing(items) => holds_text(&items.await),
        }
    }
}

/// The items of `paste`, a paste event.
fn pasted_items(paste: &ClipboardEvent) -> &[Box<dyn PastedItem>] {
    match paste {
        ClipboardEvent::Paste(items) => items,
        ClipboardEvent::Copy(_) | ClipboardEvent::Cut(_) => &[],
    }
}

fn offers_text(item: &dyn PastedItem) -> bool {
    item.types()
        .iter()
        .any(|mime| mime == Clipboard::K_TEXT_PLAIN)
}

fn holds_text(items: &[Box<dyn PastedItem>]) -> bool {
    items.iter().any(|item| offers_text(item.as_ref()))
}

async fn text_of(items: &[Box<dyn PastedItem>]) -> Option<String> {
    let item = items.iter().find(|item| offers_text(item.as_ref()))?;
    let bytes = item.read(Clipboard::K_TEXT_PLAIN).await.ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    use inset_embedder::{Clipboard as _, CopyRequest};
    use inset_foundation::AppCell;
    use inset_test::{MockClipboard, TestPlatform};

    use super::*;

    fn app_with(clipboard: &MockClipboard) -> Rc<AppCell> {
        let clipboard: Rc<dyn inset_embedder::Clipboard> = Rc::new(clipboard.clone());
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

    /// What is on `clipboard`, which a mock lists at once.
    fn listed(clipboard: &MockClipboard) -> Vec<Box<dyn PastedItem>> {
        let mut items = clipboard.items();
        match items.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(items) => items,
            Poll::Pending => panic!("a mock clipboard lists at once"),
        }
    }

    // clipboard_test.dart 'Clipboard.getData returns text'.
    #[test]
    fn get_data_returns_text() {
        let mock_clipboard = MockClipboard::new();
        let cell = app_with(&mock_clipboard);
        mock_clipboard.set_clipboard_data(Some("Hello world".to_owned()));

        let data = Clipboard::get_data(&mut cell.borrow_mut(), Clipboard::K_TEXT_PLAIN);
        let data = awaited(&cell, data).expect("the read completed");

        assert_eq!(data, Some(ClipboardData::new("Hello world")));
    }

    // clipboard_test.dart 'Clipboard.getData returns null'.
    #[test]
    fn get_data_returns_none() {
        let mock_clipboard = MockClipboard::new();
        let cell = app_with(&mock_clipboard);
        mock_clipboard.set_clipboard_data(None);

        let data = Clipboard::get_data(&mut cell.borrow_mut(), Clipboard::K_TEXT_PLAIN);
        let data = awaited(&cell, data).expect("the read completed");

        assert_eq!(data, None);
    }

    // clipboard_test.dart 'Clipboard.setData sets text'.
    #[test]
    fn set_data_sets_text() {
        let mock_clipboard = MockClipboard::new();
        let cell = app_with(&mock_clipboard);

        let written =
            Clipboard::set_data(&mut cell.borrow_mut(), ClipboardData::new("Hello world"));
        awaited(&cell, written).expect("the write completed");

        assert_eq!(
            mock_clipboard.clipboard_data().as_deref(),
            Some("Hello world")
        );
    }

    #[test]
    fn the_text_is_the_first_item_offered_as_text() {
        let mock_clipboard = MockClipboard::new();
        let cell = app_with(&mock_clipboard);
        let png = ClipboardItem {
            representations: vec![("image/png".to_owned(), Content::Data(vec![0x89]))],
        };
        let text = ClipboardItem {
            representations: vec![("text/plain".to_owned(), Content::Data(b"second".to_vec()))],
        };
        drop(mock_clipboard.write(vec![png, text], CopyOptions::new()));

        let has_strings = Clipboard::has_strings(&mut cell.borrow_mut());
        assert_eq!(awaited(&cell, has_strings), Some(true));
        let data = Clipboard::get_data(&mut cell.borrow_mut(), Clipboard::K_TEXT_PLAIN);
        assert_eq!(
            awaited(&cell, data).expect("the read completed"),
            Some(ClipboardData::new("second"))
        );
    }

    #[test]
    fn a_paste_being_handled_is_read_in_place_of_the_clipboard() {
        let on_clipboard = MockClipboard::new();
        on_clipboard.set_clipboard_data(Some("on the clipboard".to_owned()));
        let pasted = MockClipboard::new();
        pasted.set_clipboard_data(Some("pasted".to_owned()));
        let cell = app_with(&on_clipboard);

        let data = Clipboard::handle_event(
            &mut cell.borrow_mut(),
            ClipboardEvent::Paste(listed(&pasted)),
            |app| Clipboard::get_data(app, Clipboard::K_TEXT_PLAIN),
        );
        assert_eq!(
            awaited(&cell, data).expect("the read completed"),
            Some(ClipboardData::new("pasted"))
        );

        let after = Clipboard::get_data(&mut cell.borrow_mut(), Clipboard::K_TEXT_PLAIN);
        assert_eq!(
            awaited(&cell, after).expect("the read completed"),
            Some(ClipboardData::new("on the clipboard")),
            "a read after the paste reads the clipboard"
        );
    }

    #[test]
    fn a_copy_being_handled_writes_through_its_request() {
        let on_clipboard = MockClipboard::new();
        let copied = MockClipboard::new();
        let cell = app_with(&on_clipboard);
        let into = copied.clone();
        let request = CopyRequest::new(move |items, options| into.write(items, options));

        let written = Clipboard::handle_event(
            &mut cell.borrow_mut(),
            ClipboardEvent::Copy(request),
            |app| Clipboard::set_data(app, ClipboardData::new("copied")),
        );
        awaited(&cell, written).expect("the write completed");

        assert_eq!(copied.clipboard_data().as_deref(), Some("copied"));
        assert_eq!(on_clipboard.clipboard_data(), None);
    }
}
