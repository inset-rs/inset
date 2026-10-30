//! No Flutter counterpart: a listener for what is dragged from outside the application, in
//! the shape of `app_lifecycle_listener.dart`.
//!
//! What is dragged is a drag's items, as a paste's are: their types seen as the drag passes over,
//! their content read only while the drop is reported, so a listener starts every read it wants
//! in its callback for the drop, and the reads complete later.

use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_foundation::{App, Handle};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};

pub use inset_embedder::{DataTransferItem, DragChange, DragData};

/// Called for every report of an external drag: entered, moved, dropped, exited.
pub type DragChangeCallback = Rc<dyn Fn(&mut App, &DragData)>;
/// Called with the items let go over a view, to read while it runs.
pub type DroppedCallback = Rc<dyn Fn(&mut App, &[Rc<dyn DataTransferItem>])>;

/// Hears what is dragged from outside the application over the application's views, through
/// the [`WidgetsBinding`]'s observers. Dispose it when done, as an
/// [`AppLifecycleListener`](crate::AppLifecycleListener).
pub struct DropListener {
    /// The [`WidgetsBinding`] listened to.
    pub binding: Handle<WidgetsBinding>,
    /// Called for every report, whatever its phase.
    pub on_change: Option<DragChangeCallback>,
    /// Called with the dropped items.
    pub on_dropped: Option<DroppedCallback>,
    observer: Option<WidgetsBindingObserverRef>,
}

impl Debug for DropListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropListener")
            .field("binding", &self.binding)
            .finish()
    }
}

impl DropListener {
    /// A listener registered with the current [`WidgetsBinding`].
    pub fn new(app: &mut App) -> Handle<DropListener> {
        let binding = WidgetsBinding::instance(app);
        let this = app.create(DropListener {
            binding,
            on_change: None,
            on_dropped: None,
            observer: None,
        });
        let observer: WidgetsBindingObserverRef = Rc::new(this);
        app.get_mut(this).observer = Some(Rc::clone(&observer));
        binding.add_observer(app, observer);
        this
    }

    pub fn on_change(
        self: Handle<Self>,
        app: &mut App,
        on_change: DragChangeCallback,
    ) -> Handle<Self> {
        app.get_mut(self).on_change = Some(on_change);
        self
    }

    pub fn on_dropped(
        self: Handle<Self>,
        app: &mut App,
        on_dropped: DroppedCallback,
    ) -> Handle<Self> {
        app.get_mut(self).on_dropped = Some(on_dropped);
        self
    }

    /// Stops listening.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let binding = app.get(self).binding;
        if let Some(observer) = app.get_mut(self).observer.take() {
            binding.remove_observer(app, &observer);
        }
    }
}

impl WidgetsBindingObserverObject for DropListener {
    fn did_change_drag(self: Handle<Self>, app: &mut App, data: &DragData) {
        let (on_change, on_dropped) = {
            let this = app.get(self);
            (this.on_change.clone(), this.on_dropped.clone())
        };
        if let Some(on_change) = on_change {
            on_change(app, data);
        }
        if data.change == DragChange::Dropped
            && let Some(on_dropped) = on_dropped
        {
            on_dropped(app, &data.items);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use inset_embedder::{ClipboardReadFuture, URI_LIST, ViewId};
    use inset_foundation::AppCell;

    use super::*;

    /// A dropped file, offered as its `file:` URL.
    struct DroppedFile {
        types: [String; 1],
        uri: &'static str,
    }

    impl DataTransferItem for DroppedFile {
        fn types(&self) -> &[String] {
            &self.types
        }

        fn read(&self, _mime: &str) -> ClipboardReadFuture {
            Box::pin(std::future::ready(Ok(self.uri.as_bytes().to_vec())))
        }
    }

    fn dropped(uris: &[&'static str]) -> DragData {
        DragData {
            view_id: ViewId(0),
            change: DragChange::Dropped,
            physical_position: None,
            items: uris
                .iter()
                .map(|&uri| {
                    Rc::new(DroppedFile {
                        types: [URI_LIST.to_owned()],
                        uri,
                    }) as Rc<dyn DataTransferItem>
                })
                .collect(),
        }
    }

    #[test]
    fn a_drop_reaches_the_listener_through_the_binding() {
        let cell = AppCell::new();
        let app = &mut cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_slot = Rc::clone(&seen);
        DropListener::new(app).on_dropped(
            app,
            Rc::new(move |_app, items| {
                let types = items.iter().map(|item| item.types().to_vec());
                seen_slot.borrow_mut().extend(types);
            }),
        );
        let data = dropped(&["file:///a.wasm", "file:///b.wasm"]);
        WidgetsBinding::instance(app).handle_drag(app, &data);
        assert_eq!(*seen.borrow(), [[URI_LIST], [URI_LIST]]);
    }

    #[test]
    fn a_disposed_listener_hears_nothing() {
        let cell = AppCell::new();
        let app = &mut cell.borrow_mut();
        let seen = Rc::new(RefCell::new(0));
        let seen_slot = Rc::clone(&seen);
        let listener = DropListener::new(app).on_change(
            app,
            Rc::new(move |_app, _data| *seen_slot.borrow_mut() += 1),
        );
        listener.dispose(app);
        WidgetsBinding::instance(app).handle_drag(app, &dropped(&["file:///a.wasm"]));
        assert_eq!(*seen.borrow(), 0);
    }
}
