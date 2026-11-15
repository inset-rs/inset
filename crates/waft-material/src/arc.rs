//! Flutter counterpart: material_ui's `arc.dart`.

use std::f64::consts::PI;

use waft_animation::Animatable;
use waft_embedder::{Offset, Rect, lerp_double};
use waft_foundation::App;
use waft_widgets::RectTweenObject;

// How close the begin and end points must be to an axis to be considered
// vertical or horizontal.
const K_ON_AXIS_DELTA: f64 = 2.0;

/// Dart's `num.sign`: `-1.0`, `0.0` or `1.0`. `f64::signum` answers `±1.0` for `±0.0`.
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// A tween that interpolates an [`Offset`] along a circular arc.
///
/// This class specializes the interpolation of `Tween<Offset>` so that instead
/// of a straight line, the intermediate points follow the arc of a circle in a
/// manner consistent with Material Design principles.
///
/// The arc's radius is related to the bounding box that contains the
/// [`begin`](Self::begin) and [`end`](Self::end) points. If the bounding box is
/// taller than it is wide, then the center of the circle will be horizontally
/// aligned with the end point. Otherwise the center of the circle will be aligned
/// with the begin point. The arc's sweep is always less than or equal to 90
/// degrees.
///
/// The arc is worked out when [`begin`](Self::begin) and [`end`](Self::end) are
/// both set, so the geometry getters and [`lerp`](Self::lerp) read it. A tween
/// is a plain value: [`Animatable::animate`] takes a copy, so a later
/// [`set_begin`](Self::set_begin) is not seen by an animation already driven.
///
/// See also:
///
///  * `Tween`, for a discussion on how to use interpolation objects.
///  * [`MaterialRectArcTween`], which extends this concept to interpolating [`Rect`]s.
#[derive(Clone, Debug)]
pub struct MaterialPointArcTween {
    begin: Option<Offset>,
    end: Option<Offset>,
    center: Option<Offset>,
    radius: Option<f64>,
    begin_angle: Option<f64>,
    end_angle: Option<f64>,
}

impl MaterialPointArcTween {
    /// Creates a tween for animating [`Offset`]s along a circular arc.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties must be
    /// non-null before the tween is first used, but the arguments can be null if
    /// the values are going to be filled in later.
    pub fn new(begin: Option<Offset>, end: Option<Offset>) -> MaterialPointArcTween {
        let mut tween = MaterialPointArcTween {
            begin,
            end,
            center: None,
            radius: None,
            begin_angle: None,
            end_angle: None,
        };
        tween.initialize();
        tween
    }

    fn initialize(&mut self) {
        let (Some(begin), Some(end)) = (self.begin, self.end) else {
            return;
        };

        // An explanation with a diagram can be found at https://docs.google.com/document/d/1kF7vhX_RpQCIjQYT6lZdLFs4Dl6hHxoXNkTQfzyHNlw/
        let delta = end - begin;
        let delta_x = delta.dx().abs();
        let delta_y = delta.dy().abs();
        let distance_from_a_to_b = delta.distance();
        let c = Offset::new(end.dx(), begin.dy());

        let sweep_angle = |radius: f64| 2.0 * (distance_from_a_to_b / (2.0 * radius)).asin();

        if delta_x > K_ON_AXIS_DELTA && delta_y > K_ON_AXIS_DELTA {
            if delta_x < delta_y {
                let radius =
                    distance_from_a_to_b * distance_from_a_to_b / (c - begin).distance() / 2.0;
                self.radius = Some(radius);
                self.center = Some(Offset::new(
                    end.dx() + radius * sign(begin.dx() - end.dx()),
                    end.dy(),
                ));
                if begin.dx() < end.dx() {
                    self.begin_angle = Some(sweep_angle(radius) * sign(begin.dy() - end.dy()));
                    self.end_angle = Some(0.0);
                } else {
                    self.begin_angle = Some(PI + sweep_angle(radius) * sign(end.dy() - begin.dy()));
                    self.end_angle = Some(PI);
                }
            } else {
                let radius =
                    distance_from_a_to_b * distance_from_a_to_b / (c - end).distance() / 2.0;
                self.radius = Some(radius);
                self.center = Some(Offset::new(
                    begin.dx(),
                    begin.dy() + sign(end.dy() - begin.dy()) * radius,
                ));
                if begin.dy() < end.dy() {
                    let begin_angle = -PI / 2.0;
                    self.begin_angle = Some(begin_angle);
                    self.end_angle =
                        Some(begin_angle + sweep_angle(radius) * sign(end.dx() - begin.dx()));
                } else {
                    let begin_angle = PI / 2.0;
                    self.begin_angle = Some(begin_angle);
                    self.end_angle =
                        Some(begin_angle + sweep_angle(radius) * sign(begin.dx() - end.dx()));
                }
            }
        } else {
            self.begin_angle = None;
            self.end_angle = None;
        }
    }

