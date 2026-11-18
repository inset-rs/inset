//! WinUI's page navigation: a page enters as a `Frame` shows it, with
//! `NavigationThemeTransition` and its default `EntranceNavigationTransitionInfo`.
//!
//! `FluentPageRoute` builds a [`PageRouteBuilder`] that plays that entrance. The values are the
//! ones `ThemeTransitions.cpp` registers for `NavigationTrigger::NavigatingTo`: the page starts
//! 140 effective pixels lower, stays fully transparent for 150 ms, then slides up over 300 ms
//! on the cubic `(0.1, 0.9, 0.2, 1.0)` while becoming visible. Leaving plays the
//! `BackNavigatingAway` storyboard: it slides down 140 pixels over 150 ms on the cubic
//! `(0.7, 0.0, 1.0, 0.5)` and is hidden at the end.

use std::rc::Rc;
use std::time::Duration;

use waft_animation::{AnimationStatus, Cubic, Curve};
use waft_embedder::Offset;
use waft_foundation::App;
use waft_widgets::{
    AnimatedBuilder, BuildContext, IntoWidget, Opacity, PageRouteBuilder, RoutePageBuilder,
    Transform, WidgetBuilder, WidgetRef,
};

/// The page route a Fluent app navigates with.
///
/// [`new`](Self::new) answers a [`PageRouteBuilder`] playing WinUI's entrance, ready to be
/// pushed on its own or returned from a [`WidgetsApp::page_route_builder`](waft_widgets::WidgetsApp::page_route_builder).
/// Unlike a Material or Cupertino page route, it is the configured builder rather than a route
/// type of its own, so every [`PageRouteBuilder`] option stays at hand.
pub struct FluentPageRoute;

impl FluentPageRoute {
    /// How long the page takes to enter: the 150 ms the entrance stays still, then the 300 ms it
    /// slides up, as `ThemeTransitions.cpp` gives them.
    pub const TRANSITION_DURATION: Duration = Duration::from_millis(450);

    /// How long the page takes to leave, as `BackNavigatingAway` gives it.
    pub const REVERSE_TRANSITION_DURATION: Duration = Duration::from_millis(150);

    /// How far below its place the page starts, in effective pixels.
    pub const TRANSLATION_OFFSET: f64 = 140.0;

    /// The 150 ms the entering page waits before it moves and shows, out of
    /// [`TRANSITION_DURATION`](Self::TRANSITION_DURATION).
    const OUT_DURATION: Duration = Duration::from_millis(150);

    /// The curve the entering page slides up on.
    const IN_CURVE: Cubic = Cubic::new(0.1, 0.9, 0.2, 1.0);

    /// The curve the leaving page slides down on.
    const OUT_CURVE: Cubic = Cubic::new(0.7, 0.0, 1.0, 0.5);

    /// The route for `builder`'s page, playing WinUI's page entrance.
    pub fn new(app: &mut App, builder: WidgetBuilder) -> waft_foundation::Ref<PageRouteBuilder> {
        let page_builder: RoutePageBuilder =
            Rc::new(move |app, context, _animation, _secondary| builder(app, context));
        PageRouteBuilder::new(app, page_builder)
            .transition_duration(app, Self::TRANSITION_DURATION)
            .reverse_transition_duration(app, Self::REVERSE_TRANSITION_DURATION)
            .transitions_builder(app, Rc::new(fluent_transitions))
    }
}

/// The entrance and the exit, driven by the route's one animation.
fn fluent_transitions(
    _app: &mut App,
    _context: BuildContext,
    animation: waft_animation::AnimationRef<f64>,
    _secondary_animation: waft_animation::AnimationRef<f64>,
    child: WidgetRef,
) -> WidgetRef {
    let listenable: Rc<dyn waft_foundation::Listenable> = Rc::new(animation.clone());
    AnimatedBuilder::new(listenable, move |app, _context, child| {
        let (offset, opacity) = entrance(
            animation.value(app),
            animation.status(app) == AnimationStatus::Reverse,
        );
        let child = child
            .cloned()
            .unwrap_or_else(|| waft_widgets::SizedBox::shrink().into_widget());
        Opacity::new(opacity)
            .child(Transform::translate(Offset::new(0.0, offset)).child(child))
            .into_widget()
    })
    .child(child)
    .into_widget()
}

/// How far down the page is and how visible, for `value` of the route's animation: the entrance
/// while it runs forward, `BackNavigatingAway` while it reverses.
fn entrance(value: f64, reversing: bool) -> (f64, f64) {
    if reversing {
        let progress = 1.0 - value;
        (
            FluentPageRoute::TRANSLATION_OFFSET * FluentPageRoute::OUT_CURVE.transform(progress),
            if progress < 1.0 { 1.0 } else { 0.0 },
        )
    } else {
        let waited = FluentPageRoute::OUT_DURATION.as_secs_f64();
        let moving = FluentPageRoute::TRANSITION_DURATION.as_secs_f64() - waited;
        let progress = ((value * FluentPageRoute::TRANSITION_DURATION.as_secs_f64() - waited)
            / moving)
            .clamp(0.0, 1.0);
        (
            FluentPageRoute::TRANSLATION_OFFSET
                * (1.0 - FluentPageRoute::IN_CURVE.transform(progress)),
            if value * FluentPageRoute::TRANSITION_DURATION.as_secs_f64() >= waited {
                1.0
            } else {
                0.0
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entrance_waits_150_ms_invisible_140_pixels_low_then_slides_up() {
        let at = |ms: f64| entrance(ms / 450.0, false);
        assert_eq!(at(0.0), (140.0, 0.0));
        assert_eq!(at(149.0), (140.0, 0.0), "still waiting");
        assert_eq!(at(150.0), (140.0, 1.0), "the opacity keyframe at 150 ms");
        let (offset, opacity) = at(300.0);
        assert!(opacity == 1.0, "visible once the slide starts");
        assert!(
            offset > 0.0 && offset < 140.0,
            "already moving up: {offset}"
        );
        assert_eq!(at(450.0), (0.0, 1.0));
    }

    #[test]
    fn leaving_slides_down_on_the_out_curve_and_hides_at_the_end() {
        assert_eq!(entrance(1.0, true), (0.0, 1.0));
        let (offset, opacity) = entrance(0.5, true);
        assert!(offset > 0.0 && offset < 140.0, "moving down: {offset}");
        assert_eq!(opacity, 1.0);
        assert_eq!(entrance(0.0, true), (140.0, 0.0));
    }

    #[test]
    fn the_route_uses_winuis_durations() {
        assert_eq!(FluentPageRoute::TRANSITION_DURATION.as_millis(), 450);
        assert_eq!(
            FluentPageRoute::REVERSE_TRANSITION_DURATION.as_millis(),
            150
        );
        assert_eq!(FluentPageRoute::TRANSLATION_OFFSET, 140.0);
    }
}
