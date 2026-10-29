//! The general pasteboard as the framework's clipboard: AppKit's `NSPasteboard`, whose items are
//! each offered in several types, which AppKit names by UTI and the framework by MIME type
//! (`types`); a form produced when someone pastes it, and a file that exists only once pasted
//! (`promises`); a file another application promised, received when it is read
//! (`received_files`); and the options a copy is treated with. What is dragged over a window is
//! read from the dragging pasteboard as the general one is read (`dragging`).
//!
//! What a copy may hold follows AppKit's items: each item an `NSPasteboardItem` with a form for
//! each type, but an item carrying a file, given or promised, which is that file alone, as
//! Finder's are: a file is an `NSURL`, which the system lets a sandboxed receiver open, and a
//! promised file an `NSFilePromiseProvider`. A sensitive copy is marked with the
//! `org.nspasteboard.ConcealedType` the clipboard managers of macOS honour; one for this device
//! only is written for the current host only; and one that expires is cleared at its time if the
//! pasteboard has not changed since, as macOS has no expiry of its own. The time is kept by a timer
//! on the app's main dispatch queue, which dies with the app: a copy whose app quits before its
//! time stays on the pasteboard.
//!
//! The invariant: an item read is the pasteboard's as listed. Listing notes the pasteboard's count
//! of changes, and a read once it has moved fails, since AppKit answers a stale item with nothing.

mod dragging;
mod promises;
mod received_files;
mod waiting;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use dispatch2::{DispatchQueue, DispatchTime, MainThreadBound};
use inset_embedder::{
    Clipboard, ClipboardError, ClipboardItem, ClipboardItemsFuture, ClipboardReadFuture,
    ClipboardWriteFuture, Content, CopyOptions, DataTransferItem, URI_LIST,
};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardContentsOptions, NSPasteboardItem, NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSData, NSInteger, NSString, NSURL};

use super::AppKit;
use crate::clipboard::ClipboardChanges;
use crate::os::SystemClipboard;
use crate::os::apple::pasteboard_types as types;
use promises::{PromisedFile, PromisedForms};

/// The type macOS's clipboard managers leave out of their history, previews and sync.
const CONCEALED: &str = "org.nspasteboard.ConcealedType";

impl SystemClipboard for AppKit {
    type Clipboard = Pasteboard;

    /// The general pasteboard as it is at launch, which the app hears no change of.
    fn clipboard(cleared: Box<dyn Fn()>) -> Pasteboard {
        Pasteboard::new(NSPasteboard::generalPasteboard(), cleared)
    }
}

/// A pasteboard, the general one but in tests, as this app reaches it, on the app's thread.
pub(crate) struct Pasteboard {
    pasteboard: Retained<NSPasteboard>,
    /// The pasteboard's count of changes when it was last asked whether it changed, or the app
    /// last wrote it, whichever came last.
    seen: Cell<NSInteger>,
    /// What produces the forms the app's copy promised, kept until the app writes again: the
    /// pasteboard keeps a promised file's delegate only weakly.
    promises: RefCell<Vec<Retained<NSObject>>>,
    /// Tells the main loop a copy of the app's expired, and was cleared.
    cleared: Rc<dyn Fn()>,
}

impl Clipboard for Pasteboard {
    /// The pasteboard's items, listed in the call, with no content read.
    fn items(&self) -> ClipboardItemsFuture {
        Box::pin(std::future::ready(listed_items(&self.pasteboard)))
    }

    /// Written in the call, since the pasteboard takes the items at once; a form promised is
    /// asked for later, by whoever pastes it.
    fn write(&self, items: Vec<ClipboardItem>, options: CopyOptions) -> ClipboardWriteFuture {
        let written = self.write_now(items, options);
        Box::pin(std::future::ready(written))
    }
}

impl ClipboardChanges for Pasteboard {
    fn has_changed(&self) -> bool {
        let now = self.pasteboard.changeCount();
        self.seen.replace(now) != now
    }
}

impl Pasteboard {
    /// `pasteboard` as it is now, which the app hears no change of; `cleared` tells the main
    /// loop a copy of the app's expired, and was cleared.
    fn new(pasteboard: Retained<NSPasteboard>, cleared: Box<dyn Fn()>) -> Pasteboard {
        Pasteboard {
            seen: Cell::new(pasteboard.changeCount()),
            pasteboard,
            promises: RefCell::default(),
            cleared: Rc::from(cleared),
        }
    }

