//! A file another application promised on the pasteboard, received: AppKit's
//! `NSFilePromiseReceiver`, whose files the promising application writes into a folder the
//! receiver chooses. The framework's read has no folder to give, so a promised file is offered as
//! `text/uri-list` and received, when that is read, into a folder of the app's own under the
//! system's temporary folder, a new one for each read, which the list then names; the system
//! empties it in time, as it does the rest of its temporary folder.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::rc::Rc;
use std::task::{Poll, Waker};

use block2::RcBlock;
use inset_embedder::{ClipboardError, ClipboardReadFuture};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, MainThreadMarker};
use objc2_app_kit::{NSFilePromiseReceiver, NSPasteboard, NSPasteboardItem};
use objc2_foundation::{NSArray, NSDictionary, NSError, NSOperationQueue, NSString, NSURL};

thread_local! {
    /// How many folders the app has received files into. Main thread.
    static FOLDERS_MADE: Cell<u64> = const { Cell::new(0) };
}

/// Whether `item` is a file another application promised.
pub(super) fn is_promised_file(item: &NSPasteboardItem) -> bool {
    let promise_types = NSFilePromiseReceiver::readableDraggedTypes();
    item.availableTypeFromArray(&promise_types).is_some()
}

/// The files of the `index`th file promise on `pasteboard`, received, as a `text/uri-list`.
pub(super) fn receive(pasteboard: &NSPasteboard, index: usize) -> ClipboardReadFuture {
    let receiving = MainThreadMarker::new()
        .ok_or_else(|| ClipboardError::Failed("off the main thread".to_owned()))
        .and_then(|_| promise_receiver(pasteboard, index))
        .and_then(|receiver| Receiving::start(&receiver));
    match receiving {
        Ok(receiving) => Box::pin(receiving.uri_list()),
        Err(error) => Box::pin(std::future::ready(Err(error))),
    }
}

/// The receiver of the `index`th file promise on `pasteboard`.
fn promise_receiver(
    pasteboard: &NSPasteboard,
    index: usize,
) -> Result<Retained<NSFilePromiseReceiver>, ClipboardError> {
    let classes = NSArray::from_slice(&[NSFilePromiseReceiver::class()]);
    // SAFETY: the class array holds a class that reads from a pasteboard, and no options are
    // given.
    let receivers = unsafe { pasteboard.readObjectsForClasses_options(&classes, None) };
    let receiver: Option<Retained<AnyObject>> = receivers
        .filter(|all| index < all.count())
        .map(|all| all.objectAtIndex(index));
    receiver
        .and_then(|receiver| receiver.downcast::<NSFilePromiseReceiver>().ok())
        .ok_or(ClipboardError::Changed)
}

/// One receipt of promised files: the files written so far, and who waits for them.
struct Receiving {
    expected: usize,
    received: RefCell<Vec<PathBuf>>,
    failed: Cell<bool>,
    waiting: RefCell<Option<Waker>>,
}

impl Receiving {
    /// Asks the promising application to write `receiver`'s files into a new folder.
    fn start(receiver: &NSFilePromiseReceiver) -> Result<Rc<Receiving>, ClipboardError> {
        let folder = new_folder().map_err(|error| ClipboardError::Failed(error.to_string()))?;
        let receiving = Rc::new(Receiving {
            expected: receiver.fileNames().count(),
            received: RefCell::default(),
            failed: Cell::new(false),
            waiting: RefCell::default(),
        });
        let reader = RcBlock::new({
            let receiving = Rc::clone(&receiving);
            move |url: NonNull<NSURL>, error: *mut NSError| {
                // SAFETY: AppKit hands the reader a URL that lives for the call.
                let url = unsafe { url.as_ref() };
                receiving.file_written(url, error.is_null());
            }
        });
        let folder_url = NSURL::fileURLWithPath(&NSString::from_str(&folder.to_string_lossy()));
        // SAFETY: the options are an empty dictionary; the main queue runs the reader on the main
        // thread, where `receiving` lives.
        unsafe {
            receiver.receivePromisedFilesAtDestination_options_operationQueue_reader(
                &folder_url,
                &NSDictionary::new(),
                &NSOperationQueue::mainQueue(),
                &reader,
            );
        }
        Ok(receiving)
    }

    /// A file was written at `url`, or failed to be.
    fn file_written(&self, url: &NSURL, written: bool) {
        match url.path() {
            Some(path) if written => self.received.borrow_mut().push(path.to_string().into()),
            _ => self.failed.set(true),
        }
        if let Some(waiting) = self.waiting.borrow_mut().take() {
            waiting.wake();
        }
    }

    /// The files' URLs, once every one is written.
    async fn uri_list(self: Rc<Self>) -> Result<Vec<u8>, ClipboardError> {
        std::future::poll_fn(|context| {
            if self.failed.get() {
                return Poll::Ready(Err(ClipboardError::Failed(
                    "the promised file was not written".to_owned(),
                )));
            }
            if self.received.borrow().len() >= self.expected {
                return Poll::Ready(Ok(uri_list(&self.received.borrow())));
            }
            *self.waiting.borrow_mut() = Some(context.waker().clone());
            Poll::Pending
        })
        .await
    }
}

/// A new folder of the app's own under the system's temporary folder.
fn new_folder() -> std::io::Result<PathBuf> {
    let number = FOLDERS_MADE.replace(FOLDERS_MADE.get() + 1);
    let folder = std::env::temp_dir()
        .join("Inset Pasted Files")
        .join(format!("{}-{number}", std::process::id()));
    std::fs::create_dir_all(&folder)?;
    Ok(folder)
}

/// Each path's `file:` URL, one per line.
fn uri_list(paths: &[PathBuf]) -> Vec<u8> {
    let mut list = String::new();
    for path in paths {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        if let Some(url) = url.absoluteString() {
            list.push_str(&url.to_string());
            list.push_str("\r\n");
        }
    }
    list.into_bytes()
}