    /// The value this variable has at the beginning of the animation.
    pub fn begin(&self) -> Option<Offset> {
        self.begin
    }

    /// Dart's `begin` setter.
    pub fn set_begin(&mut self, value: Option<Offset>) {
        if value != self.begin {
            self.begin = value;
            self.initialize();
        }
    }

    /// The value this variable has at the end of the animation.
    pub fn end(&self) -> Option<Offset> {
        self.end
    }

    /// Dart's `end` setter.
    pub fn set_end(&mut self, value: Option<Offset>) {
        if value != self.end {
            self.end = value;
            self.initialize();
        }
    }

    /// The center of the circular arc, null if [`begin`](Self::begin) and
    /// [`end`](Self::end) are horizontally or vertically aligned, or if either is
    /// null.
    pub fn center(&self) -> Option<Offset> {
        if self.begin.is_none() || self.end.is_none() {
            return None;
        }
        self.center
    }

    /// The radius of the circular arc, null if [`begin`](Self::begin) and
    /// [`end`](Self::end) are horizontally or vertically aligned, or if either is
    /// null.
    pub fn radius(&self) -> Option<f64> {
        if self.begin.is_none() || self.end.is_none() {
            return None;
        }
        self.radius
    }

    /// The beginning of the arc's sweep in radians, measured from the positive x
    /// axis. Positive angles turn clockwise.
    ///
    /// This will be null if [`begin`](Self::begin) and [`end`](Self::end) are
    /// horizontally or vertically aligned, or if either is null.
    pub fn begin_angle(&self) -> Option<f64> {
        if self.begin.is_none() || self.end.is_none() {
            return None;
        }
        self.begin_angle
    }

    /// The end of the arc's sweep in radians, measured from the positive x axis.
    /// Positive angles turn clockwise.
    ///
    /// This will be null if [`begin`](Self::begin) and [`end`](Self::end) are
    /// horizontally or vertically aligned, or if either is null.
    pub fn end_angle(&self) -> Option<f64> {
        if self.begin.is_none() || self.end.is_none() {
            return None;
        }
        self.begin_angle
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Offset {
        let begin = self
            .begin
            .expect("MaterialPointArcTween.begin must be set before use");
        let end = self
            .end
            .expect("MaterialPointArcTween.end must be set before use");
        if t == 0.0 {
            return begin;
        }
        if t == 1.0 {
            return end;
        }
        let (Some(begin_angle), Some(end_angle)) = (self.begin_angle, self.end_angle) else {
            return Offset::lerp(Some(begin), Some(end), t).unwrap();
        };
        let radius = self.radius.unwrap();
        let angle = lerp_double(Some(begin_angle), Some(end_angle), t).unwrap();
        let x = angle.cos() * radius;
        let y = angle.sin() * radius;
        self.center.unwrap() + Offset::new(x, y)
    }
}

impl Animatable<Offset> for MaterialPointArcTween {
    fn transform(&self, _app: &App, t: f64) -> Offset {
        self.lerp(t)
    }
}

#[derive(Clone, Copy)]
enum CornerId {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy)]
struct Diagonal {
    begin_id: CornerId,
    end_id: CornerId,
}

