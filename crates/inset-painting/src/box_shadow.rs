//! Flutter counterpart: `painting/box_shadow.dart`.

use std::fmt::{self, Debug};

use inset_embedder::{BlurStyle, MaskBlur, Paint};
use inset_embedder::{Color, Offset, Shadow, lerp_double};

use crate::debug::debug_disable_shadows;

/// A shadow cast by a box.
///
/// [`BoxShadow`] can cast non-rectangular shadows if the box is non-rectangular
/// (e.g., has a border radius or a circular shape).
///
/// This class is similar to CSS box-shadow.
///
/// Dart: `extends ui.Shadow`. Separate struct; convert with [`From`].
#[derive(Clone, Copy, PartialEq)]
pub struct BoxShadow {
    /// Color that the shadow will be drawn with.
    pub color: Color,
    /// The displacement of the shadow from the casting element.
    pub offset: Offset,
    /// The standard deviation of the Gaussian to convolve with the shadow's shape.
    pub blur_radius: f64,
    /// The amount the box should be inflated prior to applying the blur.
    pub spread_radius: f64,
    /// The [`BlurStyle`] to use for this shadow.
    ///
    /// Defaults to [`BlurStyle::Normal`].
    ///
    /// When [`crate::debug_disable_shadows`] is true, [`to_paint`](Self::to_paint)
    /// ignores the [`blur_style`](Self::blur_style) and acts as if
    /// [`BlurStyle::Normal`] was used.
    pub blur_style: BlurStyle,
}

impl BoxShadow {
    /// Creates a box shadow.
    ///
    /// By default, the shadow is solid black with zero [`offset`](Self::offset),
    /// zero [`blur_radius`](Self::blur_radius), zero [`spread_radius`](Self::spread_radius),
    /// and [`BlurStyle::Normal`].
    pub fn new(
        color: Color,
        offset: Offset,
        blur_radius: f64,
        spread_radius: f64,
        blur_style: BlurStyle,
    ) -> BoxShadow {
        debug_assert!(
            blur_radius >= 0.0,
            "Text shadow blur radius should be non-negative."
        );
        BoxShadow {
            color,
            offset,
            blur_radius,
            spread_radius,
            blur_style,
        }
    }

    /// The [`blur_radius`](Self::blur_radius) in sigmas instead of logical pixels.
    pub fn blur_sigma(&self) -> f64 {
        Shadow::convert_radius_to_sigma(self.blur_radius)
    }

    /// Create the [`Paint`] object that corresponds to this shadow description.
    ///
    /// The [`offset`](Self::offset) and [`spread_radius`](Self::spread_radius) are
    /// not represented in the [`Paint`] object. To honor those as well, the shape
    /// should be inflated by [`spread_radius`](Self::spread_radius) pixels in every
    /// direction and then translated by [`offset`](Self::offset) before being
    /// filled using this [`Paint`].
    ///
    /// The [`blur_style`](Self::blur_style) is ignored if [`crate::debug_disable_shadows`]
    /// is true. This causes an especially significant change to the rendering
    /// when [`BlurStyle::Outer`] is used; the caller is responsible for adjusting
    /// for that case if necessary.
    pub fn to_paint(&self) -> Paint {
        let sigma = self.blur_sigma() as f32;
        let mask_blur = match self.blur_style {
            BlurStyle::Normal => MaskBlur::new(sigma),
            BlurStyle::Solid => MaskBlur::solid(sigma),
            BlurStyle::Inner => MaskBlur::inner(sigma),
            BlurStyle::Outer => MaskBlur::outer(sigma),
        };
        let mut result = Paint {
            color: self.color.into(),
            mask_blur: Some(mask_blur),
            ..Paint::default()
        };
        if cfg!(debug_assertions) && debug_disable_shadows() {
            result.mask_blur = None;
        }
        result
    }

