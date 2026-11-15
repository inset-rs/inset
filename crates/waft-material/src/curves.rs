//! Port of material_ui's `curves.dart`.
//!
//! Dart's `const Curve` globals are functions here, each handing out clones of one process-local `Rc` as `Curves` does, so `Rc::ptr_eq` reproduces Dart's canonicalized `==`. A constant equal to another library's hands out that one's `Rc`, as Dart canonicalizes equal constants into one object.

use std::rc::Rc;

use waft_animation::{Curve, Curves};

use crate::motion::Easing;

/// The standard easing curve in the Material 2 specification.
///
/// Elements that begin and end at rest use standard easing.
/// They speed up quickly and slow down gradually, in order
/// to emphasize the end of the transition.
///
/// See also:
/// * <https://material.io/design/motion/speed.html#easing>
#[deprecated(
    note = "Use Easing::legacy (M2) or Easing::standard (M3) instead. This curve is updated in M3. This feature was deprecated after v3.18.0-0.1.pre."
)]
pub fn standard_easing() -> Rc<dyn Curve> {
    Curves::fast_out_slow_in()
}

/// The accelerate easing curve in the Material 2 specification.
///
/// Elements exiting a screen use acceleration easing,
/// where they start at rest and end at peak velocity.
///
/// See also:
/// * <https://material.io/design/motion/speed.html#easing>
#[deprecated(
    note = "Use Easing::legacy_accelerate (M2) or Easing::standard_accelerate (M3) instead. This curve is updated in M3. This feature was deprecated after v3.18.0-0.1.pre."
)]
pub fn accelerate_easing() -> Rc<dyn Curve> {
    Easing::legacy_accelerate()
}

/// The decelerate easing curve in the Material 2 specification.
///
/// Incoming elements are animated using deceleration easing,
/// which starts a transition at peak velocity (the fastest
/// point of an element’s movement) and ends at rest.
///
/// See also:
/// * <https://material.io/design/motion/speed.html#easing>
#[deprecated(
    note = "Use Easing::legacy_decelerate (M2) or Easing::standard_decelerate (M3) instead. This curve is updated in M3. This feature was deprecated after v3.18.0-0.1.pre."
)]
pub fn decelerate_easing() -> Rc<dyn Curve> {
    Easing::legacy_decelerate()
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn equal_constants_are_one_curve() {
        assert!(Rc::ptr_eq(&standard_easing(), &Curves::fast_out_slow_in()));
        assert!(Rc::ptr_eq(&Easing::legacy(), &Curves::fast_out_slow_in()));
        assert!(Rc::ptr_eq(
            &accelerate_easing(),
            &Easing::legacy_accelerate()
        ));
        assert!(Rc::ptr_eq(
            &decelerate_easing(),
            &Easing::legacy_decelerate()
        ));
        assert!(!Rc::ptr_eq(&accelerate_easing(), &decelerate_easing()));
    }
}
