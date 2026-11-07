//! Flutter counterpart: `animation/animation_style.dart`.

use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use crate::curves::{Curve, Curves};

/// Used to override the default parameters of an animation.
///
/// If [`duration`] and [`reverse_duration`] are set to [`Duration::ZERO`], the
/// corresponding animation will be disabled.
///
/// All of the parameters are optional. If no parameters are specified, the
/// default animation will be used.
///
/// [`duration`]: AnimationStyle::duration
/// [`reverse_duration`]: AnimationStyle::reverse_duration
#[derive(Clone, Default)]
pub struct AnimationStyle {
    /// When specified, the animation will use this curve.
    pub curve: Option<Rc<dyn Curve>>,

    /// When specified, the animation will use this duration.
    pub duration: Option<Duration>,

    /// When specified, the reverse animation will use this curve.
    pub reverse_curve: Option<Rc<dyn Curve>>,

    /// When specified, the reverse animation will use this duration.
    pub reverse_duration: Option<Duration>,
}

impl AnimationStyle {
    /// An [`AnimationStyle`] with no animation.
    ///
    /// Dart's `AnimationStyle.noAnimation`.
    pub const NO_ANIMATION: AnimationStyle = AnimationStyle {
        curve: None,
        duration: Some(Duration::ZERO),
        reverse_curve: None,
        reverse_duration: Some(Duration::ZERO),
    };

    /// Creates an instance of Animation Style class.
    pub fn new(
        curve: Option<Rc<dyn Curve>>,
        duration: Option<Duration>,
        reverse_curve: Option<Rc<dyn Curve>>,
        reverse_duration: Option<Duration>,
    ) -> AnimationStyle {
        AnimationStyle {
            curve,
            duration,
            reverse_curve,
            reverse_duration,
        }
    }

    /// Creates a new [`AnimationStyle`] based on the current selection, with
    /// the provided parameters overridden.
    pub fn copy_with(
        &self,
        curve: Option<Rc<dyn Curve>>,
        duration: Option<Duration>,
        reverse_curve: Option<Rc<dyn Curve>>,
        reverse_duration: Option<Duration>,
    ) -> AnimationStyle {
        AnimationStyle {
            curve: curve.or_else(|| self.curve.clone()),
            duration: duration.or(self.duration),
            reverse_curve: reverse_curve.or_else(|| self.reverse_curve.clone()),
            reverse_duration: reverse_duration.or(self.reverse_duration),
        }
    }

    /// Creates a new [`AnimationStyle`] that is a combination of this
    /// animation style and the given `other` animation style.
    ///
    /// If `other` is `Some`, its non-`None` properties are used to override
    /// the corresponding properties of this style.
    ///
    /// Returns this animation style if `other` is `None`.
    pub fn merge(&self, other: Option<&AnimationStyle>) -> AnimationStyle {
        let Some(other) = other else {
            return self.clone();
        };
        self.copy_with(
            other.curve.clone(),
            other.duration,
            other.reverse_curve.clone(),
            other.reverse_duration,
        )
    }

    /// Linearly interpolate between two animation styles.
    pub fn lerp(
        a: Option<&AnimationStyle>,
        b: Option<&AnimationStyle>,
        t: f64,
    ) -> Option<AnimationStyle> {
        // Dart: `if (identical(a, b)) return a` — true when both are null or
        // both are the same object.
        match (a, b) {
            (None, None) => return None,
            (Some(a_ref), Some(b_ref)) if std::ptr::eq(a_ref, b_ref) => {
                return Some(a_ref.clone());
            }
            _ => {}
        }
        Some(AnimationStyle {
            curve: lerp_field(
                a.and_then(|style| style.curve.clone()),
                b.and_then(|style| style.curve.clone()),
                t,
                curves_identical,
                lerped_curve,
            ),
            duration: lerp_field(
                a.and_then(|style| style.duration),
                b.and_then(|style| style.duration),
                t,
                |a, b| a == b,
                lerp_duration,
            ),
            reverse_curve: lerp_field(
                a.and_then(|style| style.reverse_curve.clone()),
                b.and_then(|style| style.reverse_curve.clone()),
                t,
                curves_identical,
                lerped_curve,
            ),
            reverse_duration: lerp_field(
                a.and_then(|style| style.reverse_duration),
                b.and_then(|style| style.reverse_duration),
                t,
                |a, b| a == b,
                lerp_duration,
            ),
        })
    }
}

