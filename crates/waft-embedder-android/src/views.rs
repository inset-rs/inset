//! Every view the app has: the one registry of them, which the loop and the platform share, the
//! numbers the views take, and the frames they asked for.
//!
//! A view is an activity's window (`view.rs`). The app has one activity today, so the registry
//! holds one view, but nothing here counts on that: a view is reached by its id, and a frame is
//! asked for and drawn by view. Flutter's Android embedding has no counterpart, since its shell
//! feeds one implicit view and several activities are several engines; Waft's desktop host keeps
//! the same registry for its windows (`waft-embedder-winit`'s `views.rs`).
//!
//! The invariants a later editor must keep: a number is taken once and never given to another
//! view; and a view's asking for a frame is held until a refresh finds it with a window, as
//! `Platform::request_frame` promises, so a view in the background is not drawn and not forgotten.
//!
//! Every call borrows the registry for itself alone and answers copies, so a caller may call the
//! client with what it got, and the client may reach the registry through the platform meanwhile.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use waft_embedder::{View, ViewId};

use crate::view::AndroidView;

/// Every view the app has, in the order they came, and those that asked for a frame.
#[derive(Default)]
pub(crate) struct Views {
    /// By view: views are numbered as they come, so this is the order they came in.
    views: RefCell<BTreeMap<ViewId, Rc<AndroidView>>>,
    /// The number the next view takes.
    next_id: Cell<u64>,
    /// The views that asked for a frame and have not been in one since.
    frame_requests: RefCell<BTreeSet<ViewId>>,
}

impl Views {
    /// The number a view about to be made takes, which no view had before.
    pub(crate) fn take_id(&self) -> ViewId {
        let id = ViewId(self.next_id.get());
        self.next_id.set(id.0 + 1);
        id
    }

    /// `view` is one of the app's views from now on.
    pub(crate) fn add(&self, view: Rc<AndroidView>) {
        self.views.borrow_mut().insert(view.id(), view);
    }

    /// The view numbered `id`, if it is one of the app's.
    pub(crate) fn get(&self, id: ViewId) -> Option<Rc<AndroidView>> {
        self.views.borrow().get(&id).cloned()
    }

    /// Every view the app has, in the order they came.
    pub(crate) fn all(&self) -> Vec<Rc<AndroidView>> {
        self.views.borrow().values().cloned().collect()
    }

    /// Whether the system gives any view a window now.
    pub(crate) fn any_has_window(&self) -> bool {
        self.views.borrow().values().any(|view| view.has_window())
    }

    /// The view numbered `id` asks for a frame. Answers whether that is news: not for a view that
    /// is not the app's, and not for one already waiting for its frame.
    pub(crate) fn request_frame(&self, id: ViewId) -> bool {
        self.views.borrow().contains_key(&id) && self.frame_requests.borrow_mut().insert(id)
    }

    /// Whether a view that asked for a frame has a window to draw it on.
    pub(crate) fn any_frame_due(&self) -> bool {
        !self.frames_due().is_empty()
    }

    /// The views a frame is drawn for now, those that asked and have a window, their asking
    /// answered; a view without a window goes on waiting.
    pub(crate) fn take_frames_due(&self) -> Vec<ViewId> {
        let due = self.frames_due();
        let mut requests = self.frame_requests.borrow_mut();
        for id in &due {
            requests.remove(id);
        }
        due
    }

    /// The views that asked for a frame and have a window.
    fn frames_due(&self) -> Vec<ViewId> {
        let views = self.views.borrow();
        self.frame_requests
            .borrow()
            .iter()
            .copied()
            .filter(|id| views.get(id).is_some_and(|view| view.has_window()))
            .collect()
    }
}
