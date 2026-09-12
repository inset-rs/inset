//! Flutter counterpart: `widgets/scroll_context.dart`.

use std::rc::Rc;

use inset_foundation::App;
use inset_painting::AxisDirection;
use inset_scheduler::TickerProvider;

use crate::framework::BuildContext;

/// An interface that `Scrollable` widgets implement in order to use
/// `ScrollPosition`.
///
/// What a field or parameter Dart types as `ScrollContext` becomes an `Rc<dyn ScrollContext>`:
/// an arena object's `Handle` implements this trait, and `Rc::new(handle)` is its erased form.
/// A `ScrollPosition` compares contexts by identity, so a scrollable hands out one `Rc` for its
/// lifetime.
///
/// See also:
///
///  * `ScrollableState`, which is the most common implementation of this
///    interface.
///  * `ScrollPosition`, which uses this interface to communicate with the
///    scrollable widget.
pub trait ScrollContext {
    /// Dart's `context.runtimeType`, for a debug description.
    fn type_name(&self) -> &'static str;

    /// The [`BuildContext`] that should be used when dispatching
    /// `ScrollNotification`s.
    ///
    /// This context is typically different that the context of the scrollable
    /// widget itself. For example, `Scrollable` uses a context outside the
    /// `Viewport` but inside the widgets created by
    /// `ScrollBehavior.buildOverscrollIndicator` and `ScrollBehavior.buildScrollbar`.
    fn notification_context(&self, app: &mut App) -> Option<BuildContext>;

    /// The [`BuildContext`] that should be used when searching for a `PageStorage`.
    ///
    /// This context is typically the context of the scrollable widget itself. In
    /// particular, it should involve any `GlobalKey`s that are dynamically
    /// created as part of creating the scrolling widget, since those would be
    /// different each time the widget is created.
    fn storage_context(&self, app: &App) -> BuildContext;

    /// A [`TickerProvider`] to use when animating the scroll position.
    fn vsync(&self) -> Rc<dyn TickerProvider>;

    /// The direction in which the widget scrolls.
    fn axis_direction(&self, app: &App) -> AxisDirection;

    /// The `FlutterView.devicePixelRatio` of the view that the `Scrollable` this
    /// [`ScrollContext`] is associated with is drawn into.
    fn device_pixel_ratio(&self, app: &App) -> f64;

    /// Whether the contents of the widget should ignore `PointerEvent` inputs.
    ///
    /// Setting this value to true prevents the use from interacting with the
    /// contents of the widget with pointer events. The widget itself is still
    /// interactive.
    ///
    /// For example, if the scroll position is being driven by an animation, it
    /// might be appropriate to set this value to ignore pointer events to
    /// prevent the user from accidentally interacting with the contents of the
    /// widget as it animates. The user will still be able to touch the widget,
    /// potentially stopping the animation.
    fn set_ignore_pointer(&self, app: &mut App, value: bool);

    /// Whether the user can drag the widget, for example to initiate a scroll.
    fn set_can_drag(&self, app: &mut App, value: bool);

    /// Called by the `ScrollPosition` whenever scrolling ends to persist the
    /// provided scroll `offset` for state restoration purposes.
    ///
    /// The [`ScrollContext`] may pass the value back to a `ScrollPosition` by
    /// calling `ScrollPosition.restoreOffset` at a later point in time or after
    /// the application has restarted to restore the scroll offset.
    fn save_offset(&self, app: &mut App, offset: f64);
}
