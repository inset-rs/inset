//! A copy as item providers, the one way UIKit takes a form produced when someone pastes it: each
//! form registered under the UTI it is written as. A form given answers at once; a form promised,
//! and a file promised, are made by the app's producer, which UIKit asks for from a queue of its
//! own, so the ask goes to the main thread, the only one the producer may run on, and the answer
//! back through UIKit's completion handler once it is made. A promised file is written by its
//! producer into a temporary file of the app's, whose URL the provider hands over, and removed once
//! the handler has taken it, as UIKit copies a file it is handed then.

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::ptr::{self, NonNull};
use std::sync::Arc;

use block2::{DynBlock, RcBlock};
use dispatch2::{DispatchQueue, MainThreadBound};
use inset_embedder::{ClipboardError, ClipboardItem, Content, ContentFuture};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2_foundation::{
    NSData, NSError, NSItemProvider, NSItemProviderErrorCode, NSItemProviderErrorDomain,
    NSItemProviderFileOptions, NSItemProviderRepresentationVisibility, NSProgress, NSString, NSURL,
};

use crate::os::apple::app_futures;
use crate::os::apple::pasteboard_types as types;
use crate::os::apple::promised_files::{FileProducer, produce_at};

/// What produces one form, each time someone pastes it.
type FormProducer = std::rc::Rc<dyn Fn() -> ContentFuture>;

/// UIKit's completion handler for a form's data.
type DataCompletion = DynBlock<dyn Fn(*mut NSData, *mut NSError)>;

/// UIKit's completion handler for a file's URL.
type FileCompletion = DynBlock<dyn Fn(*mut NSURL, Bool, *mut NSError)>;

thread_local! {
    /// How many folders the app has written promised files into. Main thread.
    static FOLDERS_MADE: Cell<u64> = const { Cell::new(0) };
}

/// The item provider of `item`: each form registered under the UTI it is written as, or, for an
/// item carrying a file, which must be its one form, the file's URL or its promise.
pub(super) fn item_provider(
    item: ClipboardItem,
) -> Result<Retained<NSItemProvider>, ClipboardError> {
    let provider = NSItemProvider::new();
    let file_alone = item.representations.len() == 1;
    for (mime, content) in item.representations {
        match content {
            Content::Data(bytes) => {
                let uti = uti_of(&mime)?;
                register_data(&provider, &uti, bytes);
            }
            Content::Promise(produce) => {
                let uti = uti_of(&mime)?;
                register_promise(main_thread()?, &provider, &uti, produce);
            }
            Content::File(path) if file_alone => {
                let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
                provider.registerObject_visibility(
                    ProtocolObject::from_ref(&*url),
                    NSItemProviderRepresentationVisibility::All,
                );
            }
            Content::FilePromise { name, produce } if file_alone => {
                let uti = uti_of(&mime)?;
                provider.setSuggestedName(Some(&NSString::from_str(&name)));
                register_promised_file(main_thread()?, &provider, &uti, name, produce);
            }
            Content::File(_) | Content::FilePromise { .. } => {
                return Err(ClipboardError::Unsupported);
            }
        }
    }
    Ok(provider)
}

/// The main thread, the only one a promise's producer may run on, which the copy is written on.
fn main_thread() -> Result<MainThreadMarker, ClipboardError> {
    MainThreadMarker::new().ok_or_else(|| {
        ClipboardError::Failed("a promised form is written off the main thread".to_owned())
    })
}

fn uti_of(mime: &str) -> Result<Retained<NSString>, ClipboardError> {
    types::uti_of(mime).ok_or(ClipboardError::Unsupported)
}

/// Registers a form given: its bytes, answered at once.
fn register_data(provider: &NSItemProvider, uti: &NSString, bytes: Vec<u8>) {
    let data = NSData::with_bytes(&bytes);
    let load = RcBlock::new(
        move |completion: NonNull<DataCompletion>| -> *mut NSProgress {
            // SAFETY: UIKit's completion handler, alive for the call, takes the data it is handed.
            unsafe { completion.as_ref() }
                .call((Retained::as_ptr(&data).cast_mut(), ptr::null_mut()));
            ptr::null_mut()
        },
    );
    // SAFETY: the load handler holds only the data, which any thread may hand over.
    unsafe {
        provider.registerDataRepresentationForTypeIdentifier_visibility_loadHandler(
            uti,
            NSItemProviderRepresentationVisibility::All,
            &load,
        );
    }
}

