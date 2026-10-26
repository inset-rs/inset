//! The general pasteboard as the framework's clipboard: UIKit's `UIPasteboard`, whose items are
//! each offered in several types, named by UTI as on macOS (`apple::pasteboard_types`). A copy
//! with every form given is written whole with `setItems:options:`; one with a form produced when
//! someone pastes it, or a file that exists only once pasted, is written as item providers, which
//! UIKit asks on a queue of its own (`providers`).
//!
//! What a copy may hold follows macOS's: an item carrying a file, given or promised, is that file
//! alone, a file given as its URL, and a promised file written by its producer into a temporary
//! file whose URL the item provider hands over. A copy for this device only is written local only;
//! one that expires carries its expiration date, which the system keeps, after the app has quit
//! too; a sensitive copy is as any other, since iOS keeps no clipboard history and shows no
//! previews an app could keep it out of.
//!
//! Listing reads no content: the items' types come from `pasteboardTypesForItemSet:`, which iOS
//! lets an app ask without the paste alert iOS 16 shows. Reading another app's content is what
//! shows it, as the system asks the user first; a read loads the item's form through its item
//! provider, so the app's own promised form is produced on the main thread while a read of it
//! waits (`loads`). UIKit says the pasteboard changed while the app is active, and the main loop
//! looks again as the app becomes active (`notices`); the app's own writes count as seen.
//!
//! The invariant: an item read is the pasteboard's as listed. Listing notes the pasteboard's count
//! of changes, and a read once it has moved fails.

// UIKit's pasteboard calls and the blocks it calls back need `unsafe`; this module and its
// children are the one place on iOS that writes it.
#![allow(unsafe_code)]

mod loads;
mod notices;
mod providers;

use std::cell::Cell;
use std::future::ready;
use std::ptr;

use inset_embedder::{
    Clipboard, ClipboardError, ClipboardItem, ClipboardItemsFuture, ClipboardReadFuture,
    ClipboardWriteFuture, Content, CopyOptions, PastedItem,
};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{
    NSArray, NSData, NSDate, NSDictionary, NSIndexSet, NSInteger, NSNumber,
    NSPropertyListReadOptions, NSPropertyListSerialization, NSString, NSURL,
};
use objc2_ui_kit::{UIPasteboard, UIPasteboardOptionExpirationDate, UIPasteboardOptionLocalOnly};

use crate::clipboard::ClipboardChanges;
use crate::os::apple::pasteboard_types as types;
use notices::Notices;

/// A pasteboard, the general one but in tests, as this app reaches it, on the main thread.
pub(crate) struct Pasteboard {
    pasteboard: Retained<UIPasteboard>,
    /// The pasteboard's count of changes when it was last asked whether it changed, or the app
    /// last wrote it, whichever came last.
    seen: Cell<NSInteger>,
    /// UIKit's notices that have the main loop look, while the clipboard is the app's.
    _notices: Option<Notices>,
}

impl Pasteboard {
    /// The general pasteboard as it is at launch, which the app hears no change of; `look` has the
    /// main loop look at it again, each time UIKit says it changed or the app became active.
    pub(crate) fn general(look: Box<dyn Fn()>) -> Pasteboard {
        let notices =
            MainThreadMarker::new().map(|main_thread| Notices::observe(main_thread, look));
        Pasteboard::new(UIPasteboard::generalPasteboard(), notices)
    }

    fn new(pasteboard: Retained<UIPasteboard>, notices: Option<Notices>) -> Pasteboard {
        Pasteboard {
            seen: Cell::new(change_count(&pasteboard)),
            pasteboard,
            _notices: notices,
        }
    }

    /// Replaces the pasteboard's contents with `items`, treated as `options` says: a change the app
    /// made, so it counts as seen.
    fn write_now(
        &self,
        items: Vec<ClipboardItem>,
        options: CopyOptions,
    ) -> Result<(), ClipboardError> {
        let promises = items
            .iter()
            .flat_map(|item| &item.representations)
            .any(|(_, content)| {
                matches!(content, Content::Promise(_) | Content::FilePromise { .. })
            });
        if promises {
            self.write_providers(items, options)?;
        } else {
            self.write_items(items, options)?;
        }
        self.seen.set(change_count(&self.pasteboard));
        Ok(())
    }

