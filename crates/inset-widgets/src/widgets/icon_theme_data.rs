//! Flutter counterpart: `widgets/icon_theme_data.dart`.
//!
//! [`IconThemeData::resolve`] is an inherent method here, not an override point: a
//! `CupertinoIconThemeData` that resolves its color against the [`BuildContext`] waits.

use std::fmt;
use std::rc::Rc;

use inset_embedder::{Color, Shadow, clamp_double, lerp_double};
use inset_foundation::App;
use inset_painting::AnyColor;

use crate::framework::BuildContext;

/// Defines the size, font variations, color, opacity, and shadows of icons.
///
/// Used by [`IconTheme`](crate::IconTheme) to control those properties in a widget subtree.
///
/// To obtain the current icon theme, use [`IconTheme::of`](crate::IconTheme::of). To convert
/// an icon theme to a version with all the fields filled in, use
/// [`IconThemeData::fallback`].
///
/// Dart's named constructor arguments are the fluent setters
/// (`IconThemeData::new().size(16.0).opacity(0.5)`).
#[derive(Clone)]
pub struct IconThemeData {
    /// The default for `Icon.size`.
    ///
    /// Falls back to 24.0.
    pub size: Option<f64>,
    /// The default for `Icon.fill`.
    ///
    /// Falls back to 0.0.
    pub fill: Option<f64>,
    /// The default for `Icon.weight`.
    ///
    /// Falls back to 400.0.
    pub weight: Option<f64>,
    /// The default for `Icon.grade`.
    ///
    /// Falls back to 0.0.
    pub grade: Option<f64>,
    /// The default for `Icon.opticalSize`.
    ///
    /// Falls back to 48.0.
    pub optical_size: Option<f64>,
    /// The default for `Icon.color`.
    ///
    /// In material apps, if there is a `Theme` without any [`IconTheme`](crate::IconTheme)s
    /// specified, icon colors default to white if `ThemeData.brightness` is dark and black if
    /// `ThemeData.brightness` is light.
    ///
    /// Otherwise, falls back to black.
    pub color: Option<AnyColor>,
    /// An opacity to apply to both explicit and default icon colors.
    ///
    /// Falls back to 1.0. Dart clamps its private `_opacity` in this getter; here the value is
    /// clamped between 0.0 and 1.0 when it is set.
    pub opacity: Option<f64>,
    /// The default for `Icon.shadows`.
    pub shadows: Option<Vec<Shadow>>,
    /// The default for `Icon.applyTextScaling`.
    pub apply_text_scaling: Option<bool>,
    /// A subclass's [`resolve`](Self::resolve) override (`CupertinoIconThemeData.resolve`);
    /// `None` is the base class.
    resolver: Option<IconThemeDataResolver>,
}

/// Dart's `IconThemeData.resolve(BuildContext)` override: given the data, returns the data
/// that fits the context.
pub type IconThemeDataResolver =
    Rc<dyn Fn(&IconThemeData, &mut App, BuildContext) -> IconThemeData>;

impl IconThemeData {
    /// Creates an icon theme data.
    ///
    /// The opacity applies to both explicit and default icon colors. The value is clamped
    /// between 0.0 and 1.0.
    pub fn new() -> IconThemeData {
        IconThemeData {
            size: None,
            fill: None,
            weight: None,
            grade: None,
            optical_size: None,
            color: None,
            opacity: None,
            shadows: None,
            apply_text_scaling: None,
            resolver: None,
        }
    }

    /// Creates an icon theme with some reasonable default values.
    ///
    /// The [`size`](Self::size) is 24.0, [`fill`](Self::fill) is 0.0, [`weight`](Self::weight)
    /// is 400.0, [`grade`](Self::grade) is 0.0, opticalSize is 48.0, [`color`](Self::color) is
    /// black, and [`opacity`](Self::opacity) is 1.0.
    pub fn fallback() -> IconThemeData {
        IconThemeData {
            size: Some(24.0),
            fill: Some(0.0),
            weight: Some(400.0),
            grade: Some(0.0),
            optical_size: Some(48.0),
            color: Some(AnyColor::new(Color::new(0xFF000000))),
            opacity: Some(1.0),
            shadows: None,
            apply_text_scaling: Some(false),
            resolver: None,
        }
    }

    /// Dart `IconThemeData(size:)`.
    pub fn size(mut self, size: f64) -> IconThemeData {
        self.size = Some(size);
        self
    }

    /// Dart `IconThemeData(fill:)`. Must be between 0.0 and 1.0 inclusive.
    pub fn fill(mut self, fill: f64) -> IconThemeData {
        self.fill = Some(fill);
        self.debug_assert_ranges();
        self
    }

