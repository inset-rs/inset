//! Flutter counterpart: `gestures/hit_test.dart`.

use std::any::Any;
use std::fmt::{self, Debug};

use reveal_embedder::{Matrix4, Offset, ViewId};
use reveal_foundation::{App, PRECISION_ERROR_TOLERANCE};

use crate::events::PointerEvent;

/// An object that can hit-test pointers.
pub trait HitTestable {
    /// Deprecated. Use [`hit_test_in_view`](Self::hit_test_in_view) instead.
    fn hit_test(&self, app: &mut App, result: &mut HitTestResult, position: Offset);

    /// Fills the provided [`HitTestResult`] with [`HitTestEntry`]s for objects that
    /// are hit at the given `position` in the view identified by `view_id`.
    fn hit_test_in_view(
        &self,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
        view_id: ViewId,
    );
}

/// An object that can dispatch events.
pub trait HitTestDispatcher {
    /// Override this method to dispatch events.
    fn dispatch_event(&self, app: &mut App, event: PointerEvent, result: &HitTestResult);
}

/// An object that can handle events.
pub trait HitTestTarget: Debug + Any {
    /// Override this method to receive events.
    fn handle_event(&self, app: &mut App, event: &PointerEvent, entry: &HitTestEntry);
}

/// Data collected during a hit test about a specific [`HitTestTarget`].
///
/// Subclass this object to pass additional information from the hit test phase
/// to the event propagation phase.
pub struct HitTestEntry {
    target: Box<dyn HitTestTarget>,
    transform: Option<Matrix4>,
}

impl HitTestEntry {
    /// Creates a hit test entry.
    pub fn new(target: impl HitTestTarget + 'static) -> HitTestEntry {
        HitTestEntry {
            target: Box::new(target),
            transform: None,
        }
    }

    /// The [`HitTestTarget`] encountered during the hit test.
    pub fn target(&self) -> &dyn HitTestTarget {
        &*self.target
    }

    /// Returns a matrix describing how [`PointerEvent`]s delivered to this
    /// [`HitTestEntry`] should be transformed from the global coordinate space of
    /// the screen to the local coordinate space of [`target`](Self::target).
    ///
    /// See also:
    ///
    ///  * `BoxHitTestResult.addWithPaintTransform`, which is used during hit testing
    ///    to build up this transform.
    pub fn transform(&self) -> Option<Matrix4> {
        self.transform
    }
}

impl Debug for HitTestEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HitTestEntry({:?})", self.target)
    }
}

/// A type of data that can be applied to a matrix by left-multiplication.
enum TransformPart {
    Matrix(Matrix4),
    Offset(Offset),
}

impl TransformPart {
    /// Apply this transform part to `rhs` from the left.
    ///
    /// This should work as if this transform part is first converted to a matrix
    /// and then left-multiplied to `rhs`.
    fn multiply(&self, rhs: Matrix4) -> Matrix4 {
        match *self {
            TransformPart::Matrix(matrix) => matrix.then(&rhs),
            TransformPart::Offset(offset) => {
                Matrix4::translation(offset.dx() as f32, offset.dy() as f32).then(&rhs)
            }
        }
    }
}

/// The result of performing a hit test.
///
/// Dart's `HitTestResult.wrap` shares one result between a base and a subtype
/// view; here that view is `&mut HitTestResult` (or a newtype over it, such as
/// `BoxHitTestResult`).
pub struct HitTestResult {
    path: Vec<HitTestEntry>,
    // A stack of transform parts.
    //
    // The transform part stack leading from global to the current object is stored
    // in 2 parts:
    //
    //  * `transforms` are globalized matrices, meaning they have been multiplied
    //    by the ancestors and are thus relative to the global coordinate space.
    //  * `local_transforms` are local transform parts, which are relative to the
    //    parent's coordinate space.
    //
    // When new transform parts are added they're appended to `local_transforms`,
    // and are converted to global ones and moved to `transforms` only when used.
    transforms: Vec<Matrix4>,
    local_transforms: Vec<TransformPart>,
}

impl HitTestResult {
    /// Creates an empty hit test result.
    pub fn new() -> HitTestResult {
        HitTestResult {
            path: Vec::new(),
            transforms: vec![Matrix4::IDENTITY],
            local_transforms: Vec::new(),
        }
    }

    /// An unmodifiable list of [`HitTestEntry`] objects recorded during the hit test.
    ///
    /// The first entry in the path is the most specific, typically the one at
    /// the leaf of tree being hit tested. Event propagation starts with the most
    /// specific (i.e., first) entry and proceeds in order through the path.
    pub fn path(&self) -> &[HitTestEntry] {
        &self.path
    }

