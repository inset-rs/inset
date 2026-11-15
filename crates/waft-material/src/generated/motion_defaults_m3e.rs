//! Port of material_ui's `generated/motion_defaults_m3e.g.dart`: a Dart `part` of `motion.dart`, so its items are public through `motion.rs`.
//!
//! Generated from the Material token database upstream; port changes to the Dart generator's output by hand.

use waft_physics::SpringDescription;

/// The set of springs in the Material specification.
///
/// Material 3 Expressive motion uses springs, grouped into two motion schemes.
/// The standard scheme eases into place with minimal bounce, and suits more
/// utilitarian products. The expressive scheme overshoots and bounces into
/// place, and suits hero moments and key interactions.
///
/// Each scheme has springs for two types of motion, in three speeds: default,
/// fast, and slow.
///
/// * Spatial springs animate movement, such as position, size, rotation, and
///   corner radius.
/// * Effects springs animate properties that shouldn't overshoot, such as
///   color and opacity.
///
/// Most motion should use the default speed. Smaller elements can use the fast
/// speed, and larger elements can use the slow speed.
///
/// Material spring tokens define a stiffness and a damping ratio, but no
/// mass. The Material token named `damping` is the damping ratio (ζ), not
/// `SpringDescription.damping`. Each constant is the same as
/// `SpringDescription.withDampingRatio` with a mass of 1.0 and the token's
/// stiffness and damping ratio, so its damping is
/// `ratio * 2.0 * sqrt(stiffness)`. For example, `STANDARD_DEFAULT_SPATIAL` has
/// a damping of `0.9 * 2.0 * sqrt(700.0)`, about 47.62.
///
/// Use them with a `SpringSimulation`, for example through
/// `AnimationController.animateWith`.
///
/// See also:
///
/// * `Durations` and `Easing`, which are the Material tokens for curve-based
///   animations.
/// * [M3 guidelines: Motion physics system](https://m3.material.io/styles/motion/overview/how-it-works)
/// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
pub enum Springs {}

impl Springs {
    /// The standard default spatial spring in the Material specification.
    ///
    /// It has a stiffness of 700.0 and a damping ratio of 0.9.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const STANDARD_DEFAULT_SPATIAL: SpringDescription =
        SpringDescription::new(1.0, 700.0, 47.62352359916263);

    /// The standard fast spatial spring in the Material specification.
    ///
    /// It has a stiffness of 1400.0 and a damping ratio of 0.9.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const STANDARD_FAST_SPATIAL: SpringDescription =
        SpringDescription::new(1.0, 1400.0, 67.34983296193096);

    /// The standard slow spatial spring in the Material specification.
    ///
    /// It has a stiffness of 300.0 and a damping ratio of 0.9.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const STANDARD_SLOW_SPATIAL: SpringDescription =
        SpringDescription::new(1.0, 300.0, 31.176914536239796);

    /// The standard default effects spring in the Material specification.
    ///
    /// It has a stiffness of 1600.0 and a damping ratio of 1.0.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const STANDARD_DEFAULT_EFFECTS: SpringDescription =
        SpringDescription::new(1.0, 1600.0, 80.0);

    /// The standard fast effects spring in the Material specification.
    ///
    /// It has a stiffness of 3800.0 and a damping ratio of 1.0.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const STANDARD_FAST_EFFECTS: SpringDescription =
        SpringDescription::new(1.0, 3800.0, 123.28828005937953);

    /// The standard slow effects spring in the Material specification.
    ///
    /// It has a stiffness of 800.0 and a damping ratio of 1.0.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const STANDARD_SLOW_EFFECTS: SpringDescription =
        SpringDescription::new(1.0, 800.0, 56.568542494923804);

    /// The expressive default spatial spring in the Material specification.
    ///
    /// It has a stiffness of 380.0 and a damping ratio of 0.8.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const EXPRESSIVE_DEFAULT_SPATIAL: SpringDescription =
        SpringDescription::new(1.0, 380.0, 31.189741903388683);

    /// The expressive fast spatial spring in the Material specification.
    ///
    /// It has a stiffness of 800.0 and a damping ratio of 0.6.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const EXPRESSIVE_FAST_SPATIAL: SpringDescription =
        SpringDescription::new(1.0, 800.0, 33.94112549695428);

    /// The expressive slow spatial spring in the Material specification.
    ///
    /// It has a stiffness of 200.0 and a damping ratio of 0.8.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const EXPRESSIVE_SLOW_SPATIAL: SpringDescription =
        SpringDescription::new(1.0, 200.0, 22.627416997969522);

    /// The expressive default effects spring in the Material specification.
    ///
    /// It has a stiffness of 1600.0 and a damping ratio of 1.0.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const EXPRESSIVE_DEFAULT_EFFECTS: SpringDescription =
        SpringDescription::new(1.0, 1600.0, 80.0);

    /// The expressive fast effects spring in the Material specification.
    ///
    /// It has a stiffness of 3800.0 and a damping ratio of 1.0.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const EXPRESSIVE_FAST_EFFECTS: SpringDescription =
        SpringDescription::new(1.0, 3800.0, 123.28828005937953);

    /// The expressive slow effects spring in the Material specification.
    ///
    /// It has a stiffness of 800.0 and a damping ratio of 1.0.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Spring tokens](https://m3.material.io/styles/motion/overview/specs)
    pub const EXPRESSIVE_SLOW_EFFECTS: SpringDescription =
        SpringDescription::new(1.0, 800.0, 56.568542494923804);
}
