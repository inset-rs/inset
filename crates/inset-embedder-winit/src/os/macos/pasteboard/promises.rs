//! The forms a copy promises, produced when someone pastes them.
//!
//! A form is promised to AppKit by a data provider ([`PromisedForms`]), which AppKit asks for it
//! on the app's thread, inside whichever paste reads it, another application's or this one's, and
//! at the application's termination for every form still promised, as it asks every owner of the
//! pasteboard; it wants the bytes before the call returns, so the embedder waits there for the
//! app's producer, up to [`PROMISE_WAIT`], and provides nothing after that. A producer must then
//! be finished by another thread: one that waits on the app's own loop is never answered.
//!
//! A file that exists only once pasted is an `NSFilePromiseProvider` ([`PromisedFile`]): the
//! receiver chooses where the file goes, and AppKit asks for it on the main operation queue and
//! takes the answer through a completion handler, so the app's producer runs on the main thread
//! for as long as it takes, its bytes written into the file at the receiver's URL as they come,
//! and the file removed when it fails (`apple::promised_files`).

use std::path::PathBuf;
use std::ptr;
use std::rc::Rc;
use std::thread::{self, ThreadId};
use std::time::Duration;

use block2::DynBlock;
use inset_embedder::ContentFuture;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSFilePromiseProvider, NSFilePromiseProviderDelegate, NSPasteboard, NSPasteboardItem,
    NSPasteboardItemDataProvider, NSPasteboardType,
};
use objc2_foundation::{NSCocoaErrorDomain, NSData, NSError, NSString, NSURL};

use super::super::abort_on_panic;
use super::waiting;
use crate::os::apple::app_futures;
use crate::os::apple::promised_files::{FileProducer, produce_at};

/// How long a paste waits for a promised form, on the main thread, before it gets nothing.
pub(super) const PROMISE_WAIT: Duration = Duration::from_secs(10);

/// What produces one form, each time someone pastes it.
pub(super) type Producer = Rc<dyn Fn() -> ContentFuture>;

