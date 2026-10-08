//! The system pasteboard as the framework's [`Clipboard`], through arboard.

use std::cell::RefCell;

use inset_embedder::{
    Clipboard, ClipboardHasStringsFuture, ClipboardSetTextFuture, ClipboardTextFuture,
};

/// The OS pasteboard; opened on first use, so a system without one is not fatal.
#[derive(Default)]
pub(crate) struct Pasteboard {
    clipboard: RefCell<Option<arboard::Clipboard>>,
}

impl Pasteboard {
    fn with_clipboard<T>(&self, f: impl FnOnce(&mut arboard::Clipboard) -> T) -> Option<T> {
        let mut slot = self.clipboard.borrow_mut();
        if slot.is_none() {
            *slot = arboard::Clipboard::new().ok();
        }
        slot.as_mut().map(f)
    }

    fn text_now(&self) -> Option<String> {
        self.with_clipboard(|clipboard| clipboard.get_text().ok())
            .flatten()
    }
}

impl Clipboard for Pasteboard {
    /// Written in the call, since the pasteboard takes it at once; the future is ready.
    fn set_text(&self, text: &str) -> ClipboardSetTextFuture {
        self.with_clipboard(|clipboard| {
            let _ = clipboard.set_text(text);
        });
        Box::pin(std::future::ready(()))
    }

    /// Read in the call, since the pasteboard answers at once; the future is ready.
    fn text(&self) -> ClipboardTextFuture {
        Box::pin(std::future::ready(self.text_now()))
    }

    fn has_strings(&self) -> ClipboardHasStringsFuture {
        Box::pin(std::future::ready(self.text_now().is_some()))
    }
}
