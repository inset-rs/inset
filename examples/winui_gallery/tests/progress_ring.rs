//! ProgressRing's two generated visuals: the determinate arc, the turning indeterminate one,
//! and the inactive state that hides both.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::{Fixture, mount};
use inset_embedder::{Color, TextDirection};
use inset_painting::AlignmentGeometry;
use inset_widgets::*;
use inset_winui::*;
use std::time::Duration;

const VIEW: [u32; 2] = [64, 64];
const ACCENT: Color = Color::from_argb(255, 0, 103, 192);

/// The tree one ring is mounted in, so replacing it keeps the ring's own state.
fn tree(ring: ProgressRing) -> WidgetRef {
    Directionality::new(
        TextDirection::Ltr,
        ThemeScope::new(
            Theme::Light,
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(ring),
        ),
    )
    .into_widget()
}

/// Mounts one ring at the view's top-left corner.
fn ring_at_top_left(ring: ProgressRing) -> Fixture {
    Fixture::with_root(VIEW, mount(move |_| tree(ring)))
}

/// Hands the mounted ring new property values, as an owner rebuild does.
///
/// Attaches the tree in place, in the `MultiViewApp` `run_app` builds the view with. `run_app`
/// would schedule a warm-up frame that resets the scheduler epoch, which would move an
/// already-running visual independently of the properties being replaced.
fn replace(fixture: &mut Fixture, ring: ProgressRing) {
    {
        let mut app = fixture.cell.borrow_mut();
        let binding = WidgetsBinding::instance(&mut app);
        let tree = tree(ring);
        let root = MultiViewApp::new(move |_, _| tree.clone()).into_widget();
        binding.attach_root_widget(&mut app, root);
    }
    advance(fixture, Duration::from_millis(20));
}

/// Runs frames until the requested time has passed.
fn advance(fixture: &mut Fixture, duration: Duration) {
    let step = Duration::from_millis(20);
    let mut remaining = duration;
    while !remaining.is_zero() {
        let delta = step.min(remaining);
        fixture.frame_after(delta);
        remaining -= delta;
    }
}

/// How many of the four quadrants the arc covers.
fn covered(fixture: &Fixture) -> usize {
    quadrants(fixture)
        .into_iter()
        .filter(|drawn| *drawn)
        .count()
}

/// Which quadrants of the 32-pixel ring carry the accent arc.
fn quadrants(fixture: &Fixture) -> [bool; 4] {
    let pixels = fixture.view.pixels.borrow();
    let accent = |x: usize, y: usize| {
        let index = (y * VIEW[0] as usize + x) * 4;
        let pixel = &pixels[index..index + 3];
        // The arc is antialiased, so a quadrant counts as drawn when it is not plain white.
        pixel != [255, 255, 255]
    };
    // One sample per quadrant, on the ring's radius of about 14 around its centre at 16,16.
    let mut found = [false; 4];
    for (index, (dx, dy)) in [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)]
        .into_iter()
        .enumerate()
    {
        // Sweep a few pixels along the radius so antialiasing cannot hide a drawn arc.
        found[index] = (12..=16).any(|radius| {
            let x = (16.0 + dx * radius as f64).round() as usize;
            let y = (16.0 + dy * radius as f64).round() as usize;
            accent(x, y)
        });
    }
    found
}

#[test]
fn a_determinate_ring_fills_clockwise_from_the_top() {
    // ProgressRingDeterminate trims its ellipse from the start point, which is the ring's top.
    let quarter = ring_at_top_left(
        ProgressRing::new()
            .is_indeterminate(false)
            .value(25.0)
            .foreground(ACCENT),
    );
    assert_eq!(
        quadrants(&quarter),
        [true, true, false, false],
        "a quarter covers the top and right"
    );
    let half = ring_at_top_left(
        ProgressRing::new()
            .is_indeterminate(false)
            .value(50.0)
            .foreground(ACCENT),
    );
    assert_eq!(
        quadrants(&half),
        [true, true, true, false],
        "a half reaches the bottom"
    );
}

#[test]
fn a_determinate_ring_at_its_minimum_draws_no_arc() {
    // ShapeVisibilityAnimation keeps the arc's container scaled to nothing below 1/120th.
    let empty = ring_at_top_left(
        ProgressRing::new()
            .is_indeterminate(false)
            .value(0.0)
            .foreground(ACCENT),
    );
    assert_eq!(quadrants(&empty), [false; 4]);
}

#[test]
fn an_inactive_ring_draws_nothing() {
    // The Inactive visual state sets LayoutRoot.Opacity to 0.
    let inactive = ring_at_top_left(
        ProgressRing::new()
            .is_indeterminate(false)
            .value(100.0)
            .is_active(false)
            .foreground(ACCENT),
    );
    assert_eq!(quadrants(&inactive), [false; 4]);
}

#[test]
fn an_indeterminate_ring_keeps_turning_and_never_settles() {
    let mut fixture = ring_at_top_left(ProgressRing::new().foreground(ACCENT));
    let mut seen = Vec::new();
    for _ in 0..8 {
        seen.push(quadrants(&fixture));
        advance(&mut fixture, Duration::from_millis(240));
    }
    assert!(
        seen.iter().any(|frame| frame.iter().any(|drawn| *drawn)),
        "the arc is drawn at some point in the loop"
    );
    assert!(
        seen.windows(2).any(|pair| pair[0] != pair[1]),
        "the arc moves between frames: {seen:?}"
    );
}

