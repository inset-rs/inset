//! Every view the app has: the one registry of them, which the host and the platform share, the
//! numbers the views take, and the frames they asked for.
//!
//! A view is one scene's window (`view.rs`), and outlives the scene: UIKit disconnects a scene in
//! the background to free its memory and connects it again, by the same session, when the user
//! comes back, and the view the session had is the scene's again. The view the app is given at
//! launch, Flutter's implicit view, goes to the first scene that connects. A view is reached by its
//! id, or by its scene's session, and a frame is asked for and drawn by view. Flutter's iOS
//! embedding has no counterpart, since its engine feeds one view controller in one scene.
//!
//! The invariant a later editor must keep: a view's asking for a frame is held until a refresh
//! finds it in sight, its scene connected and in the foreground, as `Platform::request_frame`
//! promises, so a view in the background is neither drawn, which iOS forbids a process there, nor
//! forgotten.
//!
//! Every call borrows the registry for itself alone and answers copies, so a caller may call the
//! client with what it got, and the client may reach the registry through the platform meanwhile.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2_ui_kit::UIViewController;
use waft_embedder::{View, ViewId};

use crate::view::IosView;

/// Flutter's implicit view's id, the view the app has from launch: the first view numbered.
pub(crate) const IMPLICIT_VIEW: ViewId = ViewId(0);

/// Every view the app has, in the order they came, and those that asked for a frame.
#[derive(Default)]
pub(crate) struct Views {
    /// By view: views are numbered as they come, so this is the order they came in.
    views: RefCell<BTreeMap<ViewId, Rc<IosView>>>,
    /// The views that asked for a frame and have not been in one since.
    frame_requests: RefCell<BTreeSet<ViewId>>,
    /// The number the next view takes.
    next_id: Cell<u64>,
}

impl Views {
    /// The number a view about to be made takes, which no view had before: the implicit view's
    /// first.
    pub(crate) fn take_id(&self) -> ViewId {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        ViewId(id)
    }

    /// `view` is one of the app's views from now on.
    pub(crate) fn add(&self, view: Rc<IosView>) {
        self.views.borrow_mut().insert(view.id(), view);
    }

    /// The view numbered `id` is the app's no more, nor the frame it asked for.
    pub(crate) fn remove(&self, id: ViewId) -> Option<Rc<IosView>> {
        self.frame_requests.borrow_mut().remove(&id);
        self.views.borrow_mut().remove(&id)
    }

    /// The view numbered `id`, if it is one of the app's.
    pub(crate) fn get(&self, id: ViewId) -> Option<Rc<IosView>> {
        self.views.borrow().get(&id).cloned()
    }

    /// Every view the app has, in the order they came.
    pub(crate) fn all(&self) -> Vec<Rc<IosView>> {
        self.views.borrow().values().cloned().collect()
    }

    /// The view the scene of `session` shows: the one that session had, or else the first view
    /// that never had a scene, the implicit view for the first scene; `None` where every view has
    /// had one, and the scene needs a view of its own.
    pub(crate) fn for_session(&self, session: &str) -> Option<Rc<IosView>> {
        let views = self.views.borrow();
        let had = views
            .values()
            .find(|view| view.session().as_deref() == Some(session));
        let never_placed = || views.values().find(|view| view.session().is_none());
        had.or_else(never_placed).cloned()
    }

    /// The view of `session`, if a scene of that session ever showed one.
    pub(crate) fn of_session(&self, session: &str) -> Option<Rc<IosView>> {
        self.views
            .borrow()
            .values()
            .find(|view| view.session().as_deref() == Some(session))
            .cloned()
    }

    /// The view controller a system sheet is presented from for `parent`: its own while its scene
    /// is connected, else the first view's in sight, else the first connected one's.
    pub(crate) fn presenter(&self, parent: Option<ViewId>) -> Option<Retained<UIViewController>> {
        let views = self.views.borrow();
        let parents = parent.and_then(|id| views.get(&id)?.view_controller());
        let in_sight = || {
            views
                .values()
                .filter(|view| view.is_in_sight())
                .find_map(|view| view.view_controller())
        };
        let connected = || views.values().find_map(|view| view.view_controller());
        parents.or_else(in_sight).or_else(connected)
    }

    /// The view numbered `id` asks for a frame. Answers whether that is news: not for a view that
    /// is not the app's, and not for one already waiting for its frame.
    pub(crate) fn request_frame(&self, id: ViewId) -> bool {
        self.views.borrow().contains_key(&id) && self.frame_requests.borrow_mut().insert(id)
    }

    /// Whether a view that asked for a frame is in sight to draw it.
    pub(crate) fn any_frame_due(&self) -> bool {
        !self.frames_due().is_empty()
    }

    /// The views a frame is drawn for now, those that asked and are in sight, their asking
    /// answered; a view out of sight goes on waiting.
    pub(crate) fn take_frames_due(&self) -> Vec<ViewId> {
        let due = self.frames_due();
        let mut requests = self.frame_requests.borrow_mut();
        for id in &due {
            requests.remove(id);
        }
        due
    }

    /// The views that asked for a frame and are in sight.
    fn frames_due(&self) -> Vec<ViewId> {
        let views = self.views.borrow();
        self.frame_requests
            .borrow()
            .iter()
            .copied()
            .filter(|id| views.get(id).is_some_and(|view| view.is_in_sight()))
            .collect()
    }
}
