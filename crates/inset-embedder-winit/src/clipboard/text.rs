//! The plain text of the system's clipboard, through arboard: what the embedder reaches where
//! winit has no clipboard and the embedder has no system code of its own for one, Windows and
//! Linux among them. One item, as those systems hold; every other form is refused.
//!
//! Another writer's change is told by the system's count of changes where it has a cheap one,
//! Windows' sequence number; on Linux by reading the text whole and comparing it with the last
//! look's, since arboard has no cheaper probe: the cost of every look, until Linux has code of its
//! own. A sensitive copy is kept out of the system's clipboard history, and on Windows out of its
//! monitoring; a copy for this device only is kept off Windows' cloud clipboard; and one that
//! expires is cleared at its time by the main loop, if it is still the app's, as neither system
//! has an expiry of its own.

use std::cell::RefCell;
use std::time::Instant;

use inset_embedder::{
    Clipboard, ClipboardError, ClipboardItem, ClipboardItemsFuture, ClipboardReadFuture,
    ClipboardWriteFuture, Content, CopyOptions, DataTransferItem,
};

use super::ClipboardChanges;

const TEXT: &str = "text/plain";

/// How a system tells that its clipboard changed since a look.
#[derive(Clone, Copy)]
pub(crate) enum ChangeProbe {
    /// The system's count of the clipboard's changes, read without reading the clipboard:
    /// Windows' sequence number.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    Count(fn() -> u32),
    /// The text itself, read whole and compared with the last look's.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Text,
}

/// The clipboard as a look found it, by the system's probe.
#[derive(Clone, Debug, PartialEq)]
enum Mark {
    Count(u32),
    Text(Option<String>),
}

/// A copy of the app's that expires: when, and the clipboard as the app's write left it.
struct Expiry {
    at: Instant,
    copy: Mark,
}

/// The system's clipboard, opened on first use, so a system without one is not fatal.
pub(crate) struct TextClipboard {
    clipboard: RefCell<Option<arboard::Clipboard>>,
    probe: ChangeProbe,
    /// The clipboard when it was last asked whether it changed, or the app last wrote it,
    /// whichever came last.
    seen: RefCell<Mark>,
    /// The app's last copy, while it waits to expire; a newer copy replaces it.
    expiring: RefCell<Option<Expiry>>,
}

impl TextClipboard {
    /// The clipboard as it is at launch, which the app hears no change of, its changes told by
    /// `probe`.
    pub(crate) fn new(probe: ChangeProbe) -> TextClipboard {
        let clipboard = TextClipboard {
            clipboard: RefCell::default(),
            probe,
            seen: RefCell::new(Mark::Text(None)),
            expiring: RefCell::default(),
        };
        clipboard.seen.replace(clipboard.mark_now());
        clipboard
    }

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

    /// The clipboard as the probe finds it now.
    fn mark_now(&self) -> Mark {
        match self.probe {
            ChangeProbe::Count(count) => Mark::Count(count()),
            ChangeProbe::Text => Mark::Text(self.text_now()),
        }
    }

    /// Writes the text of `items`' one item, treated as `options` says, or empties the clipboard
    /// for none: the app's change, so it is seen, and its expiry, if it has one, replaces the last
    /// copy's.
    fn write_text(
        &self,
        items: Vec<ClipboardItem>,
        options: CopyOptions,
    ) -> Result<(), ClipboardError> {
        let text = match items.as_slice() {
            [] => None,
            [item] => Some(text_of(item)?),
            _ => return Err(ClipboardError::Unsupported),
        };
        self.with_clipboard(|clipboard| match &text {
            Some(text) => treated_as(clipboard.set(), options).text(text.as_str()),
            None => clipboard.clear(),
        })
        .ok_or_else(|| ClipboardError::Failed("the system has no clipboard".to_owned()))?
        .map_err(|error| ClipboardError::Failed(error.to_string()))?;
        let copy = self.mark_now();
        self.seen.replace(copy.clone());
        let expiry = options.expires_after.map(|after| Expiry {
            at: Instant::now() + after,
            copy,
        });
        self.expiring.replace(expiry);
        Ok(())
    }
}

/// `set`, treated as `options` says, as far as Windows' clipboard carries it.
#[cfg(target_os = "windows")]
fn treated_as(set: arboard::Set<'_>, options: CopyOptions) -> arboard::Set<'_> {
    use arboard::SetExtWindows;

    let set = if options.sensitive {
        set.exclude_from_history().exclude_from_monitoring()
    } else {
        set
    };
    if options.this_device_only {
        set.exclude_from_cloud()
    } else {
        set
    }
}

/// `set`, treated as `options` says, as far as Linux's clipboard carries it: it puts no copy on
/// another device.
#[cfg(not(target_os = "windows"))]
fn treated_as(set: arboard::Set<'_>, options: CopyOptions) -> arboard::Set<'_> {
    use arboard::SetExtLinux;

    if options.sensitive {
        set.exclude_from_history()
    } else {
        set
    }
}

/// The text of `item`, which must be offered as text alone.
fn text_of(item: &ClipboardItem) -> Result<String, ClipboardError> {
    match item.representations.as_slice() {
        [(mime, Content::Data(bytes))] if mime == TEXT => {
            Ok(String::from_utf8_lossy(bytes).into_owned())
        }
        _ => Err(ClipboardError::Unsupported),
    }
}

impl Clipboard for TextClipboard {
    /// The text, read in the call, as one item.
    fn items(&self) -> ClipboardItemsFuture {
        let items = self
            .text_now()
            .map(|text| Box::new(TextItem::new(text)) as Box<dyn DataTransferItem>)
            .into_iter()
            .collect();
        Box::pin(std::future::ready(items))
    }

    /// Written in the call, since the clipboard takes it at once.
    fn write(&self, items: Vec<ClipboardItem>, options: CopyOptions) -> ClipboardWriteFuture {
        Box::pin(std::future::ready(self.write_text(items, options)))
    }
}

impl ClipboardChanges for TextClipboard {
    /// Whether the clipboard is another than at the last look, by the system's probe.
    fn has_changed(&self) -> bool {
        let now = self.mark_now();
        self.seen.replace(now.clone()) != now
    }

    fn expiry(&self) -> Option<Instant> {
        self.expiring.borrow().as_ref().map(|expiry| expiry.at)
    }

    /// Clears the app's copy once its time has come, if the clipboard still holds it: by the
    /// text on Linux, so another writer's copy of the same text is cleared too.
    fn clear_expired(&self, now: Instant) -> bool {
        if self.expiry().is_none_or(|at| at > now) {
            return false;
        }
        let Some(expired) = self.expiring.take() else {
            return false;
        };
        if self.mark_now() != expired.copy {
            return false;
        }
        let cleared = self.with_clipboard(|clipboard| clipboard.clear().is_ok());
        self.seen.replace(self.mark_now());
        cleared == Some(true)
    }
}

/// The text on the clipboard as it was listed, which a read answers whatever the clipboard
/// holds by then.
struct TextItem {
    text: String,
    types: [String; 1],
}

impl TextItem {
    fn new(text: String) -> TextItem {
        TextItem {
            text,
            types: [TEXT.to_owned()],
        }
    }
}

impl DataTransferItem for TextItem {
    fn types(&self) -> &[String] {
        &self.types
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        let read = if mime == TEXT {
            Ok(self.text.clone().into_bytes())
        } else {
            Err(ClipboardError::NotFound)
        };
        Box::pin(std::future::ready(read))
    }
}
