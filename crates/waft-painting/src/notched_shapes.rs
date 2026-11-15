//! Flutter counterpart: `painting/notched_shapes.dart`.
//!
//! [`NotchedShape`] is an abstract class, so a trait; a widget that holds one holds a
//! [`NotchedShapeRef`].

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::{Offset, Path, PathBuilder, PathOperation, Rect, path_combine};

use crate::borders::ShapeBorder;

/// A shape with a notch in its outline.
///
/// Typically used as the outline of a 'host' widget to make a notch that
/// accommodates a 'guest' widget. e.g the `BottomAppBar` may have a notch to
/// accommodate the `FloatingActionButton`.
///
/// See also:
///
///  * [`ShapeBorder`], which defines a shaped border without a dynamic notch.
///  * [`AutomaticNotchedShape`], an adapter from [`ShapeBorder`] to [`NotchedShape`].
pub trait NotchedShape: Debug {
    /// Creates a [`Path`] that describes the outline of the shape.
    ///
    /// The `host` is the bounding rectangle of the shape.
    ///
    /// The `guest` is the bounding rectangle of the shape for which a notch will
    /// be made. It is null when there is no guest.
    fn get_outer_path(&self, host: Rect, guest: Option<Rect>) -> Path;
}

/// What a field or parameter Dart types as `NotchedShape` becomes.
pub type NotchedShapeRef = Rc<dyn NotchedShape>;

/// A rectangle with a smooth circular notch.
///
/// See also:
///
///  * `CircleBorder`, a [`ShapeBorder`] that describes a circle.
#[derive(Debug, Clone, Copy, Default)]
pub struct CircularNotchedRectangle {
    /// If [`inverted`](Self::inverted) is true, the notch is placed at the bottom of the
    /// rectangle.
    pub inverted: bool,
}

impl CircularNotchedRectangle {
    /// Creates a [`CircularNotchedRectangle`].
    ///
    /// The same object can be used to create multiple shapes.
    pub fn new() -> CircularNotchedRectangle {
        CircularNotchedRectangle::default()
    }

    /// Dart `CircularNotchedRectangle(inverted:)`.
    pub fn inverted(mut self, inverted: bool) -> CircularNotchedRectangle {
        self.inverted = inverted;
        self
    }
}

impl NotchedShape for CircularNotchedRectangle {
    /// Creates a [`Path`] that describes a rectangle with a smooth circular notch.
    ///
    /// `host` is the bounding box for the returned shape. Conceptually this is
    /// the rectangle to which the notch will be applied.
    ///
    /// `guest` is the bounding box of a circle that the notch accommodates. All
    /// points in the circle bounded by `guest` will be outside of the returned
    /// path.
    ///
    /// The notch is curve that smoothly connects the host's top edge and
    /// the guest circle.
    // TODO(amirh): add an example diagram here.
    fn get_outer_path(&self, host: Rect, guest: Option<Rect>) -> Path {
        let Some(guest) = guest.filter(|guest| host.overlaps(*guest)) else {
            let mut path = PathBuilder::new();
            path.rect(host.into());
            return path.build().into();
        };

        // The guest's shape is a circle bounded by the guest rectangle.
        // So the guest's radius is half the guest width.
        let r = guest.width() / 2.0;

        // The variables [p2yA] and [p2yB] need to be inverted
        // when the notch is drawn on the bottom of a path.
        let invert_multiplier = if self.inverted { -1.0 } else { 1.0 };

        // We build a path for the notch from 3 segments:
        // Segment A - a Bezier curve from the host's top edge to segment B.
        // Segment B - an arc with radius notchRadius.
        // Segment C - a Bezier curve from segment B back to the host's top edge.
        //
        // A detailed explanation and the derivation of the formulas below is
        // available at: https://docs.google.com/document/d/e/2PACX-1vRVPWGtR85bawGynRSWzYTKgQtqrxCnxXCKC5xM9ab3IvtRHueku4rRIuJ4TbedzyMz2oy2pkzM71-_/pub

        const S1: f64 = 15.0;
        const S2: f64 = 1.0;

        let a = -r - S2;
        let b = (if self.inverted { host.bottom } else { host.top }) - guest.center().dy();

        let n2 = (b * b * r * r * (a * a + b * b - r * r)).sqrt();
        let p2x_a = ((a * r * r) - n2) / (a * a + b * b);
        let p2x_b = ((a * r * r) + n2) / (a * a + b * b);
        let p2y_a = (r * r - p2x_a * p2x_a).sqrt() * invert_multiplier;
        let p2y_b = (r * r - p2x_b * p2x_b).sqrt() * invert_multiplier;

        let mut p = [Offset::ZERO; 6];

        // p0, p1, and p2 are the control points for segment A.
        p[0] = Offset::new(a - S1, b);
        p[1] = Offset::new(a, b);
        let cmp = if b < 0.0 { -1.0 } else { 1.0 };
        p[2] = if cmp * p2y_a > cmp * p2y_b {
            Offset::new(p2x_a, p2y_a)
        } else {
            Offset::new(p2x_b, p2y_b)
        };

        // p3, p4, and p5 are the control points for segment B, which is a mirror
        // of segment A around the y axis.
        p[3] = Offset::new(-p[2].dx(), p[2].dy());
        p[4] = Offset::new(-p[1].dx(), p[1].dy());
        p[5] = Offset::new(-p[0].dx(), p[0].dy());

        // translate all points back to the absolute coordinate system.
        for point in &mut p {
            *point = *point + guest.center();
        }

        // Use the calculated points to draw out a path object.
        let mut path = PathBuilder::new();
        path.move_to(Offset::new(host.left, host.top));
        if !self.inverted {
            path.line_to(p[0]);
            path.quad_to(p[1], p[2]);
            arc_to_point_counter_clockwise(&mut path, p[2], p[3], r);
            path.quad_to(p[4], p[5]);
            path.line_to(Offset::new(host.right, host.top));
            path.line_to(Offset::new(host.right, host.bottom));
            path.line_to(Offset::new(host.left, host.bottom));
        } else {
            path.line_to(Offset::new(host.right, host.top));
            path.line_to(Offset::new(host.right, host.bottom));
            path.line_to(p[5]);
            path.quad_to(p[4], p[3]);
            arc_to_point_counter_clockwise(&mut path, p[3], p[2], r);
            path.quad_to(p[1], p[0]);
            path.line_to(Offset::new(host.left, host.bottom));
        }

        path.close();
        path.build().into()
    }
}

