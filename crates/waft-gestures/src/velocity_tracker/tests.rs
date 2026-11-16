//! Flutter's `test/gestures/velocity_tracker_test.dart`.

use std::time::Duration;

use waft_embedder::{Offset, PointerDeviceKind};
use waft_foundation::AppCell;

use super::data::{INTERRUPTED_VELOCITY_EVENT_DATA, VELOCITY_EVENT_DATA};
use super::{
    AnyVelocityTracker, IOSScrollViewFlingVelocityTracker, MacOSScrollViewFlingVelocityTracker,
    VelocityTracker,
};
use crate::Velocity;

/// A recorded pointer event: its time stamp in milliseconds and its position.
#[derive(Clone, Copy)]
pub(super) enum Recorded {
    Down(u64, f64, f64),
    Move(u64, f64, f64),
    #[expect(
        dead_code,
        reason = "the recording keeps each up's time and place; no case reads them"
    )]
    Up(u64, f64, f64),
}

/// The App's clock where a case does not let time pass: Dart's fake clock between two pumps.
const NOW: Duration = Duration::ZERO;

fn within_tolerance(actual: f64, expected: f64) -> bool {
    // Within .1% of expected value.
    const TOLERANCE: f64 = 0.001;
    let diff = (actual - expected) / expected;
    diff.abs() < TOLERANCE
}

fn check_velocity(actual: Velocity, expected: Offset) -> bool {
    within_tolerance(actual.pixels_per_second.dx(), expected.dx())
        && within_tolerance(actual.pixels_per_second.dy(), expected.dy())
}

/// Feeds each down and move of `events` to `tracker`, and hands each up's velocity to `on_up`.
fn track(events: &[Recorded], tracker: &mut VelocityTracker, mut on_up: impl FnMut(Velocity)) {
    for event in events {
        match *event {
            Recorded::Down(ms, x, y) | Recorded::Move(ms, x, y) => {
                tracker.add_position(NOW, Duration::from_millis(ms), Offset::new(x, y));
            }
            Recorded::Up(..) => on_up(tracker.get_velocity(NOW)),
        }
    }
}

#[test]
fn velocity_tracker_gives_expected_results() {
    let expected = [
        Offset::new(219.59280094228163, 1304.701682306001),
        Offset::new(355.71046950050845, 967.2112857054104),
        Offset::new(12.657970884022308, -36.90447839251946),
        Offset::new(714.1399654786744, -2561.534447931869),
        Offset::new(-19.668121066218564, -2910.105747052462),
        Offset::new(646.8690114934209, 2976.977762577527),
        Offset::new(396.6988447819592, 2106.225572911095),
        Offset::new(298.31594440044495, -3660.8315955215294),
        Offset::new(-1.7334232785165882, -3288.13174127454),
        Offset::new(384.6361280392334, -2645.6612524779835),
        Offset::new(176.37900397918557, 2711.2542876273264),
        Offset::new(396.9328560260098, 4280.651578291764),
        Offset::new(-71.51939428321249, 3716.7385187526947),
    ];
    let mut tracker = VelocityTracker::with_kind(PointerDeviceKind::Touch);
    let mut i = 0;
    track(VELOCITY_EVENT_DATA, &mut tracker, |velocity| {
        assert!(
            check_velocity(velocity, expected[i]),
            "fling {i}: {velocity}"
        );
        i += 1;
    });
}

#[test]
fn velocity_control_test() {
    let velocity1 = Velocity::new(Offset::new(7.0, 0.0));
    let velocity2 = Velocity::new(Offset::new(12.0, 0.0));
    assert_eq!(velocity1, Velocity::new(Offset::new(7.0, 0.0)));
    assert_ne!(velocity1, velocity2);
    assert_eq!(velocity2 - velocity1, Velocity::new(Offset::new(5.0, 0.0)));
    assert_eq!((-velocity1).pixels_per_second, Offset::new(-7.0, 0.0));
    assert_eq!(velocity1 + velocity2, Velocity::new(Offset::new(19.0, 0.0)));
    let description = velocity1.to_string();
    assert!(!description.is_empty() && !description.contains('\n'));
}