    /// Dart `IconThemeData(weight:)`. Must be greater than 0.0.
    pub fn weight(mut self, weight: f64) -> IconThemeData {
        self.weight = Some(weight);
        self.debug_assert_ranges();
        self
    }

    /// Dart `IconThemeData(grade:)`.
    pub fn grade(mut self, grade: f64) -> IconThemeData {
        self.grade = Some(grade);
        self
    }

    /// Dart `IconThemeData(opticalSize:)`. Must be greater than 0.0.
    pub fn optical_size(mut self, optical_size: f64) -> IconThemeData {
        self.optical_size = Some(optical_size);
        self.debug_assert_ranges();
        self
    }

    /// Dart `IconThemeData(color:)`.
    pub fn color(mut self, color: impl Into<AnyColor>) -> IconThemeData {
        self.color = Some(color.into());
        self
    }

    /// Dart `IconThemeData(opacity:)`. The value is clamped between 0.0 and 1.0.
    pub fn opacity(mut self, opacity: f64) -> IconThemeData {
        self.opacity = Some(clamp_opacity(opacity));
        self
    }

    /// Dart `IconThemeData(shadows:)`.
    pub fn shadows(mut self, shadows: Vec<Shadow>) -> IconThemeData {
        self.shadows = Some(shadows);
        self
    }

    /// Dart `IconThemeData(applyTextScaling:)`.
    pub fn apply_text_scaling(mut self, apply_text_scaling: bool) -> IconThemeData {
        self.apply_text_scaling = Some(apply_text_scaling);
        self
    }

    /// Makes this data a subclass instance whose [`resolve`](Self::resolve) runs `resolver`:
    /// Dart's `class CupertinoIconThemeData extends IconThemeData` overriding `resolve`.
    /// `copy_with` and `merge` keep it, as the subclass's `copyWith` does; `lerp` drops it,
    /// as Dart's `IconThemeData.lerp` returns the base class.
    pub fn resolver(mut self, resolver: IconThemeDataResolver) -> IconThemeData {
        self.resolver = Some(resolver);
        self
    }

    /// Dart's constructor asserts on `fill`, `weight`, and `optical_size`.
    fn debug_assert_ranges(&self) {
        debug_assert!(self.fill.is_none_or(|fill| (0.0..=1.0).contains(&fill)));
        debug_assert!(self.weight.is_none_or(|weight| 0.0 < weight));
        debug_assert!(
            self.optical_size
                .is_none_or(|optical_size| 0.0 < optical_size)
        );
    }

    /// Creates a copy of this icon theme but with the given fields replaced with the new
    /// values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().size(20.0)`).
    pub fn copy_with(&self) -> IconThemeData {
        self.clone()
    }

    /// Returns a new icon theme that matches this icon theme but with some values replaced by
    /// the non-null parameters of the given icon theme. If the given icon theme is null,
    /// returns this icon theme.
    pub fn merge(&self, other: Option<&IconThemeData>) -> IconThemeData {
        let Some(other) = other else {
            return self.clone();
        };
        let mut merged = self.copy_with();
        merged.size = other.size.or(self.size);
        merged.fill = other.fill.or(self.fill);
        merged.weight = other.weight.or(self.weight);
        merged.grade = other.grade.or(self.grade);
        merged.optical_size = other.optical_size.or(self.optical_size);
        merged.color = other.color.clone().or_else(|| self.color.clone());
        merged.opacity = other.opacity.or(self.opacity);
        merged.shadows = other.shadows.clone().or_else(|| self.shadows.clone());
        merged.apply_text_scaling = other.apply_text_scaling.or(self.apply_text_scaling);
        merged
    }

    /// Called by [`IconTheme::of`](crate::IconTheme::of) to convert this instance to an
    /// [`IconThemeData`] that fits the given [`BuildContext`].
    ///
    /// This method gives the ambient [`IconThemeData`] a chance to update itself, after it's
    /// been retrieved by [`IconTheme::of`](crate::IconTheme::of), and before being returned as
    /// the final result. For instance, `CupertinoIconThemeData` overrides this method to
    /// resolve [`color`](Self::color), in case [`color`](Self::color) is a
    /// `CupertinoDynamicColor` and needs to be resolved against the given [`BuildContext`]
    /// before it can be used as a regular [`Color`].
    ///
    /// The default implementation returns this [`IconThemeData`] as-is.
    ///
    /// See also:
    ///
    ///  * `CupertinoIconThemeData.resolve` an implementation that resolves the color of
    ///    `CupertinoIconThemeData` before returning.
    pub fn resolve(&self, app: &mut App, context: BuildContext) -> IconThemeData {
        match &self.resolver {
            Some(resolver) => resolver(self, app, context),
            None => self.clone(),
        }
    }

