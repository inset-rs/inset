//! Flutter counterpart: `animation/tween_sequence.dart`.

use std::fmt::{self, Debug};
use std::sync::Arc;

use reveal_foundation::App;

use crate::tween::Animatable;

/// Enables creating an animation whose value is defined by a sequence of
/// tweens.
///
/// Each [`TweenSequenceItem`] has a weight that defines its percentage of the
/// animation's duration. Each tween defines the animation's value during the
/// interval indicated by its weight.
#[derive(Clone)]
pub struct TweenSequence<T> {
    items: Arc<[TweenSequenceItem<T>]>,
    intervals: Arc<[Interval]>,
}

impl<T> TweenSequence<T> {
    /// Construct a TweenSequence.
    ///
    /// The `items` parameter must be a list of one or more
    /// [`TweenSequenceItem`]s.
    ///
    /// There's a small cost associated with building a [`TweenSequence`] so
    /// it's best to reuse one, rather than rebuilding it on every frame, when
    /// that's possible.
    pub fn new(items: Vec<TweenSequenceItem<T>>) -> TweenSequence<T> {
        debug_assert!(!items.is_empty());

        let mut total_weight = 0.0;
        for item in &items {
            total_weight += item.weight;
        }
        debug_assert!(total_weight > 0.0);

        let mut intervals = Vec::with_capacity(items.len());
        let mut start = 0.0;
        for i in 0..items.len() {
            let end = if i == items.len() - 1 {
                1.0
            } else {
                start + items[i].weight / total_weight
            };
            intervals.push(Interval { start, end });
            start = end;
        }

        TweenSequence {
            items: items.into(),
            intervals: intervals.into(),
        }
    }

    fn evaluate_at(&self, app: &App, t: f64, index: usize) -> T {
        let element = &self.items[index];
        let t_interval = self.intervals[index].value(t);
        element.tween.transform(app, t_interval)
    }
}

impl<T: 'static> Animatable<T> for TweenSequence<T> {
    fn transform(&self, app: &App, t: f64) -> T {
        debug_assert!((0.0..=1.0).contains(&t));
        if t == 1.0 {
            return self.evaluate_at(app, t, self.items.len() - 1);
        }
        for index in 0..self.items.len() {
            if self.intervals[index].contains(t) {
                return self.evaluate_at(app, t, index);
            }
        }
        panic!("TweenSequence.evaluate() could not find an interval for {t}");
    }
}

impl<T> Debug for TweenSequence<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TweenSequence({} items)", self.items.len())
    }
}

/// Enables creating a flipped animation whose value is defined by a sequence
/// of tweens.
#[derive(Clone)]
pub struct FlippedTweenSequence {
    sequence: TweenSequence<f64>,
}

impl FlippedTweenSequence {
    /// Creates a flipped [`TweenSequence`].
    pub fn new(items: Vec<TweenSequenceItem<f64>>) -> FlippedTweenSequence {
        FlippedTweenSequence {
            sequence: TweenSequence::new(items),
        }
    }
}

impl Animatable<f64> for FlippedTweenSequence {
    fn transform(&self, app: &App, t: f64) -> f64 {
        1.0 - self.sequence.transform(app, 1.0 - t)
    }
}

impl Debug for FlippedTweenSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TweenSequence({} items)", self.sequence.items.len())
    }
}

/// A simple holder for one element of a [`TweenSequence`].
#[derive(Clone)]
pub struct TweenSequenceItem<T> {
    /// Defines the value of the [`TweenSequence`] for the interval within the
    /// animation's duration indicated by [`weight`] and this item's position
    /// in the list of items.
    ///
    /// [`weight`]: TweenSequenceItem::weight
    pub tween: Arc<dyn Animatable<T>>,

    /// An arbitrary value that indicates the relative percentage of a
    /// [`TweenSequence`] animation's duration when [`tween`] will be used.
    ///
    /// [`tween`]: TweenSequenceItem::tween
    pub weight: f64,
}

impl<T> TweenSequenceItem<T> {
    /// Construct a TweenSequenceItem.
    ///
    /// The `weight` must be greater than 0.0.
    pub fn new(tween: Arc<dyn Animatable<T>>, weight: f64) -> TweenSequenceItem<T> {
        debug_assert!(weight > 0.0);
        TweenSequenceItem { tween, weight }
    }
}