    /// Writes `items`, every form given, whole.
    fn write_items(
        &self,
        items: Vec<ClipboardItem>,
        options: CopyOptions,
    ) -> Result<(), ClipboardError> {
        let items = items
            .into_iter()
            .map(pasteboard_item)
            .collect::<Result<Vec<_>, _>>()?;
        let options = pasteboard_options(options);
        // SAFETY: each item maps UTIs to the data or the URL of a form, and the options map
        // UIKit's option keys to the values those keys take.
        unsafe {
            self.pasteboard
                .setItems_options(&NSArray::from_retained_slice(&items), &options);
        }
        Ok(())
    }

    /// Writes `items` as item providers, which UIKit asks for each form when someone pastes it.
    fn write_providers(
        &self,
        items: Vec<ClipboardItem>,
        options: CopyOptions,
    ) -> Result<(), ClipboardError> {
        let providers = items
            .into_iter()
            .map(providers::item_provider)
            .collect::<Result<Vec<_>, _>>()?;
        let expiration = options.expires_after.map(expiration_date);
        self.pasteboard.setItemProviders_localOnly_expirationDate(
            &NSArray::from_retained_slice(&providers),
            options.this_device_only,
            expiration.as_deref(),
        );
        Ok(())
    }
}

impl Clipboard for Pasteboard {
    /// The pasteboard's items, listed in the call, with no content read.
    fn items(&self) -> ClipboardItemsFuture {
        Box::pin(ready(listed_items(&self.pasteboard)))
    }

    /// Written in the call, since the pasteboard takes the items at once; a form promised is
    /// asked for later, by whoever pastes it.
    fn write(&self, items: Vec<ClipboardItem>, options: CopyOptions) -> ClipboardWriteFuture {
        Box::pin(ready(self.write_now(items, options)))
    }
}

impl ClipboardChanges for Pasteboard {
    fn has_changed(&self) -> bool {
        let now = change_count(&self.pasteboard);
        self.seen.replace(now) != now
    }
}

/// The pasteboard's count of changes.
fn change_count(pasteboard: &UIPasteboard) -> NSInteger {
    // SAFETY: read on the main thread, where the app uses UIKit's pasteboard.
    unsafe { pasteboard.changeCount() }
}

/// The date a copy that expires after `after` is cleared at.
fn expiration_date(after: std::time::Duration) -> Retained<NSDate> {
    NSDate::dateWithTimeIntervalSinceNow(after.as_secs_f64())
}

/// UIKit's options for a copy treated as `options` says.
fn pasteboard_options(options: CopyOptions) -> Retained<NSDictionary<NSString, AnyObject>> {
    let mut keys: Vec<&NSString> = Vec::new();
    let mut values: Vec<Retained<AnyObject>> = Vec::new();
    if options.this_device_only {
        // SAFETY: a constant UIKit exports for the life of the process.
        keys.push(unsafe { UIPasteboardOptionLocalOnly });
        values.push(Retained::into_super(Retained::into_super(
            Retained::into_super(NSNumber::numberWithBool(true)),
        )));
    }
    if let Some(after) = options.expires_after {
        // SAFETY: as above.
        keys.push(unsafe { UIPasteboardOptionExpirationDate });
        values.push(Retained::into_super(Retained::into_super(expiration_date(
            after,
        ))));
    }
    NSDictionary::from_retained_objects(&keys, &values)
}

/// An item of `setItems:`, each form given: its data by UTI, or a file's URL for an item carrying
/// a file, which must be its one form.
fn pasteboard_item(
    item: ClipboardItem,
) -> Result<Retained<NSDictionary<NSString, AnyObject>>, ClipboardError> {
    let mut utis = Vec::new();
    let mut values: Vec<Retained<AnyObject>> = Vec::new();
    let file_alone = item.representations.len() == 1;
    for (mime, content) in item.representations {
        match content {
            Content::Data(bytes) => {
                utis.push(types::uti_of(&mime).ok_or(ClipboardError::Unsupported)?);
                let data = NSData::with_bytes(&bytes);
                values.push(Retained::into_super(Retained::into_super(data)));
            }
            Content::File(path) if file_alone => {
                utis.push(NSString::from_str(types::FILE_URL));
                let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
                values.push(Retained::into_super(Retained::into_super(url)));
            }
            Content::File(_) | Content::Promise(_) | Content::FilePromise { .. } => {
                return Err(ClipboardError::Unsupported);
            }
        }
    }
    let keys: Vec<&NSString> = utis.iter().map(|uti| &**uti).collect();
    Ok(NSDictionary::from_retained_objects(&keys, &values))
}

