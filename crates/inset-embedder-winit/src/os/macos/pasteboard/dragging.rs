//! The dragging pasteboard: what is dragged over a window, read as the general pasteboard's items
//! are (`listed_items`), and the types a window is registered for, so that every drag that reader
//! takes reaches it.
//!
//! AppKit offers a drag to a window only when the pasteboard holds a type the window is registered
//! for, and winit registers text, HTML, PNG and TIFF images, sound and file URLs. A link that is
//! only a URL, rich text, and a file another application promises would pass the window by, so
//! the embedder registers those types besides.

use inset_embedder::DataTransferItem;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSFilePromiseReceiver, NSPasteboard, NSPasteboardNameDrag, NSPasteboardType,
    NSPasteboardTypeFileURL, NSPasteboardTypeHTML, NSPasteboardTypePNG, NSPasteboardTypeRTF,
    NSPasteboardTypeSound, NSPasteboardTypeString, NSPasteboardTypeTIFF, NSPasteboardTypeURL,
};
use objc2_foundation::NSArray;
use winit::window::Window;

use super::listed_items;
use crate::os::SystemDrags;
use crate::os::macos::{AppKit, ns_window};

impl SystemDrags for AppKit {
    /// The dragging pasteboard's items, as the general pasteboard's are listed: during a drag,
    /// the pasteboard AppKit hands a destination is the one named for drags. A read fails once
    /// another drag has replaced it, as a paste's does once the pasteboard changed.
    fn dragged_items() -> Option<Vec<Box<dyn DataTransferItem>>> {
        // SAFETY: AppKit's constant, valid for the life of the process.
        let name = unsafe { NSPasteboardNameDrag };
        Some(listed_items(&NSPasteboard::pasteboardWithName(name)))
    }

    /// Registers `window` for winit's types and the embedder's besides: a URL, rich text and a
    /// promised file.
    fn accept_drags(window: &dyn Window) {
        let Some(window) = ns_window(window) else {
            return;
        };
        window.registerForDraggedTypes(&accepted_types());
    }
}

/// The types a window accepts a drag of: winit's own, listed again lest a registration replace
/// them, and those the pasteboard reader takes besides.
fn accepted_types() -> Retained<NSArray<NSPasteboardType>> {
    let promised: Vec<Retained<NSPasteboardType>> = NSFilePromiseReceiver::readableDraggedTypes()
        .iter()
        .collect();
    // SAFETY: AppKit's constants, valid for the life of the process.
    let mut types: Vec<&NSPasteboardType> = unsafe {
        vec![
            NSPasteboardTypeFileURL,
            NSPasteboardTypeHTML,
            NSPasteboardTypePNG,
            NSPasteboardTypeSound,
            NSPasteboardTypeString,
            NSPasteboardTypeTIFF,
            NSPasteboardTypeURL,
            NSPasteboardTypeRTF,
        ]
    };
    types.extend(promised.iter().map(|promised| &**promised));
    NSArray::from_slice(&types)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_takes_a_url_rich_text_and_a_promised_file_besides_winits_forms() {
        let accepted: Vec<String> = accepted_types().iter().map(|uti| uti.to_string()).collect();
        for uti in [
            "public.file-url",
            "public.utf8-plain-text",
            "public.url",
            "public.rtf",
        ] {
            assert!(accepted.iter().any(|taken| taken == uti), "{uti}");
        }
        for promised in NSFilePromiseReceiver::readableDraggedTypes().iter() {
            assert!(accepted.contains(&promised.to_string()), "{promised}");
        }
    }
}