/// Dart's private `_Interval` — not `curves.dart`'s `Interval`.
#[derive(Clone, Copy)]
struct Interval {
    start: f64,
    end: f64,
}

impl Interval {
    fn contains(&self, t: f64) -> bool {
        t >= self.start && t < self.end
    }

    fn value(&self, t: f64) -> f64 {
        (t - self.start) / (self.end - self.start)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::sync::Arc;

    use reveal_foundation::{App, Handle};

    use super::*;
    use crate::animation::Animation;
    use crate::animations::{AlwaysStoppedAnimation, ProxyAnimation};
    use crate::curves::{Curves, Interval as CurveInterval};
    use crate::tween::{ConstantTween, CurveTween, Tween};

    fn set_value(app: &mut App, driver: Handle<ProxyAnimation>, value: f64) {
        let stopped = app
            .create(AlwaysStoppedAnimation::new(value))
            .as_animation();
        driver.set_parent(app, Some(stopped));
    }

    #[test]
    fn tween_sequence() {
        let mut app = App::new();
        let driver = ProxyAnimation::new(&mut app, None);

        let sequence = TweenSequence::new(vec![
            TweenSequenceItem::new(Arc::new(Tween::new(&mut app, Some(5.0), Some(10.0))), 4.0),
            TweenSequenceItem::new(Arc::new(ConstantTween::new(&mut app, 10.0)), 2.0),
            TweenSequenceItem::new(Arc::new(Tween::new(&mut app, Some(10.0), Some(5.0))), 4.0),
        ]);
        let animation = sequence.animate(&mut app, driver.as_animation());

        assert_eq!(animation.value(&app), 5.0);
        for (t, expected) in [(0.2, 7.5), (0.4, 10.0), (0.6, 10.0), (0.8, 7.5), (1.0, 5.0)] {
            set_value(&mut app, driver, t);
            assert_eq!(animation.value(&app), expected, "at t = {t}");
        }
    }

    #[test]
    fn tween_sequence_with_curves() {
        let mut app = App::new();
        let driver = ProxyAnimation::new(&mut app, None);

        let first = Tween::new(&mut app, Some(5.0), Some(10.0)).chain(CurveTween::new(
            &mut app,
            Rc::new(CurveInterval::new(0.5, 1.0, Curves::linear())),
        ));
        let middle =
            ConstantTween::new(&mut app, 10.0).chain(CurveTween::new(&mut app, Curves::linear()));
        let last = Tween::new(&mut app, Some(10.0), Some(5.0)).chain(CurveTween::new(
            &mut app,
            Rc::new(CurveInterval::new(0.0, 0.5, Curves::linear())),
        ));

        let sequence = TweenSequence::new(vec![
            TweenSequenceItem::new(Arc::new(first), 4.0),
            TweenSequenceItem::new(Arc::new(middle), 2.0),
            TweenSequenceItem::new(Arc::new(last), 4.0),
        ]);
        let animation = sequence.animate(&mut app, driver.as_animation());

        assert_eq!(animation.value(&app), 5.0);
        for (t, expected) in [(0.2, 5.0), (0.4, 10.0), (0.6, 10.0), (0.8, 5.0), (1.0, 5.0)] {
            set_value(&mut app, driver, t);
            assert_eq!(animation.value(&app), expected, "at t = {t}");
        }
    }

    #[test]
    fn tween_sequence_one_tween() {
        let mut app = App::new();
        let driver = ProxyAnimation::new(&mut app, None);

        let sequence = TweenSequence::new(vec![TweenSequenceItem::new(
            Arc::new(Tween::new(&mut app, Some(5.0), Some(10.0))),
            1.0,
        )]);
        let animation = sequence.animate(&mut app, driver.as_animation());

        assert_eq!(animation.value(&app), 5.0);
        set_value(&mut app, driver, 0.5);
        assert_eq!(animation.value(&app), 7.5);
        set_value(&mut app, driver, 1.0);
        assert_eq!(animation.value(&app), 10.0);
    }

    #[test]
    fn a_flipped_tween_sequence_flips_both_axes() {
        let mut app = App::new();
        let sequence = FlippedTweenSequence::new(vec![TweenSequenceItem::new(
            Arc::new(Tween::new(&mut app, Some(0.0), Some(0.5))),
            1.0,
        )]);
        assert_eq!(sequence.transform(&app, 0.25), 0.625);
    }
}