    /// Whether all the properties (except shadows) of this object are non-null.
    pub fn is_concrete(&self) -> bool {
        self.size.is_some()
            && self.fill.is_some()
            && self.weight.is_some()
            && self.grade.is_some()
            && self.optical_size.is_some()
            && self.color.is_some()
            && self.opacity.is_some()
            && self.apply_text_scaling.is_some()
    }

    /// Linearly interpolate between two icon theme data objects.
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning that the
    /// interpolation has not started, returning `a` (or something equivalent to `a`), 1.0
    /// meaning that the interpolation has finished, returning `b` (or something equivalent to
    /// `b`), and values in between meaning that the interpolation is at the relevant point on
    /// the timeline between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid (and can easily be
    /// generated by curves such as `Curves.elasticInOut`).
    ///
    /// Values for `t` are usually obtained from an `Animation<f64>`, such as an
    /// `AnimationController`.
    pub fn lerp(a: Option<&IconThemeData>, b: Option<&IconThemeData>, t: f64) -> IconThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        let lerped = IconThemeData {
            size: lerp_double(a.and_then(|a| a.size), b.and_then(|b| b.size), t),
            fill: lerp_double(a.and_then(|a| a.fill), b.and_then(|b| b.fill), t),
            weight: lerp_double(a.and_then(|a| a.weight), b.and_then(|b| b.weight), t),
            grade: lerp_double(a.and_then(|a| a.grade), b.and_then(|b| b.grade), t),
            optical_size: lerp_double(
                a.and_then(|a| a.optical_size),
                b.and_then(|b| b.optical_size),
                t,
            ),
            color: AnyColor::lerp(
                a.and_then(|a| a.color.as_ref()),
                b.and_then(|b| b.color.as_ref()),
                t,
            )
            .map(AnyColor::from),
            opacity: lerp_double(a.and_then(|a| a.opacity), b.and_then(|b| b.opacity), t)
                .map(clamp_opacity),
            shadows: Shadow::lerp_list(
                a.and_then(|a| a.shadows.as_deref()),
                b.and_then(|b| b.shadows.as_deref()),
                t,
            ),
            apply_text_scaling: if t < 0.5 {
                a.and_then(|a| a.apply_text_scaling)
            } else {
                b.and_then(|b| b.apply_text_scaling)
            },
            resolver: None,
        };
        lerped.debug_assert_ranges();
        lerped
    }
}

impl Default for IconThemeData {
    fn default() -> IconThemeData {
        IconThemeData::new()
    }
}

/// Dart's `opacity` getter: `clampDouble(_opacity, 0.0, 1.0)`.
fn clamp_opacity(opacity: f64) -> f64 {
    clamp_double(opacity, 0.0, 1.0)
}

/// Dart's `==` checks `runtimeType` first: a subclass instance never equals a base one.
impl PartialEq for IconThemeData {
    fn eq(&self, other: &IconThemeData) -> bool {
        self.resolver.is_some() == other.resolver.is_some()
            && self.size == other.size
            && self.fill == other.fill
            && self.weight == other.weight
            && self.grade == other.grade
            && self.optical_size == other.optical_size
            && self.color == other.color
            && self.opacity == other.opacity
            && self.shadows == other.shadows
            && self.apply_text_scaling == other.apply_text_scaling
    }
}