#[test]
fn a_rising_value_animates_and_a_falling_one_jumps() {
    // ProgressRing::UpdateLottieProgress plays a rising segment at the visual's own speed, two
    // seconds for the whole ring, and sets the progress outright when the value falls.
    let mut fixture = ring_at_top_left(
        ProgressRing::new()
            .is_indeterminate(false)
            .value(0.0)
            .foreground(ACCENT),
    );
    replace(
        &mut fixture,
        ProgressRing::new()
            .is_indeterminate(false)
            .value(100.0)
            .foreground(ACCENT),
    );
    assert_eq!(
        covered(&fixture),
        0,
        "the segment starts where the old value left off"
    );
    advance(&mut fixture, Duration::from_millis(1100));
    let midway = covered(&fixture);
    assert!(
        (1..4).contains(&midway),
        "the ring is part way round after half the two-second segment: {midway}"
    );
    advance(&mut fixture, Duration::from_millis(1100));
    assert_eq!(covered(&fixture), 4, "the ring closes");
    replace(
        &mut fixture,
        ProgressRing::new()
            .is_indeterminate(false)
            .value(25.0)
            .foreground(ACCENT),
    );
    assert_eq!(
        quadrants(&fixture),
        [true, true, false, false],
        "a falling value is shown at once"
    );
}

#[test]
fn the_gallery_page_shows_both_rings() {
    let mut fixture = Fixture::for_feature([1100, 900], winui_gallery::Feature::ProgressRing);
    fixture.find("Progress");
    fixture.tap("Value: 40%");
    fixture.find("Value: 60%");
    fixture.capture("progress_ring_light");
    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("progress_ring_dark");
}

#[test]
fn changing_a_running_segment_replaces_its_clock() {
    let mut f = ring_at_top_left(ProgressRing::new().is_indeterminate(false));
    replace(
        &mut f,
        ProgressRing::new().is_indeterminate(false).value(50.0),
    );
    advance(&mut f, Duration::from_millis(400));
    replace(
        &mut f,
        ProgressRing::new().is_indeterminate(false).value(75.0),
    );
    advance(&mut f, Duration::from_millis(600));
    let expected = ring_at_top_left(ProgressRing::new().is_indeterminate(false).value(75.0));
    assert_eq!(*f.view.pixels.borrow(), *expected.view.pixels.borrow());

    replace(&mut f, ProgressRing::new());
    replace(
        &mut f,
        ProgressRing::new().is_indeterminate(false).value(25.0),
    );
    advance(&mut f, Duration::from_secs(2));
    let expected = ring_at_top_left(ProgressRing::new().is_indeterminate(false).value(25.0));
    assert_eq!(*f.view.pixels.borrow(), *expected.view.pixels.borrow());
}

#[test]
fn changing_an_indeterminate_value_does_not_restart_the_loop() {
    let mut changed = ring_at_top_left(ProgressRing::new());
    let mut unchanged = ring_at_top_left(ProgressRing::new());
    advance(&mut changed, Duration::from_millis(640));
    advance(&mut unchanged, Duration::from_millis(640));
    replace(&mut changed, ProgressRing::new().value(50.0));
    advance(&mut unchanged, Duration::from_millis(20));
    assert_eq!(
        *changed.view.pixels.borrow(),
        *unchanged.view.pixels.borrow()
    );
}

#[test]
fn reactivating_a_determinate_ring_shows_its_requested_value() {
    let mut f = ring_at_top_left(ProgressRing::new().is_indeterminate(false));
    replace(
        &mut f,
        ProgressRing::new().is_indeterminate(false).value(100.0),
    );
    replace(
        &mut f,
        ProgressRing::new()
            .is_indeterminate(false)
            .value(100.0)
            .is_active(false),
    );
    replace(
        &mut f,
        ProgressRing::new().is_indeterminate(false).value(100.0),
    );
    assert_eq!(covered(&f), 4);
}

#[test]
fn the_ring_does_not_intercept_pointer_input() {
    use inset_embedder::{Offset, PointerChange};
    use inset_foundation::Listener;
    use std::{cell::Cell, rc::Rc};

    let clicks = Rc::new(Cell::new(0));
    let shared = clicks.clone();
    let mut f = Fixture::with_root(
        VIEW,
        mount(move |_| {
            Directionality::new(
                TextDirection::Ltr,
                ThemeScope::new(
                    Theme::Light,
                    Stack::new().children([
                        Positioned::fill(
                            GestureDetector::new()
                                .behavior(inset_rendering::HitTestBehavior::Opaque)
                                .on_tap(Listener::new(move |_| shared.set(shared.get() + 1)))
                                .child(SizedBox::expand()),
                        )
                        .into_widget(),
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(ProgressRing::new())
                            .into_widget(),
                    ]),
                ),
            )
            .into_widget()
        }),
    );
    f.send(PointerChange::Down, Offset::new(16.0, 16.0));
    f.send(PointerChange::Up, Offset::new(16.0, 16.0));
    f.pump();
    assert_eq!(clicks.get(), 1);
}

#[test]
fn a_rectangular_ring_stretches_its_visual_to_fill() {
    let f = Fixture::with_root(
        VIEW,
        mount(move |_| {
            Directionality::new(
                TextDirection::Ltr,
                ThemeScope::new(
                    Theme::Light,
                    Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                        SizedBox::new()
                            .width(64.0)
                            .height(32.0)
                            .child(ProgressRing::new().is_indeterminate(false).value(100.0)),
                    ),
                ),
            )
            .into_widget()
        }),
    );
    let pixels = f.view.pixels.borrow();
    let filled = |x: usize, y: usize| {
        let index = (y * 64 + x) * 4;
        pixels[index..index + 3] != [255, 255, 255]
    };
    assert!(
        filled(3, 16) && filled(60, 16),
        "the ellipse reaches both horizontal edges"
    );
    assert!(
        filled(32, 2) && filled(32, 29),
        "the ellipse fills the vertical extent too"
    );
    drop(pixels);
    f.capture("progress_ring_rectangular");
}
