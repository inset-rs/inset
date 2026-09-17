//! No Flutter counterpart: a listener for files dragged from outside the application, in
//! the shape of `app_lifecycle_listener.dart`.

use std::fmt::{self, Debug};
use std::path::PathBuf;
use std::rc::Rc;

use inset_foundation::{App, Handle};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};

pub use inset_embedder::{DropChange, DropData};

/// Called for every report of an external drag: entered, moved, dropped, exited.
pub type DropChangeCallback = Rc<dyn Fn(&mut App, &DropData)>;
/// Called with the files let go over a view.
pub type DroppedCallback = Rc<dyn Fn(&mut App, &[PathBuf])>;

/// Hears files dragged from outside the application over the application's views, through
/// the [`WidgetsBinding`]'s observers. Dispose it when done, as an
/// [`AppLifecycleListener`](crate::AppLifecycleListener).
pub struct DropListener {
    /// The [`WidgetsBinding`] listened to.
    pub binding: Handle<WidgetsBinding>,
    /// Called for every report, whatever its phase.
    pub on_change: Option<DropChangeCallback>,
    /// Called with the dropped files.
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
        on_change: DropChangeCallback,
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
    fn did_change_drop(self: Handle<Self>, app: &mut App, data: &DropData) {
        let (on_change, on_dropped) = {
            let this = app.get(self);
            (this.on_change.clone(), this.on_dropped.clone())
        };
        if let Some(on_change) = on_change {
            on_change(app, data);
        }
        if data.change == DropChange::Dropped
            && let Some(on_dropped) = on_dropped
        {
            on_dropped(app, &data.paths);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use inset_embedder::ViewId;
    use inset_foundation::AppCell;

    use super::*;

    fn dropped(paths: &[&str]) -> DropData {
        DropData {
            view_id: ViewId(0),
            change: DropChange::Dropped,
            physical_position: None,
            paths: paths.iter().map(PathBuf::from).collect(),
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
            Rc::new(move |_app, paths| seen_slot.borrow_mut().extend_from_slice(paths)),
        );
        WidgetsBinding::instance(app).handle_drop(app, &dropped(&["/a.wasm", "/b.wasm"]));
        assert_eq!(
            *seen.borrow(),
            [PathBuf::from("/a.wasm"), PathBuf::from("/b.wasm")]
        );
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
        WidgetsBinding::instance(app).handle_drop(app, &dropped(&["/a.wasm"]));
        assert_eq!(*seen.borrow(), 0);
    }
}