impl fmt::Debug for IconThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IconThemeData")
            .field("size", &self.size)
            .field("fill", &self.fill)
            .field("weight", &self.weight)
            .field("grade", &self.grade)
            .field("optical_size", &self.optical_size)
            .field("color", &self.color)
            .field("opacity", &self.opacity)
            .field("shadows", &self.shadows)
            .field("apply_text_scaling", &self.apply_text_scaling)
            .field("resolver", &self.resolver.as_ref().map(|_| "override"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::Offset;

    use super::*;

    /// Flutter's `isSameColorAs`: equal to within a channel step, since `Color::lerp` works in
    /// floating point.
    fn assert_same_color(actual: Option<Color>, expected: u32) {
        let actual = actual.expect("a color");
        let expected = Color::new(expected);
        let step = 1.0 / 255.0;
        assert!(
            (actual.a - expected.a).abs() < step
                && (actual.r - expected.r).abs() < step
                && (actual.g - expected.g).abs() < step
                && (actual.b - expected.b).abs() < step,
            "{actual:?} is not {expected:?}"
        );
    }

    fn shadow(argb: u32, blur_radius: f64, offset: f64) -> Shadow {
        Shadow::new(Color::new(argb), Offset::new(offset, offset), blur_radius)
    }

    /// Flutter's `IconThemeData lerp` group's fixture.
    fn white_data() -> IconThemeData {
        IconThemeData::new()
            .size(16.0)
            .fill(0.5)
            .weight(600.0)
            .grade(25.0)
            .optical_size(45.0)
            .color(Color::new(0xFFFFFFFF))
            .opacity(1.0)
            .shadows(vec![shadow(0xFFFFFFFF, 1.0, 1.0)])
    }

    #[test]
    fn control_test() {
        let data = IconThemeData::new()
            .size(16.0)
            .fill(0.5)
            .weight(600.0)
            .grade(25.0)
            .optical_size(45.0)
            .color(Color::new(0xAAAAAAAA))
            .opacity(0.5)
            .shadows(vec![shadow(0xAAAAAAAA, 1.0, 1.0)]);
        assert_eq!(data, data.copy_with());

        let lerped = IconThemeData::lerp(Some(&data), Some(&IconThemeData::fallback()), 0.25);
        assert_eq!(lerped.size, Some(18.0));
        assert_eq!(lerped.fill, Some(0.375));
        assert_eq!(lerped.weight, Some(550.0));
        assert_eq!(lerped.grade, Some(18.75));
        assert_eq!(lerped.optical_size, Some(45.75));
        assert_same_color(lerped.color.as_ref().map(AnyColor::color), 0xBF7F7F7F);
        assert_eq!(lerped.opacity, Some(0.625));
        assert_eq!(lerped.shadows, Some(vec![shadow(0xAAAAAAAA, 0.75, 0.75)]));
    }

    #[test]
    fn lerp_with_first_null() {
        let lerped = IconThemeData::lerp(None, Some(&white_data()), 0.25);
        assert_eq!(lerped.size, Some(4.0));
        assert_eq!(lerped.fill, Some(0.125));
        assert_eq!(lerped.weight, Some(150.0));
        assert_eq!(lerped.grade, Some(6.25));
        assert_eq!(lerped.optical_size, Some(11.25));
        assert_same_color(lerped.color.as_ref().map(AnyColor::color), 0x40FFFFFF);
        assert_eq!(lerped.opacity, Some(0.25));
        assert_eq!(lerped.shadows, Some(vec![shadow(0xFFFFFFFF, 0.25, 0.25)]));
    }

    #[test]
    fn lerp_with_second_null() {
        let lerped = IconThemeData::lerp(Some(&white_data()), None, 0.25);
        assert_eq!(lerped.size, Some(12.0));
        assert_eq!(lerped.fill, Some(0.375));
        assert_eq!(lerped.weight, Some(450.0));
        assert_eq!(lerped.grade, Some(18.75));
        assert_eq!(lerped.optical_size, Some(33.75));
        assert_same_color(lerped.color.as_ref().map(AnyColor::color), 0xBFFFFFFF);
        assert_eq!(lerped.opacity, Some(0.75));
        assert_eq!(lerped.shadows, Some(vec![shadow(0xFFFFFFFF, 0.75, 0.75)]));
    }

    #[test]
    fn lerp_with_both_null_and_special_cases() {
        assert_eq!(IconThemeData::lerp(None, None, 0.25), IconThemeData::new());
        let data = IconThemeData::new();
        assert_eq!(IconThemeData::lerp(Some(&data), Some(&data), 0.5), data);
    }

    #[test]
    fn merge_takes_the_other_values_where_it_has_them() {
        let black = Color::new(0xFF000000);
        let base = IconThemeData::new().size(16.0).color(black).opacity(0.5);
        let other = IconThemeData::new().size(20.0).fill(1.0);
        let merged = base.merge(Some(&other));
        assert_eq!(merged.size, Some(20.0));
        assert_eq!(merged.fill, Some(1.0));
        assert_eq!(merged.color, Some(black.into()));
        assert_eq!(merged.opacity, Some(0.5));
        assert_eq!(merged.weight, None);
        assert_eq!(base.merge(None), base);
    }

    #[test]
    fn opacity_is_clamped_and_compared_clamped() {
        assert_eq!(IconThemeData::new().opacity(1.5).opacity, Some(1.0));
        assert_eq!(IconThemeData::new().opacity(-1.0).opacity, Some(0.0));
        assert_eq!(
            IconThemeData::new().opacity(1.5),
            IconThemeData::new().opacity(1.0)
        );
    }

    #[test]
    fn is_concrete_ignores_shadows() {
        assert!(IconThemeData::fallback().is_concrete());
        assert!(IconThemeData::fallback().shadows(Vec::new()).is_concrete());
        assert!(!IconThemeData::new().is_concrete());
        assert!(!IconThemeData::new().size(16.0).is_concrete());
    }

    #[test]
    #[should_panic]
    fn throws_if_given_invalid_values() {
        let _ = IconThemeData::new().fill(1.1);
    }
}
