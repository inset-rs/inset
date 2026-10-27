//! NavigationView.cpp's selection-indicator compositor tracks on a native animation clock.

use inset_animation::{AnimationController, Cubic, Curve, Curves};
use inset_embedder::{Offset, Rect};
use inset_foundation::Handle;
use inset_painting::Alignment;
use inset_widgets::*;
use std::{cell::Cell, rc::Rc, time::Duration};

/// Source duration of both the same-level and cross-level indicator animations.
pub(crate) const NAVIGATION_INDICATOR_DURATION: Duration = Duration::from_millis(600);

/// Compositor transform applied to one indicator's original rounded rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct IndicatorFrame {
    /// Transformed bounds in the common coordinate space used to compare the two owners.
    pub rect: Rect,
    /// Visual opacity; the outgoing same-level indicator fades after the first third.
    pub opacity: f64,
    /// Horizontal scale also applies to the source corner radii.
    pub scale_x: f64,
    /// Vertical scale also applies to the source corner radii.
    pub scale_y: f64,
}

/// Evaluates PlayIndicatorAnimations and both non-same-level helpers without allocating visuals.
#[cfg(test)]
pub(crate) fn indicator_frames(
    from: Rect,
    to: Rect,
    top: bool,
    elapsed: Duration,
) -> [IndicatorFrame; 2] {
    indicator_frames_with_pivots(from, to, top, elapsed, [0.0; 2])
}

/// CenterPoint.X survives source ResetElementAnimationProperties and cross-level transitions.
fn indicator_frames_with_pivots(
    from: Rect,
    to: Rect,
    top: bool,
    elapsed: Duration,
    pivots_x: [f64; 2],
) -> [IndicatorFrame; 2] {
    let t = (elapsed.as_secs_f64() / NAVIGATION_INDICATOR_DURATION.as_secs_f64()).clamp(0.0, 1.0);
    let same_depth = if top {
        from.top == to.top
    } else {
        from.left == to.left
    };
    [true, false].map(|outgoing| {
        let rect = if outgoing { from } else { to };
        if t >= 1.0
            || from.width() <= 0.0
            || from.height() <= 0.0
            || to.width() <= 0.0
            || to.height() <= 0.0
        {
            return IndicatorFrame {
                rect,
                opacity: if outgoing { 0.0 } else { 1.0 },
                scale_x: 1.0,
                scale_y: 1.0,
            };
        }
        if !same_depth {
            // Windows supplies the implicit compositor curve; use native easeInOut for that default.
            let progress = Curves::ease_in_out().transform(t);
            let scale = if outgoing { 1.0 - progress } else { progress };
            let vertical = rect.height() > rect.width();
            let is_next_below = from.top < to.top;
            let from_top = if outgoing {
                !is_next_below
            } else {
                is_next_below
            };
            // The horizontal helper changes CenterPoint.Y but retains the owner's CenterPoint.X.
            let pivot = if vertical {
                if from_top {
                    0.0
                } else if top {
                    rect.width()
                } else {
                    rect.height()
                }
            } else {
                pivots_x[if outgoing { 0 } else { 1 }]
            };
            return transformed(
                rect,
                if vertical { 0.0 } else { pivot },
                if vertical { pivot } else { 0.0 },
                if vertical { 1.0 } else { scale },
                if vertical { scale } else { 1.0 },
                0.0,
                0.0,
                1.0,
            );
        }
        let dimension = if top { rect.width() } else { rect.height() };
        let begin_scale = if top { from.width() / dimension } else { 1.0 };
        let end_scale = if top { to.width() / dimension } else { 1.0 };
        let distance = if top {
            to.left - from.left
        } else {
            to.top - from.top
        };
        let (start, end) = if outgoing {
            (0.0, distance)
        } else {
            (-distance, 0.0)
        };
        let increasing = start < end;
        let position = if t < 0.333 {
            if increasing {
                start
            } else {
                start + dimension * (begin_scale - 1.0)
            }
        } else if increasing {
            end + dimension * (end_scale - 1.0)
        } else {
            end
        };
        let peak =
            (end - start).abs() / dimension + if increasing { end_scale } else { begin_scale };
        let scale = if t <= 0.333 {
            begin_scale + (peak - begin_scale) * Cubic::new(0.9, 0.1, 1.0, 0.2).transform(t / 0.333)
        } else {
            peak + (end_scale - peak)
                * Cubic::new(0.1, 0.9, 0.2, 1.0).transform((t - 0.333) / 0.667)
        };
        let pivot = if elapsed < Duration::from_millis(200) {
            if increasing { 0.0 } else { dimension }
        } else if increasing {
            dimension
        } else {
            0.0
        };
        let opacity = if outgoing && t > 0.333 {
            1.0 - Cubic::new(0.1, 0.9, 0.2, 1.0).transform((t - 0.333) / 0.667)
        } else {
            1.0
        };
        transformed(
            rect,
            if top { pivot } else { 0.0 },
            if top { 0.0 } else { pivot },
            if top { scale } else { 1.0 },
            if top { 1.0 } else { scale },
            if top { position } else { 0.0 },
            if top { 0.0 } else { position },
            opacity,
        )
    })
}

/// Applies local compositor scale around its center point followed by its offset.
#[expect(
    clippy::too_many_arguments,
    reason = "Keeps the source visual scale, pivot, offset and opacity components explicit"
)]
fn transformed(
    rect: Rect,
    cx: f64,
    cy: f64,
    sx: f64,
    sy: f64,
    dx: f64,
    dy: f64,
    opacity: f64,
) -> IndicatorFrame {
    IndicatorFrame {
        rect: Rect::from_ltwh(
            rect.left + dx + cx * (1.0 - sx),
            rect.top + dy + cy * (1.0 - sy),
            rect.width() * sx,
            rect.height() * sy,
        ),
        opacity,
        scale_x: sx,
        scale_y: sy,
    }
}

