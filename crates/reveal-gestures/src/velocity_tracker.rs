//! Flutter counterpart: `gestures/velocity_tracker.dart` (`VelocityTracker`
//! and the iOS / macOS fling subclasses). [`Velocity`](crate::velocity::Velocity)
//! / [`VelocityEstimate`](crate::velocity::VelocityEstimate) live in `velocity.rs`.

use std::time::{Duration, Instant};

use reveal_embedder::{Offset, PointerDeviceKind};

use crate::lsq_solver::LeastSquaresSolver;
use crate::velocity::{Velocity, VelocityEstimate};

const ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS: u128 = 40;
const HISTORY_SIZE: usize = 20;
const HORIZON_MILLISECONDS: f64 = 100.0;
const MIN_SAMPLE_SIZE: usize = 3;
const IOS_SAMPLE_SIZE: usize = 20;

#[derive(Clone, Copy, Debug)]
struct PointAtTime {
    point: Offset,
    time: Duration,
}

/// Dart's `VelocityTracker` used as a type: what a
/// [`GestureVelocityTrackerBuilder`](crate::GestureVelocityTrackerBuilder)
/// returns, and what a `Map<int, VelocityTracker>` holds.
///
/// [`IOSScrollViewFlingVelocityTracker`] and
/// [`MacOSScrollViewFlingVelocityTracker`] are Dart subclasses of
/// [`VelocityTracker`]; a caller that stores any of the three stores a
/// `Box<dyn AnyVelocityTracker>`.
pub trait AnyVelocityTracker {
    /// The kind of pointer this tracker is for.
    fn kind(&self) -> PointerDeviceKind;

    /// Adds a position as the given time to the tracker.
    fn add_position(&mut self, time: Duration, position: Offset);

    /// Returns an estimate of the velocity of the object being tracked by the
    /// tracker given the current information available to the tracker.
    fn get_velocity_estimate(&self) -> Option<VelocityEstimate>;

    /// Computes the velocity of the pointer at the time of the last provided
    /// data point.
    fn get_velocity(&self) -> Velocity;
}

impl AnyVelocityTracker for VelocityTracker {
    fn kind(&self) -> PointerDeviceKind {
        self.kind
    }

    fn add_position(&mut self, time: Duration, position: Offset) {
        VelocityTracker::add_position(self, time, position);
    }

    fn get_velocity_estimate(&self) -> Option<VelocityEstimate> {
        VelocityTracker::get_velocity_estimate(self)
    }

    fn get_velocity(&self) -> Velocity {
        VelocityTracker::get_velocity(self)
    }
}

impl AnyVelocityTracker for IOSScrollViewFlingVelocityTracker {
    fn kind(&self) -> PointerDeviceKind {
        self.kind
    }

    fn add_position(&mut self, time: Duration, position: Offset) {
        IOSScrollViewFlingVelocityTracker::add_position(self, time, position);
    }

    fn get_velocity_estimate(&self) -> Option<VelocityEstimate> {
        IOSScrollViewFlingVelocityTracker::get_velocity_estimate(self)
    }

    fn get_velocity(&self) -> Velocity {
        IOSScrollViewFlingVelocityTracker::get_velocity(self)
    }
}

impl AnyVelocityTracker for MacOSScrollViewFlingVelocityTracker {
    fn kind(&self) -> PointerDeviceKind {
        MacOSScrollViewFlingVelocityTracker::kind(self)
    }

    fn add_position(&mut self, time: Duration, position: Offset) {
        MacOSScrollViewFlingVelocityTracker::add_position(self, time, position);
    }

    fn get_velocity_estimate(&self) -> Option<VelocityEstimate> {
        MacOSScrollViewFlingVelocityTracker::get_velocity_estimate(self)
    }

    fn get_velocity(&self) -> Velocity {
        MacOSScrollViewFlingVelocityTracker::get_velocity(self)
    }
}

