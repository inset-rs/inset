//! A form of a pasted item, read through the item's provider, asynchronously: the provider answers
//! on a queue of its own, through a completion handler, and the read's future, on the app's
//! thread, hears the answer there. A form the app itself promised is produced on the main thread
//! meanwhile (`providers`), which a read that waited on the main thread would never let happen.
//!
//! Reading the providers is reading the content: for another app's copy, it is what shows iOS's
//! paste alert.

use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Poll, Waker};

use block2::RcBlock;
use inset_embedder::{ClipboardError, ClipboardReadFuture};
use objc2_foundation::{NSData, NSError, NSString};
use objc2_ui_kit::UIPasteboard;

/// The `index`th item of `pasteboard`, read in the form named `uti`.
pub(super) fn form(pasteboard: &UIPasteboard, index: usize, uti: &NSString) -> ClipboardReadFuture {
    // SAFETY: read on the main thread, where the app uses UIKit's pasteboard.
    let providers = unsafe { pasteboard.itemProviders() };
    if index >= providers.count() {
        return Box::pin(std::future::ready(Err(ClipboardError::Changed)));
    }
    let provider = providers.objectAtIndex(index);
    let load = Arc::new(Mutex::new(Load::default()));
    let answered = Arc::clone(&load);
    let completion = RcBlock::new(move |data: *mut NSData, error: *mut NSError| {
        // SAFETY: the provider hands its data, or its error, alive for the call.
        let read = match unsafe { (data.as_ref(), error.as_ref()) } {
            (Some(data), _) => Ok(data.to_vec()),
            (None, Some(error)) => Err(ClipboardError::Failed(
                error.localizedDescription().to_string(),
            )),
            (None, None) => Err(ClipboardError::Failed(
                "the item provider gave nothing".to_owned(),
            )),
        };
        locked(&answered).answer(read);
    });
    // SAFETY: the completion handler only hands its answer to the read, through a lock, which any
    // thread may.
    let _progress = unsafe {
        provider.loadDataRepresentationForTypeIdentifier_completionHandler(uti, &completion)
    };
    Box::pin(std::future::poll_fn(move |context| {
        let mut load = locked(&load);
        match load.read.take() {
            Some(read) => Poll::Ready(read),
            None => {
                load.waiting = Some(context.waker().clone());
                Poll::Pending
            }
        }
    }))
}

/// One read's answer, once the provider gives it, and the read waiting for it.
#[derive(Default)]
struct Load {
    read: Option<Result<Vec<u8>, ClipboardError>>,
    waiting: Option<Waker>,
}

impl Load {
    fn answer(&mut self, read: Result<Vec<u8>, ClipboardError>) {
        self.read.get_or_insert(read);
        if let Some(waiting) = self.waiting.take() {
            waiting.wake();
        }
    }
}

fn locked(load: &Mutex<Load>) -> MutexGuard<'_, Load> {
    load.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