const ALL_DIAGONALS: [Diagonal; 4] = [
    Diagonal {
        begin_id: CornerId::TopLeft,
        end_id: CornerId::BottomRight,
    },
    Diagonal {
        begin_id: CornerId::BottomRight,
        end_id: CornerId::TopLeft,
    },
    Diagonal {
        begin_id: CornerId::TopRight,
        end_id: CornerId::BottomLeft,
    },
    Diagonal {
        begin_id: CornerId::BottomLeft,
        end_id: CornerId::TopRight,
    },
];

// Select the element for which the key function returns the maximum value.
fn max_by<T: Copy>(input: &[T], key_func: impl Fn(T) -> f64) -> T {
    let mut max_value = None;
    let mut max_key: Option<f64> = None;
    for &value in input {
        let key = key_func(value);
        if max_key.is_none() || key > max_key.unwrap() {
            max_value = Some(value);
            max_key = Some(key);
        }
    }
    max_value.expect("max_by needs at least one element")
}

fn corner_for(rect: Rect, id: CornerId) -> Offset {
    match id {
        CornerId::TopLeft => rect.top_left(),
        CornerId::TopRight => rect.top_right(),
        CornerId::BottomLeft => rect.bottom_left(),
        CornerId::BottomRight => rect.bottom_right(),
    }
}

/// A tween that interpolates a [`Rect`] by having its opposite corners follow
/// circular arcs.
///
/// This class specializes the interpolation of `Tween<Rect>` so that instead of
/// growing or shrinking linearly, opposite corners of the rectangle follow arcs
/// in a manner consistent with Material Design principles.
///
/// Specifically, the rectangle corners whose diagonals are closest to the overall
/// direction of the animation follow arcs defined with [`MaterialPointArcTween`].
///
/// The arcs are worked out when [`begin`](Self::begin) and [`end`](Self::end) are
/// both set. A tween is a plain value, held as `Rc<dyn RectTweenObject>` where a
/// hero wants one.
///
/// See also:
///
///  * [`MaterialRectCenterArcTween`], which interpolates a rect along a circular
///    arc between the begin and end [`Rect`]'s centers.
///  * `Tween`, for a discussion on how to use interpolation objects.
///  * [`MaterialPointArcTween`], the analog for [`Offset`] interpolation.
///  * `RectTween`, which does a linear rectangle interpolation.
///  * `Hero::create_rect_tween`, which can be used to specify the tween that defines
///    a hero's path.
#[derive(Clone, Debug)]
pub struct MaterialRectArcTween {
    begin: Option<Rect>,
    end: Option<Rect>,
    begin_arc: Option<MaterialPointArcTween>,
    end_arc: Option<MaterialPointArcTween>,
}

impl MaterialRectArcTween {
    /// Creates a tween for animating [`Rect`]s along a circular arc.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties must be
    /// non-null before the tween is first used, but the arguments can be null if
    /// the values are going to be filled in later.
    pub fn new(begin: Option<Rect>, end: Option<Rect>) -> MaterialRectArcTween {
        let mut tween = MaterialRectArcTween {
            begin,
            end,
            begin_arc: None,
            end_arc: None,
        };
        tween.initialize();
        tween
    }

    fn initialize(&mut self) {
        let (Some(begin), Some(end)) = (self.begin, self.end) else {
            self.begin_arc = None;
            self.end_arc = None;
            return;
        };
        let centers_vector = end.center() - begin.center();
        let diagonal = max_by(&ALL_DIAGONALS, |d| {
            Self::diagonal_support(begin, centers_vector, d)
        });
        self.begin_arc = Some(MaterialPointArcTween::new(
            Some(corner_for(begin, diagonal.begin_id)),
            Some(corner_for(end, diagonal.begin_id)),
        ));
        self.end_arc = Some(MaterialPointArcTween::new(
            Some(corner_for(begin, diagonal.end_id)),
            Some(corner_for(end, diagonal.end_id)),
        ));
    }

