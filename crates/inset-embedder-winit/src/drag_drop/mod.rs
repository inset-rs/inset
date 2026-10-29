//! What is dragged over a window, as winit reports a drag: as it enters a window, at each move over
//! it, as it is dropped and as it leaves, by the id of its data transfer.
//!
//! What a drag carries is read as the system's clipboard reads a paste where the embedder has code
//! of its own for the system's drags, macOS's dragging pasteboard (`os/macos/pasteboard`), and
//! through winit's data transfer elsewhere (`transfer`). Its items are listed as it enters, and the
//! app hears it enter at once; a drag carrying nothing the app could read is refused, and the app
//! hears nothing of it.
//!
//! The invariant: the items' content is read only while the drop is reported
//! (`inset_embedder::DraggedItems`), and the reads the app asked of winit then are fetched as the
//! report returns, while winit still holds the drop's data (`transfer`).

mod transfer;

use std::collections::HashMap;

use inset_embedder::{DataTransferItem, DragChange, DragData, DraggedItems, Offset, ViewId};
use winit::data_transfer::{DataTransferId, TransferType, TypedData};
use winit::error::TransferError;
use winit::event_loop::{ActiveEventLoop, AsyncRequestSerial};

use crate::os::{SystemDrags, ThisOs};
use transfer::{Fetches, TransferForms};

/// The drags over the views, by their transfer, and the reads of dropped ones winit has yet to
/// answer.
#[derive(Default)]
pub(crate) struct Drags {
    over: HashMap<DataTransferId, Drag>,
    fetches: Fetches,
}

/// One drag over a view.
struct Drag {
    view: ViewId,
    /// Where it was last reported, in the view's physical pixels.
    position: Option<Offset>,
    items: DraggedItems,
    /// Its forms, where it is read through winit.
    forms: Option<TransferForms>,
}

/// What a drag carries, as the embedder read it when it entered.
pub(crate) struct Carried {
    items: Vec<Box<dyn DataTransferItem>>,
    /// The forms the items are read in, where they are read through winit.
    forms: Option<TransferForms>,
}

impl Carried {
    /// What the drag `transfer` carries now: listed as the system's clipboard lists a paste, or
    /// else read through winit.
    pub(crate) fn by(event_loop: &dyn ActiveEventLoop, transfer: DataTransferId) -> Carried {
        if let Some(items) = ThisOs::dragged_items() {
            return Carried { items, forms: None };
        }
        let forms = event_loop
            .data_transfer(transfer)
            .ok()
            .and_then(|offered| TransferForms::offered(&*offered));
        Carried {
            items: forms.iter().map(TransferForms::item).collect(),
            forms,
        }
    }
}

impl Drags {
    /// The drag `transfer` came over `view` at `position`, carrying `carried`: the app hears it
    /// enter, unless it carries nothing the app could read, and it is refused.
    pub(crate) fn entered(
        &mut self,
        transfer: DataTransferId,
        view: ViewId,
        position: Option<Offset>,
        carried: Carried,
    ) -> Option<DragData> {
        let items = DraggedItems::new(carried.items);
        if items.is_empty() {
            return None;
        }
        let drag = Drag {
            view,
            position,
            items,
            forms: carried.forms,
        };
        let report = drag.report(DragChange::Entered);
        self.over.insert(transfer, drag);
        Some(report)
    }

    /// The drag `transfer` moved to `position`.
    pub(crate) fn moved(&mut self, transfer: DataTransferId, position: Offset) -> Option<DragData> {
        let drag = self.over.get_mut(&transfer)?;
        drag.position = Some(position);
        Some(drag.report(DragChange::Moved))
    }

    /// The drag `transfer` was let go: `report` reports it, its items read while it runs, and the
    /// reads they asked of winit are fetched through `fetch` as it returns.
    pub(crate) fn dropped(
        &mut self,
        transfer: DataTransferId,
        report: impl FnOnce(DragData),
        fetch: impl FnMut(&dyn TransferType) -> Result<AsyncRequestSerial, TransferError>,
    ) {
        let Some(drag) = self.over.remove(&transfer) else {
            return;
        };
        drag.items
            .report_drop(|| report(drag.report(DragChange::Dropped)));
        if let Some(forms) = &drag.forms {
            forms.fetch_asked(fetch, &mut self.fetches);
        }
    }