/// What produces each promised form of one item, by the UTI it is promised as, and the thread
/// they were made on, the only one they may run on.
pub(super) struct Producers {
    thread: ThreadId,
    forms: Vec<(Retained<NSString>, Producer)>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `PromisedForms` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "InsetPasteboardPromisedForms"]
    #[ivars = Producers]
    pub(super) struct PromisedForms;

    unsafe impl NSObjectProtocol for PromisedForms {}

    unsafe impl NSPasteboardItemDataProvider for PromisedForms {
        #[unsafe(method(pasteboard:item:provideDataForType:))]
        fn pasteboard_item_provide_data_for_type(
            &self,
            _pasteboard: Option<&NSPasteboard>,
            item: &NSPasteboardItem,
            r#type: &NSPasteboardType,
        ) {
            abort_on_panic(|| self.provide(item, r#type));
        }
    }
);

impl PromisedForms {
    /// The provider of the forms `forms` produce, on this thread. They are promised once it is
    /// set on an item for their types ([`Self::types`]).
    pub(super) fn new(forms: Vec<(Retained<NSString>, Producer)>) -> Retained<PromisedForms> {
        let this = PromisedForms::alloc().set_ivars(Producers {
            thread: thread::current().id(),
            forms,
        });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }

    /// The UTIs of the forms promised.
    pub(super) fn types(&self) -> Vec<Retained<NSString>> {
        self.ivars()
            .forms
            .iter()
            .map(|(uti, _)| uti.clone())
            .collect()
    }

    pub(super) fn as_provider(&self) -> &ProtocolObject<dyn NSPasteboardItemDataProvider> {
        ProtocolObject::from_ref(self)
    }

    /// Sets the form `uti` of `item` from its producer, waited for here; nothing when it gave
    /// none in time, or when AppKit asks on another thread than the app's, where the producer may
    /// not run.
    fn provide(&self, item: &NSPasteboardItem, uti: &NSString) {
        let producers = self.ivars();
        if thread::current().id() != producers.thread {
            return;
        }
        let Some((_, produce)) = producers
            .forms
            .iter()
            .find(|(promised, _)| &**promised == uti)
        else {
            return;
        };
        if let Some(Some(bytes)) = waiting::wait(produce(), PROMISE_WAIT) {
            item.setData_forType(&NSData::with_bytes(&bytes), uti);
        }
    }
}

/// A file promised: its name, and what produces its bytes.
pub(super) struct FileWriter {
    name: String,
    produce: FileProducer,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `PromisedFile` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetPasteboardPromisedFile"]
    #[ivars = FileWriter]
    pub(super) struct PromisedFile;

    unsafe impl NSObjectProtocol for PromisedFile {}

    unsafe impl NSFilePromiseProviderDelegate for PromisedFile {
        #[unsafe(method_id(filePromiseProvider:fileNameForType:))]
        fn file_promise_provider_file_name_for_type(
            &self,
            _provider: &NSFilePromiseProvider,
            _file_type: &NSString,
        ) -> Retained<NSString> {
            abort_on_panic(|| NSString::from_str(&self.ivars().name))
        }

        #[unsafe(method(filePromiseProvider:writePromiseToURL:completionHandler:))]
        fn file_promise_provider_write_promise_to_url_completion_handler(
            &self,
            _provider: &NSFilePromiseProvider,
            url: &NSURL,
            completion: &DynBlock<dyn Fn(*mut NSError)>,
        ) {
            abort_on_panic(|| self.write_to(url, completion));
        }
    }
);

impl PromisedFile {
    /// The promise of a file named `name`, of the type AppKit names `uti`, whose bytes `produce`
    /// makes: the provider, which the pasteboard takes, and its delegate, which the pasteboard
    /// holds only weakly, so the caller keeps it for as long as the file may be pasted.
    pub(super) fn new(
        main_thread: MainThreadMarker,
        uti: &NSString,
        name: String,
        produce: FileProducer,
    ) -> (Retained<NSFilePromiseProvider>, Retained<PromisedFile>) {
        let this = main_thread
            .alloc::<PromisedFile>()
            .set_ivars(FileWriter { name, produce });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        let delegate: Retained<PromisedFile> = unsafe { msg_send![super(this), init] };
        let provider = NSFilePromiseProvider::initWithFileType_delegate(
            main_thread.alloc(),
            uti,
            ProtocolObject::from_ref(&*delegate),
        );
        (provider, delegate)
    }

    /// Writes the file at `url` with what the producer makes, and calls `completion` once it is
    /// whole or has failed, when what was written is removed.
    fn write_to(&self, url: &NSURL, completion: &DynBlock<dyn Fn(*mut NSError)>) {
        let completion = completion.copy();
        let Some(path) = url.path().map(|path| PathBuf::from(path.to_string())) else {
            completion.call((write_error(),));
            return;
        };
        let produced = produce_at(path, &self.ivars().produce);
        app_futures::spawn(Box::pin(async move {
            let error = match produced.await {
                Ok(()) => ptr::null_mut(),
                Err(_) => write_error(),
            };
            completion.call((error,));
        }));
    }
}

/// Cocoa's error for a file that could not be written, which the completion handler takes; it
/// lives as long as the autorelease pool the handler's caller drains.
fn write_error() -> *mut NSError {
    // Cocoa's `NSFileWriteUnknownError`.
    const FILE_WRITE_UNKNOWN: isize = 512;
    // SAFETY: the domain is a constant Foundation exports for the life of the process, and an
    // error needs no user info.
    let error = unsafe {
        NSError::errorWithDomain_code_userInfo(NSCocoaErrorDomain, FILE_WRITE_UNKNOWN, None)
    };
    Retained::autorelease_ptr(error)
}