    fn diagonal_support(begin: Rect, centers_vector: Offset, diagonal: Diagonal) -> f64 {
        let delta = corner_for(begin, diagonal.end_id) - corner_for(begin, diagonal.begin_id);
        let length = delta.distance();
        centers_vector.dx() * delta.dx() / length + centers_vector.dy() * delta.dy() / length
    }

    /// The value this variable has at the beginning of the animation.
    pub fn begin(&self) -> Option<Rect> {
        self.begin
    }

    /// Dart's `begin` setter.
    pub fn set_begin(&mut self, value: Option<Rect>) {
        if value != self.begin {
            self.begin = value;
            self.initialize();
        }
    }

    /// The value this variable has at the end of the animation.
    pub fn end(&self) -> Option<Rect> {
        self.end
    }

    /// Dart's `end` setter.
    pub fn set_end(&mut self, value: Option<Rect>) {
        if value != self.end {
            self.end = value;
            self.initialize();
        }
    }

    /// The path of the corresponding [`begin`](Self::begin), [`end`](Self::end)
    /// rectangle corners that lead the animation.
    pub fn begin_arc(&self) -> Option<&MaterialPointArcTween> {
        self.begin?;
        Some(
            self.begin_arc
                .as_ref()
                .expect("MaterialRectArcTween.end must be set before use"),
        )
    }

    /// The path of the corresponding [`begin`](Self::begin), [`end`](Self::end)
    /// rectangle corners that trail the animation.
    pub fn end_arc(&self) -> Option<&MaterialPointArcTween> {
        self.end?;
        Some(
            self.end_arc
                .as_ref()
                .expect("MaterialRectArcTween.begin must be set before use"),
        )
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Rect {
        let begin = self
            .begin
            .expect("MaterialRectArcTween.begin must be set before use");
        let end = self
            .end
            .expect("MaterialRectArcTween.end must be set before use");
        if t == 0.0 {
            return begin;
        }
        if t == 1.0 {
            return end;
        }
        Rect::from_points(
            self.begin_arc.as_ref().unwrap().lerp(t),
            self.end_arc.as_ref().unwrap().lerp(t),
        )
    }
}

impl RectTweenObject for MaterialRectArcTween {
    fn begin(&self, _app: &App) -> Option<Rect> {
        self.begin
    }

    fn end(&self, _app: &App) -> Option<Rect> {
        self.end
    }

    fn lerp(&self, _app: &App, t: f64) -> Option<Rect> {
        Some(MaterialRectArcTween::lerp(self, t))
    }
}

impl Animatable<Option<Rect>> for MaterialRectArcTween {
    fn transform(&self, app: &App, t: f64) -> Option<Rect> {
        RectTweenObject::lerp(self, app, t)
    }
}

/// A tween that interpolates a [`Rect`] by moving it along a circular arc from
/// [`begin`](Self::begin)'s [`Rect::center`] to [`end`](Self::end)'s
/// [`Rect::center`] while interpolating the rectangle's width and height.
///
/// The arc that defines that center of the interpolated rectangle as it morphs
/// from [`begin`](Self::begin) to [`end`](Self::end) is a [`MaterialPointArcTween`].
///
/// See also:
///
///  * [`MaterialRectArcTween`], A tween that interpolates a [`Rect`] by having
///    its opposite corners follow circular arcs.
///  * `Tween`, for a discussion on how to use interpolation objects.
///  * [`MaterialPointArcTween`], the analog for [`Offset`] interpolation.
///  * `RectTween`, which does a linear rectangle interpolation.
///  * `Hero::create_rect_tween`, which can be used to specify the tween that defines
///    a hero's path.
#[derive(Clone, Debug)]
pub struct MaterialRectCenterArcTween {
    begin: Option<Rect>,
    end: Option<Rect>,
    center_arc: Option<MaterialPointArcTween>,
}

impl MaterialRectCenterArcTween {
    /// Creates a tween for animating [`Rect`]s along a circular arc.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties must be
    /// non-null before the tween is first used, but the arguments can be null if
    /// the values are going to be filled in later.
    pub fn new(begin: Option<Rect>, end: Option<Rect>) -> MaterialRectCenterArcTween {
        let mut tween = MaterialRectCenterArcTween {
            begin,
            end,
            center_arc: None,
        };
        tween.initialize();
        tween
    }

