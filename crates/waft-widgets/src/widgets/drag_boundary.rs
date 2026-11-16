//! Flutter counterpart: `widgets/drag_boundary.dart`.

use waft_embedder::{Offset, Rect, clamp_double};
use waft_foundation::App;

use crate::framework::{BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

/// The interface for defining the algorithm for a boundary that a specified shape is dragged
/// within.
///
/// See also:
///  * [`DragBoundary`], an [`InheritedWidget`] that provides a [`DragBoundaryDelegate`] to its
///    descendants.
///
/// `T` is a data class that defines the shape being dragged. For example, when dragging a
/// rectangle within the boundary, `T` should be a `Rect`.
pub trait DragBoundaryDelegate<T> {
    /// Returns whether the specified dragged object is within the boundary.
    fn is_within_boundary(&self, dragged_object: T) -> bool;

    /// Returns the given dragged object after moving it fully inside the boundary with the
    /// shortest distance.
    ///
    /// If the bounds cannot contain the dragged object, it panics.
    fn nearest_position_within_boundary(&self, dragged_object: T) -> T;
}

/// Dart's `_DragBoundaryDelegateForRect`.
struct DragBoundaryDelegateForRect {
    boundary: Option<Rect>,
}

impl DragBoundaryDelegateForRect {
    fn new(boundary: Option<Rect>) -> DragBoundaryDelegateForRect {
        DragBoundaryDelegateForRect { boundary }
    }
}

impl DragBoundaryDelegate<Rect> for DragBoundaryDelegateForRect {
    fn is_within_boundary(&self, dragged_object: Rect) -> bool {
        let Some(boundary) = self.boundary else {
            return true;
        };
        boundary.contains(dragged_object.top_left())
            && boundary.contains(dragged_object.bottom_right())
    }

    fn nearest_position_within_boundary(&self, dragged_object: Rect) -> Rect {
        let Some(boundary) = self.boundary else {
            return dragged_object;
        };
        if boundary.right - dragged_object.width() < boundary.left
            || boundary.bottom - dragged_object.height() < boundary.top
        {
            panic!(
                "The rect is larger than the boundary. The rect width must be less than the \
                 boundary width, and the rect height must be less than the boundary height."
            );
        }
        let left = clamp_double(
            dragged_object.left,
            boundary.left,
            boundary.right - dragged_object.width(),
        );
        let top = clamp_double(
            dragged_object.top,
            boundary.top,
            boundary.bottom - dragged_object.height(),
        );
        Rect::from_ltwh(left, top, dragged_object.width(), dragged_object.height())
    }
}

/// Provides a [`DragBoundaryDelegate`] for its descendants whose bounds are those defined by
/// this widget.
///
/// [`for_rect_of`](Self::for_rect_of) and [`for_rect_maybe_of`](Self::for_rect_maybe_of)
/// return a delegate for a drag object of type [`Rect`].
#[derive(Debug)]
pub struct DragBoundary {
    pub key: Option<KeyRef>,
    pub child: WidgetRef,
}

impl DragBoundary {
    /// Creates a widget that provides a boundary to its descendants.
    pub fn new<K>(child: impl IntoWidget<K>) -> DragBoundary {
        DragBoundary {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `DragBoundary(key:)`.
    pub fn key(mut self, key: KeyRef) -> DragBoundary {
        self.key = Some(key);
        self
    }

    /// Retrieve the [`DragBoundary`] from the nearest ancestor to get its
    /// [`DragBoundaryDelegate`] of [`Rect`].
    ///
    /// The `use_global_position` specifies whether to retrieve the [`DragBoundaryDelegate`] of
    /// type [`Rect`] in global coordinates. If false, the local coordinates of the boundary are
    /// used. Dart defaults it to true.
    ///
    /// If no [`DragBoundary`] ancestor is found, the delegate will return a delegate that
    /// allows the drag object to move freely.
    pub fn for_rect_of(
        app: &App,
        context: BuildContext,
        use_global_position: bool,
    ) -> Box<dyn DragBoundaryDelegate<Rect>> {
        DragBoundary::for_rect_maybe_of(app, context, use_global_position)
            .unwrap_or_else(|| Box::new(DragBoundaryDelegateForRect::new(None)))
    }

    /// Retrieve the [`DragBoundary`] from the nearest ancestor to get its
    /// [`DragBoundaryDelegate`] of [`Rect`], as [`for_rect_of`](Self::for_rect_of) does.
    ///
    /// Returns `None` if no ancestor is found.
    pub fn for_rect_maybe_of(
        app: &App,
        context: BuildContext,
        use_global_position: bool,
    ) -> Option<Box<dyn DragBoundaryDelegate<Rect>>> {
        let element =
            context.get_element_for_inherited_widget_of_exact_type::<DragBoundary>(app)?;
        let rb = element
            .find_render_object(app)
            .and_then(|render_object| render_object.as_box());
        debug_assert!(
            rb.is_some_and(|rb| rb.has_size(app)),
            "DragBoundary is not available"
        );
        let rb = rb.expect("DragBoundary is not available");
        let size = rb.size(app);
        let boundary = if use_global_position {
            Rect::from_points(
                rb.local_to_global(app, Offset::ZERO, None),
                rb.local_to_global(app, size.bottom_right(Offset::ZERO), None),
            )
        } else {
            Offset::ZERO & size
        };
        Some(Box::new(DragBoundaryDelegateForRect::new(Some(boundary))))
    }
}

impl InheritedWidget for DragBoundary {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, _old_widget: &DragBoundary) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry};

    use super::*;
    use crate::framework::GlobalKey;
    use crate::testing::{WidgetTester, test_widgets};
    use crate::widgets::basic::SizedBox;
    use crate::widgets::container::Container;

    /// The three cases' tree: a 100x100 box at (100, 100), under a `DragBoundary` or not.
    fn pump_box(tester: &mut WidgetTester, key: &GlobalKey, boundary: bool) {
        let sized_box = SizedBox::new()
            .key(key.clone().into())
            .width(100.0)
            .height(100.0);
        let container = Container::new()
            .margin(EdgeInsetsGeometry::only(100.0, 100.0, 0.0, 0.0))
            .alignment(AlignmentGeometry::TOP_LEFT);
        if boundary {
            tester.pump_widget(container.child(DragBoundary::new(sized_box)));
        } else {
            tester.pump_widget(container.child(sized_box));
        }
    }

    fn for_rect_of(
        tester: &WidgetTester,
        key: &GlobalKey,
        use_global_position: bool,
    ) -> Box<dyn DragBoundaryDelegate<Rect>> {
        tester.update(|app| {
            let context = key.current_context(app).expect("mounted");
            DragBoundary::for_rect_of(app, context, use_global_position)
        })
    }

    #[test]
    fn test_drag_boundary_with_use_global_position() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            pump_box(tester, &key, true);
            let boundary = for_rect_of(tester, &key, true);
            assert!(!boundary.is_within_boundary(Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)));
            assert!(boundary.is_within_boundary(Rect::from_ltwh(100.0, 100.0, 20.0, 20.0)));
            assert_eq!(
                boundary.nearest_position_within_boundary(Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)),
                Rect::from_ltwh(100.0, 100.0, 20.0, 20.0)
            );
            assert_eq!(
                boundary
                    .nearest_position_within_boundary(Rect::from_ltwh(150.0, 150.0, 20.0, 20.0)),
                Rect::from_ltwh(150.0, 150.0, 20.0, 20.0)
            );
        });
    }

    #[test]
    fn test_drag_boundary_without_use_global_position() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            pump_box(tester, &key, true);
            let boundary = for_rect_of(tester, &key, false);
            assert!(boundary.is_within_boundary(Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)));
            assert!(!boundary.is_within_boundary(Rect::from_ltwh(90.0, 90.0, 20.0, 20.0)));
            assert_eq!(
                boundary.nearest_position_within_boundary(Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)),
                Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)
            );
            assert_eq!(
                boundary.nearest_position_within_boundary(Rect::from_ltwh(90.0, 90.0, 20.0, 20.0)),
                Rect::from_ltwh(80.0, 80.0, 20.0, 20.0)
            );
        });
    }

    #[test]
    fn for_rect_of_returns_a_free_boundary_when_no_ancestor_has_a_drag_boundary() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            pump_box(tester, &key, false);
            let boundary = for_rect_of(tester, &key, true);
            assert!(boundary.is_within_boundary(Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)));
            assert!(boundary.is_within_boundary(Rect::from_ltwh(100.0, 100.0, 20.0, 20.0)));
            assert!(boundary.is_within_boundary(Rect::from_ltwh(300.0, 300.0, 300.0, 300.0)));
            assert_eq!(
                boundary.nearest_position_within_boundary(Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)),
                Rect::from_ltwh(50.0, 50.0, 20.0, 20.0)
            );
            assert_eq!(
                boundary
                    .nearest_position_within_boundary(Rect::from_ltwh(150.0, 150.0, 20.0, 20.0)),
                Rect::from_ltwh(150.0, 150.0, 20.0, 20.0)
            );
            assert_eq!(
                boundary
                    .nearest_position_within_boundary(Rect::from_ltwh(300.0, 300.0, 300.0, 300.0)),
                Rect::from_ltwh(300.0, 300.0, 300.0, 300.0)
            );
        });
    }
}
