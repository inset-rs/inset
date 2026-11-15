//! Flutter counterpart: `painting/decoration.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::ops::Deref;
use std::rc::Rc;

use waft_embedder::{Canvas, Offset, Path, Rect, Size};
use waft_foundation::{App, VoidCallback};

use crate::basic_types::TextDirection;
use crate::edge_insets::{EdgeInsets, EdgeInsetsGeometry};
use crate::image_provider::ImageConfiguration;

/// A description of a box decoration (a decoration applied to a [`Rect`]).
///
/// This trait presents the abstract interface for all decorations.
/// See [`BoxDecoration`](crate::BoxDecoration) for a concrete example.
///
/// To actually paint a [`Decoration`], use the [`create_box_painter`](Self::create_box_painter)
/// method to obtain a [`BoxPainter`]. [`Decoration`] objects can be
/// shared between boxes; [`BoxPainter`] objects can cache resources to
/// make painting on a particular surface faster.
pub trait Decoration: Any + Debug {
    /// In debug mode, panics if the object is not in a valid configuration.
    /// Otherwise, returns true.
    fn debug_assert_is_valid(&self) -> bool {
        true
    }

    /// Returns the insets to apply when using this decoration on a box
    /// that has contents, so that the contents do not overlap the edges
    /// of the decoration.
    fn padding(&self) -> EdgeInsetsGeometry {
        EdgeInsets::ZERO.into()
    }

    /// Whether this decoration is complex enough to benefit from caching its painting.
    fn is_complex(&self) -> bool {
        false
    }

    /// Linearly interpolates from another [`Decoration`] (which may be of a
    /// different class) to `this`.
    ///
    /// Return None if this class cannot interpolate from `a`. If `a` is None,
    /// this must not return None.
    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
        let _ = (a, t);
        None
    }

    /// Linearly interpolates from `this` to another [`Decoration`] (which may be of
    /// a different class).
    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
        let _ = (b, t);
        None
    }

    /// Tests whether the given point, on a rectangle of a given size,
    /// would be considered to hit the decoration or not.
    fn hit_test(
        &self,
        size: Size,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        let _ = (size, position, text_direction);
        true
    }

    /// Returns a [`BoxPainter`] that will paint this decoration.
    fn create_box_painter(&self, on_changed: Option<VoidCallback>) -> Box<dyn BoxPainter>;

    /// Returns a closed [`Path`] that describes the outer edge of this decoration.
    ///
    /// The default implementation panics. Subclasses must override this
    /// implementation to describe the clip path that should be applied to the
    /// decoration when it is used in a `Container` with an explicit clip behavior.
    fn get_clip_path(&self, rect: Rect, text_direction: TextDirection) -> Path {
        let _ = (rect, text_direction);
        panic!(
            "{} does not expect to be used for clipping.",
            std::any::type_name::<Self>()
        );
    }

    /// Downcast support for `is` / `as` in Dart.
    fn as_any(&self) -> &dyn Any;

    /// Field equality. Dart's default `==` is identity; subclasses that override
    /// `==` implement this.
    fn eq_decoration(&self, other: &dyn Decoration) -> bool {
        std::ptr::addr_eq(self.as_any(), other.as_any())
    }
}

/// A [`Decoration`] held by value: what a Dart field or parameter typed `Decoration` becomes.
///
/// Clones share one decoration, as Dart's references do, so a decoration handed on keeps its
/// identity: a widget given the same decoration again sees an unchanged one. Two refs are equal
/// when they share a decoration or when [`eq_decoration`](Decoration::eq_decoration) says so,
/// Dart's `==`.
#[derive(Clone)]
pub struct DecorationRef(Rc<dyn Decoration>);

impl DecorationRef {
    /// Erases a [`Decoration`].
    pub fn new(decoration: impl Decoration) -> DecorationRef {
        DecorationRef(Rc::new(decoration))
    }