#[test]
fn interrupted_velocity_estimation() {
    // Regression test for https://github.com/flutter/flutter/pull/7510
    let mut tracker = VelocityTracker::with_kind(PointerDeviceKind::Touch);
    track(INTERRUPTED_VELOCITY_EVENT_DATA, &mut tracker, |velocity| {
        assert!(
            check_velocity(velocity, Offset::new(649.5, 3890.3)),
            "{velocity}"
        );
    });
}

#[test]
fn no_data_velocity_estimation() {
    let tracker = VelocityTracker::with_kind(PointerDeviceKind::Touch);
    assert_eq!(tracker.get_velocity(NOW), Velocity::ZERO);
}

#[test]
#[should_panic(expected = "at least 1 point")]
fn free_scroll_start_velocity_tracker_get_velocity_panics_when_no_points() {
    let tracker = IOSScrollViewFlingVelocityTracker::new(PointerDeviceKind::Touch);
    tracker.get_velocity(NOW);
}

#[test]
#[should_panic(expected = "has a smaller timestamp")]
fn free_scroll_start_velocity_tracker_panics_when_the_new_point_precedes_the_previous_point() {
    let mut tracker = IOSScrollViewFlingVelocityTracker::new(PointerDeviceKind::Touch);
    tracker.add_position(NOW, Duration::from_secs(60 * 60), Offset::ZERO);
    tracker.get_velocity(NOW);
    tracker.add_position(NOW, Duration::from_secs(1), Offset::ZERO);
}

#[test]
fn estimate_does_not_panic_when_there_are_more_than_1_point() {
    let mut tracker = IOSScrollViewFlingVelocityTracker::new(PointerDeviceKind::Touch);
    let mut position = Offset::ZERO;
    let mut time = Duration::ZERO;
    let position_delta = Offset::new(0.0, -1.0);
    let duration_delta = Duration::from_secs(1);
    for _ in 0..5 {
        position = position + position_delta;
        time += duration_delta;
        tracker.add_position(NOW, time, position);
        tracker.get_velocity(NOW);
    }
}

#[test]
fn makes_consistent_velocity_estimates_with_consistent_velocity() {
    let mut tracker = IOSScrollViewFlingVelocityTracker::new(PointerDeviceKind::Touch);
    let mut position = Offset::ZERO;
    let mut time = Duration::ZERO;
    let position_delta = Offset::new(0.0, -1.0);
    let duration_delta = Duration::from_secs(1);
    for i in 0..10 {
        position = position + position_delta;
        time += duration_delta;
        tracker.add_position(NOW, time, position);
        if i >= 3 {
            assert_eq!(tracker.get_velocity(NOW).pixels_per_second, position_delta);
        }
    }
}

/// The "Assume zero velocity when there are no recent samples" cases: ten samples a second
/// apart, then `pumpAndSettle`'s one 100 ms frame on the App's clock, after which the pointer has
/// stopped.
fn velocity_after_no_recent_samples(tracker: &mut dyn AnyVelocityTracker) -> Velocity {
    let cell = AppCell::new();
    let now = || cell.borrow().clock();
    let mut position = Offset::ZERO;
    let mut time = Duration::ZERO;
    for _ in 0..10 {
        position = position + Offset::new(0.0, -1.0);
        time += Duration::from_secs(1);
        tracker.add_position(now(), time, position);
    }
    cell.elapse(Duration::from_millis(100));
    tracker.get_velocity(now())
}

#[test]
fn assume_zero_velocity_when_there_are_no_recent_samples_base_velocity_tracker() {
    let mut tracker = VelocityTracker::with_kind(PointerDeviceKind::Touch);
    let velocity = velocity_after_no_recent_samples(&mut tracker);
    assert_eq!(velocity.pixels_per_second, Offset::ZERO);
}

#[test]
fn assume_zero_velocity_when_there_are_no_recent_samples_ios() {
    let mut tracker = IOSScrollViewFlingVelocityTracker::new(PointerDeviceKind::Touch);
    let velocity = velocity_after_no_recent_samples(&mut tracker);
    assert_eq!(velocity.pixels_per_second, Offset::ZERO);
}

#[test]
fn assume_zero_velocity_when_there_are_no_recent_samples_macos() {
    let mut tracker = MacOSScrollViewFlingVelocityTracker::new(PointerDeviceKind::Touch);
    let velocity = velocity_after_no_recent_samples(&mut tracker);
    assert_eq!(velocity.pixels_per_second, Offset::ZERO);
}