/// Computes a pointer's velocity based on data from `PointerMoveEvent`s.
///
/// The input data is provided by calling [`add_position`](Self::add_position).
/// Adding data is cheap.
///
/// To obtain a velocity, call [`get_velocity`](Self::get_velocity) or
/// [`get_velocity_estimate`](Self::get_velocity_estimate). This will compute
/// the velocity based on the data added so far. Only call these when you need
/// to use the velocity, as they are comparatively expensive.
///
/// The quality of the velocity estimation will be better if more data points
/// have been received.
pub struct VelocityTracker {
    /// The kind of pointer this tracker is for.
    pub kind: PointerDeviceKind,
    last_sample: Option<Instant>,
    samples: [Option<PointAtTime>; HISTORY_SIZE],
    index: usize,
}

impl VelocityTracker {
    /// Create a new velocity tracker for a pointer `kind`.
    pub fn with_kind(kind: PointerDeviceKind) -> VelocityTracker {
        VelocityTracker {
            kind,
            last_sample: None,
            samples: [None; HISTORY_SIZE],
            index: 0,
        }
    }

    fn note_sample(&mut self) {
        self.last_sample = Some(Instant::now());
    }

    fn recently_stopped(&self) -> bool {
        self.last_sample
            .is_some_and(|at| at.elapsed().as_millis() > ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS)
    }

    /// Adds a position as the given time to the tracker.
    pub fn add_position(&mut self, time: Duration, position: Offset) {
        self.note_sample();
        self.index += 1;
        if self.index == HISTORY_SIZE {
            self.index = 0;
        }
        self.samples[self.index] = Some(PointAtTime {
            point: position,
            time,
        });
    }

    /// Returns an estimate of the velocity of the object being tracked by the
    /// tracker given the current information available to the tracker.
    ///
    /// Information is added using [`add_position`](Self::add_position).
    ///
    /// Returns [`None`] if there is no data on which to base an estimate.
    pub fn get_velocity_estimate(&self) -> Option<VelocityEstimate> {
        if self.recently_stopped() {
            return Some(VelocityEstimate::new(
                Offset::ZERO,
                1.0,
                Duration::ZERO,
                Offset::ZERO,
            ));
        }

        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut w = Vec::new();
        let mut time = Vec::new();
        let mut sample_count = 0usize;
        let mut index = self.index;

        let newest_sample = self.samples[index]?;
        let mut previous_sample = newest_sample;
        let mut oldest_sample = newest_sample;

        while let Some(sample) = self.samples[index] {
            let age = duration_diff_ms(newest_sample.time, sample.time);
            let delta = duration_abs_diff_ms(sample.time, previous_sample.time);
            previous_sample = sample;
            if age > HORIZON_MILLISECONDS || delta > ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS as f64
            {
                break;
            }

            oldest_sample = sample;
            let position = sample.point;
            x.push(position.dx());
            y.push(position.dy());
            w.push(1.0);
            time.push(-age);
            index = if index == 0 { HISTORY_SIZE } else { index } - 1;

            sample_count += 1;
            if sample_count >= HISTORY_SIZE {
                break;
            }
        }

        if sample_count >= MIN_SAMPLE_SIZE {
            let x_fit = LeastSquaresSolver::new(time.clone(), x, w.clone()).solve(2);
            let y_fit = LeastSquaresSolver::new(time, y, w).solve(2);

            if let (Some(x_fit), Some(y_fit)) = (x_fit, y_fit) {
                return Some(VelocityEstimate::new(
                    Offset::new(
                        x_fit.coefficients[1] * 1000.0,
                        y_fit.coefficients[1] * 1000.0,
                    ),
                    x_fit.confidence * y_fit.confidence,
                    newest_sample.time.saturating_sub(oldest_sample.time),
                    newest_sample.point - oldest_sample.point,
                ));
            }
        }

        Some(VelocityEstimate::new(
            Offset::ZERO,
            1.0,
            newest_sample.time.saturating_sub(oldest_sample.time),
            newest_sample.point - oldest_sample.point,
        ))
    }

