//! The system clipboard: what is on it, as items each offered in several forms named by MIME
//! types, and writing it. Not a dart:ui file: Flutter's clipboard is plain text over a platform
//! channel (`services/clipboard.dart`), which `inset-services` keeps as a layer over this one.
//!
//! The shape is the one Apple's, Android's and the web's clipboards share: one clipboard holding
//! one or more items, each with one or more forms keyed by a type. A reader lists the items and
//! their types without reading any content ([`Clipboard::items`]), then reads one item in one
//! form ([`DataTransferItem::read`]); a writer gives each form's content at once or promises it
//! ([`Content`]), and files travel as references. Where a system holds one item, as Windows and
//! Linux do, its host writes the first.
//!
//! The invariant: a read never lands on another copy. An item is a handle on the clipboard as it
//! was listed, and its read fails with [`ClipboardError::Changed`] once anything has replaced
//! it, as the web's `ClipboardItem` and a stale `NSPasteboardItem` do.

use std::fmt;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

/// The type a pasted file is read in: its `file:` URL, one line per file (RFC 2483), as every
/// system carries a file on its clipboard as a reference.
pub const URI_LIST: &str = "text/uri-list";

/// The system clipboard. Every call answers later, as a host may have to ask its user, or
/// another process, first.
pub trait Clipboard {
    /// What is on the clipboard now: each item, to read in one of its types. Empty when the
    /// clipboard is, and where the host may not look.
    fn items(&self) -> ClipboardItemsFuture;

    /// Replaces the clipboard with `items`, each in its forms, treated as `options` says.
    /// Completes once the system holds them: a form promised is produced later, each time
    /// someone pastes it.
    fn write(&self, items: Vec<ClipboardItem>, options: CopyOptions) -> ClipboardWriteFuture;
}

/// What is on the clipboard, on its way.
pub type ClipboardItemsFuture = Pin<Box<dyn Future<Output = Vec<Box<dyn DataTransferItem>>>>>;

/// A write of the clipboard, on its way.
pub type ClipboardWriteFuture = Pin<Box<dyn Future<Output = Result<(), ClipboardError>>>>;

/// One form of a pasted item, on its way.
pub type ClipboardReadFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, ClipboardError>>>>;

/// A promised form being produced: its content, or none when it could not be made.
pub type ContentFuture = Pin<Box<dyn Future<Output = Option<Vec<u8>>>>>;

/// A promised file's bytes being produced into its sink: whole once it resolves `Ok`.
pub type WriteFuture = Pin<Box<dyn Future<Output = io::Result<()>>>>;

/// Where a promised file's bytes go, in order: the host's, which writes them into the file where
/// the receiver chose, as `NSFilePromiseProvider` writes at the receiver's URL.
pub trait FileSink {
    /// Appends `bytes`; resolves once the host has taken them.
    fn write<'a>(
        &'a mut self,
        bytes: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + 'a>>;
}

/// One item on the clipboard, as it was when the clipboard was listed: the web's
/// `DataTransferItem`, the item its clipboard events and its drag and drop both hand over.
pub trait DataTransferItem {
    /// The forms it is offered in, by MIME type, in the order its writer gave them: the most
    /// faithful first. A file is offered as [`URI_LIST`].
    fn types(&self) -> &[String];

    /// Its content in `mime`, one of its types. Fails with [`ClipboardError::Changed`] if the
    /// clipboard changed since it was listed. A file promised is written into a folder of the
    /// host's when its [`URI_LIST`] is read, and the list names it there.
    fn read(&self, mime: &str) -> ClipboardReadFuture;
}

/// One item to put on the clipboard: the same thing in several forms, each keyed by its MIME
/// type, the most faithful first.
pub struct ClipboardItem {
    pub representations: Vec<(String, Content)>,
}

/// One form's content. A file, given or promised, is keyed by the type of what is in it, and
/// a reader finds it as [`URI_LIST`].
pub enum Content {
    /// The bytes, given at once.
    Data(Vec<u8>),
    /// Produced when someone pastes it, each time.
    Promise(Rc<dyn Fn() -> ContentFuture>),
    /// A file the app has, put on the clipboard by its path.
    File(PathBuf),
    /// A file that exists only once pasted: its name, and what produces its bytes into the sink
    /// the host hands it. The file is whole when the future resolves `Ok`; an `Err` fails the
    /// paste, and the host removes what it wrote.
    FilePromise {
        name: String,
        produce: Rc<dyn Fn(Box<dyn FileSink>) -> WriteFuture>,
    },
}

impl fmt::Debug for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::Data(bytes) => f.debug_tuple("Data").field(&bytes.len()).finish(),
            Content::Promise(_) => f.write_str("Promise"),
            Content::File(path) => f.debug_tuple("File").field(path).finish(),
            Content::FilePromise { name, .. } => {
                f.debug_struct("FilePromise").field("name", name).finish()
            }
        }
    }
}

impl fmt::Debug for ClipboardItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipboardItem")
            .field("representations", &self.representations)
            .finish()
    }
}