    /// Replaces the pasteboard's contents with `items`, treated as `options` says: a change the app
    /// made, so it counts as seen.
    fn write_now(
        &self,
        items: Vec<ClipboardItem>,
        options: CopyOptions,
    ) -> Result<(), ClipboardError> {
        let mut promises = Vec::new();
        let writers = items
            .into_iter()
            .map(|item| writer_of(item, options, &mut promises))
            .collect::<Result<Vec<_>, _>>()?;
        let pasteboard = &self.pasteboard;
        pasteboard.prepareForNewContentsWithOptions(contents_options(options));
        if !pasteboard.writeObjects(&NSArray::from_retained_slice(&writers)) {
            return Err(ClipboardError::Failed(
                "the pasteboard did not take the items".to_owned(),
            ));
        }
        *self.promises.borrow_mut() = promises;
        self.seen.set(pasteboard.changeCount());
        if let Some(expiry) = options.expires_after {
            self.clear_after(expiry, pasteboard.changeCount());
        }
        Ok(())
    }

    /// Clears the pasteboard after `expiry` if it holds the copy that left it at `change_count`.
    fn clear_after(&self, expiry: Duration, change_count: NSInteger) {
        let Some(main_thread) = MainThreadMarker::new() else {
            return;
        };
        let Ok(when) = DispatchTime::try_from(expiry) else {
            return;
        };
        let expiring = MainThreadBound::new(
            (self.pasteboard.clone(), Rc::clone(&self.cleared)),
            main_thread,
        );
        let _ = DispatchQueue::main().after(when, move || {
            let Some(main_thread) = MainThreadMarker::new() else {
                return;
            };
            let (pasteboard, cleared) = expiring.get(main_thread);
            if pasteboard.changeCount() == change_count {
                pasteboard.clearContents();
                cleared();
            }
        });
    }
}

/// How the pasteboard is prepared for a copy treated as `options` says.
fn contents_options(options: CopyOptions) -> NSPasteboardContentsOptions {
    if options.this_device_only {
        NSPasteboardContentsOptions::CurrentHostOnly
    } else {
        NSPasteboardContentsOptions::empty()
    }
}

/// What writes `item` to the pasteboard: a file's `NSURL` or promise for an item carrying a file,
/// an `NSPasteboardItem` otherwise. What produces its promised forms joins `promises`.
fn writer_of(
    item: ClipboardItem,
    options: CopyOptions,
    promises: &mut Vec<Retained<NSObject>>,
) -> Result<Retained<ProtocolObject<dyn NSPasteboardWriting>>, ClipboardError> {
    let carries_a_file = item
        .representations
        .iter()
        .any(|(_, content)| matches!(content, Content::File(_) | Content::FilePromise { .. }));
    if carries_a_file {
        file_writer(item, promises)
    } else {
        let written = pasteboard_item(item, options, promises)?;
        Ok(ProtocolObject::from_retained(written))
    }
}

/// The writer of an item carrying a file, which must be the item's one form. A promised file is
/// AppKit's to ask for on the main thread, so it is written from there alone.
fn file_writer(
    item: ClipboardItem,
    promises: &mut Vec<Retained<NSObject>>,
) -> Result<Retained<ProtocolObject<dyn NSPasteboardWriting>>, ClipboardError> {
    let mut representations = item.representations.into_iter();
    let (Some((mime, content)), None) = (representations.next(), representations.next()) else {
        return Err(ClipboardError::Unsupported);
    };
    match content {
        Content::File(path) => {
            let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
            Ok(ProtocolObject::from_retained(url))
        }
        Content::FilePromise { name, produce } => {
            let main_thread = MainThreadMarker::new().ok_or_else(|| {
                ClipboardError::Failed("a promised file is written off the main thread".to_owned())
            })?;
            let uti = types::uti_of(&mime).ok_or(ClipboardError::Unsupported)?;
            let (provider, delegate) = PromisedFile::new(main_thread, &uti, name, produce);
            promises.push(Retained::into_super(delegate));
            Ok(ProtocolObject::from_retained(provider))
        }
        Content::Data(_) | Content::Promise(_) => Err(ClipboardError::Unsupported),
    }
}

/// An `NSPasteboardItem` with each form of `item`: given ones set, promised ones set on a data
/// provider, which joins `promises`.
fn pasteboard_item(
    item: ClipboardItem,
    options: CopyOptions,
    promises: &mut Vec<Retained<NSObject>>,
) -> Result<Retained<NSPasteboardItem>, ClipboardError> {
    let written = NSPasteboardItem::new();
    let mut producers = Vec::new();
    for (mime, content) in item.representations {
        let uti = types::uti_of(&mime).ok_or(ClipboardError::Unsupported)?;
        match content {
            Content::Data(bytes) => {
                written.setData_forType(&NSData::with_bytes(&bytes), &uti);
            }
            Content::Promise(produce) => producers.push((uti, produce)),
            Content::File(_) | Content::FilePromise { .. } => {
                return Err(ClipboardError::Unsupported);
            }
        }
    }
    if !producers.is_empty() {
        let provider = PromisedForms::new(producers);
        let types = NSArray::from_retained_slice(&provider.types());
        written.setDataProvider_forTypes(provider.as_provider(), &types);
        promises.push(Retained::into_super(provider));
    }
    if options.sensitive {
        written.setData_forType(&NSData::new(), &NSString::from_str(CONCEALED));
    }
    Ok(written)
}

