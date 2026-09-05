//! Flutter counterpart: `gestures/drag.dart`.

use reveal_foundation::{App, Handle};

use crate::drag_details::{DragEndDetails, DragUpdateDetails};

/// Interface for objects that receive updates about drags.
///
/// This interface is used in various ways. For example,
/// `MultiDragGestureRecognizer` uses it to update its clients when it recognizes
/// a gesture. Similarly, the scrolling infrastructure in the widgets library
/// uses it to notify the `DragScrollActivity` when the user drags the
/// scrollable.
///
/// What a field or parameter Dart types as `Drag` becomes an `Rc<dyn Drag>`; an arena object
/// implements [`DragObject`], and `Rc::new(handle)` is its erased form.
pub trait Drag {
    /// The pointer has moved.
    fn update(&self, app: &mut App, details: DragUpdateDetails);

    /// The pointer is no longer in contact with the screen.
    ///
    /// The velocity at which the pointer was moving when it stopped contacting
    /// the screen is available in the `details`.
    fn end(&self, app: &mut App, details: DragEndDetails);

    /// The input from the pointer is no longer directed towards this receiver.
    ///
    /// For example, the user might have been interrupted by a system-modal dialog
    /// in the middle of the drag.
    fn cancel(&self, app: &mut App);
}

/// The object side of [`Drag`]: an arena object that receives drag updates. Implementing it
/// makes `Handle<Self>` a [`Drag`]; Dart's empty default bodies are the defaults here.
pub trait DragObject: Sized + 'static {
    /// See [`Drag::update`].
    fn update(self: Handle<Self>, _app: &mut App, _details: DragUpdateDetails) {}

    /// See [`Drag::end`].
    fn end(self: Handle<Self>, _app: &mut App, _details: DragEndDetails) {}

    /// See [`Drag::cancel`].
    fn cancel(self: Handle<Self>, _app: &mut App) {}
}

impl<T: DragObject> Drag for Handle<T> {
    fn update(&self, app: &mut App, details: DragUpdateDetails) {
        T::update(*self, app, details);
    }

    fn end(&self, app: &mut App, details: DragEndDetails) {
        T::end(*self, app, details);
    }

    fn cancel(&self, app: &mut App) {
        T::cancel(*self, app);
    }
}
