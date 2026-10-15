//! Flutter counterpart: `Window` and `WindowScope` from `widgets/_window.dart` (experimental).
//!
//! A [`Window`] pairs a window the app asked for with the widget drawn into it: a [`View`] on
//! the window's view, under a [`WindowScope`] that hands the window's controller to the
//! subtree. The controller's view arrives a moment after the controller exists, so the widget
//! draws nothing until then, and rebuilds when the view comes and goes, as Flutter's `Window`
//! listens to its controller.

use std::fmt;
use std::rc::Rc;

use inset_foundation::{App, Handle, Listenable};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, StatelessWidget, WidgetRef,
};
use crate::view::{View, ViewCollection};
use crate::widgets::transitions::ListenableBuilder;
use crate::windowing::WindowController;

/// A widget that renders its `child` into the window of a [`WindowController`].
///
/// The window's view is what the framework draws into, so the widget is a [`View`] at heart
/// and, like one, may only sit in a non-rendering zone: at the root of the tree, in a
/// [`crate::ViewCollection`], or in the `view` slot of a [`crate::ViewAnchor`].
pub struct Window {
    pub key: Option<KeyRef>,
    pub controller: Handle<WindowController>,
    pub child: WidgetRef,
}

impl Window {
    pub fn new<K>(controller: Handle<WindowController>, child: impl IntoWidget<K>) -> Window {
        Window {
            key: None,
            controller,
            child: child.into_widget(),
        }
    }

    /// Dart `Window(key:)`.
    pub fn key(mut self, key: KeyRef) -> Window {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for Window {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let controller = self.controller;
        let listenable: Rc<dyn Listenable> = Rc::new(controller);
        ListenableBuilder::new(listenable, move |app, _context, child| {
            let child = child
                .cloned()
                .expect("the window's content is passed as the child");
            match controller.view(app) {
                Some(view) => WindowScope::new(controller, View::new(view, child)).into_widget(),
                None => ViewCollection::new(Vec::new()).into_widget(),
            }
        })
        .child(self.child.clone())
        .into_widget()
    }
}

/// Provides the [`WindowController`] of a [`Window`] to the widgets below it.
pub struct WindowScope {
    pub controller: Handle<WindowController>,
    pub child: WidgetRef,
}

impl WindowScope {
    pub fn new<K>(controller: Handle<WindowController>, child: impl IntoWidget<K>) -> WindowScope {
        WindowScope {
            controller,
            child: child.into_widget(),
        }
    }

    /// The controller of the window the `context` is drawn in, or `None` outside any
    /// [`Window`].
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<WindowController>> {
        context
            .depend_on_inherited_widget_of_exact_type::<WindowScope>(app)
            .map(|scope| scope.controller)
    }

    /// The controller of the window the `context` is drawn in.
    ///
    /// # Panics
    ///
    /// Outside any [`Window`].
    pub fn of(app: &mut App, context: BuildContext) -> Handle<WindowController> {
        WindowScope::maybe_of(app, context)
            .expect("WindowScope.of() was called with a context that is not inside a Window")
    }
}

impl fmt::Debug for WindowScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowScope")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for WindowScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &WindowScope) -> bool {
        self.controller != old_widget.controller
    }
}