/// Dart's private generic `AnimationStyle._lerp`, with the equality passed in:
/// Dart's `a == b` dispatches per type (identity for curves, value for
/// `Duration`), and Rust needs that spelled out.
fn lerp_field<T: Clone>(
    a: Option<T>,
    b: Option<T>,
    t: f64,
    eq: impl Fn(&T, &T) -> bool,
    lerp: impl Fn(Option<&T>, Option<&T>, f64) -> T,
) -> Option<T> {
    let equal = match (&a, &b) {
        (None, None) => true,
        (Some(a), Some(b)) => eq(a, b),
        _ => false,
    };
    if equal || t == 0.0 {
        return a;
    }
    if t == 1.0 {
        return b;
    }
    Some(lerp(a.as_ref(), b.as_ref(), t))
}

fn curves_identical(a: &Rc<dyn Curve>, b: &Rc<dyn Curve>) -> bool {
    Rc::ptr_eq(a, b)
}

fn lerped_curve(a: Option<&Rc<dyn Curve>>, b: Option<&Rc<dyn Curve>>, t: f64) -> Rc<dyn Curve> {
    Rc::new(LerpedCurve::new(a.cloned(), b.cloned(), t))
}

/// Dart's `AnimationStyle._lerpDuration`.
fn lerp_duration(a: Option<&Duration>, b: Option<&Duration>, t: f64) -> Duration {
    let a_micros = a.map_or(0.0, |duration| duration.as_micros() as f64);
    let b_micros = b.map_or(0.0, |duration| duration.as_micros() as f64);
    Duration::from_micros((a_micros * (1.0 - t) + b_micros * t).round() as u64)
}

impl PartialEq for AnimationStyle {
    fn eq(&self, other: &AnimationStyle) -> bool {
        // Dart compares the curves with `==`, which no framework curve
        // overrides — identity — except `_LerpedCurve` below, whose structural
        // `==` is not reproduced; see PORTING.md.
        option_curve_eq(&self.curve, &other.curve)
            && self.duration == other.duration
            && option_curve_eq(&self.reverse_curve, &other.reverse_curve)
            && self.reverse_duration == other.reverse_duration
    }
}

impl Eq for AnimationStyle {}

fn option_curve_eq(a: &Option<Rc<dyn Curve>>, b: &Option<Rc<dyn Curve>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl Hash for AnimationStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for curve in [&self.curve, &self.reverse_curve] {
            match curve {
                // The address `Rc::ptr_eq` compares, so equal styles hash
                // equally.
                Some(curve) => (Rc::as_ptr(curve) as *const () as usize).hash(state),
                None => 0usize.hash(state),
            }
        }
        self.duration.hash(state);
        self.reverse_duration.hash(state);
    }
}

impl Debug for AnimationStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Dart's debugFillProperties: only non-default (non-null) fields.
        let mut s = f.debug_struct("AnimationStyle");
        if let Some(curve) = &self.curve {
            s.field("curve", curve);
        }
        if let Some(duration) = &self.duration {
            s.field("duration", duration);
        }
        if let Some(reverse_curve) = &self.reverse_curve {
            s.field("reverseCurve", reverse_curve);
        }
        if let Some(reverse_duration) = &self.reverse_duration {
            s.field("reverseDuration", reverse_duration);
        }
        s.finish()
    }
}

/// Dart's private `_LerpedCurve`.
struct LerpedCurve {
    first: Rc<dyn Curve>,
    second: Rc<dyn Curve>,
    t: f64,
}

impl LerpedCurve {
    fn new(a: Option<Rc<dyn Curve>>, b: Option<Rc<dyn Curve>>, t: f64) -> LerpedCurve {
        LerpedCurve {
            first: a.unwrap_or_else(Curves::linear),
            second: b.unwrap_or_else(Curves::linear),
            t,
        }
    }
}

impl Curve for LerpedCurve {
    // Dart overrides `transform` directly, bypassing the 0.0/1.0 shortcut.
    fn transform(&self, t: f64) -> f64 {
        let a = self.first.transform(t);
        let b = self.second.transform(t);
        a * (1.0 - self.t) + b * self.t
    }
}

