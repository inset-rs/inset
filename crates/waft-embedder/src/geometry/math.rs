//! Flutter counterpart: `engine/src/flutter/lib/ui/math.dart`.

/// Same as [`f64::clamp`] but maps NaN to `max`.
pub fn clamp_double(x: f64, min: f64, max: f64) -> f64 {
    debug_assert!(min <= max && !max.is_nan() && !min.is_nan());
    if x < min {
        return min;
    }
    if x > max {
        return max;
    }
    if x.is_nan() {
        return max;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamping_holds_the_value_inside_the_bounds() {
        assert_eq!(clamp_double(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(clamp_double(2.0, 0.0, 1.0), 1.0);
        assert_eq!(clamp_double(0.5, 0.0, 1.0), 0.5);
    }

    #[test]
    fn clamping_maps_nan_to_max_where_f64_clamp_returns_nan() {
        assert_eq!(clamp_double(f64::NAN, 0.0, 1.0), 1.0);
        assert!(f64::NAN.clamp(0.0, 1.0).is_nan());
    }
}
