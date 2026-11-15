//! Port of material_ui's `generated/motion_defaults_m3.g.dart`: a Dart `part` of `motion.dart`, so its items are public through `motion.rs`.
//!
//! Generated from the Material token database upstream; port changes to the Dart generator's output by hand.

use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Cubic, Curve, Curves};

/// The set of durations in the Material specification.
///
/// See also:
///
/// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
/// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
pub enum Durations {}

impl Durations {
    /// The short1 duration (50ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const SHORT1: Duration = Duration::from_millis(50);

    /// The short2 duration (100ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const SHORT2: Duration = Duration::from_millis(100);

    /// The short3 duration (150ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const SHORT3: Duration = Duration::from_millis(150);

    /// The short4 duration (200ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const SHORT4: Duration = Duration::from_millis(200);

    /// The medium1 duration (250ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const MEDIUM1: Duration = Duration::from_millis(250);

    /// The medium2 duration (300ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const MEDIUM2: Duration = Duration::from_millis(300);

    /// The medium3 duration (350ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const MEDIUM3: Duration = Duration::from_millis(350);

    /// The medium4 duration (400ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const MEDIUM4: Duration = Duration::from_millis(400);

    /// The long1 duration (450ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const LONG1: Duration = Duration::from_millis(450);

    /// The long2 duration (500ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const LONG2: Duration = Duration::from_millis(500);

    /// The long3 duration (550ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const LONG3: Duration = Duration::from_millis(550);

    /// The long4 duration (600ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const LONG4: Duration = Duration::from_millis(600);

    /// The extralong1 duration (700ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const EXTRALONG1: Duration = Duration::from_millis(700);

    /// The extralong2 duration (800ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const EXTRALONG2: Duration = Duration::from_millis(800);

    /// The extralong3 duration (900ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const EXTRALONG3: Duration = Duration::from_millis(900);

    /// The extralong4 duration (1000ms) in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Duration tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#c009dec6-f29b-4503-b9f0-482af14a8bbd)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub const EXTRALONG4: Duration = Duration::from_millis(1000);
}

/// The set of easing curves in the Material specification.
///
/// See also:
///
/// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
/// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
/// * `Curves`, for a collection of non-Material animation easing curves.
pub enum Easing {}

macro_rules! easing_curves {
    ($( $(#[$doc:meta])* $name:ident = $value:expr; )*) => {
        impl Easing {
            $(
                $(#[$doc])*
                pub fn $name() -> Rc<dyn Curve> {
                    thread_local! {
                        static CURVE: Rc<dyn Curve> = Rc::new($value);
                    }
                    CURVE.with(Rc::clone)
                }
            )*
        }
    };
}

easing_curves! {
    /// The emphasizedAccelerate easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    emphasized_accelerate = Cubic::new(0.3, 0.0, 0.8, 0.15);

    /// The emphasizedDecelerate easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    emphasized_decelerate = Cubic::new(0.05, 0.7, 0.1, 1.0);

    /// The linear easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    linear = Cubic::new(0.0, 0.0, 1.0, 1.0);

    /// The standard easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    standard = Cubic::new(0.2, 0.0, 0.0, 1.0);

    /// The standardAccelerate easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    standard_accelerate = Cubic::new(0.3, 0.0, 1.0, 1.0);

    /// The standardDecelerate easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    standard_decelerate = Cubic::new(0.0, 0.0, 0.0, 1.0);

    /// The legacyDecelerate easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    legacy_decelerate = Cubic::new(0.0, 0.0, 0.2, 1.0);

    /// The legacyAccelerate easing curve in the Material specification.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    legacy_accelerate = Cubic::new(0.4, 0.0, 1.0, 1.0);
}

impl Easing {
    /// The legacy easing curve in the Material specification.
    ///
    /// Dart's `Cubic(0.4, 0.0, 0.2, 1.0)` is the same constant as `Curves.fastOutSlowIn`, so
    /// this hands out that curve.
    ///
    /// See also:
    ///
    /// * [M3 guidelines: Easing tokens](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs#433b1153-2ea3-4fe2-9748-803a47bc97ee)
    /// * [M3 guidelines: Applying easing and duration](https://m3.material.io/styles/motion/easing-and-duration/applying-easing-and-duration)
    pub fn legacy() -> Rc<dyn Curve> {
        Curves::fast_out_slow_in()
    }
}