    // Globalize all transform parts in `local_transforms` and move them to
    // `transforms`.
    fn globalize_transforms(&mut self) {
        if self.local_transforms.is_empty() {
            return;
        }
        let mut last = *self.transforms.last().unwrap();
        for part in self.local_transforms.drain(..) {
            last = part.multiply(last);
            self.transforms.push(last);
        }
    }

    fn last_transform(&mut self) -> Matrix4 {
        self.globalize_transforms();
        debug_assert!(self.local_transforms.is_empty());
        *self.transforms.last().unwrap()
    }

    /// Add a [`HitTestEntry`] to the path.
    ///
    /// The new entry is added at the end of the path, which means entries should
    /// be added in order from most specific to least specific, typically during an
    /// upward walk of the tree being hit tested.
    pub fn add(&mut self, mut entry: HitTestEntry) {
        debug_assert!(entry.transform.is_none());
        entry.transform = Some(self.last_transform());
        self.path.push(entry);
    }

    /// Pushes a new transform matrix that is to be applied to all future
    /// [`HitTestEntry`]s added via [`add`](Self::add) until it is removed via [`pop_transform`](Self::pop_transform).
    ///
    /// This method is only to be used by subclasses, which must provide
    /// coordinate space specific public wrappers around this function for their
    /// users (see `BoxHitTestResult.addWithPaintTransform` for such an example).
    ///
    /// The provided `transform` matrix should describe how to transform
    /// [`PointerEvent`]s from the coordinate space of the method caller to the
    /// coordinate space of its children. In most cases `transform` is derived
    /// from running the inverted result of `RenderObject.applyPaintTransform`
    /// through `PointerEvent.removePerspectiveTransform` to remove
    /// the perspective component.
    ///
    /// If the provided `transform` is a translation matrix, it is much faster
    /// to use [`push_offset`](Self::push_offset) with the translation offset instead.
    ///
    /// [`HitTestable`]s need to call this method indirectly through a convenience
    /// method defined on a subclass before hit testing a child that does not
    /// have the same origin as the parent. After hit testing the child,
    /// [`pop_transform`](Self::pop_transform) has to be called to remove the child-specific `transform`.
    ///
    /// See also:
    ///
    ///  * [`push_offset`](Self::push_offset), which is similar to [`push_transform`](Self::push_transform) but is limited to
    ///    translations, and is faster in such cases.
    ///  * `BoxHitTestResult.addWithPaintTransform`, which is a public wrapper
    ///    around this function for hit testing on `RenderBox`s.
    pub fn push_transform(&mut self, transform: Matrix4) {
        debug_assert!(
            {
                let values = transform.to_flutter_array();
                let row2 = [values[2], values[6], values[10], values[14]];
                let col2 = [values[8], values[9], values[10], values[11]];
                debug_vector_more_or_less_equals(row2, [0.0, 0.0, 1.0, 0.0])
                    && debug_vector_more_or_less_equals(col2, [0.0, 0.0, 1.0, 0.0])
            },
            "The third row and third column of a transform matrix for pointer \
             events must be Vector4(0, 0, 1, 0) to ensure that a transformed \
             point is directly under the pointing device. Did you forget to run the paint \
             matrix through PointerEvent.removePerspectiveTransform? \
             The provided matrix is:\n{transform:?}"
        );
        self.local_transforms.push(TransformPart::Matrix(transform));
    }

    /// Pushes a new translation offset that is to be applied to all future
    /// [`HitTestEntry`]s added via [`add`](Self::add) until it is removed via [`pop_transform`](Self::pop_transform).
    ///
    /// This method is only to be used by subclasses, which must provide
    /// coordinate space specific public wrappers around this function for their
    /// users (see `BoxHitTestResult.addWithPaintOffset` for such an example).
    ///
    /// The provided `offset` should describe how to transform [`PointerEvent`]s from
    /// the coordinate space of the method caller to the coordinate space of its
    /// children. Usually `offset` is the inverse of the offset of the child
    /// relative to the parent.
    ///
    /// [`HitTestable`]s need to call this method indirectly through a convenience
    /// method defined on a subclass before hit testing a child that does not
    /// have the same origin as the parent. After hit testing the child,
    /// [`pop_transform`](Self::pop_transform) has to be called to remove the child-specific `transform`.
    ///
    /// See also:
    ///
    ///  * [`push_transform`](Self::push_transform), which is similar to [`push_offset`](Self::push_offset) but allows general
    ///    transform besides translation.
    ///  * `BoxHitTestResult.addWithPaintOffset`, which is a public wrapper
    ///    around this function for hit testing on `RenderBox`s.
    ///  * `SliverHitTestResult.addWithAxisOffset`, which is a public wrapper
    ///    around this function for hit testing on `RenderSliver`s.
    pub fn push_offset(&mut self, offset: Offset) {
        self.local_transforms.push(TransformPart::Offset(offset));
    }