    /// Linearly interpolates between two [`Decoration`]s.
    ///
    /// This attempts to use [`lerp_from`](Decoration::lerp_from) and
    /// [`lerp_to`](Decoration::lerp_to) on `b` and `a` respectively to find a
    /// solution. If the two values can't directly be interpolated, then the
    /// interpolation is done via null (at `t == 0.5`).
    pub fn lerp(
        a: Option<&DecorationRef>,
        b: Option<&DecorationRef>,
        t: f64,
    ) -> Option<DecorationRef> {
        let (a, b) = match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if Rc::ptr_eq(&a.0, &b.0) => return Some(a.clone()),
            (None, Some(b)) => return b.lerp_from(None, t).or_else(|| Some(b.clone())),
            (Some(a), None) => return a.lerp_to(None, t).or_else(|| Some(a.clone())),
            (Some(a), Some(b)) => (a, b),
        };
        if t == 0.0 {
            return Some(a.clone());
        }
        if t == 1.0 {
            return Some(b.clone());
        }
        b.lerp_from(Some(&**a), t)
            .or_else(|| a.lerp_to(Some(&**b), t))
            .or_else(|| {
                if t < 0.5 {
                    a.lerp_to(None, t * 2.0).or_else(|| Some(a.clone()))
                } else {
                    b.lerp_from(None, (t - 0.5) * 2.0)
                        .or_else(|| Some(b.clone()))
                }
            })
    }
}

impl<T: Decoration> From<T> for DecorationRef {
    fn from(decoration: T) -> DecorationRef {
        DecorationRef::new(decoration)
    }
}

impl Deref for DecorationRef {
    type Target = dyn Decoration;

    fn deref(&self) -> &dyn Decoration {
        &*self.0
    }
}

impl PartialEq for DecorationRef {
    fn eq(&self, other: &DecorationRef) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.0.eq_decoration(&*other.0)
    }
}

impl Debug for DecorationRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&*self.0, f)
    }
}

/// A stateful object that can paint a particular [`Decoration`].
///
/// [`BoxPainter`] objects can cache resources so that they can be used
/// multiple times.
///
/// Some resources used by [`BoxPainter`] may load asynchronously. When this
/// happens, the [`on_changed`](Self::on_changed) callback will be invoked. To
/// stop this callback from being called after the painter has been discarded,
/// call [`dispose`](Self::dispose).
pub trait BoxPainter {
    /// Paints the [`Decoration`] for which this object was created on the
    /// given canvas using the given configuration.
    ///
    /// The [`ImageConfiguration`] passed as the third argument must, at a
    /// minimum, have a non-null [`ImageConfiguration::size`].
    fn paint(&mut self, canvas: &mut Canvas, offset: Offset, configuration: &ImageConfiguration);

    /// Callback that is invoked if an asynchronously-loading resource used by
    /// the decoration finishes loading.
    fn on_changed(&self) -> Option<&dyn Fn(&mut App)>;

    /// Discard any resources being held by the object.
    ///
    /// The [`on_changed`](Self::on_changed) callback will not be invoked after
    /// this method has been called.
    fn dispose(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct DummyDecoration;

    impl Decoration for DummyDecoration {
        fn create_box_painter(&self, _on_changed: Option<VoidCallback>) -> Box<dyn BoxPainter> {
            panic!("DummyDecoration does not paint");
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
            if a.is_none() {
                let _ = t;
                Some(DecorationRef::new(*self))
            } else {
                None
            }
        }

        fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
            if b.is_none() {
                let _ = t;
                Some(DecorationRef::new(*self))
            } else {
                None
            }
        }
    }

    #[test]
    fn a_decoration_ref_equals_its_clones_and_not_another_of_an_identity_type() {
        let decoration = DecorationRef::new(DummyDecoration);
        assert_eq!(decoration.clone(), decoration);
        assert_ne!(DecorationRef::new(DummyDecoration), decoration);
    }

    #[test]
    fn lerp_identical_and_null() {
        assert!(DecorationRef::lerp(None, None, 0.0).is_none());
        let decoration = DecorationRef::new(DummyDecoration);
        let lerped = DecorationRef::lerp(Some(&decoration), Some(&decoration), 0.5).unwrap();
        assert!(
            Rc::ptr_eq(&lerped.0, &decoration.0),
            "identical decorations lerp to the same one"
        );
    }
}
