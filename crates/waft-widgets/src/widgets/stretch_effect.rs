//! Flutter counterpart: `widgets/stretch_effect.dart`.
//!
//! Flutter draws the stretch with a fragment shader where the engine supports shader image
//! filters, and with a matrix scale everywhere else. Waft has no custom shaders, so the matrix
//! scale is the one path; it is the look Flutter's own fallback gives.

use std::fmt::{self, Debug};

use waft_embedder::{Matrix4, TextDirection};
use waft_foundation::App;
use waft_painting::{AlignmentDirectional, AlignmentGeometry, Axis};

use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::{Directionality, Transform};

/// A widget that applies a stretching visual effect to its child.
///
/// When shader-based effects are supported, this effect replicates the native Android stretch
/// overscroll effect. Otherwise, a matrix transform provides an approximation.
///
/// Used by `StretchingOverscrollIndicator` widget.
pub struct StretchEffect {
    pub key: Option<KeyRef>,

    /// The overscroll strength applied for the stretching effect.
    ///
    /// The value should be between -1.0 and 1.0 inclusive.
    ///
    /// For the horizontal axis:
    /// - Positive values apply a pull/stretch from left to right,
    ///   where 1.0 represents the maximum stretch to the right.
    /// - Negative values apply a pull/stretch from right to left,
    ///   where -1.0 represents the maximum stretch to the left.
    ///
    /// For the vertical axis:
    /// - Positive values apply a pull/stretch from top to bottom,
    ///   where 1.0 represents the maximum stretch downward.
    /// - Negative values apply a pull/stretch from bottom to top,
    ///   where -1.0 represents the maximum stretch upward.
    ///
    /// This example sets the horizontal stretch strength to pull right:
    ///
    /// ```ignore
    /// StretchEffect::new(Axis::Horizontal, Text::new("Hello, World!")).stretch_strength(0.5)
    /// ```
    pub stretch_strength: f64,

    /// The axis along which the stretching overscroll effect is applied.
    ///
    /// Determines the direction of the stretch, either horizontal or vertical.
    pub axis: Axis,

    /// The child widget that the stretching overscroll effect applies to.
    pub child: WidgetRef,
}

impl StretchEffect {
    /// Creates a [`StretchEffect`] widget that applies a stretch effect
    /// when the user overscrolls horizontally or vertically.
    ///
    /// The [`stretch_strength`](Self::stretch_strength) controls the intensity of the stretch
    /// effect and must be between -1.0 and 1.0.
    pub fn new<K>(axis: Axis, child: impl IntoWidget<K>) -> StretchEffect {
        StretchEffect {
            key: None,
            stretch_strength: 0.0,
            axis,
            child: child.into_widget(),
        }
    }

    /// Dart `StretchEffect(key:)`.
    pub fn key(mut self, key: KeyRef) -> StretchEffect {
        self.key = Some(key);
        self
    }

    /// Dart `StretchEffect(stretchStrength:)`.
    pub fn stretch_strength(mut self, stretch_strength: f64) -> StretchEffect {
        debug_assert!(
            (-1.0..=1.0).contains(&stretch_strength),
            "stretchStrength must be between -1.0 and 1.0"
        );
        self.stretch_strength = stretch_strength;
        self
    }

    fn get_alignment(&self, direction: TextDirection) -> AlignmentGeometry {
        let is_forward = self.stretch_strength > 0.0;

        if self.axis == Axis::Vertical {
            return if is_forward {
                AlignmentGeometry::Directional(AlignmentDirectional::TOP_CENTER)
            } else {
                AlignmentGeometry::Directional(AlignmentDirectional::BOTTOM_CENTER)
            };
        }

        // RTL horizontal.
        if direction == TextDirection::Rtl {
            if is_forward {
                AlignmentGeometry::CENTER_END
            } else {
                AlignmentGeometry::CENTER_START
            }
        } else if is_forward {
            AlignmentGeometry::CENTER_START
        } else {
            AlignmentGeometry::CENTER_END
        }
    }
}

