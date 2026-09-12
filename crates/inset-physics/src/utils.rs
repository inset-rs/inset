//! Flutter counterpart: `physics/utils.dart`.

/// Whether two doubles are within a given distance of each other.
///
/// The `epsilon` argument must be positive.
/// A null value is only considered near-equal to another null value.
pub fn near_equal(a: Option<f64>, b: Option<f64>, epsilon: f64) -> bool {
    debug_assert!(epsilon >= 0.0);
    let (Some(a), Some(b)) = (a, b) else {
        return a == b;
    };
    (a > (b - epsilon)) && (a < (b + epsilon)) || a == b
}

/// Whether a double is within a given distance of zero.
///
/// The epsilon argument must be positive.
pub fn near_zero(a: f64, epsilon: f64) -> bool {
    near_equal(Some(a), Some(0.0), epsilon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_equals() {
        assert!(near_equal(Some(f64::INFINITY), Some(f64::INFINITY), 0.1));
        assert!(near_equal(
            Some(f64::NEG_INFINITY),
            Some(f64::NEG_INFINITY),
            0.1
        ));

        assert!(!near_equal(
            Some(f64::INFINITY),
            Some(f64::NEG_INFINITY),
            0.1
        ));

        assert!(!near_equal(Some(0.1), Some(0.11), 0.001));
        assert!(near_equal(Some(0.1), Some(0.11), 0.1));
        assert!(near_equal(Some(0.1), Some(0.1), 0.0000001));
    }

    #[test]
    fn test_friction() {
        assert!(near_equal(Some(5.0), Some(6.0), 2.0));
        assert!(near_equal(Some(6.0), Some(5.0), 2.0));
        assert!(!near_equal(Some(5.0), Some(6.0), 0.5));
        assert!(!near_equal(Some(6.0), Some(5.0), 0.5));
    }

    #[test]
    fn test_null() {
        assert!(!near_equal(Some(5.0), None, 2.0));
        assert!(!near_equal(None, Some(5.0), 2.0));
        assert!(near_equal(None, None, 2.0));
    }
}
