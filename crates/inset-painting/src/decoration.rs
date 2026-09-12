//! Flutter counterpart: `painting/decoration.dart`.

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use inset_embedder::{Canvas, Offset, Path, Rect, Size};

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
    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        let _ = (a, t);
        None
    }

    /// Linearly interpolates from `this` to another [`Decoration`] (which may be of
    /// a different class).
    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
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
    fn create_box_painter(&self, on_changed: Option<Box<dyn Fn()>>) -> Box<dyn BoxPainter>;

    /// Returns a closed [`Path`] that describes the outer edge of this decoration.
    ///
    /// The default implementation panics. Subclasses must override this
    /// implementation to describe the clip path that should be applied to the
    /// decoration when it is used in a `Container` with an explicit clip behavior.
    fn get_clip_path(&self, rect: Rect, text_direction: TextDirection) -> Arc<Path> {
        let _ = (rect, text_direction);
        panic!(
            "{} does not expect to be used for clipping.",
            std::any::type_name::<Self>()
        );
    }

    /// A heap clone, so lerp can duplicate like Dart.
    fn clone_box(&self) -> Box<dyn Decoration>;

    /// Downcast support for `is` / `as` in Dart.
    fn as_any(&self) -> &dyn Any;

    /// Field equality. Dart's default `==` is identity; subclasses that override
    /// `==` implement this.
    fn eq_decoration(&self, other: &dyn Decoration) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

impl dyn Decoration {
    /// Linearly interpolates between two [`Decoration`]s.
    ///
    /// This attempts to use [`lerp_from`](Decoration::lerp_from) and
    /// [`lerp_to`](Decoration::lerp_to) on `b` and `a` respectively to find a
    /// solution. If the two values can't directly be interpolated, then the
    /// interpolation is done via null (at `t == 0.5`).
    pub fn lerp(
        a: Option<&dyn Decoration>,
        b: Option<&dyn Decoration>,
        t: f64,
    ) -> Option<Box<dyn Decoration>> {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a.as_any(), b.as_any())
        {
            return Some(a.clone_box());
        }
        if a.is_none() {
            return match b {
                None => None,
                Some(b) => b.lerp_from(None, t).or_else(|| Some(b.clone_box())),
            };
        }
        if b.is_none() {
            return a
                .unwrap()
                .lerp_to(None, t)
                .or_else(|| Some(a.unwrap().clone_box()));
        }
        if t == 0.0 {
            return a.map(Decoration::clone_box);
        }
        if t == 1.0 {
            return b.map(Decoration::clone_box);
        }
        let a = a.unwrap();
        let b = b.unwrap();
        b.lerp_from(Some(a), t)
            .or_else(|| a.lerp_to(Some(b), t))
            .or_else(|| {
                if t < 0.5 {
                    a.lerp_to(None, t * 2.0).or_else(|| Some(a.clone_box()))
                } else {
                    b.lerp_from(None, (t - 0.5) * 2.0)
                        .or_else(|| Some(b.clone_box()))
                }
            })
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
    fn on_changed(&self) -> Option<&dyn Fn()>;

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
        fn create_box_painter(&self, _on_changed: Option<Box<dyn Fn()>>) -> Box<dyn BoxPainter> {
            panic!("DummyDecoration does not paint");
        }

        fn clone_box(&self) -> Box<dyn Decoration> {
            Box::new(*self)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
            if a.is_none() {
                let _ = t;
                Some(self.clone_box())
            } else {
                None
            }
        }

        fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
            if b.is_none() {
                let _ = t;
                Some(self.clone_box())
            } else {
                None
            }
        }
    }

    #[test]
    fn lerp_identical_and_null() {
        assert!(<dyn Decoration>::lerp(None, None, 0.0).is_none());
        let decoration = DummyDecoration;
        let lerped = <dyn Decoration>::lerp(Some(&decoration), Some(&decoration), 0.5).unwrap();
        assert!(lerped.as_any().downcast_ref::<DummyDecoration>().is_some());
    }
}
