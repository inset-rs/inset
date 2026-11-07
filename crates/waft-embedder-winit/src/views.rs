//! Every view the app has, whoever holds it: the one registry of them, which the main loop and the
//! platform share, the numbers the views take, and whether the system gives them surfaces.
//!
//! A view comes, changes and goes as Flutter's embedder API has it, whoever holds it: its holder
//! makes it, and the main loop adds it on the layer its holder gives it, gives it new metrics and
//! removes it (`main_loop/views.rs`). The registry does the view's own part of each: the view draws
//! into its layer, on a surface while the system gives the views theirs, and joins or leaves the
//! app's views. A window's code is one holder, which keeps what is a window's (`windows.rs`); the
//! app is another, for the views it adds on layers of its own (`platform.rs`). Each holder says it
//! holds a view as it adds it, and the registry keeps who: the app's asking to resize a view a
//! window's code holds, to say whether it is in sight, or to remove it, is refused.
//!
//! The screen view, where there is one, is Flutter's implicit view: the app has it from the start,
//! at no size and on no layer, and it draws into its window's layer once the window opens.
//!
//! Every call borrows the registry for itself alone and answers copies, so a caller may call the
//! client with what it got, and the client may reach the registry through the platform meanwhile.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use waft_embedder::{View, ViewId, ViewMetrics};

use crate::os::ViewLayer;
use crate::view::WinitView;

/// The screen view's id, as in Flutter: every other view is numbered after it.
pub(crate) const SCREEN_VIEW: ViewId = ViewId(0);

/// Who holds a view, as it says when it adds the view: the view's metrics, sight and end are its.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Holder {
    /// A window's code, for its window's view, the screen view's included.
    Window,
    /// The app, for a view it adds on a layer of its own.
    App,
}

/// A view of the app's, and who holds it.
struct Held {
    view: Rc<WinitView>,
    holder: Holder,
}

/// Every view the app has, in the order they came.
pub(crate) struct Views {
    /// By view: views are numbered as they come, so this is the order they came in.
    views: RefCell<BTreeMap<ViewId, Held>>,
    /// The number the next view takes.
    next_id: Cell<u64>,
    /// Whether the system gives the views surfaces now: from winit's `can_create_surfaces` until
    /// its `destroy_surfaces`.
    surfaces_given: Cell<bool>,
    /// Whether the app's asking to change a view it did not add was refused yet.
    refusal_reported: Cell<bool>,
}

impl Default for Views {
    fn default() -> Views {
        Views {
            views: RefCell::default(),
            next_id: Cell::new(SCREEN_VIEW.0 + 1),
            surfaces_given: Cell::new(false),
            refusal_reported: Cell::new(false),
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
        self.insert(view, Holder::Window);
    }

    /// `view` draws into `layer` at `metrics` from now on, and is one of the app's views, held by
    /// `holder`: the view's own part of Flutter's `AddView`.
    pub(crate) fn add(
        &self,
        view: Rc<WinitView>,
        layer: ViewLayer,
        metrics: ViewMetrics,
        holder: Holder,
    ) {
        self.draw_into(&view, layer, metrics);
        self.insert(view, holder);
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
        let Some(held) = self.views.borrow_mut().remove(&id) else {
            return false;
        };
        held.view.remove();
        true
    }

    /// Whether the app may give the view numbered `id` metrics, say whether it is in sight, or
    /// remove it: one the app holds. One a window's code holds is refused, the first refusal
    /// reported; one that is gone does nothing.
    pub(crate) fn app_may_change(&self, id: ViewId) -> bool {
        let Some(holder) = self.views.borrow().get(&id).map(|held| held.holder) else {
            return false;
        };
        let held_by_app = holder == Holder::App;
        if !held_by_app && !self.refusal_reported.replace(true) {
            eprintln!("waft: the app asked to change a view it did not add; refused");
        }
        held_by_app
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
        self.views
            .borrow()
            .get(&id)
            .map(|held| Rc::clone(&held.view))
    }

    /// Every view, in the order they came.
    pub(crate) fn all(&self) -> Vec<Rc<WinitView>> {
        self.views
            .borrow()
            .values()
            .map(|held| Rc::clone(&held.view))
            .collect()
    }

    /// `view` draws into `layer` at `metrics`, on a surface at once while the system gives the
    /// views theirs.
    fn draw_into(&self, view: &WinitView, layer: ViewLayer, metrics: ViewMetrics) {
        view.add(layer, metrics);
        if self.surfaces_given.get() {
            view.give_surface();
        }
    }

    fn insert(&self, view: Rc<WinitView>, holder: Holder) {
        let id = view.id();
        let held = Held { view, holder };
        assert!(
            self.views.borrow_mut().insert(id, held).is_none(),
            "duplicate view id {id:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{
        SystemMouseCursorKind, TextInputHost, ViewFocusDirection, ViewFocusState, ViewHost,
    };

    use super::*;
    use crate::gpu::LazyGpu;

    /// A view's host that is asked nothing here.
    struct NoHost;

    impl ViewHost for NoHost {
        fn text_input(&self) -> Option<&dyn TextInputHost> {
            None
        }

        fn activate_system_cursor(&self, _device: i64, _kind: SystemMouseCursorKind) {}

        fn request_focus_change(&self, _state: ViewFocusState, _direction: ViewFocusDirection) {}
    }

    /// A view numbered by `views`, on no layer and in no window.
    fn view_of(views: &Views) -> Rc<WinitView> {
        let view = WinitView::new(views.take_id(), &LazyGpu::default(), false, Rc::new(NoHost));
        Rc::new(view)
    }

    #[test]
    fn the_app_may_change_a_view_it_holds_and_not_one_a_window_holds_though_in_no_window() {
        let views = Views::default();
        let windows = view_of(&views);
        let apps = view_of(&views);
        views.insert(Rc::clone(&windows), Holder::Window);
        views.insert(Rc::clone(&apps), Holder::App);

        assert!(!views.app_may_change(windows.id()));
        assert!(views.app_may_change(apps.id()));
    }
}