    /// Computes the velocity of the pointer at the time of the last
    /// provided data point.
    ///
    /// This can be expensive. Only call this when you need the velocity.
    ///
    /// Returns [`Velocity::ZERO`] if there is no data from which to compute an
    /// estimate or if the estimated velocity is zero.
    pub fn get_velocity(&self) -> Velocity {
        velocity_from_estimate(self.get_velocity_estimate())
    }
}

/// A [`VelocityTracker`] subclass that provides a close approximation of iOS
/// scroll view's velocity estimation strategy.
pub struct IOSScrollViewFlingVelocityTracker {
    /// The kind of pointer this tracker is for.
    pub kind: PointerDeviceKind,
    last_sample: Option<Instant>,
    index: usize,
    touch_samples: [Option<PointAtTime>; IOS_SAMPLE_SIZE],
}

impl IOSScrollViewFlingVelocityTracker {
    /// Create a new IOSScrollViewFlingVelocityTracker.
    pub fn new(kind: PointerDeviceKind) -> IOSScrollViewFlingVelocityTracker {
        IOSScrollViewFlingVelocityTracker {
            kind,
            last_sample: None,
            index: 0,
            touch_samples: [None; IOS_SAMPLE_SIZE],
        }
    }

    fn recently_stopped(&self) -> bool {
        self.last_sample
            .is_some_and(|at| at.elapsed().as_millis() > ASSUME_POINTER_MOVE_STOPPED_MILLISECONDS)
    }

    /// Adds a position as the given time to the tracker.
    pub fn add_position(&mut self, time: Duration, position: Offset) {
        self.last_sample = Some(Instant::now());
        if cfg!(debug_assertions) {
            let previous_point = self.touch_samples[self.index];
            debug_assert!(
                previous_point.is_none_or(|previous| previous.time <= time),
                "The position being added ({position:?}) has a smaller timestamp ({time:?}) \
                 than its predecessor: {previous_point:?}.",
            );
        }
        self.index = (self.index + 1) % IOS_SAMPLE_SIZE;
        self.touch_samples[self.index] = Some(PointAtTime {
            point: position,
            time,
        });
    }

    fn previous_velocity_at(&self, index: i32) -> Offset {
        let end_index = (self.index as i32 + index).rem_euclid(IOS_SAMPLE_SIZE as i32) as usize;
        let start_index =
            (self.index as i32 + index - 1).rem_euclid(IOS_SAMPLE_SIZE as i32) as usize;
        let (Some(end), Some(start)) = (
            self.touch_samples[end_index],
            self.touch_samples[start_index],
        ) else {
            return Offset::ZERO;
        };

        let dt = if end.time >= start.time {
            end.time - start.time
        } else {
            Duration::ZERO
        };
        debug_assert!(end.time >= start.time);
        let dt_us = dt.as_micros();
        if dt_us > 0 {
            (end.point - start.point) * 1000.0 / (dt_us as f64 / 1000.0)
        } else {
            Offset::ZERO
        }
    }

    /// Returns an estimate of the velocity of the object being tracked.
    pub fn get_velocity_estimate(&self) -> Option<VelocityEstimate> {
        if self.recently_stopped() {
            return Some(VelocityEstimate::new(
                Offset::ZERO,
                1.0,
                Duration::ZERO,
                Offset::ZERO,
            ));
        }

        let estimated_velocity = self.previous_velocity_at(-2) * 0.6
            + self.previous_velocity_at(-1) * 0.35
            + self.previous_velocity_at(0) * 0.05;

        let newest_sample = self.touch_samples[self.index];
        let mut oldest_non_null_sample = None;
        for i in 1..=IOS_SAMPLE_SIZE {
            oldest_non_null_sample = self.touch_samples[(self.index + i) % IOS_SAMPLE_SIZE];
            if oldest_non_null_sample.is_some() {
                break;
            }
        }

        match (oldest_non_null_sample, newest_sample) {
            (Some(oldest), Some(newest)) => Some(VelocityEstimate::new(
                estimated_velocity,
                1.0,
                newest.time.saturating_sub(oldest.time),
                newest.point - oldest.point,
            )),
            _ => {
                debug_assert!(
                    false,
                    "There must be at least 1 point in touch_samples: {:?}",
                    self.touch_samples
                );
                Some(VelocityEstimate::new(
                    Offset::ZERO,
                    0.0,
                    Duration::ZERO,
                    Offset::ZERO,
                ))
            }
        }
    }

