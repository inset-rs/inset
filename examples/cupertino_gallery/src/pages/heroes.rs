//! Hero flights: one widget in two routes, sharing a tag.
//!
//! Tap a swatch. The small square does not fade out while a large one fades in — the same tag
//! is matched across the two routes, lifted into the navigator's overlay, and flown along a
//! rect tween.
//!
//! Both swatches set `transition_on_user_gestures`, so the edge swipe back flies too. Flutter's
//! default is `false` and so is ours, and both ends have to opt in for it to take effect — a
//! hero that flies on a button press and not on a swipe is the default, not an oversight.
//!
//! They also supply a `flight_shuttle_builder`, the only one in the gallery. Watch the corner:
//! the rect tween carries the box between the two sizes, and the shuttle carries the corner
//! radius with it, because the two ends bake different radii and the default shuttle would fly
//! one end's child at the other end's size.

use std::rc::Rc;

use reveal_animation::{AnyAnimation, Tween};
use reveal_cupertino::{
    CupertinoColors, CupertinoDynamicColor, CupertinoListTile, CupertinoListTileChevron,
};
use reveal_foundation::{App, Listener};
use reveal_painting::{AnyColor, BorderRadiusGeometry, BoxDecoration};
use reveal_widgets::{
    AnimatedBuilder, BuildContext, Container, Hero, HeroTagRef, IntoWidget, Text, WidgetRef,
};

use crate::app::open_sub_page;
use crate::catalog::Entry;
use crate::support::{screen, scrolling_body, section_with_footer, showcase};

pub struct Subject {
    pub title: &'static str,
    pub tag: &'static str,
    color: AnyColor,
}

pub const SUBJECTS: [Subject; 3] = [
    Subject {
        title: "Blue",
        tag: "swatch-blue",
        color: CupertinoColors::SYSTEM_BLUE,
    },
    Subject {
        title: "Orange",
        tag: "swatch-orange",
        color: CupertinoColors::SYSTEM_ORANGE,
    },
    Subject {
        title: "Teal",
        tag: "swatch-teal",
        color: CupertinoColors::SYSTEM_TEAL,
    },
];

/// The swatch's side in the list, and on the detail screen.
const SMALL: f64 = 36.0;
const LARGE: f64 = 220.0;

/// The corner radius as a fraction of the side, so the squircle reads as the same shape at both
/// sizes.
const CORNER: f64 = 0.22;

pub fn body(app: &mut App, context: BuildContext) -> WidgetRef {
    let rows = SUBJECTS
        .iter()
        .enumerate()
        .map(|(index, subject)| row(index, subject, app, context))
        .collect();

    screen(scrolling_body(vec![section_with_footer(
        "Swatches",
        "Tap one and watch the corner: a flight_shuttle_builder interpolates the radius while \
         the rect tween moves the box.",
        rows,
    )]))
}

pub fn sub_body(index: usize, app: &mut App, context: BuildContext) -> WidgetRef {
    let subject = &SUBJECTS[index];
    screen(scrolling_body(vec![section_with_footer(
        subject.title,
        "The same Hero at 220 points. Swipe back from the left edge — it flies then too, and \
         the corner shrinks with it.",
        vec![showcase(swatch(subject, LARGE, app, context))],
    )]))
}

fn row(index: usize, subject: &Subject, app: &mut App, context: BuildContext) -> WidgetRef {
    CupertinoListTile::notched(Text::new(subject.title))
        .leading(swatch(subject, SMALL, app, context))
        .leading_size(SMALL)
        .trailing(CupertinoListTileChevron::new())
        .on_tap(Listener::new(move |app: &mut App| {
            open_sub_page(app, context, Entry::Heroes, index);
        }))
        .into_widget()
}

fn swatch(subject: &Subject, side: f64, app: &mut App, context: BuildContext) -> WidgetRef {
    let color = CupertinoDynamicColor::resolve(&subject.color, app, context);
    let tag: HeroTagRef = Rc::new(subject.tag);
    let flying = color.clone();

    Hero::new(
        tag,
        Container::new()
            .decoration(box_decoration(color, side * CORNER))
            .width(side)
            .height(side),
    )
    // Both ends set it, which is the requirement: the flight consults BOTH heroes and skips a
    // gesture transition unless each has opted in. One end alone would look like the flag doing
    // nothing.
    .transition_on_user_gestures(true)
    // Without this the flight would SNAP. The default shuttle carries one end's child across at
    // the other end's size, and these two ends are not "essentially identical" the way `Hero`'s
    // doc requires: each bakes a radius from its own side, 7.92 against 48.4. The rect tween
    // moves the box; this moves the corner with it.
    .flight_shuttle_builder(Rc::new(
        move |app: &mut App, _context, animation: AnyAnimation<f64>, _direction, _from, _to| {
            // Small at 0 and large at 1 in BOTH directions, with no match on the direction: the
            // flight animation is the detail route's either way, so 0 is always the list end.
            let tween = Tween::new(app, Some(SMALL * CORNER), Some(LARGE * CORNER));
            let radius = animation.drive(app, tween);
            let color = flying.clone();
            // A builder rather than a value read once: the flight builds its shuttle ONCE and
            // then only repositions it, so anything that has to change during the flight has to
            // watch the animation itself.
            AnimatedBuilder::new(Rc::new(radius), move |app: &mut App, _context, _child| {
                // No size: the overlay gives the shuttle the flight rect's tight constraints, so
                // the box is already the right size on every tick and only the corner is ours.
                Container::new()
                    .decoration(box_decoration(color.clone(), radius.value(app)))
                    .into_widget()
            })
            .into_widget()
        },
    ))
    .into_widget()
}

fn box_decoration(color: AnyColor, radius: f64) -> BoxDecoration {
    BoxDecoration::new()
        .color(color)
        .border_radius(BorderRadiusGeometry::circular(radius))
}