/// Dart's `Path.arcToPoint(end, radius: Radius.circular(radius), clockwise: false)`, from the
/// path's current point `start`: the small circular arc of that radius through both points that
/// turns counterclockwise on screen. A radius too small to reach `end` grows to half the chord,
/// as SVG's elliptical arc does.
fn arc_to_point_counter_clockwise(path: &mut PathBuilder, start: Offset, end: Offset, radius: f64) {
    let chord = end - start;
    let half_chord = chord.distance() / 2.0;
    if half_chord == 0.0 || radius <= 0.0 {
        path.line_to(end);
        return;
    }
    let radius = radius.max(half_chord);
    let middle = Offset::new((start.dx() + end.dx()) / 2.0, (start.dy() + end.dy()) / 2.0);
    let distance_to_center = (radius * radius - half_chord * half_chord).sqrt();
    // A small arc turning counterclockwise on screen has its center on the left of the chord's
    // direction of travel, which with y down is the perpendicular (dy, -dx).
    let normal = Offset::new(chord.dy(), -chord.dx()) / chord.distance();
    let center = Offset::new(
        middle.dx() + normal.dx() * distance_to_center,
        middle.dy() + normal.dy() * distance_to_center,
    );
    let start_angle = (start.dy() - center.dy()).atan2(start.dx() - center.dx());
    let end_angle = (end.dy() - center.dy()).atan2(end.dx() - center.dx());
    // Valo's angles grow clockwise on screen, so a counterclockwise arc sweeps negatively.
    let mut sweep = end_angle - start_angle;
    while sweep > 0.0 {
        sweep -= std::f64::consts::TAU;
    }
    path.arc(center, radius as f32, start_angle as f32, sweep as f32);
}

/// A [`NotchedShape`] created from [`ShapeBorder`]s.
///
/// Two shapes can be provided. The [`host`](Self::host) is the shape of the widget that
/// uses the [`NotchedShape`] (typically a `BottomAppBar`). The [`guest`](Self::guest) is
/// subtracted from the host to create the notch (typically to make room
/// for a `FloatingActionButton`).
#[derive(Debug)]
pub struct AutomaticNotchedShape {
    /// The shape of the widget that uses the [`NotchedShape`] (typically a
    /// `BottomAppBar`).
    ///
    /// This shape cannot depend on the `TextDirection`, as no text direction
    /// is available to [`NotchedShape`]s.
    pub host: Box<dyn ShapeBorder>,
    /// The shape to subtract from the [`host`](Self::host) to make the notch.
    ///
    /// This shape cannot depend on the `TextDirection`, as no text direction
    /// is available to [`NotchedShape`]s.
    ///
    /// If this is null, [`get_outer_path`](NotchedShape::get_outer_path) ignores the guest
    /// rectangle.
    pub guest: Option<Box<dyn ShapeBorder>>,
}

