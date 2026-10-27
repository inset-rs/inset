//! SplitView DisplayModeStates setters and storyboards, transcribed from
//! controls/dev/SplitView/SplitView_themeresources.xaml.

use crate::{GridLength, ThemeResources};
use inset_animation::{Cubic, Curve};
use std::time::Duration;

/// The named states in the template's `DisplayModeStates` group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DisplayModeState {
    Closed,
    ClosedCompactLeft,
    ClosedCompactRight,
    OpenOverlayLeft,
    OpenOverlayRight,
    OpenInlineLeft,
    OpenInlineRight,
    OpenCompactOverlayLeft,
    OpenCompactOverlayRight,
}

/// Values applied to the named template parts at a single instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Visuals {
    /// Width of `ColumnDefinition1`.
    pub column1: GridLength,
    /// Width of `ColumnDefinition2`.
    pub column2: GridLength,
    /// `Grid.Column` of `ContentRoot`.
    pub content_column: usize,
    /// `Grid.ColumnSpan` of `ContentRoot`.
    pub content_span: usize,
    /// `Grid.Column` of `PaneRoot`.
    pub pane_column: usize,
    /// `Grid.ColumnSpan` of `PaneRoot`.
    pub pane_span: usize,
    /// Whether `PaneRoot.HorizontalAlignment` is Right rather than Left.
    pub pane_right: bool,
    /// Whether `PaneRoot.Visibility` is Visible.
    pub pane_visible: bool,
    /// Horizontal translation of `PaneTransform`.
    pub pane_translate: f64,
    /// Horizontal translation of `PaneClipRectangleTransform`.
    pub clip_translate: f64,
    /// Horizontal translation of `ContentTransform`.
    pub content_translate: f64,
    /// Whether `LightDismissLayer.Visibility` is Visible.
    pub overlay_visible: bool,
    /// Opacity of `LightDismissLayer`, independent of its themed fill.
    pub overlay_opacity: f64,
}

/// Resolves the template defaults and the setters of a completed visual state.
pub(super) fn settled(state: DisplayModeState, open: f64, compact: f64) -> Visuals {
    use DisplayModeState::*;
    // Base values on the named template elements, before visual-state setters.
    let mut v = Visuals {
        column1: GridLength::pixel(open),
        column2: GridLength::STAR,
        content_column: 0,
        content_span: 2,
        pane_column: 0,
        pane_span: 2,
        pane_right: false,
        pane_visible: false,
        pane_translate: 0.0,
        clip_translate: 0.0,
        content_translate: 0.0,
        overlay_visible: false,
        overlay_opacity: 1.0,
    };
    match state {
        Closed => {}
        ClosedCompactLeft => {
            v.column1 = GridLength::pixel(compact);
            v.content_column = 1;
            v.content_span = 1;
            v.pane_visible = true;
            v.clip_translate = -(open - compact);
        }
        ClosedCompactRight => {
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(compact);
            v.content_span = 1;
            v.pane_visible = true;
            v.pane_span = 2;
            v.pane_right = true;
            v.clip_translate = open - compact;
        }
        OpenOverlayLeft => {
            v.pane_visible = true;
            v.overlay_visible = true;
        }
        OpenOverlayRight => {
            v.pane_visible = true;
            v.pane_right = true;
            v.overlay_visible = true;
        }
        OpenInlineLeft => {
            v.pane_visible = true;
            v.content_span = 1;
            v.content_column = 1;
            v.pane_span = 1;
            v.pane_translate = 0.0;
            v.content_translate = 0.0;
            v.clip_translate = 0.0;
        }
        OpenInlineRight => {
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(open);
            v.content_span = 1;
            v.pane_visible = true;
            v.pane_column = 1;
            v.pane_span = 1;
        }
        OpenCompactOverlayLeft => {
            v.content_span = 1;
            v.content_column = 1;
            v.column1 = GridLength::pixel(compact);
            v.pane_visible = true;
            v.overlay_visible = true;
        }
        OpenCompactOverlayRight => {
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(compact);
            v.content_span = 1;
            v.pane_visible = true;
            v.pane_right = true;
            v.overlay_visible = true;
        }
    }
    v
}