    /// Returns a new box shadow with its offset, blurRadius, and spreadRadius
    /// scaled by the given factor.
    pub fn scale(&self, factor: f64) -> BoxShadow {
        BoxShadow::new(
            self.color,
            self.offset * factor,
            self.blur_radius * factor,
            self.spread_radius * factor,
            self.blur_style,
        )
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    pub fn copy_with(
        &self,
        color: Option<Color>,
        offset: Option<Offset>,
        blur_radius: Option<f64>,
        spread_radius: Option<f64>,
        blur_style: Option<BlurStyle>,
    ) -> BoxShadow {
        BoxShadow::new(
            color.unwrap_or(self.color),
            offset.unwrap_or(self.offset),
            blur_radius.unwrap_or(self.blur_radius),
            spread_radius.unwrap_or(self.spread_radius),
            blur_style.unwrap_or(self.blur_style),
        )
    }

    /// Linearly interpolate between two box shadows.
    ///
    /// If either box shadow is null, this function linearly interpolates from
    /// a box shadow that matches the other box shadow in color but has a zero
    /// offset and a zero blurRadius.
    pub fn lerp(a: Option<BoxShadow>, b: Option<BoxShadow>, t: f64) -> Option<BoxShadow> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (Some(a), Some(b)) => Some(BoxShadow::new(
                Color::lerp(Some(a.color), Some(b.color), t).unwrap(),
                Offset::lerp(Some(a.offset), Some(b.offset), t).unwrap(),
                lerp_double(Some(a.blur_radius), Some(b.blur_radius), t).unwrap(),
                lerp_double(Some(a.spread_radius), Some(b.spread_radius), t).unwrap(),
                if a.blur_style == BlurStyle::Normal {
                    b.blur_style
                } else {
                    a.blur_style
                },
            )),
        }
    }

    /// Linearly interpolate between two lists of box shadows.
    ///
    /// If the lists differ in length, excess items are lerped with null.
    pub fn lerp_list(
        a: Option<&[BoxShadow]>,
        b: Option<&[BoxShadow]>,
        t: f64,
    ) -> Option<Vec<BoxShadow>> {
        if let (Some(a), Some(b)) = (a, b) {
            if a == b {
                return Some(a.to_vec());
            }
        } else if a.is_none() && b.is_none() {
            return None;
        }
        let a = a.unwrap_or(&[]);
        let b = b.unwrap_or(&[]);
        let common_length = a.len().min(b.len());
        let mut result = Vec::new();
        for i in 0..common_length {
            result.push(BoxShadow::lerp(Some(a[i]), Some(b[i]), t).unwrap());
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

impl Default for BoxShadow {
    fn default() -> BoxShadow {
        BoxShadow::new(
            Color::new(0xFF000000),
            Offset::ZERO,
            0.0,
            0.0,
            BlurStyle::Normal,
        )
    }
}

impl From<BoxShadow> for Shadow {
    fn from(shadow: BoxShadow) -> Shadow {
        Shadow::new(shadow.color, shadow.offset, shadow.blur_radius)
    }
}

impl Debug for BoxShadow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BoxShadow({:?}, {:?}, {:.1}, {:.1}, {:?})",
            self.color, self.offset, self.blur_radius, self.spread_radius, self.blur_style
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::set_debug_disable_shadows;

    #[test]
    fn lerp_null_arms_and_list_padding_match_dart() {
        let shadow1 = BoxShadow {
            blur_radius: 4.0,
            ..BoxShadow::default()
        };
        let shadow2 = BoxShadow::lerp(None, Some(shadow1), 0.25).unwrap();
        let shadow3 = BoxShadow::lerp(Some(shadow1), None, 0.25).unwrap();
        assert_eq!(
            shadow2,
            BoxShadow {
                blur_radius: 1.0,
                ..BoxShadow::default()
            }
        );
        assert_eq!(
            shadow3,
            BoxShadow {
                blur_radius: 3.0,
                ..BoxShadow::default()
            }
        );
        assert_eq!(
            shadow1,
            BoxShadow {
                blur_radius: 4.0,
                ..BoxShadow::default()
            }
        );

        let shadow4 = BoxShadow::lerp(Some(shadow2), Some(shadow3), 0.5).unwrap();
        assert_eq!(
            shadow4,
            BoxShadow {
                blur_radius: 2.0,
                ..BoxShadow::default()
            }
        );

        let shadow_list =
            BoxShadow::lerp_list(Some(&[shadow2, shadow1]), Some(&[shadow3]), 0.5).unwrap();
        assert_eq!(shadow_list, vec![shadow4, shadow1.scale(0.5)]);
        let shadow_list =
            BoxShadow::lerp_list(Some(&[shadow2]), Some(&[shadow3, shadow1]), 0.5).unwrap();
        assert_eq!(shadow_list, vec![shadow4, shadow1.scale(0.5)]);
    }

    #[test]
    fn lerp_identical_a_b() {
        assert_eq!(BoxShadow::lerp(None, None, 0.0), None);
        let border = BoxShadow::default();
        assert_eq!(
            BoxShadow::lerp(Some(border), Some(border), 0.5),
            Some(border)
        );
    }

    #[test]
    fn lerp_list_identical_a_b() {
        assert_eq!(BoxShadow::lerp_list(None, None, 0.0), None);
        let border = [BoxShadow::default()];
        assert_eq!(
            BoxShadow::lerp_list(Some(&border), Some(&border), 0.5),
            Some(border.to_vec())
        );
    }

    #[test]
    fn blur_style_lerp_keeps_non_normal() {
        let shadow1 = BoxShadow {
            blur_radius: 4.0,
            ..BoxShadow::default()
        };
        let shadow2 = BoxShadow {
            blur_radius: 4.0,
            blur_style: BlurStyle::Outer,
            ..BoxShadow::default()
        };
        let shadow5 = BoxShadow::lerp(Some(shadow1), Some(shadow2), 0.25).unwrap();
        assert_eq!(shadow5.blur_style, BlurStyle::Outer);
        let shadow6 = BoxShadow::lerp(
            Some(BoxShadow {
                blur_style: BlurStyle::Solid,
                ..BoxShadow::default()
            }),
            Some(shadow1),
            0.25,
        )
        .unwrap();
        assert_eq!(shadow6.blur_style, BlurStyle::Solid);
    }

    #[test]
    fn copy_with_replaces_one_field() {
        assert_ne!(
            BoxShadow::default(),
            BoxShadow {
                color: Color::new(0xFF112233),
                ..BoxShadow::default()
            }
        );
        assert_eq!(
            BoxShadow::default().copy_with(Some(Color::new(0xFF112233)), None, None, None, None),
            BoxShadow {
                color: Color::new(0xFF112233),
                ..BoxShadow::default()
            }
        );
        assert_eq!(
            BoxShadow::default().copy_with(None, Some(Offset::new(1.0, 2.0)), None, None, None),
            BoxShadow {
                offset: Offset::new(1.0, 2.0),
                ..BoxShadow::default()
            }
        );
        assert_eq!(
            BoxShadow::default().copy_with(None, None, Some(123.0), None, None),
            BoxShadow {
                blur_radius: 123.0,
                ..BoxShadow::default()
            }
        );
        assert_eq!(
            BoxShadow::default().copy_with(None, None, None, Some(123.0), None),
            BoxShadow {
                spread_radius: 123.0,
                ..BoxShadow::default()
            }
        );
        assert_eq!(
            BoxShadow::default().copy_with(None, None, None, None, Some(BlurStyle::Outer)),
            BoxShadow {
                blur_style: BlurStyle::Outer,
                ..BoxShadow::default()
            }
        );
    }

    #[test]
    fn to_paint_drops_mask_blur_when_debug_disable_shadows() {
        set_debug_disable_shadows(true);
        let paint = BoxShadow {
            blur_radius: 4.0,
            ..BoxShadow::default()
        }
        .to_paint();
        set_debug_disable_shadows(false);
        if cfg!(debug_assertions) {
            assert!(paint.mask_blur.is_none());
        } else {
            assert!(paint.mask_blur.is_some());
        }
    }
}