impl AutomaticNotchedShape {
    /// Creates a [`NotchedShape`] that is defined by two [`ShapeBorder`]s.
    ///
    /// The [`guest`](Self::guest) may be null, in which case no notch is created even
    /// if a guest rectangle is provided to [`get_outer_path`](NotchedShape::get_outer_path).
    pub fn new(host: impl ShapeBorder + 'static) -> AutomaticNotchedShape {
        AutomaticNotchedShape {
            host: Box::new(host),
            guest: None,
        }
    }

    /// Dart `AutomaticNotchedShape(host, guest)`.
    pub fn guest(mut self, guest: impl ShapeBorder + 'static) -> AutomaticNotchedShape {
        self.guest = Some(Box::new(guest));
        self
    }
}

impl Clone for AutomaticNotchedShape {
    fn clone(&self) -> AutomaticNotchedShape {
        AutomaticNotchedShape {
            host: self.host.clone_box(),
            guest: self.guest.as_ref().map(|guest| guest.clone_box()),
        }
    }
}

impl NotchedShape for AutomaticNotchedShape {
    fn get_outer_path(&self, host_rect: Rect, guest_rect: Option<Rect>) -> Path {
        let host_path = self.host.get_outer_path(host_rect, None);
        if let (Some(guest), Some(guest_rect)) = (&self.guest, guest_rect) {
            let guest_path = guest.get_outer_path(guest_rect, None);
            return path_combine(PathOperation::Difference, &host_path, &guest_path);
        }
        host_path
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;
    use crate::rounded_rectangle_border::RoundedRectangleBorder;

    fn path_contains(path: &Path, point: Offset) -> bool {
        path.contains(point)
    }

    /// Dart's `pathDoesNotContainCircle`.
    fn path_does_not_contain_circle(path: &Path, circle_bounds: Rect) -> bool {
        assert_eq!(circle_bounds.width(), circle_bounds.height());
        let radius = circle_bounds.width() / 2.0;

        let mut theta = 0.0;
        while theta <= 2.0 * PI {
            let mut i = 0.0;
            while i < 1.0 {
                let x = i * radius * f64::cos(theta);
                let y = i * radius * f64::sin(theta);
                if path_contains(path, Offset::new(x, y) + circle_bounds.center()) {
                    return false;
                }
                i += 0.01;
            }
            theta += PI / 20.0;
        }
        true
    }

    /// Dart's `coversSameAreaAs`: both paths agree on every sample of a `sample_size` grid over
    /// `area_to_compare`.
    fn covers_same_area_as(
        actual: &Path,
        expected: &Path,
        area_to_compare: Rect,
        sample_size: usize,
    ) -> bool {
        for row in 0..sample_size {
            for column in 0..sample_size {
                let point = Offset::new(
                    area_to_compare.left
                        + (column as f64 + 0.5) * area_to_compare.width() / sample_size as f64,
                    area_to_compare.top
                        + (row as f64 + 0.5) * area_to_compare.height() / sample_size as f64,
                );
                if path_contains(actual, point) != path_contains(expected, point) {
                    return false;
                }
            }
        }
        true
    }

    fn rect_path(rect: Rect) -> Path {
        let mut path = PathBuilder::new();
        path.rect(rect.into());
        path.build().into()
    }

    fn host() -> Rect {
        Rect::from_ltrb(0.0, 100.0, 300.0, 300.0)
    }

    #[test]
    fn a_circular_notched_rectangle_without_overlap_is_the_host_rectangle() {
        let shape = CircularNotchedRectangle::new();
        let host = host();
        let guest = Rect::from_ltwh(50.0, 50.0, 10.0, 10.0);

        let actual = shape.get_outer_path(host, Some(guest));

        assert!(covers_same_area_as(
            &actual,
            &rect_path(host),
            host.inflate(5.0),
            40
        ));
    }

    #[test]
    fn a_circular_notch_excludes_a_guest_centered_above_the_host() {
        let shape = CircularNotchedRectangle::new();
        let guest = Rect::from_ltrb(190.0, 85.0, 210.0, 105.0);

        let actual = shape.get_outer_path(host(), Some(guest));

        assert!(path_does_not_contain_circle(&actual, guest));
    }

    #[test]
    fn a_circular_notch_excludes_a_guest_centered_below_the_host_top() {
        let shape = CircularNotchedRectangle::new();
        let guest = Rect::from_ltrb(190.0, 95.0, 210.0, 115.0);

        let actual = shape.get_outer_path(host(), Some(guest));

        assert!(path_does_not_contain_circle(&actual, guest));
        assert!(path_contains(&actual, Offset::new(200.0, 118.0)));
        assert!(path_contains(&actual, Offset::new(10.0, 110.0)));
    }

    #[test]
    fn an_inverted_circular_notch_excludes_a_guest_centered_above_the_host_bottom() {
        let shape = CircularNotchedRectangle::new().inverted(true);
        let guest = Rect::from_ltrb(190.0, 285.0, 210.0, 305.0);

        let actual = shape.get_outer_path(host(), Some(guest));

        assert!(path_does_not_contain_circle(&actual, guest));
    }

    #[test]
    fn an_inverted_circular_notch_excludes_a_guest_centered_below_the_host_bottom() {
        let shape = CircularNotchedRectangle::new().inverted(true);
        let guest = Rect::from_ltrb(190.0, 295.0, 210.0, 315.0);

        let actual = shape.get_outer_path(host(), Some(guest));

        assert!(path_does_not_contain_circle(&actual, guest));
    }

    #[test]
    fn a_circular_notched_rectangle_without_a_guest_is_the_host_rectangle() {
        let host = host();

        let actual = CircularNotchedRectangle::new().get_outer_path(host, None);

        assert!(covers_same_area_as(
            &actual,
            &rect_path(host),
            host.inflate(800.0),
            100
        ));
    }

    /// Dart's `AutomaticNotchedShape - with guest`. Its expected path keeps the overlap and
    /// cuts the host's bottom right instead, and passes only because Dart's `coversSameAreaAs`
    /// samples from the origin, outside both paths; here the expected path is the host with
    /// the guest's overlap cut out, which is what `Path.combine` gives.
    #[test]
    fn an_automatic_notched_shape_with_a_guest_cuts_the_guest_from_the_host() {
        let actual = AutomaticNotchedShape::new(RoundedRectangleBorder::default())
            .guest(RoundedRectangleBorder::default())
            .get_outer_path(
                Rect::from_ltwh(-200.0, -100.0, 50.0, 100.0),
                Some(Rect::from_ltwh(-175.0, -110.0, 100.0, 100.0)),
            );

        let mut expected = PathBuilder::new();
        expected.move_to(Offset::new(-200.0, -100.0));
        expected.line_to(Offset::new(-175.0, -100.0));
        expected.line_to(Offset::new(-175.0, -10.0));
        expected.line_to(Offset::new(-150.0, -10.0));
        expected.line_to(Offset::new(-150.0, 0.0));
        expected.line_to(Offset::new(-200.0, 0.0));
        expected.close();
        assert!(covers_same_area_as(
            &actual,
            &Path::new(expected.build()),
            Rect::from_ltwh(-300.0, -300.0, 600.0, 600.0),
            100
        ));
    }

    // ---- not in notched_shapes_test.dart: a guest wholly inside the host ----

    #[test]
    fn an_automatic_notched_shape_cuts_a_hole_for_a_guest_wholly_inside_the_host() {
        let host = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
        let guest = Rect::from_ltwh(25.0, 25.0, 50.0, 50.0);

        let actual = AutomaticNotchedShape::new(RoundedRectangleBorder::default())
            .guest(RoundedRectangleBorder::default())
            .get_outer_path(host, Some(guest));

        assert_eq!(actual.fill_type, waft_embedder::PathFillType::EvenOdd);
        assert!(path_contains(&actual, Offset::new(10.0, 10.0)));
        assert!(
            !path_contains(&actual, guest.center()),
            "the guest is a hole"
        );
        assert!(!path_contains(&actual, Offset::new(150.0, 50.0)));
    }

    #[test]
    fn an_automatic_notched_shape_without_a_guest_is_the_host_shape() {
        let host = Rect::from_ltwh(-200.0, -100.0, 50.0, 100.0);

        let actual = AutomaticNotchedShape::new(RoundedRectangleBorder::default())
            .guest(RoundedRectangleBorder::default())
            .get_outer_path(host, None);

        assert!(covers_same_area_as(
            &actual,
            &rect_path(host),
            Rect::from_ltwh(-300.0, -300.0, 600.0, 600.0),
            100
        ));
    }
}