    /// The drag `transfer` left without dropping.
    pub(crate) fn left(&mut self, transfer: DataTransferId) -> Option<DragData> {
        let drag = self.over.remove(&transfer)?;
        Some(DragData {
            physical_position: None,
            ..drag.report(DragChange::Exited)
        })
    }

    /// winit answered the fetch `serial` with `value`: the read waiting for it completes.
    pub(crate) fn fetched(&mut self, serial: AsyncRequestSerial, value: &dyn TypedData) {
        self.fetches.answered(serial, value);
    }
}

impl Drag {
    fn report(&self, change: DragChange) -> DragData {
        DragData {
            view_id: self.view,
            change,
            physical_position: self.position,
            items: self.items.items(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::{Context, Poll, Waker};

    use inset_embedder::{ClipboardError, ClipboardReadFuture};

    use super::*;

    const A: ViewId = ViewId(1);
    const DRAG: DataTransferId = DataTransferId::from_raw(7);

    /// Text, read in the call.
    struct Text([String; 1]);

    impl DataTransferItem for Text {
        fn types(&self) -> &[String] {
            &self.0
        }

        fn read(&self, _mime: &str) -> ClipboardReadFuture {
            Box::pin(std::future::ready(Ok(b"hi".to_vec())))
        }
    }

    fn text() -> Carried {
        Carried {
            items: vec![Box::new(Text(["text/plain".to_owned()]))],
            forms: None,
        }
    }

    fn nothing() -> Carried {
        Carried {
            items: Vec::new(),
            forms: None,
        }
    }

    fn read_now(item: &dyn DataTransferItem) -> Result<Vec<u8>, ClipboardError> {
        let mut read = item.read("text/plain");
        match read.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(read) => read,
            Poll::Pending => panic!("the item answers in the call"),
        }
    }

    fn no_fetch(_form: &dyn TransferType) -> Result<AsyncRequestSerial, TransferError> {
        panic!("a drag read as a paste fetches nothing through winit")
    }

    #[test]
    fn a_drag_enters_with_its_items_moves_and_drops_with_them() {
        let mut drags = Drags::default();
        let entered = drags
            .entered(DRAG, A, Some(Offset::new(1.0, 2.0)), text())
            .unwrap();
        assert_eq!(entered.change, DragChange::Entered);
        assert_eq!(entered.items[0].types(), ["text/plain"]);
        let moved = drags.moved(DRAG, Offset::new(3.0, 4.0)).unwrap();
        assert_eq!(moved.change, DragChange::Moved);
        assert_eq!(moved.physical_position, Some(Offset::new(3.0, 4.0)));

        let mut dropped = None;
        drags.dropped(DRAG, |report| dropped = Some(report), no_fetch);
        let dropped = dropped.unwrap();
        assert_eq!(dropped.change, DragChange::Dropped);
        assert_eq!(dropped.physical_position, Some(Offset::new(3.0, 4.0)));
        assert!(drags.left(DRAG).is_none(), "a dropped drag is gone");
    }

    #[test]
    fn the_items_are_read_only_while_the_drop_is_reported() {
        let mut drags = Drags::default();
        let entered = drags.entered(DRAG, A, None, text()).unwrap();
        assert_eq!(read_now(&*entered.items[0]), Err(ClipboardError::Denied));
        let mut read = None;
        drags.dropped(
            DRAG,
            |report| read = Some(read_now(&*report.items[0])),
            no_fetch,
        );
        assert_eq!(read, Some(Ok(b"hi".to_vec())));
        assert_eq!(read_now(&*entered.items[0]), Err(ClipboardError::Changed));
    }

    #[test]
    fn a_drag_that_leaves_is_reported_without_a_position() {
        let mut drags = Drags::default();
        drags.entered(DRAG, A, Some(Offset::ZERO), text()).unwrap();
        let left = drags.left(DRAG).unwrap();
        assert_eq!(left.change, DragChange::Exited);
        assert_eq!(left.physical_position, None);
    }

    #[test]
    fn a_drag_carrying_nothing_is_refused_and_forgotten() {
        let mut drags = Drags::default();
        assert!(drags.entered(DRAG, A, None, nothing()).is_none());
        assert!(drags.moved(DRAG, Offset::ZERO).is_none());
        assert!(drags.left(DRAG).is_none());
    }
}
