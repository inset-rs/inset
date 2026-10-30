//! What is dragged from outside the application over a view: the same items a paste hands over.
//!
//! No dart:ui counterpart: Flutter leaves external drops to plugins. A report is a drag in
//! `PointerData`'s terms, a view, a phase and a physical position, carrying what is dragged in the
//! one model every system has for paste and for drag and drop alike: macOS's dragging pasteboard is
//! an `NSPasteboard`, Windows' drop hands an `IDataObject`, Wayland's drag a data offer, Android's
//! `DragEvent` a `ClipData`, the web's `DragEvent` a `DataTransfer`. So a drag carries
//! [`DataTransferItem`]s, as a paste does, a file among them as a pasted file is: offered as
//! [`URI_LIST`](crate::URI_LIST).
//!
//! The invariant: a drag passing over the app does not hand it what is dragged. As on the web, an
//! item's types are seen from [`DragChange::Entered`] on, and its content is read only while the
//! drop is reported: a read asked for before fails with [`ClipboardError::Denied`], one asked for
//! after with [`ClipboardError::Changed`], as every system lets a drop's data go once its drop is
//! handled; one asked for during the report may complete later. A host hands out the items of a
//! [`DraggedItems`], which holds them to it.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use crate::{ClipboardError, ClipboardReadFuture, DataTransferItem, Offset, ViewId};

/// One report of an external drag over a view.
#[derive(Clone)]
pub struct DragData {
    /// The [`crate::View`] the drag is over.
    pub view_id: ViewId,
    /// What happened since the last report.
    pub change: DragChange,
    /// Where over the view, in physical pixels, when the host knows: a host may report no
    /// position at all, and none reports one on [`DragChange::Exited`].
    pub physical_position: Option<Offset>,
    /// What is dragged, as a paste reads it: each item's types, each read when asked, while the
    /// drop is reported. The same items in every report of one drag; a file is an item as a
    /// pasted file is.
    pub items: Vec<Rc<dyn DataTransferItem>>,
}

impl fmt::Debug for DragData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let types: Vec<&[String]> = self.items.iter().map(|item| item.types()).collect();
        f.debug_struct("DragData")
            .field("view_id", &self.view_id)
            .field("change", &self.change)
            .field("physical_position", &self.physical_position)
            .field("items", &types)
            .finish()
    }
}

/// The four phases of an external drag, as every toolkit has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragChange {
    /// The drag came over the view.
    Entered,
    /// The drag moved over the view.
    Moved,
    /// What is dragged was let go over the view: the one report its items are read in.
    Dropped,
    /// The drag left the view without dropping.
    Exited,
}

/// The items of one drag as a host hands them out, readable only while their drop is reported
/// ([`DraggedItems::report_drop`]).
pub struct DraggedItems {
    reading: Rc<Cell<Reading>>,
    items: Vec<Rc<dyn DataTransferItem>>,
}

/// Whether a drag's items may be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    /// The drag is passing over: nothing is read.
    NotDropped,
    /// Its drop is being reported: what is asked for is read.
    Dropping,
    /// Its drop was reported, and the system has let its data go.
    Over,
}

impl DraggedItems {
    /// `items` as the system lists them, each read only while their drop is reported.
    pub fn new(items: Vec<Box<dyn DataTransferItem>>) -> DraggedItems {
        let reading = Rc::new(Cell::new(Reading::NotDropped));
        let items = items
            .into_iter()
            .map(|item| {
                Rc::new(DraggedItem {
                    item,
                    reading: Rc::clone(&reading),
                }) as Rc<dyn DataTransferItem>
            })
            .collect();
        DraggedItems { reading, items }
    }

    /// Whether the drag carries nothing the app could read, which a host refuses.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items, for a report.
    pub fn items(&self) -> Vec<Rc<dyn DataTransferItem>> {
        self.items.clone()
    }

    /// Runs `report`, which reports the drop: the items are read while it runs, and never after.
    pub fn report_drop<R>(&self, report: impl FnOnce() -> R) -> R {
        self.reading.set(Reading::Dropping);
        let _over = ReadingEnds(&self.reading);
        report()
    }
}

/// Ends the reading of a drag's items as the report of its drop returns, or unwinds.
struct ReadingEnds<'a>(&'a Cell<Reading>);

impl Drop for ReadingEnds<'_> {
    fn drop(&mut self) {
        self.0.set(Reading::Over);
    }
}

/// An item of a drag, read through only while its drop is reported.
struct DraggedItem {
    item: Box<dyn DataTransferItem>,
    reading: Rc<Cell<Reading>>,
}

impl DataTransferItem for DraggedItem {
    fn types(&self) -> &[String] {
        self.item.types()
    }

    fn read(&self, mime: &str) -> ClipboardReadFuture {
        match self.reading.get() {
            Reading::NotDropped => Box::pin(std::future::ready(Err(ClipboardError::Denied))),
            Reading::Dropping => self.item.read(mime),
            Reading::Over => Box::pin(std::future::ready(Err(ClipboardError::Changed))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    use super::*;

    /// Text, read in the call.
    struct Text([String; 1]);

    impl Text {
        fn item() -> Box<dyn DataTransferItem> {
            Box::new(Text(["text/plain".to_owned()]))
        }
    }

    impl DataTransferItem for Text {
        fn types(&self) -> &[String] {
            &self.0
        }

        fn read(&self, _mime: &str) -> ClipboardReadFuture {
            Box::pin(std::future::ready(Ok(b"hi".to_vec())))
        }
    }

    fn now<T>(mut future: Pin<Box<dyn Future<Output = T>>>) -> T {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("the item answers in the call"),
        }
    }

    #[test]
    fn an_item_shows_its_types_while_the_drag_passes_over_but_not_its_content() {
        let dragged = DraggedItems::new(vec![Text::item()]);
        let items = dragged.items();
        assert_eq!(items[0].types(), ["text/plain"]);
        assert_eq!(
            now(items[0].read("text/plain")),
            Err(ClipboardError::Denied)
        );
    }

    #[test]
    fn an_item_is_read_while_its_drop_is_reported_and_not_after() {
        let dragged = DraggedItems::new(vec![Text::item()]);
        let items = dragged.items();
        let read = dragged.report_drop(|| now(items[0].read("text/plain")));
        assert_eq!(read, Ok(b"hi".to_vec()));
        assert_eq!(
            now(items[0].read("text/plain")),
            Err(ClipboardError::Changed)
        );
    }

    #[test]
    fn a_read_asked_for_during_the_drop_completes_after_it() {
        let dragged = DraggedItems::new(vec![Text::item()]);
        let items = dragged.items();
        let read = dragged.report_drop(|| items[0].read("text/plain"));
        assert_eq!(now(read), Ok(b"hi".to_vec()));
    }

    #[test]
    fn a_drag_carrying_nothing_is_empty() {
        assert!(DraggedItems::new(Vec::new()).is_empty());
        assert!(!DraggedItems::new(vec![Text::item()]).is_empty());
    }
}