/// Source compositor tracks applied to one actual item-owned SelectionIndicator.
#[derive(Clone, Debug)]
pub(crate) struct NavigationIndicatorTransition {
    /// Original settled outgoing bounds, used only to compute relative motion.
    pub from: Rect,
    /// Original settled incoming bounds in the same coordinate space.
    pub to: Rect,
    /// Selects the horizontal top-navigation axis.
    pub top: bool,
    /// Chooses the outgoing or incoming source storyboard.
    pub outgoing: bool,
    /// This owner's retained CenterPoint.X before a cross-level storyboard.
    pub pivot_x: Rc<Cell<f64>>,
}

impl NavigationIndicatorTransition {
    /// Wraps the real indicator, preserving its visual parent's clips, scrolling and visibility.
    pub(crate) fn build(
        controller: Handle<AnimationController>,
        animation: Option<Self>,
        settled_opacity: f64,
        child: WidgetRef,
    ) -> WidgetRef {
        AnimatedBuilder::new(Rc::new(controller), move |app, _, child| {
            let (opacity, offset, scale_x, scale_y) = if let Some(animation) = &animation {
                let frame = indicator_frames_with_pivots(
                    animation.from,
                    animation.to,
                    animation.top,
                    NAVIGATION_INDICATOR_DURATION.mul_f64(controller.value(app)),
                    [animation.pivot_x.get(); 2],
                )[if animation.outgoing { 0 } else { 1 }];
                let origin = if animation.outgoing {
                    animation.from
                } else {
                    animation.to
                };
                (
                    frame.opacity,
                    Offset::new(frame.rect.left - origin.left, frame.rect.top - origin.top),
                    frame.scale_x,
                    frame.scale_y,
                )
            } else {
                (settled_opacity, Offset::ZERO, 1.0, 1.0)
            };
            Opacity::new(opacity)
                .child(
                    Transform::translate(offset).child(
                        Transform::scale(None, Some(scale_x), Some(scale_y))
                            .alignment(Alignment::TOP_LEFT.into())
                            .child(child.unwrap().clone()),
                    ),
                )
                .into_widget()
        })
        .child(child)
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cross_level_horizontal_scale_retains_existing_center_point_x() {
        let from = Rect::from_ltwh(40.0, 20.0, 16.0, 3.0);
        let to = Rect::from_ltwh(40.0, 80.0, 2.0, 24.0);
        let frames =
            indicator_frames_with_pivots(from, to, true, Duration::from_millis(300), [16.0, 0.0]);
        assert!(frames[0].rect.left > from.left);
        assert!((frames[0].rect.right - from.right).abs() < 0.001);
    }

    #[test]
    fn both_visuals_start_at_previous_indicator_and_finish_at_destination() {
        for top in [false, true] {
            let from = Rect::from_ltwh(
                4.0,
                12.0,
                if top { 50.0 } else { 3.0 },
                if top { 3.0 } else { 16.0 },
            );
            let to = if top {
                Rect::from_ltwh(100.0, 12.0, 80.0, 3.0)
            } else {
                Rect::from_ltwh(4.0, 120.0, 3.0, 16.0)
            };
            for (from, to) in [(from, to), (to, from)] {
                let first = indicator_frames(from, to, top, Duration::ZERO);
                for frame in first {
                    assert!((frame.rect.left - from.left).abs() < 0.001);
                    assert!((frame.rect.top - from.top).abs() < 0.001);
                    assert!((frame.rect.width() - from.width()).abs() < 0.001);
                }
                let last = indicator_frames(from, to, top, NAVIGATION_INDICATOR_DURATION);
                assert_eq!(last[0].opacity, 0.0);
                assert_eq!(last[1].rect, to);
                assert_eq!(last[1].opacity, 1.0);
            }
        }
    }
    #[test]
    fn elongation_and_outgoing_fade_follow_two_source_cubic_segments() {
        let from = Rect::from_ltwh(4.0, 10.0, 3.0, 16.0);
        let to = Rect::from_ltwh(4.0, 110.0, 3.0, 16.0);
        let stretch = indicator_frames(from, to, false, Duration::from_millis(199));
        assert!(stretch[0].rect.height() > from.height());
        let peak = indicator_frames(from, to, false, Duration::from_micros(199_800));
        assert!((peak[0].rect.height() - 116.0).abs() < 0.001);
        assert_eq!(stretch[0].opacity, 1.0);
        let shrink = indicator_frames(from, to, false, Duration::from_millis(400));
        assert!(shrink[0].opacity > 0.0 && shrink[0].opacity < 0.2);
        assert!(shrink[1].rect.height() < stretch[1].rect.height());
    }
    #[test]
    fn cross_depth_indicators_scale_locally_without_connecting_the_depths() {
        let from = Rect::from_ltwh(4.0, 10.0, 3.0, 16.0);
        let to = Rect::from_ltwh(36.0, 50.0, 3.0, 16.0);
        let frame = indicator_frames(from, to, false, Duration::from_millis(300));
        assert_eq!(frame[0].rect.left, 4.0);
        assert_eq!(frame[1].rect.left, 36.0);
        assert!(frame[0].rect.height() > 0.0 && frame[0].rect.height() < 16.0);
        assert_eq!(frame[1].rect.top, 50.0);
    }
}