impl Debug for LerpedCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LerpedCurve({:?}, {:?}, t: {})",
            self.first, self.second, self.t
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // animation_style_test.dart:17 — 'AnimationStyle.copyWith() overrides all
    // properties'.
    #[test]
    fn copy_with_overrides_all_properties() {
        let style = AnimationStyle::new(
            Some(Curves::ease()),
            Some(Duration::from_secs(1)),
            Some(Curves::ease()),
            Some(Duration::from_secs(1)),
        );
        let copy = style.copy_with(
            Some(Curves::linear()),
            Some(Duration::from_secs(2)),
            Some(Curves::linear()),
            Some(Duration::from_secs(2)),
        );

        assert!(Rc::ptr_eq(copy.curve.as_ref().unwrap(), &Curves::linear()));
        assert_eq!(copy.duration, Some(Duration::from_secs(2)));
        assert!(Rc::ptr_eq(
            copy.reverse_curve.as_ref().unwrap(),
            &Curves::linear()
        ));
        assert_eq!(copy.reverse_duration, Some(Duration::from_secs(2)));
    }

    // animation_style_test.dart:12 — 'copyWith, ==, hashCode basics'.
    #[test]
    fn copy_with_of_the_default_is_equal() {
        let default = AnimationStyle::default();
        let copy = default.copy_with(None, None, None, None);
        assert_eq!(default, copy);
    }

    // animation_style_test.dart:36 — 'AnimationStyle.merge() fills in null
    // properties'. The base's first two fields are set, so the merge must
    // override set fields, not only fill empty ones.
    #[test]
    fn merge_fills_in_null_properties() {
        let base = AnimationStyle::new(
            Some(Curves::ease()),
            Some(Duration::from_secs(1)),
            Some(Curves::ease()),
            Some(Duration::from_secs(1)),
        );
        let overrides = AnimationStyle::new(
            Some(Curves::linear()),
            Some(Duration::from_secs(2)),
            None,
            None,
        );
        let merged = base.merge(Some(&overrides));

        assert!(Rc::ptr_eq(
            merged.curve.as_ref().unwrap(),
            &Curves::linear()
        ));
        assert_eq!(merged.duration, Some(Duration::from_secs(2)));
        assert!(Rc::ptr_eq(
            merged.reverse_curve.as_ref().unwrap(),
            &Curves::ease()
        ));
        assert_eq!(merged.reverse_duration, Some(Duration::from_secs(1)));
    }

    // animation_style_test.dart:51 — 'AnimationStyle.lerp identical a,b'.
    #[test]
    fn lerp_of_identical_styles_returns_the_first() {
        assert!(AnimationStyle::lerp(None, None, 0.0).is_none());
        let data = AnimationStyle::default();
        let lerped = AnimationStyle::lerp(Some(&data), Some(&data), 0.5).unwrap();
        assert_eq!(lerped, data);
    }

    // animation_style_test.dart:104 — 'AnimationStyle.lerp with null
    // property': a null side counts as zero microseconds.
    #[test]
    fn lerp_with_a_null_property_counts_it_as_zero() {
        let a = AnimationStyle::new(None, Some(Duration::from_secs(2)), None, None);
        let b = AnimationStyle::default();

        assert_eq!(
            AnimationStyle::lerp(Some(&a), Some(&b), 0.5)
                .unwrap()
                .duration,
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            AnimationStyle::lerp(Some(&a), Some(&b), 0.25)
                .unwrap()
                .duration,
            Some(Duration::from_millis(1500))
        );
    }

    // animation_style_test.dart:75 — lerp endpoints return the inputs.
    #[test]
    fn lerp_endpoints_return_the_inputs() {
        let a = AnimationStyle::new(
            Some(Curves::ease()),
            Some(Duration::from_secs(1)),
            None,
            None,
        );
        let b = AnimationStyle::new(
            Some(Curves::linear()),
            Some(Duration::from_secs(2)),
            None,
            None,
        );

        assert_eq!(AnimationStyle::lerp(Some(&a), Some(&b), 0.0).unwrap(), a);
        assert_eq!(AnimationStyle::lerp(Some(&a), Some(&b), 1.0).unwrap(), b);
    }

    // animation_style_test.dart:57 — the in-between curve blends the two.
    #[test]
    fn lerp_blends_the_curves() {
        let a = AnimationStyle::new(Some(Curves::linear()), None, None, None);
        let b = AnimationStyle::new(Some(Curves::ease()), None, None, None);

        let half = AnimationStyle::lerp(Some(&a), Some(&b), 0.5).unwrap();
        let lerped = half.curve.unwrap();
        let expected =
            Curves::linear().transform(0.25) * 0.5 + Curves::ease().transform(0.25) * 0.5;
        assert_eq!(lerped.transform(0.25), expected);
    }
}
