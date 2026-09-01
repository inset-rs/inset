//! Flutter counterpart: `engine/src/flutter/lib/ui/lerp.dart`.

/// Linearly interpolate between two numbers, `a` and `b`, by an extrapolation
/// factor `t`.
///
/// When `a` and `b` are equal or both NaN, `a` is returned. Otherwise, `a`, `b`,
/// and `t` are required to be finite or null, and the result of `a + (b - a) *
/// t` is returned, where nulls are defaulted to 0.0.
pub fn lerp_double(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    if a == b || (a.is_some_and(f64::is_nan) && b.is_some_and(f64::is_nan)) {
        return a;
    }
    let a = a.unwrap_or(0.0);
    let b = b.unwrap_or(0.0);
    debug_assert!(
        a.is_finite(),
        "Cannot interpolate between finite and non-finite values"
    );
    debug_assert!(
        b.is_finite(),
        "Cannot interpolate between finite and non-finite values"
    );
    debug_assert!(
        t.is_finite(),
        "t must be finite when interpolating between values"
    );
    Some(a * (1.0 - t) + b * t)
}

/// Linearly interpolate between two doubles.
///
/// Same as [`lerp_double`] but specialized for non-null `f64`.
pub(crate) fn lerp_double_non_null(a: f64, b: f64, t: f64) -> f64 {
    // This doesn't match _lerpInt to preserve specific behaviors when dealing
    // with infinity and nan.
    a * (1.0 - t) + b * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolating_between_two_values_walks_the_line() {
        assert_eq!(lerp_double(Some(0.0), Some(10.0), 0.25), Some(2.5));
        assert_eq!(lerp_double(Some(10.0), Some(0.0), 0.25), Some(7.5));
    }

    #[test]
    fn extrapolating_past_the_unit_interval_is_allowed() {
        assert_eq!(lerp_double(Some(0.0), Some(10.0), -0.5), Some(-5.0));
        assert_eq!(lerp_double(Some(0.0), Some(10.0), 1.5), Some(15.0));
    }

    #[test]
    fn a_null_end_defaults_to_zero_and_two_nulls_stay_null() {
        assert_eq!(lerp_double(None, Some(10.0), 0.25), Some(2.5));
        assert_eq!(lerp_double(Some(10.0), None, 0.25), Some(7.5));
        assert_eq!(lerp_double(None, None, 0.25), None);
    }

    #[test]
    fn two_nans_return_the_first_where_equality_would_say_they_differ() {
        assert!(lerp_double(Some(f64::NAN), Some(f64::NAN), 0.25).is_some_and(f64::is_nan));
    }
}
