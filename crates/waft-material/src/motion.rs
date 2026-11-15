//! Port of material_ui's `motion.dart`.
//!
//! The library's two `part`s, `generated/motion_defaults_m3.g.dart` and `generated/motion_defaults_m3e.g.dart`, live in `generated/`; their public items are exposed here, as Dart's library exposes them.

pub use crate::generated::motion_defaults_m3::{Durations, Easing};
pub use crate::generated::motion_defaults_m3e::Springs;

#[cfg(test)]
mod tests {
    use waft_physics::SpringDescription;

    use super::*;

    fn assert_spring_description(spring: SpringDescription, stiffness: f64, damping_ratio: f64) {
        assert_eq!(spring.mass, 1.0);
        assert_eq!(spring.stiffness, stiffness);
        let damping = 2.0 * damping_ratio * stiffness.sqrt();
        assert!(
            (spring.damping - damping).abs() < 1e-10,
            "{} != {damping}",
            spring.damping
        );
    }

    #[test]
    fn springs_exposes_standard_spring_tokens() {
        assert_spring_description(Springs::STANDARD_DEFAULT_SPATIAL, 700.0, 0.9);
        assert_spring_description(Springs::STANDARD_FAST_SPATIAL, 1400.0, 0.9);
        assert_spring_description(Springs::STANDARD_SLOW_SPATIAL, 300.0, 0.9);
        assert_spring_description(Springs::STANDARD_DEFAULT_EFFECTS, 1600.0, 1.0);
        assert_spring_description(Springs::STANDARD_FAST_EFFECTS, 3800.0, 1.0);
        assert_spring_description(Springs::STANDARD_SLOW_EFFECTS, 800.0, 1.0);
    }

    #[test]
    fn springs_exposes_expressive_spring_tokens() {
        assert_spring_description(Springs::EXPRESSIVE_DEFAULT_SPATIAL, 380.0, 0.8);
        assert_spring_description(Springs::EXPRESSIVE_FAST_SPATIAL, 800.0, 0.6);
        assert_spring_description(Springs::EXPRESSIVE_SLOW_SPATIAL, 200.0, 0.8);
        assert_spring_description(Springs::EXPRESSIVE_DEFAULT_EFFECTS, 1600.0, 1.0);
        assert_spring_description(Springs::EXPRESSIVE_FAST_EFFECTS, 3800.0, 1.0);
        assert_spring_description(Springs::EXPRESSIVE_SLOW_EFFECTS, 800.0, 1.0);
    }
}
