//! Flutter counterpart: `WindowScope` from `widgets/_window.dart` (experimental), which there hands
//! a subtree its window's controller; here it hands the window itself.
//!
//! A [`View`](crate::View) in a window, one the app asked for or one the system made or restored,
//! provides a [`WindowScope`] to its content: the window the host keeps around the view
//! (`View::window`), its frame, its title, minimizing, its size limits and its closing. Widgets
//! that read it rebuild when the host reports the window changed, minimized or restored, occluded
//! or shown, or moved, as widgets reading `MediaQuery` rebuild when the view's metrics change.
//!
//! The invariant a later editor will break: a window's change reaches the scope through the
//! binding's crate-private window listeners, never through a public observer method; the scope
//! notifies its dependents at each one.

use std::fmt;
use std::rc::Rc;

use waft_embedder::{ViewRef, WindowRef};
use waft_foundation::{App, Handle, Listener};

use crate::binding::WidgetsBinding;
use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, State, StateData, StatefulWidget, WidgetRef,
};

/// Provides the window a view is in to the widgets below it.
pub struct WindowScope {
    window: WindowRef,
    /// How many changes of the window the host has reported, which tells the scope's dependents
    /// to read it again.
    changes: u64,
    child: WidgetRef,
}

impl WindowScope {
    /// The window the `context` is drawn in, or `None` for a view in no window, as an iPhone's
    /// screen or a web page's element is. The widget of `context` rebuilds when the window
    /// changes.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<WindowRef> {
        context
            .depend_on_inherited_widget_of_exact_type::<WindowScope>(app)
            .map(|scope| Rc::clone(&scope.window))
    }

    /// The window the `context` is drawn in.
    ///
    /// # Panics
    ///
    /// Outside a view in a window.
    pub fn of(app: &mut App, context: BuildContext) -> WindowRef {
        WindowScope::maybe_of(app, context)
            .expect("WindowScope::of() was called with a context that is not inside a window")
    }
}

impl fmt::Debug for WindowScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowScope")
            .field("changes", &self.changes)
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for WindowScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &WindowScope) -> bool {
        self.changes != old_widget.changes || !Rc::ptr_eq(&self.window, &old_widget.window)
    }
}

/// The [`WindowScope`] of `view`'s window, which `View` builds for a view in a window, rebuilt at
/// each change of the window the host reports.
pub(crate) struct WindowScopeOfView {
    pub(crate) view: ViewRef,
    pub(crate) window: WindowRef,
    pub(crate) child: WidgetRef,
}

impl fmt::Debug for WindowScopeOfView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowScopeOfView")
            .field("view", &self.view.id())
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for WindowScopeOfView {
    type State = WindowScopeOfViewState;

    fn create_state(&self) -> WindowScopeOfViewState {
        WindowScopeOfViewState {
            state: StateData::new(),
            changes: 0,
            listener: None,
        }
    }
}

pub(crate) struct WindowScopeOfViewState {
    state: StateData<WindowScopeOfView>,
    changes: u64,
    /// Retains the listener's identity for removal on disposal.
    listener: Option<Listener>,
}

impl WindowScopeOfViewState {
    fn window_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.changes += 1);
    }
}

impl State for WindowScopeOfViewState {
    type Widget = WindowScopeOfView;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let view = self.widget(app).view.id();
        let listener = Listener::handle_method(self, WindowScopeOfViewState::window_changed);
        WidgetsBinding::instance(app).add_window_listener(app, view, listener.clone());
        app.get_mut(self).listener = Some(listener);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(listener) = app.get_mut(self).listener.take() {
            let view = self.widget(app).view.id();
            WidgetsBinding::instance(app).remove_window_listener(app, view, &listener);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        WindowScope {
            window: Rc::clone(&widget.window),
            changes: app.get(self).changes,
            child: widget.child.clone(),
        }
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use waft_embedder::{ViewId, WindowConfig};
    use waft_foundation::AppCell;
    use waft_test::{TestView, TestWindow, TestWindows};

    use super::*;
    use crate::test_harness::{Quits, binding_pump, report_window_changed, views_cell};
    use crate::widgets::basic::Builder;
    use crate::{MultiViewApp, SizedBox};

    /// What the content of each view read of its window, each time it built.
    type Read = Rc<RefCell<Vec<Option<bool>>>>;

    /// An app showing, in each of `views`, content that reads whether its window is minimized.
    fn reading(views: &[Rc<TestView>]) -> (Rc<AppCell>, Read) {
        let cell = views_cell(
            views,
            &Rc::new(TestWindows::new()),
            &Rc::new(Quits::default()),
        );
        let read: Read = Rc::default();
        let kept = Rc::clone(&read);
        let content = Builder::new(move |app, context| {
            let window = WindowScope::maybe_of(app, context);
            kept.borrow_mut()
                .push(window.map(|window| window.is_minimized()));
            SizedBox::new().into_widget()
        })
        .into_widget();
        crate::binding::run_widget(
            &mut cell.borrow_mut(),
            MultiViewApp::new(move |_app, _view| Some(content.clone())).into_widget(),
        );
        cell.elapse(Duration::ZERO);
        binding_pump(&mut cell.borrow_mut(), Duration::ZERO);
        (cell, read)
    }

    #[test]
    fn a_view_in_a_window_hands_its_content_the_window_and_its_changes() {
        let window = TestWindow::around(
            TestView::new(300.0, 200.0).with_id(ViewId(1)),
            WindowConfig::default(),
        );
        let (cell, read) = reading(&[Rc::clone(window.test_view())]);
        assert_eq!(*read.borrow(), [Some(false)]);

        window.set_minimized(true);
        report_window_changed(&cell, ViewId(1));
        binding_pump(&mut cell.borrow_mut(), Duration::from_millis(16));
        assert_eq!(
            *read.borrow(),
            [Some(false), Some(true)],
            "rebuilt at the change"
        );
    }

    #[test]
    fn a_view_in_no_window_has_none() {
        let (_cell, read) = reading(&[Rc::new(TestView::new(300.0, 200.0).with_id(ViewId(1)))]);
        assert_eq!(*read.borrow(), [None]);
    }
}