/// `pasteboard`'s items now, each in the types it is offered in. An item with no type the
/// framework can name is left out.
fn listed_items(pasteboard: &Retained<UIPasteboard>) -> Vec<Box<dyn PastedItem>> {
    let change_count = change_count(pasteboard);
    let Some(items) = pasteboard.pasteboardTypesForItemSet(None) else {
        return Vec::new();
    };
    let mut listed: Vec<Box<dyn PastedItem>> = Vec::new();
    for (index, utis) in items.iter().enumerate() {
        let item = ListedItem::new(pasteboard.clone(), index, change_count, &utis);
        if !item.types.is_empty() {
            listed.push(Box::new(item));
        }
    }
    listed
}

/// One item of a pasteboard as it was listed.
struct ListedItem {
    pasteboard: Retained<UIPasteboard>,
    /// Its place among the pasteboard's items.
    index: usize,
    change_count: NSInteger,
    /// The MIME types it is offered in, each with the UTI it is read as.
    types: Vec<String>,
    utis: Vec<Retained<NSString>>,
}

impl ListedItem {
    fn new(
        pasteboard: Retained<UIPasteboard>,
        index: usize,
        change_count: NSInteger,
        utis: &NSArray<NSString>,
    ) -> ListedItem {
        let mut listed = ListedItem {
            pasteboard,
            index,
            change_count,
            types: Vec::new(),
            utis: Vec::new(),
        };
        for uti in utis.iter() {
            if let Some(mime) = types::mime_of(&uti)
                && !listed.types.contains(&mime)
            {
                listed.types.push(mime);
                listed.utis.push(uti);
            }
        }
        listed
    }

    /// The item's URL in `uti`, as a line of a `text/uri-list`.
    fn read_url(&self, uti: &NSString) -> Result<Vec<u8>, ClipboardError> {
        let item = NSIndexSet::indexSetWithIndex(self.index);
        let values = self
            .pasteboard
            .valuesForPasteboardType_inItemSet(uti, Some(&item));
        let url = values
            .and_then(|values| values.firstObject())
            .and_then(|value| url_of(&value))
            .ok_or_else(|| ClipboardError::Failed("the pasteboard gave no URL".to_owned()))?;
        Ok(format!("{url}\r\n").into_bytes())
    }
}

/// The URL a pasteboard's value for a URL type holds: an `NSURL`, its string, or the property list
/// UIKit keeps an `NSURL` as, an array of its string and its base's.
fn url_of(value: &AnyObject) -> Option<String> {
    if let Some(url) = value.downcast_ref::<NSURL>() {
        return url.absoluteString().map(|url| url.to_string());
    }
    if let Some(text) = value.downcast_ref::<NSString>() {
        return Some(text.to_string());
    }
    let data = value.downcast_ref::<NSData>()?;
    // SAFETY: the format is not asked for.
    let list = unsafe {
        NSPropertyListSerialization::propertyListWithData_options_format_error(
            data,
            NSPropertyListReadOptions::Immutable,
            ptr::null_mut(),
        )
    }
    .ok()?;
    let parts = list.downcast_ref::<NSArray>()?;
    let first = parts.firstObject()?;
    first.downcast_ref::<NSString>().map(|url| url.to_string())
}

impl PastedItem for ListedItem {
    fn types(&self) -> &[String] {
        &self.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        if change_count(&self.pasteboard) != self.change_count {
            return Box::pin(ready(Err(ClipboardError::Changed)));
        }
        let Some(position) = self.types.iter().position(|offered| offered == mime) else {
            return Box::pin(ready(Err(ClipboardError::NotFound)));
        };
        let uti = &self.utis[position];
        if types::is_url(uti) {
            Box::pin(ready(self.read_url(uti)))
        } else {
            loads::form(&self.pasteboard, self.index, uti)
        }
    }
}

#[cfg(test)]
mod tests;