/// Returns the duration of an explicitly declared transition; unlisted pairs change immediately.
pub(super) fn transition_duration(
    r: &ThemeResources,
    from: DisplayModeState,
    to: DisplayModeState,
) -> Option<Duration> {
    use DisplayModeState::*;
    match (from, to) {
        (Closed, OpenOverlayLeft) => Some(Duration::from_millis(350)),
        (Closed, OpenOverlayRight) => Some(Duration::from_millis(350)),
        (ClosedCompactLeft, OpenCompactOverlayLeft) => Some(Duration::from_millis(350)),
        (ClosedCompactRight, OpenCompactOverlayRight) => Some(Duration::from_millis(350)),
        (OpenOverlayLeft, Closed) => Some(Duration::from_millis(120)),
        (OpenOverlayRight, Closed) => Some(Duration::from_millis(120)),
        (OpenCompactOverlayLeft, ClosedCompactLeft) => Some(Duration::from_millis(120)),
        (OpenCompactOverlayRight, ClosedCompactRight) => Some(Duration::from_millis(120)),
        (OpenInlineLeft, Closed) => Some(r.split_view_pane_animation_close_duration),
        (Closed, OpenInlineLeft) => Some(r.split_view_pane_animation_open_duration),
        (ClosedCompactLeft, OpenInlineLeft) => Some(r.split_view_pane_animation_open_duration),
        (OpenInlineLeft, ClosedCompactLeft) => Some(r.split_view_pane_animation_open_duration),
        (Closed, OpenInlineRight) => Some(r.split_view_pane_animation_open_duration),
        (OpenInlineRight, Closed) => Some(r.split_view_pane_animation_close_duration),
        (ClosedCompactRight, OpenInlineRight) => {
            Some(r.split_view_pane_animation_open_pre_duration)
        }
        (OpenInlineRight, ClosedCompactRight) => Some(r.split_view_pane_animation_close_duration),
        _ => None,
    }
}

/// Evaluates a transition starting from the settled source state.
#[cfg(test)]
pub(super) fn transition(
    from: DisplayModeState,
    to: DisplayModeState,
    elapsed: Duration,
    open: f64,
    compact: f64,
) -> Visuals {
    transition_from_visuals(
        &ThemeResources::new(crate::Theme::Light, crate::AccentPalette::default()),
        from,
        to,
        elapsed,
        open,
        compact,
        settled(from, open, compact),
    )
}