    fn initialize(&mut self) {
        let (Some(begin), Some(end)) = (self.begin, self.end) else {
            self.center_arc = None;
            return;
        };
        self.center_arc = Some(MaterialPointArcTween::new(
            Some(begin.center()),
            Some(end.center()),
        ));
    }

    /// The value this variable has at the beginning of the animation.
    pub fn begin(&self) -> Option<Rect> {
        self.begin
    }

    /// Dart's `begin` setter.
    pub fn set_begin(&mut self, value: Option<Rect>) {
        if value != self.begin {
            self.begin = value;
            self.initialize();
        }
    }

    /// The value this variable has at the end of the animation.
    pub fn end(&self) -> Option<Rect> {
        self.end
    }

    /// Dart's `end` setter.
    pub fn set_end(&mut self, value: Option<Rect>) {
        if value != self.end {
            self.end = value;
            self.initialize();
        }
    }

    /// If [`begin`](Self::begin) and [`end`](Self::end) are non-null, returns a
    /// tween that interpolates along a circular arc between
    /// [`begin`](Self::begin)'s [`Rect::center`] and [`end`](Self::end)'s
    /// [`Rect::center`].
    pub fn center_arc(&self) -> Option<&MaterialPointArcTween> {
        if self.begin.is_none() || self.end.is_none() {
            return None;
        }
        self.center_arc.as_ref()
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Rect {
        let begin = self
            .begin
            .expect("MaterialRectCenterArcTween.begin must be set before use");
        let end = self
            .end
            .expect("MaterialRectCenterArcTween.end must be set before use");
        if t == 0.0 {
            return begin;
        }
        if t == 1.0 {
            return end;
        }
        let center = self.center_arc.as_ref().unwrap().lerp(t);
        let width = lerp_double(Some(begin.width()), Some(end.width()), t).unwrap();
        let height = lerp_double(Some(begin.height()), Some(end.height()), t).unwrap();
        Rect::from_ltwh(
            center.dx() - width / 2.0,
            center.dy() - height / 2.0,
            width,
            height,
        )
    }
}

impl RectTweenObject for MaterialRectCenterArcTween {
    fn begin(&self, _app: &App) -> Option<Rect> {
        self.begin
    }

    fn end(&self, _app: &App) -> Option<Rect> {
        self.end
    }