/// `pasteboard`'s items now, each in the types it is offered in: a promised file as
/// `text/uri-list`. An item with no type the framework can name is left out.
fn listed_items(pasteboard: &Retained<NSPasteboard>) -> Vec<Box<dyn DataTransferItem>> {
    let change_count = pasteboard.changeCount();
    let Some(items) = pasteboard.pasteboardItems() else {
        return Vec::new();
    };
    let mut promised_files = 0;
    let mut listed: Vec<Box<dyn DataTransferItem>> = Vec::new();
    for item in items.iter() {
        let promise = received_files::is_promised_file(&item).then(|| {
            promised_files += 1;
            promised_files - 1
        });
        let listed_item = ListedItem::new(pasteboard.clone(), item, change_count, promise);
        if !listed_item.types.is_empty() {
            listed.push(Box::new(listed_item));
        }
    }
    listed
}

/// One item of a pasteboard as it was listed.
struct ListedItem {
    pasteboard: Retained<NSPasteboard>,
    item: Retained<NSPasteboardItem>,
    change_count: NSInteger,
    /// The MIME types it is offered in, each with the UTI it is read as; none for a promised
    /// file's list, which is received instead.
    types: Vec<String>,
    utis: Vec<Option<Retained<NSString>>>,
    /// Which of the pasteboard's file promises the item is, if it is one.
    promised_file: Option<usize>,
}

impl ListedItem {
    fn new(
        pasteboard: Retained<NSPasteboard>,
        item: Retained<NSPasteboardItem>,
        change_count: NSInteger,
        promised_file: Option<usize>,
    ) -> ListedItem {
        let mut listed = ListedItem {
            pasteboard,
            item,
            change_count,
            types: Vec::new(),
            utis: Vec::new(),
            promised_file,
        };
        for uti in listed.item.types().iter() {
            if let Some(mime) = types::mime_of(&uti) {
                listed.offer(mime, Some(uti));
            }
        }
        if promised_file.is_some() {
            listed.offer(URI_LIST.to_owned(), None);
        }
        listed
    }

    /// Offers the item in `mime`, read as `uti`, unless a form before already offers it.
    fn offer(&mut self, mime: String, uti: Option<Retained<NSString>>) {
        if !self.types.contains(&mime) {
            self.types.push(mime);
            self.utis.push(uti);
        }
    }

    /// The form read as `uti`: a URL as a line of a `text/uri-list`.
    fn read_form(&self, uti: &NSString) -> Result<Vec<u8>, ClipboardError> {
        let not_given = || ClipboardError::Failed("the pasteboard gave nothing".to_owned());
        if types::is_url(uti) {
            let url = self.item.stringForType(uti).ok_or_else(not_given)?;
            Ok(format!("{url}\r\n").into_bytes())
        } else {
            let data = self.item.dataForType(uti).ok_or_else(not_given)?;
            Ok(data.to_vec())
        }
    }
}