/// Registers a form promised: made by `produce` on the main thread each time UIKit asks.
fn register_promise(
    main_thread: MainThreadMarker,
    provider: &NSItemProvider,
    uti: &NSString,
    produce: FormProducer,
) {
    let produce = Arc::new(MainThreadBound::new(produce, main_thread));
    let load = RcBlock::new(
        move |completion: NonNull<DataCompletion>| -> *mut NSProgress {
            // SAFETY: UIKit's completion handler, alive for the call, which the copy keeps alive after.
            let completion = Answer(unsafe { completion.as_ref() }.copy());
            let produce = Arc::clone(&produce);
            DispatchQueue::main().exec_async(move || {
                let main_thread =
                    MainThreadMarker::new().expect("the main queue runs on the main thread");
                let made = (produce.get(main_thread))();
                app_futures::spawn(Box::pin(async move {
                    match made.await {
                        Some(bytes) => {
                            let data = NSData::with_bytes(&bytes);
                            completion
                                .handler()
                                .call((Retained::as_ptr(&data).cast_mut(), ptr::null_mut()));
                        }
                        None => {
                            let error = unavailable();
                            completion
                                .handler()
                                .call((ptr::null_mut(), Retained::as_ptr(&error).cast_mut()));
                        }
                    }
                }));
            });
            ptr::null_mut()
        },
    );
    // SAFETY: the load handler holds the producer bound to the main thread, and hands the work
    // there, so it may be called from any thread.
    unsafe {
        provider.registerDataRepresentationForTypeIdentifier_visibility_loadHandler(
            uti,
            NSItemProviderRepresentationVisibility::All,
            &load,
        );
    }
}

/// Registers a file promised: written by `produce` on the main thread into a temporary file each
/// time UIKit asks, whose URL the provider hands over.
fn register_promised_file(
    main_thread: MainThreadMarker,
    provider: &NSItemProvider,
    uti: &NSString,
    name: String,
    produce: FileProducer,
) {
    let promised = Arc::new(MainThreadBound::new((name, produce), main_thread));
    let load = RcBlock::new(
        move |completion: NonNull<FileCompletion>| -> *mut NSProgress {
            // SAFETY: as for a form's.
            let completion = Answer(unsafe { completion.as_ref() }.copy());
            let promised = Arc::clone(&promised);
            DispatchQueue::main().exec_async(move || {
                let main_thread =
                    MainThreadMarker::new().expect("the main queue runs on the main thread");
                let (name, produce) = promised.get(main_thread);
                let folder = new_folder();
                let path = folder.join(name);
                let written =
                    fs::create_dir_all(&folder).map(|()| produce_at(path.clone(), produce));
                app_futures::spawn(Box::pin(async move {
                    let written = match written {
                        Ok(written) => written.await,
                        Err(error) => Err(error),
                    };
                    match written {
                        Ok(()) => {
                            let url = NSURL::fileURLWithPath(&NSString::from_str(
                                &path.to_string_lossy(),
                            ));
                            let url = Retained::as_ptr(&url).cast_mut();
                            completion.handler().call((url, Bool::NO, ptr::null_mut()));
                        }
                        Err(_) => {
                            let error = unavailable();
                            let error = Retained::as_ptr(&error).cast_mut();
                            completion
                                .handler()
                                .call((ptr::null_mut(), Bool::NO, error));
                        }
                    }
                    let _ = fs::remove_dir_all(&folder);
                }));
            });
            ptr::null_mut()
        },
    );
    // SAFETY: as for a form's.
    unsafe {
        provider.registerFileRepresentationForTypeIdentifier_fileOptions_visibility_loadHandler(
            uti,
            NSItemProviderFileOptions::empty(),
            NSItemProviderRepresentationVisibility::All,
            &load,
        );
    }
}

/// A folder of the app's own under the system's temporary folder, a new one each time, which a
/// promised file is written into. Main thread.
fn new_folder() -> PathBuf {
    let number = FOLDERS_MADE.replace(FOLDERS_MADE.get() + 1);
    std::env::temp_dir().join(format!(
        "inset-promised-files-{}-{number}",
        std::process::id()
    ))
}

/// UIKit's completion handler, carried to the main thread to be called once the app's producer
/// has made what it promised.
struct Answer<B: ?Sized>(RcBlock<B>);

impl<B: ?Sized> Answer<B> {
    fn handler(&self) -> &DynBlock<B> {
        &self.0
    }
}

// SAFETY: UIKit's completion handlers may be called from any thread, and the block is called
// once, from one thread at a time.
unsafe impl<B: ?Sized> Send for Answer<B> {}

/// The item provider's error for a form or a file the producer did not make.
fn unavailable() -> Retained<NSError> {
    // SAFETY: the domain is a constant Foundation exports for the life of the process, and an
    // error needs no user info.
    unsafe {
        NSError::errorWithDomain_code_userInfo(
            NSItemProviderErrorDomain,
            NSItemProviderErrorCode::ItemUnavailableError.0,
            None,
        )
    }
}