    fn lerp(&self, _app: &App, t: f64) -> Option<Rect> {
        Some(MaterialRectCenterArcTween::lerp(self, t))
    }
}

impl Animatable<Option<Rect>> for MaterialRectCenterArcTween {
    fn transform(&self, app: &App, t: f64) -> Option<Rect> {
        RectTweenObject::lerp(self, app, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_offset_within(actual: Offset, from: Offset, distance: f64) {
        assert!(
            (actual - from).distance() <= distance,
            "{actual:?} is not within {distance} of {from:?}"
        );
    }

    fn same_rect(a: Rect, b: Rect) -> bool {
        (a.left - b.left).abs() < 2.0
            && (a.top - b.top).abs() < 2.0
            && (a.right - b.right).abs() < 2.0
            && (a.bottom - b.bottom).abs() < 2.0
    }

    #[test]
    fn point_arc_tween_gives_equal_geometry_for_equal_ends() {
        let a = MaterialPointArcTween::new(Some(Offset::ZERO), Some(Offset::new(0.0, 10.0)));
        let b = MaterialPointArcTween::new(Some(Offset::ZERO), Some(Offset::new(0.0, 10.0)));
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn rect_arc_tween_gives_equal_geometry_for_equal_ends() {
        let make = || {
            MaterialRectArcTween::new(
                Some(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0)),
                Some(Rect::from_ltwh(0.0, 10.0, 10.0, 10.0)),
            )
        };
        assert_eq!(format!("{:?}", make()), format!("{:?}", make()));
    }

    #[test]
    fn on_axis_point_arc_tween_moves_in_a_straight_line() {
        let tween = MaterialPointArcTween::new(Some(Offset::ZERO), Some(Offset::new(0.0, 10.0)));
        assert_eq!(tween.lerp(0.5), Offset::new(0.0, 5.0));

        let tween = MaterialPointArcTween::new(Some(Offset::ZERO), Some(Offset::new(10.0, 0.0)));
        assert_eq!(tween.lerp(0.5), Offset::new(5.0, 0.0));
    }

    #[test]
    fn on_axis_rect_arc_tween_moves_in_a_straight_line() {
        let tween = MaterialRectArcTween::new(
            Some(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0)),
            Some(Rect::from_ltwh(0.0, 10.0, 10.0, 10.0)),
        );
        assert_eq!(tween.lerp(0.5), Rect::from_ltwh(0.0, 5.0, 10.0, 10.0));

        let tween = MaterialRectArcTween::new(
            Some(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0)),
            Some(Rect::from_ltwh(10.0, 0.0, 10.0, 10.0)),
        );
        assert_eq!(tween.lerp(0.5), Rect::from_ltwh(5.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn point_arc_tween_follows_a_circular_arc() {
        let begin = Offset::new(180.0, 110.0);
        let end = Offset::new(37.0, 250.0);

        let tween = MaterialPointArcTween::new(Some(begin), Some(end));
        assert_eq!(tween.lerp(0.0), begin);
        assert_offset_within(tween.lerp(0.25), Offset::new(126.0, 120.0), 2.0);
        assert_offset_within(tween.lerp(0.75), Offset::new(48.0, 196.0), 2.0);
        assert_eq!(tween.lerp(1.0), end);

        let tween = MaterialPointArcTween::new(Some(end), Some(begin));
        assert_eq!(tween.lerp(0.0), end);
        assert_offset_within(tween.lerp(0.25), Offset::new(91.0, 239.0), 2.0);
        assert_offset_within(tween.lerp(0.75), Offset::new(168.3, 163.8), 2.0);
        assert_eq!(tween.lerp(1.0), begin);
    }

    #[test]
    fn rect_arc_tween_moves_opposite_corners_along_arcs() {
        let begin = Rect::from_ltrb(180.0, 100.0, 330.0, 200.0);
        let end = Rect::from_ltrb(32.0, 275.0, 132.0, 425.0);

        let tween = MaterialRectArcTween::new(Some(begin), Some(end));
        assert_eq!(tween.lerp(0.0), begin);
        assert!(same_rect(
            tween.lerp(0.25),
            Rect::from_ltrb(120.0, 113.0, 259.0, 237.0)
        ));
        assert!(same_rect(
            tween.lerp(0.75),
            Rect::from_ltrb(42.3, 206.5, 153.5, 354.7)
        ));
        assert_eq!(tween.lerp(1.0), end);

        let tween = MaterialRectArcTween::new(Some(end), Some(begin));
        assert_eq!(tween.lerp(0.0), end);
        assert!(same_rect(
            tween.lerp(0.25),
            Rect::from_ltrb(92.0, 262.0, 203.0, 388.0)
        ));
        assert!(same_rect(
            tween.lerp(0.75),
            Rect::from_ltrb(169.7, 168.5, 308.5, 270.3)
        ));
        assert_eq!(tween.lerp(1.0), begin);
    }

    #[test]
    #[should_panic(expected = "MaterialRectArcTween.end must be set before use")]
    fn rect_arc_tween_begin_arc_fails_once_its_end_is_cleared() {
        let mut tween = MaterialRectArcTween::new(
            Some(Rect::from_ltrb(180.0, 100.0, 330.0, 200.0)),
            Some(Rect::from_ltrb(32.0, 275.0, 132.0, 425.0)),
        );
        assert!(tween.begin_arc().is_some());
        tween.set_end(None);
        tween.begin_arc();
    }
}