impl DataTransferItem for ListedItem {
    fn types(&self) -> &[String] {
        &self.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        if self.pasteboard.changeCount() != self.change_count {
            return Box::pin(std::future::ready(Err(ClipboardError::Changed)));
        }
        let Some(position) = self.types.iter().position(|offered| offered == mime) else {
            return Box::pin(std::future::ready(Err(ClipboardError::NotFound)));
        };
        match (&self.utis[position], self.promised_file) {
            (Some(uti), _) => Box::pin(std::future::ready(self.read_form(uti))),
            (None, Some(index)) => received_files::receive(&self.pasteboard, index),
            (None, None) => Box::pin(std::future::ready(Err(ClipboardError::NotFound))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    use super::*;

    /// A pasteboard of the test's own, which no other application reads.
    fn private_pasteboard() -> Pasteboard {
        Pasteboard::new(NSPasteboard::pasteboardWithUniqueName(), Box::new(|| {}))
    }

    /// `future`'s output: everything here answers in the call.
    fn now<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("the pasteboard answers in the call"),
        }
    }

    fn data(mime: &str, bytes: &[u8]) -> (String, Content) {
        (mime.to_owned(), Content::Data(bytes.to_vec()))
    }

    fn item(representations: Vec<(String, Content)>) -> ClipboardItem {
        ClipboardItem { representations }
    }

    #[test]
    fn items_are_written_and_read_in_their_types() {
        let pasteboard = private_pasteboard();
        let first = item(vec![
            data("text/html", b"<b>hi</b>"),
            data("text/plain", b"hi"),
        ]);
        let second = item(vec![data("image/png", b"\x89PNG")]);
        now(pasteboard.write(vec![first, second], CopyOptions::new())).unwrap();

        let items = now(pasteboard.items());
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].types(), ["text/html", "text/plain"]);
        assert_eq!(now(items[0].read("text/plain")), Ok(b"hi".to_vec()));
        assert_eq!(now(items[0].read("text/html")), Ok(b"<b>hi</b>".to_vec()));
        assert_eq!(now(items[1].read("image/png")), Ok(b"\x89PNG".to_vec()));
        assert_eq!(
            now(items[1].read("text/plain")),
            Err(ClipboardError::NotFound)
        );
    }

    #[test]
    fn a_read_after_another_write_fails() {
        let pasteboard = private_pasteboard();
        now(pasteboard.write(
            vec![item(vec![data("text/plain", b"first")])],
            CopyOptions::new(),
        ))
        .unwrap();
        let items = now(pasteboard.items());
        now(pasteboard.write(
            vec![item(vec![data("text/plain", b"second")])],
            CopyOptions::new(),
        ))
        .unwrap();
        assert_eq!(
            now(items[0].read("text/plain")),
            Err(ClipboardError::Changed)
        );
    }

    #[test]
    fn a_promised_form_is_produced_when_it_is_read_and_not_before() {
        let pasteboard = private_pasteboard();
        let produced = Rc::new(Cell::new(0));
        let counter = Rc::clone(&produced);
        let promise = Content::Promise(Rc::new(move || {
            counter.set(counter.get() + 1);
            Box::pin(std::future::ready(Some(b"made".to_vec())))
        }));
        now(pasteboard.write(
            vec![item(vec![("text/plain".to_owned(), promise)])],
            CopyOptions::new(),
        ))
        .unwrap();
        assert_eq!(produced.get(), 0);

        let items = now(pasteboard.items());
        assert_eq!(items[0].types(), ["text/plain"]);
        assert_eq!(now(items[0].read("text/plain")), Ok(b"made".to_vec()));
        assert_eq!(produced.get(), 1);
    }

    #[test]
    fn a_sensitive_copy_is_marked_concealed_for_the_clipboard_managers() {
        let pasteboard = private_pasteboard();
        let password = item(vec![data("text/plain", b"hunter2")]);
        now(pasteboard.write(vec![password], CopyOptions::new().sensitive(true))).unwrap();

        let types = pasteboard.pasteboard.types().unwrap();
        assert!(types.iter().any(|uti| uti.to_string() == CONCEALED));
        let items = now(pasteboard.items());
        assert_eq!(
            items[0].types(),
            ["text/plain"],
            "the mark is no form of the item"
        );
    }

    #[test]
    fn a_file_is_written_as_its_url_and_read_as_a_uri_list() {
        let pasteboard = private_pasteboard();
        let file = Content::File(PathBuf::from("/tmp/a b.png"));
        now(pasteboard.write(
            vec![item(vec![("image/png".to_owned(), file)])],
            CopyOptions::new(),
        ))
        .unwrap();

        let items = now(pasteboard.items());
        assert_eq!(items[0].types(), [URI_LIST]);
        assert_eq!(
            now(items[0].read(URI_LIST)),
            Ok(b"file:///tmp/a%20b.png\r\n".to_vec())
        );
    }

    #[test]
    fn an_item_carrying_a_file_and_another_form_is_refused() {
        let pasteboard = private_pasteboard();
        let file = (
            "image/png".to_owned(),
            Content::File(PathBuf::from("/tmp/a.png")),
        );
        let written = now(pasteboard.write(
            vec![item(vec![file, data("text/plain", b"a.png")])],
            CopyOptions::new(),
        ));
        assert_eq!(written, Err(ClipboardError::Unsupported));
    }

    #[test]
    fn another_writers_change_is_seen_once_and_the_apps_own_counts_as_seen() {
        let pasteboard = private_pasteboard();
        assert!(
            !pasteboard.has_changed(),
            "the pasteboard as it was at the start"
        );
        now(pasteboard.write(
            vec![item(vec![data("text/plain", b"x")])],
            CopyOptions::new(),
        ))
        .unwrap();
        assert!(!pasteboard.has_changed(), "the app made it");
        pasteboard.pasteboard.clearContents();
        assert!(pasteboard.has_changed(), "another writer's");
        assert!(!pasteboard.has_changed());
    }
}