    /// Computes the velocity of the pointer at the time of the last sample.
    pub fn get_velocity(&self) -> Velocity {
        velocity_from_estimate(self.get_velocity_estimate())
    }
}

/// A [`VelocityTracker`] subclass that provides a close approximation of macOS
/// scroll view's velocity estimation strategy.
pub struct MacOSScrollViewFlingVelocityTracker {
    inner: IOSScrollViewFlingVelocityTracker,
}

impl MacOSScrollViewFlingVelocityTracker {
    /// Create a new MacOSScrollViewFlingVelocityTracker.
    pub fn new(kind: PointerDeviceKind) -> MacOSScrollViewFlingVelocityTracker {
        MacOSScrollViewFlingVelocityTracker {
            inner: IOSScrollViewFlingVelocityTracker::new(kind),
        }
    }

    /// The kind of pointer this tracker is for.
    pub fn kind(&self) -> PointerDeviceKind {
        self.inner.kind
    }

    /// Adds a position as the given time to the tracker.
    pub fn add_position(&mut self, time: Duration, position: Offset) {
        self.inner.add_position(time, position);
    }

    /// Returns an estimate of the velocity of the object being tracked.
    pub fn get_velocity_estimate(&self) -> Option<VelocityEstimate> {
        if self.inner.recently_stopped() {
            return Some(VelocityEstimate::new(
                Offset::ZERO,
                1.0,
                Duration::ZERO,
                Offset::ZERO,
            ));
        }

        let estimated_velocity = self.inner.previous_velocity_at(-2) * 0.15
            + self.inner.previous_velocity_at(-1) * 0.65
            + self.inner.previous_velocity_at(0) * 0.2;

        let newest_sample = self.inner.touch_samples[self.inner.index];
        let mut oldest_non_null_sample = None;
        for i in 1..=IOS_SAMPLE_SIZE {
            oldest_non_null_sample =
                self.inner.touch_samples[(self.inner.index + i) % IOS_SAMPLE_SIZE];
            if oldest_non_null_sample.is_some() {
                break;
            }
        }

        match (oldest_non_null_sample, newest_sample) {
            (Some(oldest), Some(newest)) => Some(VelocityEstimate::new(
                estimated_velocity,
                1.0,
                newest.time.saturating_sub(oldest.time),
                newest.point - oldest.point,
            )),
            _ => {
                debug_assert!(
                    false,
                    "There must be at least 1 point in touch_samples: {:?}",
                    self.inner.touch_samples
                );
                Some(VelocityEstimate::new(
                    Offset::ZERO,
                    0.0,
                    Duration::ZERO,
                    Offset::ZERO,
                ))
            }
        }
    }

    /// Computes the velocity of the pointer at the time of the last sample.
    pub fn get_velocity(&self) -> Velocity {
        velocity_from_estimate(self.get_velocity_estimate())
    }
}

fn velocity_from_estimate(estimate: Option<VelocityEstimate>) -> Velocity {
    match estimate {
        None => Velocity::ZERO,
        Some(estimate) if estimate.pixels_per_second == Offset::ZERO => Velocity::ZERO,
        Some(estimate) => Velocity::new(estimate.pixels_per_second),
    }
}

fn duration_diff_ms(newer: Duration, older: Duration) -> f64 {
    newer.saturating_sub(older).as_micros() as f64 / 1000.0
}

fn duration_abs_diff_ms(a: Duration, b: Duration) -> f64 {
    if a >= b {
        (a - b).as_micros() as f64 / 1000.0
    } else {
        (b - a).as_micros() as f64 / 1000.0
    }
}
