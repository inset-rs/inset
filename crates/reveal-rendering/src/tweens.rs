//! Flutter counterpart: `rendering/tweens.dart`.

use reveal_animation::Animatable;
use reveal_foundation::App;
use reveal_painting::{Alignment, AlignmentGeometry, FractionalOffset};

/// An interpolation between two fractional offsets.
///
/// This class specializes the interpolation of `Tween<FractionalOffset>` to be
/// appropriate for fractional offsets.
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Unlike the tweens in `reveal_animation`, this one is a value rather than an arena object:
/// the orphan rule leaves no local type in `impl Animatable<Option<FractionalOffset>> for
/// Handle<FractionalOffsetTween>`. A driven animation therefore holds a clone, and writing
/// [`begin`](Self::begin) or [`end`](Self::end) afterwards does not reach it.
///
/// See also:
///
///  * [`AlignmentTween`], which interpolates between two [`Alignment`] objects.
#[derive(Clone, Debug)]
pub struct FractionalOffsetTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<FractionalOffset>,

    /// The value this variable has at the end of the animation.
    pub end: Option<FractionalOffset>,
}

impl FractionalOffsetTween {
    /// Creates a fractional offset tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the
    /// `None` value is treated as meaning the center.
    pub fn new(
        begin: Option<FractionalOffset>,
        end: Option<FractionalOffset>,
    ) -> FractionalOffsetTween {
        FractionalOffsetTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Option<FractionalOffset> {
        FractionalOffset::lerp(self.begin, self.end, t)
    }
}

impl Animatable<Option<FractionalOffset>> for FractionalOffsetTween {
    fn transform(&self, _app: &App, t: f64) -> Option<FractionalOffset> {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        self.lerp(t)
    }
}

/// An interpolation between two alignments.
///
/// This class specializes the interpolation of `Tween<Alignment>` to be
/// appropriate for alignments.
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Like [`FractionalOffsetTween`], this one is a value rather than an arena object (see
/// PORTING.md).
///
/// See also:
///
///  * [`AlignmentGeometryTween`], which interpolates between two
///    [`AlignmentGeometry`] objects.
#[derive(Clone, Debug)]
pub struct AlignmentTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Alignment>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Alignment>,
}

impl AlignmentTween {
    /// Creates a fractional offset tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the
    /// `None` value is treated as meaning the center.
    pub fn new(begin: Option<Alignment>, end: Option<Alignment>) -> AlignmentTween {
        AlignmentTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Alignment {
        Alignment::lerp(self.begin, self.end, t)
            .expect("AlignmentTween.begin or end must be set before use")
    }
}

impl Animatable<Alignment> for AlignmentTween {
    fn transform(&self, _app: &App, t: f64) -> Alignment {
        if t == 0.0 {
            return self.begin.expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

/// An interpolation between two [`AlignmentGeometry`].
///
/// This class specializes the interpolation of `Tween<AlignmentGeometry>` to be
/// appropriate for alignments.
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Like [`FractionalOffsetTween`], this one is a value rather than an arena object (see
/// PORTING.md).
///
/// See also:
///
///  * [`AlignmentTween`], which interpolates between two [`Alignment`] objects.
#[derive(Clone, Debug)]
pub struct AlignmentGeometryTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<AlignmentGeometry>,

    /// The value this variable has at the end of the animation.
    pub end: Option<AlignmentGeometry>,
}

impl AlignmentGeometryTween {
    /// Creates a fractional offset geometry tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the
    /// `None` value is treated as meaning the center.
    pub fn new(
        begin: Option<AlignmentGeometry>,
        end: Option<AlignmentGeometry>,
    ) -> AlignmentGeometryTween {
        AlignmentGeometryTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Option<AlignmentGeometry> {
        AlignmentGeometry::lerp(self.begin, self.end, t)
    }
}

impl Animatable<Option<AlignmentGeometry>> for AlignmentGeometryTween {
    fn transform(&self, _app: &App, t: f64) -> Option<AlignmentGeometry> {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        self.lerp(t)
    }
}

#[cfg(test)]
mod tests {
    use reveal_painting::{AlignmentDirectional, TextDirection};

    use super::*;

    #[test]
    fn an_alignment_tween_walks_between_the_corners() {
        let app = App::new();
        let tween = AlignmentTween::new(Some(Alignment::TOP_LEFT), Some(Alignment::BOTTOM_LEFT));
        assert_eq!(tween.transform(&app, 0.0), Alignment::TOP_LEFT);
        assert_eq!(tween.transform(&app, 0.5), Alignment::CENTER_LEFT);
        assert_eq!(tween.transform(&app, 1.0), Alignment::BOTTOM_LEFT);
    }

    #[test]
    fn an_alignment_geometry_tween_crosses_the_two_kinds() {
        let app = App::new();
        let tween = AlignmentGeometryTween::new(
            Some(Alignment::CENTER_RIGHT.into()),
            Some(AlignmentDirectional::CENTER_END.into()),
        );
        let halfway = tween.transform(&app, 0.5).expect("both ends are set");
        assert_eq!(
            halfway.resolve(Some(TextDirection::Ltr)),
            Alignment::CENTER_RIGHT
        );
        assert_eq!(halfway.resolve(Some(TextDirection::Rtl)), Alignment::CENTER);
    }

    #[test]
    fn a_fractional_offset_tween_walks_between_the_corners() {
        let app = App::new();
        let tween = FractionalOffsetTween::new(
            Some(FractionalOffset::new(0.0, 0.0)),
            Some(FractionalOffset::new(1.0, 1.0)),
        );
        assert_eq!(
            tween.transform(&app, 0.25),
            Some(FractionalOffset::new(0.25, 0.25))
        );
    }
}