/// Implicit From values snapshot the live presentation when an animation is interrupted.
/// Explicit zero-time keyframes still take the values declared by the template.
pub(super) fn transition_from_visuals(
    r: &ThemeResources,
    from: DisplayModeState,
    to: DisplayModeState,
    elapsed: Duration,
    open: f64,
    compact: f64,
    previous: Visuals,
) -> Visuals {
    use DisplayModeState::*;
    let mut v = settled(to, open, compact);
    let Some(duration) = transition_duration(r, from, to) else {
        return v;
    };
    if elapsed >= duration {
        return v;
    }
    match (from, to) {
        (Closed, OpenOverlayLeft) => {
            v.pane_visible = true;
            v.pane_translate = tween(
                -open,
                0.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.clip_translate = tween(
                open,
                0.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                0.0,
                1.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (Closed, OpenOverlayRight) => {
            v.pane_visible = true;
            v.pane_right = true;
            v.pane_translate = tween(
                open,
                0.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.clip_translate = tween(
                -open,
                0.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                0.0,
                1.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (ClosedCompactLeft, OpenCompactOverlayLeft) => {
            v.column1 = GridLength::pixel(compact);
            v.content_column = 1;
            v.content_span = 1;
            v.pane_visible = true;
            v.clip_translate = tween(
                -(open - compact),
                0.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                0.0,
                1.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (ClosedCompactRight, OpenCompactOverlayRight) => {
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(compact);
            v.content_span = 1;
            v.pane_visible = true;
            v.pane_right = true;
            v.clip_translate = tween(
                open - compact,
                0.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                0.0,
                1.0,
                elapsed,
                Duration::from_millis(350),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (OpenOverlayLeft, Closed) => {
            v.pane_visible = true;
            v.pane_translate = tween(
                previous.pane_translate,
                -open,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.clip_translate = tween(
                previous.clip_translate,
                open,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                1.0,
                0.0,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (OpenOverlayRight, Closed) => {
            v.pane_visible = true;
            v.pane_right = true;
            v.pane_translate = tween(
                previous.pane_translate,
                open,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.clip_translate = tween(
                previous.clip_translate,
                -open,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                1.0,
                0.0,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (OpenCompactOverlayLeft, ClosedCompactLeft) => {
            v.column1 = GridLength::pixel(compact);
            v.content_column = 1;
            v.content_span = 1;
            v.pane_visible = true;
            v.clip_translate = tween(
                0.0,
                -(open - compact),
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                1.0,
                0.0,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (OpenCompactOverlayRight, ClosedCompactRight) => {
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(compact);
            v.content_span = 1;
            v.pane_visible = true;
            v.pane_right = true;
            v.clip_translate = tween(
                0.0,
                open - compact,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
            v.overlay_visible = true;
            v.overlay_opacity = tween(
                1.0,
                0.0,
                elapsed,
                Duration::from_millis(120),
                Cubic::new(0.1, 0.9, 0.2, 1.0),
            );
        }
        (OpenInlineLeft, Closed) => {
            v.pane_visible = if elapsed < r.split_view_pane_animation_close_duration {
                previous.pane_visible
            } else {
                false
            };
            v.content_span = 2;
            v.content_column = 0;
            v.pane_span = if elapsed < r.split_view_pane_animation_close_duration {
                previous.pane_span
            } else {
                2
            };
            v.clip_translate = tween(
                previous.clip_translate,
                -(open - compact),
                elapsed,
                r.split_view_pane_animation_close_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.pane_translate = tween(
                previous.pane_translate,
                -open,
                elapsed,
                r.split_view_pane_animation_close_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.content_translate = tween(
                open,
                0.0,
                elapsed,
                r.split_view_pane_animation_close_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        (Closed, OpenInlineLeft) => {
            v.pane_visible = true;
            v.content_span = 1;
            v.content_column = 1;
            v.pane_span = 1;
            v.pane_translate = tween(
                -open,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.content_translate = tween(
                -open,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_pre_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.clip_translate = tween(
                -open,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        (ClosedCompactLeft, OpenInlineLeft) => {
            v.pane_visible = true;
            v.content_span = 1;
            v.content_column = 1;
            v.pane_span = 1;
            v.content_translate = tween(
                -(open - compact),
                0.0,
                elapsed,
                r.split_view_pane_animation_open_pre_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.clip_translate = tween(
                -(open - compact),
                0.0,
                elapsed,
                r.split_view_pane_animation_open_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        (OpenInlineLeft, ClosedCompactLeft) => {
            v.column1 = GridLength::pixel(compact);
            v.content_column = 1;
            v.content_span = 1;
            v.pane_visible = true;
            v.content_translate = tween(
                open - compact,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_pre_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.clip_translate = tween(
                0.0,
                -(open - compact),
                elapsed,
                r.split_view_pane_animation_open_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        (Closed, OpenInlineRight) => {
            v.pane_visible = true;
            v.content_span = 1;
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(open);
            v.pane_column = 1;
            v.pane_span = 1;
            v.pane_translate = tween(
                open,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.content_translate = tween(
                open,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_pre_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.clip_translate = tween(
                open,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        (OpenInlineRight, Closed) => {
            v.pane_visible = if elapsed < r.split_view_pane_animation_close_duration {
                previous.pane_visible
            } else {
                false
            };
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(open);
            v.content_span = 2;
            v.content_column = 0;
            v.pane_column = 2;
            v.pane_span = if elapsed < r.split_view_pane_animation_close_duration {
                previous.pane_span
            } else {
                1
            };
            v.clip_translate = tween(
                previous.clip_translate,
                -open,
                elapsed,
                r.split_view_pane_animation_close_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.pane_translate = tween(
                previous.pane_translate,
                open,
                elapsed,
                r.split_view_pane_animation_close_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
            v.content_translate = tween(
                -open,
                0.0,
                elapsed,
                r.split_view_pane_animation_close_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        (ClosedCompactRight, OpenInlineRight) => {
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(open);
            v.content_span = 1;
            v.pane_visible = true;
            v.pane_column = 1;
            v.pane_span = 1;
            v.clip_translate = tween(
                open - compact,
                0.0,
                elapsed,
                r.split_view_pane_animation_open_pre_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        (OpenInlineRight, ClosedCompactRight) => {
            v.column1 = GridLength::STAR;
            v.column2 = GridLength::pixel(compact);
            v.content_span = 1;
            v.pane_visible = true;
            v.pane_span = 2;
            v.pane_right = true;
            v.clip_translate = tween(
                previous.clip_translate,
                open - compact,
                elapsed,
                r.split_view_pane_animation_close_duration,
                Cubic::new(0.0, 0.35, 0.15, 1.0),
            );
        }
        _ => {}
    }
    v
}

/// Evaluates one spline keyframe track at the elapsed storyboard time.
fn tween(from: f64, to: f64, elapsed: Duration, duration: Duration, curve: Cubic) -> f64 {
    let fraction = (elapsed.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0);
    from + (to - from) * curve.transform(fraction)
}

#[cfg(test)]
mod tests {
    use super::{DisplayModeState::*, *};

    /// WinUI's own values.
    fn winui() -> ThemeResources {
        ThemeResources::new(crate::Theme::Light, crate::AccentPalette::default())
    }

    #[test]
    fn overlay_preserves_full_content_and_compact_reserves_only_compact_length() {
        for (state, right) in [(OpenOverlayLeft, false), (OpenOverlayRight, true)] {
            let v = settled(state, 320.0, 48.0);
            assert_eq!((v.content_column, v.content_span), (0, 2));
            assert!(v.overlay_visible && v.pane_visible);
            assert_eq!(v.pane_right, right);
        }
        for state in [ClosedCompactLeft, OpenCompactOverlayLeft] {
            let v = settled(state, 320.0, 48.0);
            assert_eq!(v.column1, GridLength::pixel(48.0));
            assert_eq!((v.content_column, v.content_span), (1, 1));
        }
        for state in [ClosedCompactRight, OpenCompactOverlayRight] {
            let v = settled(state, 320.0, 48.0);
            assert_eq!(v.column2, GridLength::pixel(48.0));
            assert_eq!((v.content_column, v.content_span), (0, 1));
        }
        assert_eq!(
            settled(ClosedCompactLeft, 320.0, 48.0).clip_translate,
            -272.0
        );
        assert_eq!(
            settled(ClosedCompactRight, 320.0, 48.0).clip_translate,
            272.0
        );
    }

    #[test]
    fn overlay_closing_keeps_dismiss_layer_and_pane_until_animation_finishes() {
        let start = transition(OpenOverlayRight, Closed, Duration::ZERO, 320.0, 48.0);
        assert!(start.pane_visible && start.overlay_visible && start.pane_right);
        assert_eq!(start.overlay_opacity, 1.0);
        let middle = transition(
            OpenOverlayRight,
            Closed,
            Duration::from_millis(60),
            320.0,
            48.0,
        );
        assert!(middle.pane_visible && middle.overlay_visible);
        assert!(middle.pane_translate > 0.0 && middle.pane_translate < 320.0);
        assert_eq!(middle.pane_translate, -middle.clip_translate);
        assert!(middle.overlay_opacity > 0.0 && middle.overlay_opacity < 1.0);
        let end = transition(
            OpenOverlayRight,
            Closed,
            Duration::from_millis(120),
            320.0,
            48.0,
        );
        assert!(!end.pane_visible && !end.overlay_visible);
    }

    #[test]
    fn inline_close_discrete_visibility_waits_until_close_duration() {
        let v = transition(
            OpenInlineLeft,
            Closed,
            Duration::from_millis(99),
            320.0,
            48.0,
        );
        assert!(v.pane_visible);
        assert_eq!(v.pane_span, 1);
        assert_eq!((v.content_column, v.content_span), (0, 2));
        let v = transition(
            OpenInlineLeft,
            Closed,
            Duration::from_millis(100),
            320.0,
            48.0,
        );
        assert!(!v.pane_visible);
        assert_eq!(v.pane_span, 2);
    }

    #[test]
    fn compact_inline_transition_asymmetry_matches_the_template() {
        assert_eq!(
            transition_duration(&winui(), OpenInlineLeft, ClosedCompactLeft),
            Some(Duration::from_millis(200))
        );
        assert_eq!(
            transition_duration(&winui(), OpenInlineRight, ClosedCompactRight),
            Some(Duration::from_millis(100))
        );
        assert_eq!(
            transition_duration(&winui(), ClosedCompactRight, OpenInlineRight),
            Some(Duration::from_nanos(199_990_000))
        );
        let left = transition(
            ClosedCompactLeft,
            OpenInlineLeft,
            Duration::ZERO,
            320.0,
            48.0,
        );
        let right = transition(
            ClosedCompactRight,
            OpenInlineRight,
            Duration::ZERO,
            320.0,
            48.0,
        );
        assert_eq!(left.content_translate, -272.0);
        assert_eq!(right.content_translate, 0.0);
        assert_eq!(right.clip_translate, 272.0);
    }

    #[test]
    fn interrupted_overlay_close_starts_implicit_tracks_at_the_live_position() {
        for (opened, direction) in [(OpenOverlayLeft, -1.0), (OpenOverlayRight, 1.0)] {
            let live = transition(Closed, opened, Duration::from_millis(40), 320.0, 48.0);
            let close = transition_from_visuals(
                &winui(),
                opened,
                Closed,
                Duration::ZERO,
                320.0,
                48.0,
                live,
            );
            assert_eq!(close.pane_translate, live.pane_translate);
            assert_eq!(close.clip_translate, live.clip_translate);
            // Opacity has an explicit value of one at time zero in the closing storyboard.
            assert_eq!(close.overlay_opacity, 1.0);
            let moving = transition_from_visuals(
                &winui(),
                opened,
                Closed,
                Duration::from_millis(20),
                320.0,
                48.0,
                live,
            );
            assert!(moving.pane_translate * direction > live.pane_translate * direction);
        }
    }

    #[test]
    fn all_transitions_finish_at_the_target_and_unlisted_pairs_change_immediately() {
        let states = [
            Closed,
            ClosedCompactLeft,
            ClosedCompactRight,
            OpenOverlayLeft,
            OpenOverlayRight,
            OpenInlineLeft,
            OpenInlineRight,
            OpenCompactOverlayLeft,
            OpenCompactOverlayRight,
        ];
        for from in states {
            for to in states {
                let duration = transition_duration(&winui(), from, to).unwrap_or_default();
                assert_eq!(
                    transition(from, to, duration, 320.0, 48.0),
                    settled(to, 320.0, 48.0)
                );
            }
        }
        assert_eq!(
            transition_duration(&winui(), OpenOverlayLeft, OpenOverlayRight),
            None
        );
    }
}
