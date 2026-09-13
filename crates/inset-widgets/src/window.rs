//! Flutter counterpart: `Window` and `WindowScope` from `widgets/_window.dart`
//! (experimental), over the host's [`HostWindow`] rather than a controller archetype.
//!
//! A [`Window`] pairs a window the host made with the widget drawn into it: a [`View`] on the
//! window's view, under a [`WindowScope`] that hands the window to the subtree.

use std::fmt;
use std::rc::Rc;

use inset_embedder::WindowRef;
use inset_foundation::App;

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, StatelessWidget, WidgetRef,
};
use crate::view::View;

/// A widget that renders its `child` into a window the host made.
///
/// The window's view is what the framework draws into, so the widget is a [`View`] at heart
/// and, like one, may only sit in a non-rendering zone: at the root of the tree, in a
/// [`crate::ViewCollection`], or in the `view` slot of a [`crate::ViewAnchor`].
pub struct Window {
    pub key: Option<KeyRef>,
    pub window: WindowRef,
    pub child: WidgetRef,
}

impl Window {
    pub fn new<K>(window: WindowRef, child: impl IntoWidget<K>) -> Window {
        Window {
            key: None,
            window,
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
            .field("view", &self.window.view().id())
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for Window {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        WindowScope::new(
            Rc::clone(&self.window),
            View::new(self.window.view(), self.child.clone()),
        )
        .into_widget()
    }
}

/// Provides the [`HostWindow`] of a [`Window`] to the widgets below it.
pub struct WindowScope {
    pub window: WindowRef,
    pub child: WidgetRef,
}

impl WindowScope {
    pub fn new<K>(window: WindowRef, child: impl IntoWidget<K>) -> WindowScope {
        WindowScope {
            window,
            child: child.into_widget(),
        }
    }

    /// The window the `context` is drawn in, or `None` outside any [`Window`].
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<WindowRef> {
        context
            .depend_on_inherited_widget_of_exact_type::<WindowScope>(app)
            .map(|scope| Rc::clone(&scope.window))
    }

    /// The window the `context` is drawn in.
    ///
    /// # Panics
    ///
    /// Outside any [`Window`].
    pub fn of(app: &mut App, context: BuildContext) -> WindowRef {
        WindowScope::maybe_of(app, context)
            .expect("WindowScope.of() was called with a context that is not inside a Window")
    }
}

impl fmt::Debug for WindowScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowScope")
            .field("view", &self.window.view().id())
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for WindowScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &WindowScope) -> bool {
        !Rc::ptr_eq(&self.window, &old_widget.window)
    }
}
