//! What iOS asks the app to open: a document or a link whose scheme or kind is the app's, which a
//! scene is connected with or told later (`scene_delegate.rs`), as Flutter's scene delegate hears
//! `scene:openURLContexts:`. A file URL opened in place is reached only while its security-scoped
//! access lasts, which starts as it is answered and is kept for the app's life, as a picked file's
//! is (`file_dialogs.rs`).

// A URL's security-scoped access needs `unsafe`; this module writes it.
#![allow(unsafe_code)]

use std::cell::RefCell;

use objc2::Message;
use objc2::rc::Retained;
use objc2_foundation::NSURL;
use waft_embedder::{OpenRequest, PickedFile};

thread_local! {
    /// Each file URL opened in place whose access is kept for the app's life. Main thread.
    static ACCESSED: RefCell<Vec<Retained<NSURL>>> = const { RefCell::new(Vec::new()) };
}

/// What `urls` ask to open: their files, each one's access started and kept, and their links.
pub(crate) fn request_for(urls: &[Retained<NSURL>]) -> OpenRequest {
    let mut request = OpenRequest {
        files: Vec::new(),
        links: Vec::new(),
    };
    for url in urls {
        if url.isFileURL() {
            if let Some(path) = url.to_file_path() {
                keep_access(url);
                request.files.push(PickedFile::Path(path));
            }
        } else if let Some(link) = url.absoluteString() {
            request.links.push(link.to_string());
        }
    }
    request
}

/// Starts the access `url` needs and keeps it for the app's life.
fn keep_access(url: &NSURL) {
    // SAFETY: a URL iOS handed the app, whose access it grants until stopped.
    if unsafe { url.startAccessingSecurityScopedResource() } {
        ACCESSED.with_borrow_mut(|accessed| accessed.push(url.retain()));
    }
}