impl Debug for StretchEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StretchEffect")
            .field("stretchStrength", &self.stretch_strength)
            .field("axis", &self.axis)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for StretchEffect {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_direction = Directionality::of(app, context);
        let mut x = 1.0;
        let mut y = 1.0;

        match self.axis {
            Axis::Horizontal => x += self.stretch_strength.abs(),
            Axis::Vertical => y += self.stretch_strength.abs(),
        }

        // Dart's `Matrix4.diagonal3Values(x, y, 1.0)`.
        Transform::new(Matrix4::scale(x as f32, y as f32))
            .alignment(self.get_alignment(text_direction))
            .child(self.child.clone())
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_foundation::AppCell;

    use super::*;
    use crate::framework::downcast_widget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};

    /// What `StretchEffect::build` returns under `text_direction`.
    fn built(effect: StretchEffect, text_direction: TextDirection) -> WidgetRef {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let effect = Rc::new(effect);
        let built: Rc<RefCell<Option<WidgetRef>>> = Rc::default();
        let probe = Builder::new({
            let built = Rc::clone(&built);
            move |app, context| {
                *built.borrow_mut() = Some(effect.build(app, context));
                SizedBox::shrink().into_widget()
            }
        });
        let harness = Harness::mount(
            &mut app,
            Directionality::new(text_direction, probe).into_widget(),
        );
        harness.pump(&mut app);
        let widget = built.borrow().clone();
        widget.expect("the probe built")
    }

    fn transform_of(widget: &WidgetRef) -> &Transform {
        downcast_widget::<Transform>(&**widget).expect("the stretch is a Transform")
    }

    #[test]
    fn a_vertical_stretch_scales_from_the_edge_it_is_pulled_from() {
        let pulled_down = built(
            StretchEffect::new(Axis::Vertical, SizedBox::shrink()).stretch_strength(0.5),
            TextDirection::Ltr,
        );
        let transform = transform_of(&pulled_down);
        assert_eq!(transform.transform, Matrix4::scale(1.0, 1.5));
        assert_eq!(
            transform.alignment,
            Some(AlignmentGeometry::Directional(
                AlignmentDirectional::TOP_CENTER
            ))
        );

        let pulled_up = built(
            StretchEffect::new(Axis::Vertical, SizedBox::shrink()).stretch_strength(-0.25),
            TextDirection::Ltr,
        );
        let transform = transform_of(&pulled_up);
        assert_eq!(transform.transform, Matrix4::scale(1.0, 1.25));
        assert_eq!(
            transform.alignment,
            Some(AlignmentGeometry::Directional(
                AlignmentDirectional::BOTTOM_CENTER
            ))
        );
    }

    #[test]
    fn a_horizontal_stretch_scales_from_the_start_or_end_by_text_direction() {
        for (strength, text_direction, alignment) in [
            (0.5, TextDirection::Ltr, AlignmentGeometry::CENTER_START),
            (-0.5, TextDirection::Ltr, AlignmentGeometry::CENTER_END),
            (0.5, TextDirection::Rtl, AlignmentGeometry::CENTER_END),
            (-0.5, TextDirection::Rtl, AlignmentGeometry::CENTER_START),
        ] {
            let stretched = built(
                StretchEffect::new(Axis::Horizontal, SizedBox::shrink()).stretch_strength(strength),
                text_direction,
            );
            let transform = transform_of(&stretched);
            assert_eq!(transform.transform, Matrix4::scale(1.5, 1.0));
            assert_eq!(
                transform.alignment,
                Some(alignment),
                "{strength} {text_direction:?}"
            );
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "stretchStrength must be between -1.0 and 1.0")]
    fn a_stretch_strength_past_one_is_refused() {
        let _ = StretchEffect::new(Axis::Vertical, SizedBox::shrink()).stretch_strength(1.5);
    }
}