    /// Removes the last transform added via [`push_transform`](Self::push_transform) or [`push_offset`](Self::push_offset).
    ///
    /// This method is only to be used by subclasses, which must provide
    /// coordinate space specific public wrappers around this function for their
    /// users (see `BoxHitTestResult.addWithPaintTransform` for such an example).
    ///
    /// This method must be called after hit testing is done on a child that
    /// required a call to [`push_transform`](Self::push_transform) or [`push_offset`](Self::push_offset).
    ///
    /// See also:
    ///
    ///  * [`push_transform`](Self::push_transform) and [`push_offset`](Self::push_offset), which describes the use case of this
    ///    function pair in more details.
    pub fn pop_transform(&mut self) {
        if !self.local_transforms.is_empty() {
            self.local_transforms.pop();
        } else {
            self.transforms.pop();
        }
        debug_assert!(!self.transforms.is_empty());
    }
}

impl Default for HitTestResult {
    fn default() -> HitTestResult {
        HitTestResult::new()
    }
}

impl Debug for HitTestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            return write!(f, "HitTestResult(<empty path>)");
        }
        write!(f, "HitTestResult(")?;
        for (i, entry) in self.path.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{entry:?}")?;
        }
        write!(f, ")")
    }
}

fn debug_vector_more_or_less_equals(a: [f32; 4], b: [f32; 4]) -> bool {
    let mut result = true;
    if cfg!(debug_assertions) {
        result = a.iter().zip(b).all(|(component, expected)| {
            (*component as f64 - expected as f64).abs() < PRECISION_ERROR_TOLERANCE
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyHitTestTarget;

    impl HitTestTarget for DummyHitTestTarget {
        fn handle_event(&self, _app: &mut App, _event: &PointerEvent, _entry: &HitTestEntry) {}
    }

    fn current_transform(result: &mut HitTestResult) -> Matrix4 {
        result.add(HitTestEntry::new(DummyHitTestTarget));
        result.path().last().unwrap().transform().unwrap()
    }

    #[test]
    fn add_records_the_current_transform_on_the_entry() {
        let transform = Matrix4::translation(40.0, 150.0);
        let mut result = HitTestResult::new();
        result.push_transform(transform);
        result.add(HitTestEntry::new(DummyHitTestTarget));
        result.add(HitTestEntry::new(DummyHitTestTarget));
        assert_eq!(result.path().len(), 2);
        assert!(
            result
                .path()
                .iter()
                .all(|entry| entry.transform() == Some(transform))
        );
    }

    #[test]
    fn push_and_pop_transforms() {
        let mut result = HitTestResult::new();

        let m1 = Matrix4::translation(10.0, 20.0);
        let m2 = Matrix4::rotation(1.0);
        let m3 = Matrix4::scale(1.1, 1.2);

        result.push_transform(m1);
        assert_eq!(current_transform(&mut result), m1);

        result.push_transform(m2);
        assert_eq!(current_transform(&mut result), m2.then(&m1));
        assert_eq!(current_transform(&mut result), m2.then(&m1)); // Test repeated add

        result.push_transform(m3);
        assert_eq!(current_transform(&mut result), m3.then(&m2.then(&m1)));

        result.pop_transform();
        result.pop_transform();
        assert_eq!(current_transform(&mut result), m1);

        result.pop_transform();
        result.push_transform(m3);
        assert_eq!(current_transform(&mut result), m3);

        result.push_transform(m2);
        assert_eq!(current_transform(&mut result), m2.then(&m3));
    }

    #[test]
    fn push_and_pop_offsets() {
        let mut result = HitTestResult::new();

        let m1 = Matrix4::rotation(1.0);
        let m2 = Matrix4::scale(1.1, 1.2);
        let o3 = Offset::new(10.0, 20.0);
        let m3 = Matrix4::translation(o3.dx() as f32, o3.dy() as f32);

        // Test pushing offset as the first element
        result.push_offset(o3);
        assert_eq!(current_transform(&mut result), m3);
        result.pop_transform();

        result.push_offset(o3);
        result.push_transform(m1);
        assert_eq!(current_transform(&mut result), m1.then(&m3));
        assert_eq!(current_transform(&mut result), m1.then(&m3)); // Test repeated add

        result.push_transform(m2);
        assert_eq!(current_transform(&mut result), m2.then(&m1.then(&m3)));

        result.pop_transform();
        result.pop_transform();
        result.pop_transform();
        assert_eq!(current_transform(&mut result), Matrix4::IDENTITY);

        result.push_transform(m2);
        result.push_offset(o3);
        result.push_transform(m1);
        assert_eq!(current_transform(&mut result), m1.then(&m3.then(&m2)));

        result.pop_transform();
        assert_eq!(current_transform(&mut result), m3.then(&m2));
    }
}