/// How a copy is to be treated. Each option names what it is for, and a host whose system lacks
/// it leaves it out, but for the expiry, which a host does itself where the system cannot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyOptions {
    /// A password or a code: kept out of previews, clipboard history and sync, as Android's
    /// `EXTRA_IS_SENSITIVE` and macOS's `org.nspasteboard.ConcealedType` keep it.
    pub sensitive: bool,
    /// Kept off other devices' clipboards, as Apple's `localOnly` and `currentHostOnly` keep
    /// it.
    pub this_device_only: bool,
    /// Cleared after this long, if still on the clipboard, as iOS's `expirationDate` clears it.
    /// Where the system has no expiry of its own (macOS, Windows, Linux, Android), the host keeps
    /// the time while the app runs: a copy whose app quits before its time stays on the clipboard.
    pub expires_after: Option<Duration>,
}

impl CopyOptions {
    /// A copy treated as any other.
    pub const fn new() -> CopyOptions {
        CopyOptions {
            sensitive: false,
            this_device_only: false,
            expires_after: None,
        }
    }

    pub const fn sensitive(mut self, sensitive: bool) -> CopyOptions {
        self.sensitive = sensitive;
        self
    }

    pub const fn this_device_only(mut self, this_device_only: bool) -> CopyOptions {
        self.this_device_only = this_device_only;
        self
    }

    pub const fn expires_after(mut self, expires_after: Duration) -> CopyOptions {
        self.expires_after = Some(expires_after);
        self
    }
}

/// Why the clipboard was not read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    /// The clipboard changed since the item was listed.
    Changed,
    /// The item is not offered in that type.
    NotFound,
    /// The user, or what they set, does not let the app read or write it.
    Denied,
    /// A form or a type this host's system cannot carry, such as a promised file where it has
    /// no file promises.
    Unsupported,
    /// The system failed, or a promised form was not produced.
    Failed(String),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::Changed => f.write_str("the clipboard changed since it was listed"),
            ClipboardError::NotFound => f.write_str("the item is not offered in that type"),
            ClipboardError::Denied => f.write_str("the clipboard may not be reached"),
            ClipboardError::Unsupported => f.write_str("the system cannot carry that form"),
            ClipboardError::Failed(reason) => write!(f, "the clipboard failed: {reason}"),
        }
    }
}

impl std::error::Error for ClipboardError {}

/// The user copied, cut or pasted in a view through the platform's own command: the web's
/// `copy`, `cut` and `paste` events, AppKit's `copy:`, `cut:` and `paste:` actions. What the
/// user copies is written, and what they pasted read, through the event, which may reach what
/// the app's own calls may not.
pub enum ClipboardEvent {
    /// Write what is selected.
    Copy(CopyRequest),
    /// Write what is selected, and remove it.
    Cut(CopyRequest),
    /// The clipboard as the user pasted it.
    Paste(Vec<Box<dyn DataTransferItem>>),
}

impl fmt::Debug for ClipboardEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardEvent::Copy(_) => f.write_str("Copy"),
            ClipboardEvent::Cut(_) => f.write_str("Cut"),
            ClipboardEvent::Paste(items) => f.debug_tuple("Paste").field(&items.len()).finish(),
        }
    }
}

/// Where the user's copy or cut is written: the clipboard, as the platform lets the user's own
/// copy write it.
pub struct CopyRequest(Box<dyn Fn(Vec<ClipboardItem>, CopyOptions) -> ClipboardWriteFuture>);

impl CopyRequest {
    /// A request whose writes go to `write`.
    pub fn new(
        write: impl Fn(Vec<ClipboardItem>, CopyOptions) -> ClipboardWriteFuture + 'static,
    ) -> CopyRequest {
        CopyRequest(Box::new(write))
    }

    /// Replaces the clipboard with `items`, as [`Clipboard::write`] does.
    pub fn write(&self, items: Vec<ClipboardItem>, options: CopyOptions) -> ClipboardWriteFuture {
        (self.0)(items, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_options_start_as_any_other_copy_and_set_one_option_each() {
        assert_eq!(CopyOptions::new(), CopyOptions::default());
        let options = CopyOptions::new()
            .sensitive(true)
            .expires_after(Duration::from_secs(60));
        assert!(options.sensitive);
        assert!(!options.this_device_only);
        assert_eq!(options.expires_after, Some(Duration::from_secs(60)));
    }

    #[test]
    fn a_copy_request_writes_through_what_it_was_made_with() {
        use std::cell::RefCell;

        let written = Rc::new(RefCell::new(Vec::new()));
        let into = Rc::clone(&written);
        let request = CopyRequest::new(move |items, _options| {
            into.borrow_mut().extend(items);
            Box::pin(std::future::ready(Ok(())))
        });
        let item = ClipboardItem {
            representations: vec![("text/plain".to_owned(), Content::Data(b"hi".to_vec()))],
        };
        let mut write = request.write(vec![item], CopyOptions::new());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let done = write.as_mut().poll(&mut context);
        assert_eq!(done, std::task::Poll::Ready(Ok(())));
        assert_eq!(written.borrow().len(), 1);
    }
}
