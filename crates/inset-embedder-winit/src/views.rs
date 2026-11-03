//! Every view the app has, whoever holds it: the one registry of them, which the main loop and the
//! platform share, the numbers the views take, and whether the system gives them surfaces.
//!
//! A view comes, changes and goes as Flutter's embedder API has it, whoever holds it: its holder
//! makes it, and the main loop adds it on the layer its holder gives it, gives it new metrics and
//! removes it (`main_loop/views.rs`). The registry does the view's own part of each: the view draws
//! into its layer, on a surface while the system gives the views theirs, and joins or leaves the
//! app's views. A window's code is one holder, which keeps what is a window's (`windows.rs`).
//!
//! The screen view, where there is one, is Flutter's implicit view: the app has it from the start,
//! at no size and on no layer, and it draws into its window's layer once the window opens.
//!
//! Every call borrows the registry for itself alone and answers copies, so a caller may call the
//! client with what it got, and the client may reach the registry through the platform meanwhile.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use inset_embedder::{View, ViewId, ViewMetrics};

use crate::os::ViewLayer;
use crate::view::WinitView;

/// The screen view's id, as in Flutter: every other view is numbered after it.
pub(crate) const SCREEN_VIEW: ViewId = ViewId(0);

/// Every view the app has, in the order they came.
pub(crate) struct Views {
    /// By view: views are numbered as they come, so this is the order they came in.
    views: RefCell<BTreeMap<ViewId, Rc<WinitView>>>,
    /// The number the next view takes.
    next_id: Cell<u64>,
    /// Whether the system gives the views surfaces now: from winit's `can_create_surfaces` until
    /// its `destroy_surfaces`.
    surfaces_given: Cell<bool>,
}

impl Default for Views {
    fn default() -> Views {
        Views {
            views: RefCell::default(),
            next_id: Cell::new(SCREEN_VIEW.0 + 1),
            surfaces_given: Cell::new(false),
        }
    }
}

impl Views {
    /// The number a view about to be made takes, which no view had before.
    pub(crate) fn take_id(&self) -> ViewId {
        let id = ViewId(self.next_id.get());
        self.next_id.set(id.0 + 1);
        id
    }

    /// The screen view is the app's from its start, at no size and on no layer, as Flutter's engine
    /// adds its implicit view with empty metrics.
    pub(crate) fn add_screen_view(&self, view: Rc<WinitView>) {
        self.insert(view);
    }

    /// `view` draws into `layer` at `metrics` from now on, and is one of the app's views: the
    /// view's own part of Flutter's `AddView`.
    pub(crate) fn add(&self, view: Rc<WinitView>, layer: ViewLayer, metrics: ViewMetrics) {
        self.draw_into(&view, layer, metrics);
        self.insert(view);
    }

    /// The screen view draws into `layer` from now on, at the metrics it has: its window opened.
    pub(crate) fn place_screen_view(&self, layer: ViewLayer) {
        let view = self
            .get(SCREEN_VIEW)
            .expect("the screen view was added as the app started");
        self.draw_into(&view, layer, view.metrics());
    }

    /// The view numbered `id` is the app's no more, and lets go of its layer, while the framework
    /// and the app may still hold it, drawing nowhere: the view's own part of Flutter's
    /// `RemoveView`. Answers whether it was the app's.
    pub(crate) fn remove(&self, id: ViewId) -> bool {
        let Some(view) = self.views.borrow_mut().remove(&id) else {
            return false;
        };
        view.remove();
        true
    }

    /// The system gives the views surfaces, at launch and each time it gives them back: every view
    /// gets one, made from its layer at its next draw, and so does every view added until the
    /// system takes them away.
    pub(crate) fn give_surfaces(&self) {
        self.surfaces_given.set(true);
        for view in self.all() {
            view.give_surface();
        }
    }

    /// The system takes every view's surface away: each view keeps its last scene, owed until it is
    /// given a surface again.
    pub(crate) fn take_surfaces(&self) {
        self.surfaces_given.set(false);
        for view in self.all() {
            view.take_surface();
        }
    }

    /// The view numbered `id`.
    pub(crate) fn get(&self, id: ViewId) -> Option<Rc<WinitView>> {
        self.views.borrow().get(&id).map(Rc::clone)
    }

    /// Every view, in the order they came.
    pub(crate) fn all(&self) -> Vec<Rc<WinitView>> {
        self.views.borrow().values().map(Rc::clone).collect()
    }

    /// `view` draws into `layer` at `metrics`, on a surface at once while the system gives the
    /// views theirs.
    fn draw_into(&self, view: &WinitView, layer: ViewLayer, metrics: ViewMetrics) {
        view.add(layer, metrics);
        if self.surfaces_given.get() {
            view.give_surface();
        }
    }

    fn insert(&self, view: Rc<WinitView>) {
        let id = view.id();
        assert!(
            self.views.borrow_mut().insert(id, view).is_none(),
            "duplicate view id {id:?}"
        );
    }
}
