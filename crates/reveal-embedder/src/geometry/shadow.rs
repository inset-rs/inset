//! Flutter counterpart: dart:ui `Shadow` in `engine/src/flutter/lib/ui/painting.dart`.

use std::fmt::{self, Debug};

use crate::color::Color;
use crate::geometry::Offset;
use crate::lerp::lerp_double_non_null;

/// A representation of a shadow.
#[derive(Clone, Copy, PartialEq)]
pub struct Shadow {
    /// Color that the shadow will be drawn with.
    ///
    /// The shadows are shapes composited directly over the base canvas, and do not
    /// represent optical occlusion.
    pub color: Color,
    /// The displacement of the shadow from the casting element.
    ///
    /// Positive x/y offsets will shift the shadow to the right and down, while
    /// negative offsets shift the shadow to the left and up. The offsets are
    /// relative to the position of the element that is casting it.
    pub offset: Offset,
    /// The standard deviation of the Gaussian to convolve with the shadow's shape.
    pub blur_radius: f64,
}

impl Shadow {
    /// Construct a shadow.
    ///
    /// The default shadow is a black shadow with zero offset and zero blur.
    /// Default shadows should be completely covered by the casting element,
    /// and not be visible.
    ///
    /// Transparency should be adjusted through the [`color`](Shadow::color) alpha.
    ///
    /// Shadow order matters due to compositing multiple translucent objects not
    /// being commutative.
    pub fn new(color: Color, offset: Offset, blur_radius: f64) -> Shadow {
        debug_assert!(
            blur_radius >= 0.0,
            "Text shadow blur radius should be non-negative."
        );
        Shadow {
            color,
            offset,
            blur_radius,
        }
    }

    /// Converts a blur radius in pixels to sigmas.
    ///
    /// See SkBlurMask::ConvertRadiusToSigma().
    pub fn convert_radius_to_sigma(radius: f64) -> f64 {
        if radius > 0.0 {
            radius * 0.57735 + 0.5
        } else {
            0.0
        }
    }

    /// The [`blur_radius`](Shadow::blur_radius) in sigmas instead of logical pixels.
    pub fn blur_sigma(&self) -> f64 {
        Self::convert_radius_to_sigma(self.blur_radius)
    }

    /// Returns a new shadow with its [`offset`](Shadow::offset) and
    /// [`blur_radius`](Shadow::blur_radius) scaled by the given factor.
    pub fn scale(&self, factor: f64) -> Shadow {
        Shadow::new(self.color, self.offset * factor, self.blur_radius * factor)
    }

    /// Linearly interpolate between two shadows.
    ///
    /// If either shadow is null, this function linearly interpolates from
    /// a shadow that matches the other shadow in color but has a zero
    /// offset and a zero blurRadius.
    pub fn lerp(a: Option<Shadow>, b: Option<Shadow>, t: f64) -> Option<Shadow> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), Some(b)) => Some(Shadow::new(
                Color::lerp(Some(a.color), Some(b.color), t).unwrap(),
                Offset::lerp(Some(a.offset), Some(b.offset), t).unwrap(),
                lerp_double_non_null(a.blur_radius, b.blur_radius, t),
            )),
        }
    }

    /// Linearly interpolate between two lists of shadows.
    ///
    /// If the lists differ in length, excess items are lerped with null.
    pub fn lerp_list(a: Option<&[Shadow]>, b: Option<&[Shadow]>, t: f64) -> Option<Vec<Shadow>> {
        if a.is_none() && b.is_none() {
            return None;
        }
        let a = a.unwrap_or(&[]);
        let b = b.unwrap_or(&[]);
        let common_length = a.len().min(b.len());
        let mut result = Vec::new();
        for i in 0..common_length {
            result.push(Shadow::lerp(Some(a[i]), Some(b[i]), t).unwrap());
        }
        for shadow in &a[common_length..] {
            result.push(shadow.scale(1.0 - t));
        }
        for shadow in &b[common_length..] {
            result.push(shadow.scale(t));
        }
        Some(result)
    }
}

impl Default for Shadow {
    fn default() -> Shadow {
        Shadow::new(Color::new(0xFF000000), Offset::ZERO, 0.0)
    }
}

impl Debug for Shadow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TextShadow({:?}, {:?}, {})",
            self.color, self.offset, self.blur_radius
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_radius_to_sigma_matches_skia() {
        assert_eq!(Shadow::convert_radius_to_sigma(0.0), 0.0);
        assert_eq!(Shadow::convert_radius_to_sigma(-1.0), 0.0);
        assert_eq!(Shadow::convert_radius_to_sigma(1.0), 1.0 * 0.57735 + 0.5);
    }

    #[test]
    fn lerp_null_arms_scale_toward_nothing() {
        let shadow = Shadow::new(Color::new(0xFF000000), Offset::new(8.0, 0.0), 4.0);
        assert_eq!(
            Shadow::lerp(None, Some(shadow), 0.25).unwrap(),
            shadow.scale(0.25)
        );
        assert_eq!(
            Shadow::lerp(Some(shadow), None, 0.25).unwrap(),
            shadow.scale(0.75)
        );
        assert_eq!(Shadow::lerp(None, None, 0.25), None);
    }
}
