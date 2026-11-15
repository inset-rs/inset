//! Flutter counterpart: `painting/gradient.dart`.
//!
//! A gradient is a value: [`Gradient`] is Dart's abstract class as a trait, held as
//! `Box<dyn Gradient>` the way [`ShapeBorder`](crate::ShapeBorder) is, and its transform is a
//! shared [`GradientTransformRef`] so that every copy a gradient makes of itself (`scale`,
//! `with_opacity`, `lerp`) holds the same transform object, as Dart's copies do.
//!
//! `create_shader` builds Valo's [`Shader`] through the dart:ui `Gradient.linear`, `.radial` and
//! `.sweep` constructors at the bottom of this file; what Valo's shader has no room for is said
//! there.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use waft_embedder::valo::{FocalCircle, GradientStop, Shader, SpreadMode, TileMode};
use waft_embedder::{Color, Matrix4, Offset, Rect, lerp_double};

use crate::alignment::AlignmentGeometry;
use crate::basic_types::TextDirection;

struct ColorsAndStops {
    colors: Vec<Color>,
    stops: Vec<f64>,
}

/// Calculate the color at position `t` of the gradient defined by `colors` and `stops`.
fn sample(colors: &[Color], stops: &[f64], t: f64) -> Color {
    debug_assert!(!colors.is_empty());
    debug_assert!(!stops.is_empty());
    if t <= stops[0] {
        return colors[0];
    }
    if t >= stops[stops.len() - 1] {
        return colors[colors.len() - 1];
    }
    let index = stops.iter().rposition(|&s| s <= t);
    debug_assert!(index.is_some());
    let index = index.unwrap();
    Color::lerp(
        Some(colors[index]),
        Some(colors[index + 1]),
        (t - stops[index]) / (stops[index + 1] - stops[index]),
    )
    .unwrap()
}

fn interpolate_colors_and_stops(
    a_colors: &[Color],
    a_stops: &[f64],
    b_colors: &[Color],
    b_stops: &[f64],
    t: f64,
) -> ColorsAndStops {
    debug_assert!(a_colors.len() >= 2);
    debug_assert!(b_colors.len() >= 2);
    debug_assert!(a_stops.len() == a_colors.len());
    debug_assert!(b_stops.len() == b_colors.len());
    // Dart's `SplayTreeSet<double>`: sorted by `compareTo`, which `total_cmp` matches, duplicates
    // dropped.
    let mut interpolated_stops: Vec<f64> = a_stops.iter().chain(b_stops).copied().collect();
    interpolated_stops.sort_by(f64::total_cmp);
    interpolated_stops.dedup_by(|a, b| a.total_cmp(b).is_eq());
    let interpolated_colors = interpolated_stops
        .iter()
        .map(|&stop| {
            Color::lerp(
                Some(sample(a_colors, a_stops, stop)),
                Some(sample(b_colors, b_stops, stop)),
                t,
            )
            .unwrap()
        })
        .collect();
    ColorsAndStops {
        colors: interpolated_colors,
        stops: interpolated_stops,
    }
}

/// A shared, immutable [`GradientTransform`] (Dart's `GradientTransform` reference).
pub type GradientTransformRef = Rc<dyn GradientTransform>;

/// Base class for transforming gradient shaders without applying the same
/// transform to the entire canvas.
///
/// For example, a [`SweepGradient`] normally starts its gradation at 3 o'clock
/// and draws clockwise. To have the sweep appear to start at 6 o'clock, supply
/// a [`GradientRotation`] of `pi/4` radians (i.e. 45 degrees).
pub trait GradientTransform: Any + Debug {
    /// When a [`Gradient`] creates its [`Shader`], it will call this method to
    /// determine what transform to apply to the shader for the given [`Rect`] and
    /// [`TextDirection`].
    ///
    /// Implementers may return None from this method, which achieves the same
    /// final effect as returning [`Matrix4::IDENTITY`].
    fn transform(&self, bounds: Rect, text_direction: Option<TextDirection>) -> Option<Matrix4>;

    /// Downcast support for `is` / `as` in Dart.
    fn as_any(&self) -> &dyn Any;

    /// Field equality. Dart's default `==` is identity; subclasses that override
    /// `==` implement this.
    fn eq_transform(&self, other: &dyn GradientTransform) -> bool {
        std::ptr::addr_eq(self.as_any(), other.as_any())
    }
}

impl PartialEq for dyn GradientTransform {
    fn eq(&self, other: &dyn GradientTransform) -> bool {
        self.eq_transform(other)
    }
}

/// A [`GradientTransform`] that rotates the gradient around the center-point of
/// its bounding box.
///
/// This sample would rotate a sweep gradient by a quarter turn clockwise:
///
/// ```ignore
/// SweepGradient::new(vec![Color::new(0xFFFFFFFF), Color::new(0xFF009900)])
///     .transform(GradientRotation::new(PI / 4.0));
/// ```
#[derive(Clone, Copy, PartialEq)]
pub struct GradientRotation {
    /// The angle of rotation in radians in the clockwise direction.
    pub radians: f64,
}

impl GradientRotation {
    /// Constructs a [`GradientRotation`] for the specified angle.
    ///
    /// The angle is in radians in the clockwise direction.
    pub const fn new(radians: f64) -> GradientRotation {
        GradientRotation { radians }
    }
}

impl GradientTransform for GradientRotation {
    fn transform(&self, bounds: Rect, _text_direction: Option<TextDirection>) -> Option<Matrix4> {
        let sin_radians = self.radians.sin();
        let one_minus_cos_radians = 1.0 - self.radians.cos();
        let center = bounds.center();
        let origin_x = sin_radians * center.dy() + one_minus_cos_radians * center.dx();
        let origin_y = -sin_radians * center.dx() + one_minus_cos_radians * center.dy();

        Some(
            Matrix4::translation(origin_x as f32, origin_y as f32)
                .then(&Matrix4::rotation(self.radians as f32)),
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_transform(&self, other: &dyn GradientTransform) -> bool {
        other
            .as_any()
            .downcast_ref::<GradientRotation>()
            .is_some_and(|other| other == self)
    }
}

impl From<GradientRotation> for GradientTransformRef {
    fn from(rotation: GradientRotation) -> GradientTransformRef {
        Rc::new(rotation)
    }
}

impl Debug for GradientRotation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "GradientRotation(radians: {:.1})", self.radians)
    }
}

/// A 2D gradient.
///
/// This is an interface that allows [`LinearGradient`], [`RadialGradient`], and
/// [`SweepGradient`] classes to be used interchangeably in `BoxDecoration`s.
pub trait Gradient: Any + Debug {
    /// The colors the gradient should obtain at each of the stops.
    ///
    /// If [`stops`](Self::stops) is non-null, this list must have the same length as
    /// [`stops`](Self::stops).
    ///
    /// This list must have at least two colors in it (otherwise, it's not a
    /// gradient!).
    fn colors(&self) -> &[Color];

    /// A list of values from 0.0 to 1.0 that denote fractions along the gradient.
    ///
    /// If non-null, this list must have the same length as [`colors`](Self::colors).
    ///
    /// If the first value is not 0.0, then a stop with position 0.0 and a color
    /// equal to the first color in [`colors`](Self::colors) is implied.
    ///
    /// If the last value is not 1.0, then a stop with position 1.0 and a color
    /// equal to the last color in [`colors`](Self::colors) is implied.
    ///
    /// The values in the [`stops`](Self::stops) list must be in ascending order. If a value in
    /// the [`stops`](Self::stops) list is less than an earlier value in the list, then its value
    /// is assumed to equal the previous value.
    ///
    /// If stops is null, then a set of uniformly distributed stops is implied,
    /// with the first stop at 0.0 and the last stop at 1.0.
    fn stops(&self) -> Option<&[f64]>;

    /// The transform, if any, to apply to the gradient.
    ///
    /// This transform is in addition to any other transformations applied to the
    /// canvas, but does not add any transformations to the canvas.
    fn transform(&self) -> Option<&GradientTransformRef>;

    /// Creates a [`Shader`] for this gradient to fill the given rect.
    ///
    /// If the gradient's configuration is text-direction-dependent, for example
    /// it uses `AlignmentDirectional` objects instead of `Alignment`
    /// objects, then the `text_direction` argument must not be null.
    ///
    /// The shader's transform will be resolved from the [`transform`](Self::transform) of this
    /// gradient.
    fn create_shader(&self, rect: Rect, text_direction: Option<TextDirection>) -> Shader;

    /// Returns a new gradient with its properties scaled by the given factor.
    ///
    /// A factor of 0.0 (or less) should result in a variant of the gradient that
    /// is invisible; any two factors epsilon apart should be unnoticeably
    /// different from each other at first glance. From this it follows that
    /// scaling a gradient with values from 1.0 to 0.0 over time should cause the
    /// gradient to smoothly disappear.
    ///
    /// Typically this is the same as interpolating from null (with [`lerp`](<dyn Gradient>::lerp)).
    fn scale(&self, factor: f64) -> Box<dyn Gradient>;

    /// Returns a new [`Gradient`] with each color set to the given opacity.
    fn with_opacity(&self, opacity: f64) -> Box<dyn Gradient>;

    /// Returns a copy of this gradient with all of its [`colors`](Self::colors) replaced by the
    /// given `color`.
    ///
    /// The geometry of the gradient (such as its [`stops`](Self::stops) and the
    /// subclass-specific positioning) is preserved, so the result paints as a
    /// uniform `color`. This is useful to represent a solid color as a gradient,
    /// for example when interpolating between a color and a gradient with
    /// [`lerp`](<dyn Gradient>::lerp).
    ///
    /// Subclasses should override this method to preserve their own geometry. The
    /// base implementation returns a [`LinearGradient`]; this is sufficient because
    /// a gradient with uniform colors paints as a solid color regardless of its
    /// geometry.
    #[allow(clippy::wrong_self_convention)]
    fn from_color(&self, color: Color) -> Box<dyn Gradient> {
        Box::new(LinearGradient {
            stops: self.stops().map(<[f64]>::to_vec),
            transform: self.transform().cloned(),
            ..LinearGradient::new(vec![color; self.colors().len()])
        })
    }

    /// Linearly interpolates from another [`Gradient`] to `this`.
    ///
    /// When implementing this method in subclasses, return None if this class
    /// cannot interpolate from `a`. In that case, [`lerp`](<dyn Gradient>::lerp) will try `a`'s
    /// [`lerp_to`](Self::lerp_to) method instead.
    ///
    /// If `a` is None, this must not return None. The base class implements this
    /// by deferring to [`scale`](Self::scale).
    ///
    /// Instead of calling this directly, use [`lerp`](<dyn Gradient>::lerp).
    fn lerp_from(&self, a: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        if a.is_none() {
            return Some(self.scale(t));
        }
        None
    }

    /// Linearly interpolates from `this` to another [`Gradient`].
    ///
    /// This is called if `b`'s [`lerp_to`](Self::lerp_to) did not know how to handle this class.
    ///
    /// When implementing this method in subclasses, return None if this class
    /// cannot interpolate from `b`. In that case, [`lerp`](<dyn Gradient>::lerp) will apply a
    /// default behavior instead.
    ///
    /// If `b` is None, this must not return None. The base class implements this
    /// by deferring to [`scale`](Self::scale).
    ///
    /// Instead of calling this directly, use [`lerp`](<dyn Gradient>::lerp).
    fn lerp_to(&self, b: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        if b.is_none() {
            return Some(self.scale(1.0 - t));
        }
        None
    }

    /// A heap clone, so decorations and lerp can duplicate like Dart.
    fn clone_box(&self) -> Box<dyn Gradient>;

    /// Downcast support for `is` / `as` in Dart.
    fn as_any(&self) -> &dyn Any;

    /// Field equality. Dart's default `==` is identity; subclasses that override
    /// `==` implement this.
    fn eq_gradient(&self, other: &dyn Gradient) -> bool {
        std::ptr::addr_eq(self.as_any(), other.as_any())
    }
}

impl dyn Gradient {
    fn implied_stops(&self) -> Vec<f64> {
        if let Some(stops) = self.stops() {
            return stops.to_vec();
        }
        let colors = self.colors();
        debug_assert!(
            colors.len() >= 2,
            "colors list must have at least two colors"
        );
        let separation = 1.0 / (colors.len() as f64 - 1.0);
        (0..colors.len())
            .map(|index| index as f64 * separation)
            .collect()
    }

    /// Linearly interpolates between two [`Gradient`]s.
    ///
    /// This defers to `b`'s [`lerp_to`](Gradient::lerp_to) function if `b` is not None. If `b` is
    /// None or if its [`lerp_to`](Gradient::lerp_to) returns None, it uses `a`'s
    /// [`lerp_from`](Gradient::lerp_from) function instead. If both return None, it returns `a`
    /// before `t == 0.5` and `b` after `t == 0.5`.
    pub fn lerp(
        a: Option<&dyn Gradient>,
        b: Option<&dyn Gradient>,
        t: f64,
    ) -> Option<Box<dyn Gradient>> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::addr_eq(a, b) => return Some(a.clone_box()),
            _ => {}
        }
        let mut result = None;
        if let Some(b) = b {
            result = b.lerp_from(a, t); // if a is None, this must return Some
        }
        if result.is_none()
            && let Some(a) = a
        {
            result = a.lerp_to(b, t); // if b is None, this must return Some
        }
        if result.is_some() {
            return result;
        }
        debug_assert!(a.is_some() && b.is_some());
        Some(if t < 0.5 {
            a.unwrap().scale(1.0 - (t * 2.0))
        } else {
            b.unwrap().scale((t - 0.5) * 2.0)
        })
    }

    fn resolve_transform(
        &self,
        bounds: Rect,
        text_direction: Option<TextDirection>,
    ) -> Option<Matrix4> {
        self.transform()
            .and_then(|transform| transform.transform(bounds, text_direction))
    }
}

impl<T: Gradient> From<T> for Box<dyn Gradient> {
    fn from(gradient: T) -> Box<dyn Gradient> {
        Box::new(gradient)
    }
}

impl Clone for Box<dyn Gradient> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for dyn Gradient {
    fn eq(&self, other: &dyn Gradient) -> bool {
        self.eq_gradient(other)
    }
}

fn boxed<T: Gradient>(gradient: Option<T>) -> Option<Box<dyn Gradient>> {
    gradient.map(|gradient| Box::new(gradient) as Box<dyn Gradient>)
}

/// Dart's `TileMode.toString()`.
fn tile_mode_name(tile_mode: TileMode) -> &'static str {
    match tile_mode {
        TileMode::Clamp => "TileMode.clamp",
        TileMode::Repeat => "TileMode.repeated",
        TileMode::Mirror => "TileMode.mirror",
        TileMode::Decal => "TileMode.decal",
    }
}

/// A 2D linear gradient.
///
/// This class is used by `BoxDecoration` to represent linear gradients. This
/// abstracts out the arguments to the dart:ui `Gradient.linear` constructor.
///
/// A gradient has two anchor points, [`begin`](Self::begin) and [`end`](Self::end). The
/// [`begin`](Self::begin) point corresponds to 0.0, and the [`end`](Self::end) point corresponds
/// to 1.0. These points are expressed in fractions, so that the same gradient can be reused with
/// varying sized boxes without changing the parameters. (This contrasts with dart:ui
/// `Gradient.linear`, whose arguments are expressed in logical pixels.)
///
/// The [`colors`](Self::colors) are described by a list of [`Color`] objects. There must be at
/// least two colors. The [`stops`](Self::stops) list, if specified, must have the same length
/// as [`colors`](Self::colors). It specifies fractions of the vector from start to end, between
/// 0.0 and 1.0, for each color. If it is null, a uniform distribution is
/// assumed.
///
/// The region of the canvas before [`begin`](Self::begin) and after [`end`](Self::end) is
/// colored according to [`tile_mode`](Self::tile_mode).
///
/// Typically this class is used with `BoxDecoration`, which does the painting.
/// To use a [`LinearGradient`] to paint on a canvas directly, see
/// [`create_shader`](Gradient::create_shader).
#[derive(Clone, PartialEq)]
pub struct LinearGradient {
    /// The offset at which stop 0.0 of the gradient is placed.
    ///
    /// If this is an `Alignment`, then it is expressed as a vector from
    /// coordinate (0.0, 0.0), in a coordinate space that maps the center of the
    /// paint box at (0.0, 0.0) and the bottom right at (1.0, 1.0).
    ///
    /// It can also be an `AlignmentDirectional`, where the start is the
    /// left in left-to-right contexts and the right in right-to-left contexts. If
    /// a text-direction-dependent value is provided here, then the
    /// [`create_shader`](Gradient::create_shader) method will need to be given a
    /// [`TextDirection`].
    pub begin: AlignmentGeometry,
    /// The offset at which stop 1.0 of the gradient is placed.
    ///
    /// See [`begin`](Self::begin) for how the coordinates are mapped.
    pub end: AlignmentGeometry,
    /// The colors the gradient should obtain at each of the stops.
    pub colors: Vec<Color>,
    /// A list of values from 0.0 to 1.0 that denote fractions along the gradient.
    pub stops: Option<Vec<f64>>,
    /// How this gradient should tile the plane beyond in the region before
    /// [`begin`](Self::begin) and after [`end`](Self::end).
    pub tile_mode: TileMode,
    /// The transform, if any, to apply to the gradient.
    pub transform: Option<GradientTransformRef>,
}

impl LinearGradient {
    /// Creates a linear gradient.
    ///
    /// If [`stops`](Self::stops) is non-null, it must have the same length as
    /// [`colors`](Self::colors).
    pub fn new(colors: Vec<Color>) -> LinearGradient {
        LinearGradient {
            begin: AlignmentGeometry::CENTER_LEFT,
            end: AlignmentGeometry::CENTER_RIGHT,
            colors,
            stops: None,
            tile_mode: TileMode::Clamp,
            transform: None,
        }
    }

    /// Dart `LinearGradient(begin:)`.
    pub fn begin(mut self, begin: impl Into<AlignmentGeometry>) -> LinearGradient {
        self.begin = begin.into();
        self
    }

    /// Dart `LinearGradient(end:)`.
    pub fn end(mut self, end: impl Into<AlignmentGeometry>) -> LinearGradient {
        self.end = end.into();
        self
    }

    /// Dart `LinearGradient(stops:)`.
    pub fn stops(mut self, stops: Vec<f64>) -> LinearGradient {
        self.stops = Some(stops);
        self
    }

    /// Dart `LinearGradient(tileMode:)`.
    pub fn tile_mode(mut self, tile_mode: TileMode) -> LinearGradient {
        self.tile_mode = tile_mode;
        self
    }

    /// Dart `LinearGradient(transform:)`.
    pub fn transform(mut self, transform: impl Into<GradientTransformRef>) -> LinearGradient {
        self.transform = Some(transform.into());
        self
    }

    /// Returns a new [`LinearGradient`] with its colors scaled by the given factor.
    ///
    /// Since the alpha component of the Color is what is scaled, a factor
    /// of 0.0 or less results in a gradient that is fully transparent.
    pub fn scale(&self, factor: f64) -> LinearGradient {
        LinearGradient {
            colors: self
                .colors
                .iter()
                .map(|&color| Color::lerp(None, Some(color), factor).unwrap())
                .collect(),
            ..self.clone()
        }
    }

    /// Dart's `LinearGradient.fromColor`: the geometry kept, every color replaced by `color`.
    pub fn from_color(&self, color: Color) -> LinearGradient {
        LinearGradient {
            colors: vec![color; self.colors.len()],
            ..self.clone()
        }
    }

    /// Linearly interpolate between two [`LinearGradient`]s.
    ///
    /// If either gradient is None, this function linearly interpolates from
    /// a gradient that matches the other gradient in [`begin`](Self::begin),
    /// [`end`](Self::end), [`stops`](Self::stops) and [`tile_mode`](Self::tile_mode) and with the
    /// same [`colors`](Self::colors) but transparent (using [`scale`](Self::scale)).
    ///
    /// If neither gradient is None, they must have the same number of [`colors`](Self::colors).
    pub fn lerp(
        a: Option<&LinearGradient>,
        b: Option<&LinearGradient>,
        t: f64,
    ) -> Option<LinearGradient> {
        let (a, b) = match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            (None, Some(b)) => return Some(b.scale(t)),
            (Some(a), None) => return Some(a.scale(1.0 - t)),
            (Some(a), Some(b)) => (a, b),
        };
        let interpolated = interpolate_colors_and_stops(
            &a.colors,
            &<dyn Gradient>::implied_stops(a),
            &b.colors,
            &<dyn Gradient>::implied_stops(b),
            t,
        );
        Some(LinearGradient {
            begin: AlignmentGeometry::lerp(Some(a.begin), Some(b.begin), t).unwrap(),
            end: AlignmentGeometry::lerp(Some(a.end), Some(b.end), t).unwrap(),
            colors: interpolated.colors,
            stops: Some(interpolated.stops),
            // TODO(ianh): interpolate tile mode
            tile_mode: if t < 0.5 { a.tile_mode } else { b.tile_mode },
            transform: if t < 0.5 {
                a.transform.clone()
            } else {
                b.transform.clone()
            },
        })
    }

    /// Returns a new [`LinearGradient`] with each color set to the given opacity.
    #[allow(deprecated)]
    pub fn with_opacity(&self, opacity: f64) -> LinearGradient {
        LinearGradient {
            colors: self
                .colors
                .iter()
                .map(|color| color.with_opacity(opacity))
                .collect(),
            ..self.clone()
        }
    }
}

impl Gradient for LinearGradient {
    fn colors(&self) -> &[Color] {
        &self.colors
    }

    fn stops(&self) -> Option<&[f64]> {
        self.stops.as_deref()
    }

    fn transform(&self) -> Option<&GradientTransformRef> {
        self.transform.as_ref()
    }

    fn create_shader(&self, rect: Rect, text_direction: Option<TextDirection>) -> Shader {
        gradient_linear(
            self.begin.resolve(text_direction).within_rect(rect),
            self.end.resolve(text_direction).within_rect(rect),
            &self.colors,
            Some(&<dyn Gradient>::implied_stops(self)),
            self.tile_mode,
            <dyn Gradient>::resolve_transform(self, rect, text_direction),
        )
    }

    fn scale(&self, factor: f64) -> Box<dyn Gradient> {
        Box::new(LinearGradient::scale(self, factor))
    }

    fn with_opacity(&self, opacity: f64) -> Box<dyn Gradient> {
        Box::new(LinearGradient::with_opacity(self, opacity))
    }

    fn from_color(&self, color: Color) -> Box<dyn Gradient> {
        Box::new(LinearGradient::from_color(self, color))
    }

    fn lerp_from(&self, a: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        match a {
            None => boxed(LinearGradient::lerp(None, Some(self), t)),
            Some(a) => {
                let a = a.as_any().downcast_ref::<LinearGradient>()?;
                boxed(LinearGradient::lerp(Some(a), Some(self), t))
            }
        }
    }

    fn lerp_to(&self, b: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        match b {
            None => boxed(LinearGradient::lerp(Some(self), None, t)),
            Some(b) => {
                let b = b.as_any().downcast_ref::<LinearGradient>()?;
                boxed(LinearGradient::lerp(Some(self), Some(b), t))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Gradient> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_gradient(&self, other: &dyn Gradient) -> bool {
        other
            .as_any()
            .downcast_ref::<LinearGradient>()
            .is_some_and(|other| other == self)
    }
}

impl Debug for LinearGradient {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LinearGradient(begin: {:?}, end: {:?}, colors: {:?}",
            self.begin, self.end, self.colors
        )?;
        if let Some(stops) = &self.stops {
            write!(f, ", stops: {stops:?}")?;
        }
        write!(f, ", tileMode: {}", tile_mode_name(self.tile_mode))?;
        if let Some(transform) = &self.transform {
            write!(f, ", transform: {transform:?}")?;
        }
        write!(f, ")")
    }
}

/// A 2D radial gradient.
///
/// This class is used by `BoxDecoration` to represent radial gradients. This
/// abstracts out the arguments to the dart:ui `Gradient.radial` constructor.
///
/// A normal radial gradient has a [`center`](Self::center) and a [`radius`](Self::radius). The
/// [`center`](Self::center) point corresponds to 0.0, and the ring at [`radius`](Self::radius)
/// from the center corresponds to 1.0. These lengths are expressed in fractions, so that the
/// same gradient can be reused with varying sized boxes without changing the parameters.
/// (This contrasts with dart:ui `Gradient.radial`, whose arguments are expressed in logical
/// pixels.)
///
/// It is also possible to create a two-point (or focal pointed) radial gradient
/// (which is sometimes referred to as a two point conic gradient, but is not the
/// same as a CSS conic gradient which corresponds to a [`SweepGradient`]). A
/// [`focal`](Self::focal) point and [`focal_radius`](Self::focal_radius) can be specified
/// similarly to [`center`](Self::center) and [`radius`](Self::radius), which will make the
/// rendered gradient appear to be pointed or directed in the direction of the
/// [`focal`](Self::focal) point. This is only important if [`focal`](Self::focal) and
/// [`center`](Self::center) are not equal or [`focal_radius`](Self::focal_radius) > 0.0 (as this
/// case is visually identical to a normal radial gradient). One important case to avoid is
/// having [`focal`](Self::focal) and [`center`](Self::center) both resolve to
/// [`Offset::ZERO`] when [`focal_radius`](Self::focal_radius) > 0.0. In such a case, a valid
/// shader cannot be created by the framework.
///
/// The [`colors`](Self::colors) are described by a list of [`Color`] objects. There must be at
/// least two colors. The [`stops`](Self::stops) list, if specified, must have the same length
/// as [`colors`](Self::colors). It specifies fractions of the radius between 0.0 and 1.0,
/// giving concentric rings for each color stop. If it is null, a uniform
/// distribution is assumed.
///
/// The region of the canvas beyond [`radius`](Self::radius) from the [`center`](Self::center)
/// is colored according to [`tile_mode`](Self::tile_mode).
#[derive(Clone, PartialEq)]
pub struct RadialGradient {
    /// The center of the gradient, as an offset into the (-1.0, -1.0) x (1.0, 1.0)
    /// square describing the gradient which will be mapped onto the paint box.
    ///
    /// For example, an alignment of (0.0, 0.0) will place the radial
    /// gradient in the center of the box.
    ///
    /// It can also be an `AlignmentDirectional`, where the start is the left in
    /// left-to-right contexts and the right in right-to-left contexts. If a
    /// text-direction-dependent value is provided here, then the
    /// [`create_shader`](Gradient::create_shader) method will need to be given a
    /// [`TextDirection`].
    pub center: AlignmentGeometry,
    /// The radius of the gradient, as a fraction of the shortest side
    /// of the paint box.
    ///
    /// For example, if a radial gradient is painted on a box that is
    /// 100.0 pixels wide and 200.0 pixels tall, then a radius of 1.0
    /// will place the 1.0 stop at 100.0 pixels from the [`center`](Self::center).
    pub radius: f64,
    /// The colors the gradient should obtain at each of the stops.
    pub colors: Vec<Color>,
    /// A list of values from 0.0 to 1.0 that denote fractions along the gradient.
    pub stops: Option<Vec<f64>>,
    /// How this gradient should tile the plane beyond the outer ring at
    /// [`radius`](Self::radius) pixels from the [`center`](Self::center).
    pub tile_mode: TileMode,
    /// The focal point of the gradient. If specified, the gradient will appear
    /// to be focused along the vector from [`center`](Self::center) to focal.
    ///
    /// See [`center`](Self::center) for a description of how the coordinates are mapped.
    ///
    /// If this value is specified and [`focal_radius`](Self::focal_radius) > 0.0, care should
    /// be taken to ensure that either this value or [`center`](Self::center) will not both
    /// resolve to [`Offset::ZERO`], which would fail to create a valid gradient.
    pub focal: Option<AlignmentGeometry>,
    /// The radius of the focal point of gradient, as a fraction of the shortest
    /// side of the paint box.
    ///
    /// For example, if a radial gradient is painted on a box that is
    /// 100.0 pixels wide and 200.0 pixels tall, then a radius of 1.0
    /// will place the 1.0 stop at 100.0 pixels from the [`focal`](Self::focal) point.
    ///
    /// If this value is specified and is greater than 0.0, either [`focal`](Self::focal) or
    /// [`center`](Self::center) must not resolve to [`Offset::ZERO`], which would fail to
    /// create a valid gradient.
    pub focal_radius: f64,
    /// The transform, if any, to apply to the gradient.
    pub transform: Option<GradientTransformRef>,
}

impl RadialGradient {
    /// Creates a radial gradient.
    ///
    /// If [`stops`](Self::stops) is non-null, it must have the same length as
    /// [`colors`](Self::colors).
    pub fn new(colors: Vec<Color>) -> RadialGradient {
        RadialGradient {
            center: AlignmentGeometry::CENTER,
            radius: 0.5,
            colors,
            stops: None,
            tile_mode: TileMode::Clamp,
            focal: None,
            focal_radius: 0.0,
            transform: None,
        }
    }

    /// Dart `RadialGradient(center:)`.
    pub fn center(mut self, center: impl Into<AlignmentGeometry>) -> RadialGradient {
        self.center = center.into();
        self
    }

    /// Dart `RadialGradient(radius:)`.
    pub fn radius(mut self, radius: f64) -> RadialGradient {
        self.radius = radius;
        self
    }

    /// Dart `RadialGradient(stops:)`.
    pub fn stops(mut self, stops: Vec<f64>) -> RadialGradient {
        self.stops = Some(stops);
        self
    }

    /// Dart `RadialGradient(tileMode:)`.
    pub fn tile_mode(mut self, tile_mode: TileMode) -> RadialGradient {
        self.tile_mode = tile_mode;
        self
    }

    /// Dart `RadialGradient(focal:)`.
    pub fn focal(mut self, focal: impl Into<AlignmentGeometry>) -> RadialGradient {
        self.focal = Some(focal.into());
        self
    }

    /// Dart `RadialGradient(focalRadius:)`.
    pub fn focal_radius(mut self, focal_radius: f64) -> RadialGradient {
        self.focal_radius = focal_radius;
        self
    }

    /// Dart `RadialGradient(transform:)`.
    pub fn transform(mut self, transform: impl Into<GradientTransformRef>) -> RadialGradient {
        self.transform = Some(transform.into());
        self
    }

    /// Returns a new [`RadialGradient`] with its colors scaled by the given factor.
    ///
    /// Since the alpha component of the Color is what is scaled, a factor
    /// of 0.0 or less results in a gradient that is fully transparent.
    pub fn scale(&self, factor: f64) -> RadialGradient {
        RadialGradient {
            colors: self
                .colors
                .iter()
                .map(|&color| Color::lerp(None, Some(color), factor).unwrap())
                .collect(),
            ..self.clone()
        }
    }

    /// Dart's `RadialGradient.fromColor`: the geometry kept, every color replaced by `color`.
    pub fn from_color(&self, color: Color) -> RadialGradient {
        RadialGradient {
            colors: vec![color; self.colors.len()],
            ..self.clone()
        }
    }

    /// Linearly interpolate between two [`RadialGradient`]s.
    ///
    /// If either gradient is None, this function linearly interpolates from
    /// a gradient that matches the other gradient in [`center`](Self::center),
    /// [`radius`](Self::radius), [`stops`](Self::stops) and [`tile_mode`](Self::tile_mode) and
    /// with the same [`colors`](Self::colors) but transparent (using [`scale`](Self::scale)).
    ///
    /// If neither gradient is None, they must have the same number of [`colors`](Self::colors).
    pub fn lerp(
        a: Option<&RadialGradient>,
        b: Option<&RadialGradient>,
        t: f64,
    ) -> Option<RadialGradient> {
        let (a, b) = match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            (None, Some(b)) => return Some(b.scale(t)),
            (Some(a), None) => return Some(a.scale(1.0 - t)),
            (Some(a), Some(b)) => (a, b),
        };
        let interpolated = interpolate_colors_and_stops(
            &a.colors,
            &<dyn Gradient>::implied_stops(a),
            &b.colors,
            &<dyn Gradient>::implied_stops(b),
            t,
        );
        Some(RadialGradient {
            center: AlignmentGeometry::lerp(Some(a.center), Some(b.center), t).unwrap(),
            radius: lerp_double(Some(a.radius), Some(b.radius), t)
                .unwrap()
                .max(0.0),
            colors: interpolated.colors,
            stops: Some(interpolated.stops),
            // TODO(ianh): interpolate tile mode
            tile_mode: if t < 0.5 { a.tile_mode } else { b.tile_mode },
            focal: AlignmentGeometry::lerp(a.focal, b.focal, t),
            focal_radius: lerp_double(Some(a.focal_radius), Some(b.focal_radius), t)
                .unwrap()
                .max(0.0),
            transform: if t < 0.5 {
                a.transform.clone()
            } else {
                b.transform.clone()
            },
        })
    }

    /// Returns a new [`RadialGradient`] with each color set to the given opacity.
    #[allow(deprecated)]
    pub fn with_opacity(&self, opacity: f64) -> RadialGradient {
        RadialGradient {
            colors: self
                .colors
                .iter()
                .map(|color| color.with_opacity(opacity))
                .collect(),
            ..self.clone()
        }
    }
}

impl Gradient for RadialGradient {
    fn colors(&self) -> &[Color] {
        &self.colors
    }

    fn stops(&self) -> Option<&[f64]> {
        self.stops.as_deref()
    }

    fn transform(&self) -> Option<&GradientTransformRef> {
        self.transform.as_ref()
    }

    fn create_shader(&self, rect: Rect, text_direction: Option<TextDirection>) -> Shader {
        gradient_radial(
            self.center.resolve(text_direction).within_rect(rect),
            self.radius * rect.shortest_side(),
            &self.colors,
            Some(&<dyn Gradient>::implied_stops(self)),
            self.tile_mode,
            <dyn Gradient>::resolve_transform(self, rect, text_direction),
            self.focal
                .map(|focal| focal.resolve(text_direction).within_rect(rect)),
            self.focal_radius * rect.shortest_side(),
        )
    }

    fn scale(&self, factor: f64) -> Box<dyn Gradient> {
        Box::new(RadialGradient::scale(self, factor))
    }

    fn with_opacity(&self, opacity: f64) -> Box<dyn Gradient> {
        Box::new(RadialGradient::with_opacity(self, opacity))
    }

    fn from_color(&self, color: Color) -> Box<dyn Gradient> {
        Box::new(RadialGradient::from_color(self, color))
    }

    fn lerp_from(&self, a: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        match a {
            None => boxed(RadialGradient::lerp(None, Some(self), t)),
            Some(a) => {
                let a = a.as_any().downcast_ref::<RadialGradient>()?;
                boxed(RadialGradient::lerp(Some(a), Some(self), t))
            }
        }
    }

    fn lerp_to(&self, b: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        match b {
            None => boxed(RadialGradient::lerp(Some(self), None, t)),
            Some(b) => {
                let b = b.as_any().downcast_ref::<RadialGradient>()?;
                boxed(RadialGradient::lerp(Some(self), Some(b), t))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Gradient> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_gradient(&self, other: &dyn Gradient) -> bool {
        other
            .as_any()
            .downcast_ref::<RadialGradient>()
            .is_some_and(|other| other == self)
    }
}

impl Debug for RadialGradient {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RadialGradient(center: {:?}, radius: {:.1}, colors: {:?}",
            self.center, self.radius, self.colors
        )?;
        if let Some(stops) = &self.stops {
            write!(f, ", stops: {stops:?}")?;
        }
        write!(f, ", tileMode: {}", tile_mode_name(self.tile_mode))?;
        if let Some(focal) = &self.focal {
            write!(f, ", focal: {focal:?}")?;
        }
        write!(f, ", focalRadius: {:.1}", self.focal_radius)?;
        if let Some(transform) = &self.transform {
            write!(f, ", transform: {transform:?}")?;
        }
        write!(f, ")")
    }
}

/// A 2D sweep gradient.
///
/// This class is used by `BoxDecoration` to represent sweep gradients. This
/// abstracts out the arguments to the dart:ui `Gradient.sweep` constructor.
///
/// A gradient has a [`center`](Self::center), a [`start_angle`](Self::start_angle), and an
/// [`end_angle`](Self::end_angle). The [`start_angle`](Self::start_angle) corresponds to 0.0,
/// and the [`end_angle`](Self::end_angle) corresponds to 1.0. These angles are expressed in
/// radians.
///
/// The [`colors`](Self::colors) are described by a list of [`Color`] objects. There must be at
/// least two colors. The [`stops`](Self::stops) list, if specified, must have the same length
/// as [`colors`](Self::colors). It specifies fractions of the vector from start to end, between
/// 0.0 and 1.0, for each color. If it is null, a uniform distribution is
/// assumed.
///
/// The region of the canvas before [`start_angle`](Self::start_angle) and after
/// [`end_angle`](Self::end_angle) is colored according to [`tile_mode`](Self::tile_mode).
#[derive(Clone, PartialEq)]
pub struct SweepGradient {
    /// The center of the gradient, as an offset into the (-1.0, -1.0) x (1.0, 1.0)
    /// square describing the gradient which will be mapped onto the paint box.
    ///
    /// For example, an alignment of (0.0, 0.0) will place the sweep
    /// gradient in the center of the box.
    ///
    /// It can also be an `AlignmentDirectional`, where the start is the left in
    /// left-to-right contexts and the right in right-to-left contexts. If a
    /// text-direction-dependent value is provided here, then the
    /// [`create_shader`](Gradient::create_shader) method will need to be given a
    /// [`TextDirection`].
    pub center: AlignmentGeometry,
    /// The angle in radians at which stop 0.0 of the gradient is placed.
    ///
    /// The angle is measured in radians clockwise from the positive x-axis.
    ///
    /// Values outside the range `[0, 2π]` are normalized to the equivalent angle
    /// within this range using modulo arithmetic.
    ///
    /// The gradient will be painted in the sector between [`start_angle`](Self::start_angle)
    /// and [`end_angle`](Self::end_angle). The behavior outside this sector is determined by
    /// [`tile_mode`](Self::tile_mode).
    ///
    /// Defaults to 0.0.
    pub start_angle: f64,
    /// The angle in radians at which stop 1.0 of the gradient is placed.
    ///
    /// The angle is measured in radians clockwise from the positive x-axis.
    ///
    /// Values outside the range `[0, 2π]` are normalized to the equivalent angle
    /// within this range using modulo arithmetic.
    ///
    /// Defaults to `PI * 2` (2π = a full circle).
    pub end_angle: f64,
    /// The colors the gradient should obtain at each of the stops.
    pub colors: Vec<Color>,
    /// A list of values from 0.0 to 1.0 that denote fractions along the gradient.
    pub stops: Option<Vec<f64>>,
    /// How this gradient should tile the plane in the region before
    /// [`start_angle`](Self::start_angle) and after [`end_angle`](Self::end_angle).
    ///
    /// * [`TileMode::Clamp`]: The edge colors are extended to fill the remaining area.
    /// * [`TileMode::Repeat`]: The gradient is repeated in the angular direction.
    /// * [`TileMode::Mirror`]: The gradient is mirrored in the angular direction.
    /// * [`TileMode::Decal`]: Only the gradient is drawn, leaving the remaining area
    ///   transparent.
    pub tile_mode: TileMode,
    /// The transform, if any, to apply to the gradient.
    pub transform: Option<GradientTransformRef>,
}

impl SweepGradient {
    /// Creates a sweep gradient.
    ///
    /// If [`stops`](Self::stops) is non-null, it must have the same length as
    /// [`colors`](Self::colors).
    pub fn new(colors: Vec<Color>) -> SweepGradient {
        SweepGradient {
            center: AlignmentGeometry::CENTER,
            start_angle: 0.0,
            end_angle: PI * 2.0,
            colors,
            stops: None,
            tile_mode: TileMode::Clamp,
            transform: None,
        }
    }

    /// Dart `SweepGradient(center:)`.
    pub fn center(mut self, center: impl Into<AlignmentGeometry>) -> SweepGradient {
        self.center = center.into();
        self
    }

    /// Dart `SweepGradient(startAngle:)`.
    pub fn start_angle(mut self, start_angle: f64) -> SweepGradient {
        self.start_angle = start_angle;
        self
    }

    /// Dart `SweepGradient(endAngle:)`.
    pub fn end_angle(mut self, end_angle: f64) -> SweepGradient {
        self.end_angle = end_angle;
        self
    }

    /// Dart `SweepGradient(stops:)`.
    pub fn stops(mut self, stops: Vec<f64>) -> SweepGradient {
        self.stops = Some(stops);
        self
    }

    /// Dart `SweepGradient(tileMode:)`.
    pub fn tile_mode(mut self, tile_mode: TileMode) -> SweepGradient {
        self.tile_mode = tile_mode;
        self
    }

    /// Dart `SweepGradient(transform:)`.
    pub fn transform(mut self, transform: impl Into<GradientTransformRef>) -> SweepGradient {
        self.transform = Some(transform.into());
        self
    }

    /// Returns a new [`SweepGradient`] with its colors scaled by the given factor.
    ///
    /// Since the alpha component of the Color is what is scaled, a factor
    /// of 0.0 or less results in a gradient that is fully transparent.
    pub fn scale(&self, factor: f64) -> SweepGradient {
        SweepGradient {
            colors: self
                .colors
                .iter()
                .map(|&color| Color::lerp(None, Some(color), factor).unwrap())
                .collect(),
            ..self.clone()
        }
    }

    /// Dart's `SweepGradient.fromColor`: the geometry kept, every color replaced by `color`.
    pub fn from_color(&self, color: Color) -> SweepGradient {
        SweepGradient {
            colors: vec![color; self.colors.len()],
            ..self.clone()
        }
    }

    /// Linearly interpolate between two [`SweepGradient`]s.
    ///
    /// If either gradient is None, then the non-null gradient is returned with
    /// its color scaled in the same way as the [`scale`](Self::scale) function.
    ///
    /// If neither gradient is None, they must have the same number of [`colors`](Self::colors).
    pub fn lerp(
        a: Option<&SweepGradient>,
        b: Option<&SweepGradient>,
        t: f64,
    ) -> Option<SweepGradient> {
        let (a, b) = match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            (None, Some(b)) => return Some(b.scale(t)),
            (Some(a), None) => return Some(a.scale(1.0 - t)),
            (Some(a), Some(b)) => (a, b),
        };
        let interpolated = interpolate_colors_and_stops(
            &a.colors,
            &<dyn Gradient>::implied_stops(a),
            &b.colors,
            &<dyn Gradient>::implied_stops(b),
            t,
        );
        Some(SweepGradient {
            center: AlignmentGeometry::lerp(Some(a.center), Some(b.center), t).unwrap(),
            start_angle: lerp_double(Some(a.start_angle), Some(b.start_angle), t)
                .unwrap()
                .max(0.0),
            end_angle: lerp_double(Some(a.end_angle), Some(b.end_angle), t)
                .unwrap()
                .max(0.0),
            colors: interpolated.colors,
            stops: Some(interpolated.stops),
            // TODO(ianh): interpolate tile mode
            tile_mode: if t < 0.5 { a.tile_mode } else { b.tile_mode },
            transform: if t < 0.5 {
                a.transform.clone()
            } else {
                b.transform.clone()
            },
        })
    }

    /// Returns a new [`SweepGradient`] with each color set to the given opacity.
    #[allow(deprecated)]
    pub fn with_opacity(&self, opacity: f64) -> SweepGradient {
        SweepGradient {
            colors: self
                .colors
                .iter()
                .map(|color| color.with_opacity(opacity))
                .collect(),
            ..self.clone()
        }
    }
}

impl Gradient for SweepGradient {
    fn colors(&self) -> &[Color] {
        &self.colors
    }

    fn stops(&self) -> Option<&[f64]> {
        self.stops.as_deref()
    }

    fn transform(&self) -> Option<&GradientTransformRef> {
        self.transform.as_ref()
    }

    fn create_shader(&self, rect: Rect, text_direction: Option<TextDirection>) -> Shader {
        gradient_sweep(
            self.center.resolve(text_direction).within_rect(rect),
            &self.colors,
            Some(&<dyn Gradient>::implied_stops(self)),
            self.tile_mode,
            self.start_angle,
            self.end_angle,
            <dyn Gradient>::resolve_transform(self, rect, text_direction),
        )
    }

    fn scale(&self, factor: f64) -> Box<dyn Gradient> {
        Box::new(SweepGradient::scale(self, factor))
    }

    fn with_opacity(&self, opacity: f64) -> Box<dyn Gradient> {
        Box::new(SweepGradient::with_opacity(self, opacity))
    }

    fn from_color(&self, color: Color) -> Box<dyn Gradient> {
        Box::new(SweepGradient::from_color(self, color))
    }

    fn lerp_from(&self, a: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        match a {
            None => boxed(SweepGradient::lerp(None, Some(self), t)),
            Some(a) => {
                let a = a.as_any().downcast_ref::<SweepGradient>()?;
                boxed(SweepGradient::lerp(Some(a), Some(self), t))
            }
        }
    }

    fn lerp_to(&self, b: Option<&dyn Gradient>, t: f64) -> Option<Box<dyn Gradient>> {
        match b {
            None => boxed(SweepGradient::lerp(Some(self), None, t)),
            Some(b) => {
                let b = b.as_any().downcast_ref::<SweepGradient>()?;
                boxed(SweepGradient::lerp(Some(self), Some(b), t))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Gradient> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_gradient(&self, other: &dyn Gradient) -> bool {
        other
            .as_any()
            .downcast_ref::<SweepGradient>()
            .is_some_and(|other| other == self)
    }
}

impl Debug for SweepGradient {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SweepGradient(center: {:?}, startAngle: {:.1}, endAngle: {:.1}, colors: {:?}",
            self.center, self.start_angle, self.end_angle, self.colors
        )?;
        if let Some(stops) = &self.stops {
            write!(f, ", stops: {stops:?}")?;
        }
        write!(f, ", tileMode: {}", tile_mode_name(self.tile_mode))?;
        if let Some(transform) = &self.transform {
            write!(f, ", transform: {transform:?}")?;
        }
        write!(f, ")")
    }
}

// The dart:ui `Gradient` constructors the three `create_shader`s call, building Valo's shader.

fn offset_is_valid(offset: Offset) -> bool {
    debug_assert!(
        !offset.dx().is_nan() && !offset.dy().is_nan(),
        "Offset argument contained a NaN value."
    );
    true
}

/// dart:ui `Gradient._validateColorStops`, which throws an `ArgumentError` in release too.
fn validate_color_stops(colors: &[Color], color_stops: Option<&[f64]>) {
    match color_stops {
        None => assert!(
            colors.len() == 2,
            "\"colors\" must have length 2 if \"colorStops\" is omitted."
        ),
        Some(color_stops) => assert!(
            colors.len() == color_stops.len(),
            "\"colors\" and \"colorStops\" arguments must have equal length."
        ),
    }
}

/// The engine's color and stop buffers as Valo's stops: without `color_stops`, the colors
/// are spread evenly.
fn encode_stops(colors: &[Color], color_stops: Option<&[f64]>) -> Vec<GradientStop> {
    colors
        .iter()
        .enumerate()
        .map(|(index, &color)| GradientStop {
            offset: match color_stops {
                Some(color_stops) => color_stops[index] as f32,
                None => index as f32 / (colors.len() - 1) as f32,
            },
            color: color.into(),
        })
        .collect()
}

/// dart:ui `TileMode` as Valo's spread.
fn spread_mode(tile_mode: TileMode) -> SpreadMode {
    match tile_mode {
        TileMode::Clamp => SpreadMode::Pad,
        TileMode::Repeat => SpreadMode::Repeat,
        TileMode::Mirror => SpreadMode::Reflect,
        TileMode::Decal => SpreadMode::Decal,
    }
}

/// dart:ui `Gradient.linear`.
fn gradient_linear(
    from: Offset,
    to: Offset,
    colors: &[Color],
    color_stops: Option<&[f64]>,
    tile_mode: TileMode,
    matrix4: Option<Matrix4>,
) -> Shader {
    debug_assert!(offset_is_valid(from));
    debug_assert!(offset_is_valid(to));
    validate_color_stops(colors, color_stops);
    Shader::Linear {
        start: from.into(),
        end: to.into(),
        stops: encode_stops(colors, color_stops),
        spread: spread_mode(tile_mode),
        local: matrix4.unwrap_or(Matrix4::IDENTITY),
    }
}

/// dart:ui `Gradient.radial`.
#[allow(clippy::too_many_arguments)]
fn gradient_radial(
    center: Offset,
    radius: f64,
    colors: &[Color],
    color_stops: Option<&[f64]>,
    tile_mode: TileMode,
    matrix4: Option<Matrix4>,
    focal: Option<Offset>,
    focal_radius: f64,
) -> Shader {
    debug_assert!(offset_is_valid(center));
    validate_color_stops(colors, color_stops);
    // If focal is null or focal radius is null, this should be treated as a regular radial gradient
    // If focal == center and the focal radius is 0.0, it's still a regular radial gradient
    let focus = match focal {
        Some(focal) if !(focal == center && focal_radius == 0.0) => {
            // will result in exception(s) in Skia side
            debug_assert!(center != Offset::ZERO || focal != Offset::ZERO);
            Some(FocalCircle {
                center: focal.into(),
                radius: focal_radius as f32,
            })
        }
        _ => None,
    };
    Shader::Radial {
        center: center.into(),
        radius: radius as f32,
        stops: encode_stops(colors, color_stops),
        spread: spread_mode(tile_mode),
        focus,
        local: matrix4.unwrap_or(Matrix4::IDENTITY),
    }
}

/// dart:ui `Gradient.sweep`.
fn gradient_sweep(
    center: Offset,
    colors: &[Color],
    color_stops: Option<&[f64]>,
    tile_mode: TileMode,
    start_angle: f64,
    end_angle: f64,
    matrix4: Option<Matrix4>,
) -> Shader {
    debug_assert!(offset_is_valid(center));
    debug_assert!(start_angle < end_angle);
    validate_color_stops(colors, color_stops);
    Shader::Sweep {
        center: center.into(),
        start_angle: start_angle as f32,
        end_angle: end_angle as f32,
        stops: encode_stops(colors, color_stops),
        spread: spread_mode(tile_mode),
        local: matrix4.unwrap_or(Matrix4::IDENTITY),
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use waft_embedder::valo::Point;

    use super::*;

    fn list_double_matches(x: Option<&[f64]>, y: Option<&[f64]>) -> bool {
        match (x, y) {
            (None, None) => true,
            (Some(x), Some(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(x, y)| (x - y).abs() < 0.0001)
            }
            _ => false,
        }
    }

    fn list_color_matches(x: &[Color], y: &[Color]) -> bool {
        const LIMIT: f64 = 1.0 / 255.0;
        x.len() == y.len()
            && x.iter().zip(y).all(|(x, y)| {
                (x.a - y.a).abs() < LIMIT
                    && (x.r - y.r).abs() < LIMIT
                    && (x.g - y.g).abs() < LIMIT
                    && (x.b - y.b).abs() < LIMIT
            })
    }

    /// Dart's `_matchesLinearGradient`.
    fn assert_matches_linear(item: &dyn Gradient, target: &LinearGradient) {
        let matches = item
            .as_any()
            .downcast_ref::<LinearGradient>()
            .is_some_and(|item| {
                item.begin == target.begin
                    && item.end == target.end
                    && item.tile_mode == target.tile_mode
                    && item.transform == target.transform
                    && list_color_matches(&item.colors, &target.colors)
                    && list_double_matches(item.stops.as_deref(), target.stops.as_deref())
            });
        assert!(matches, "expected {target:?}, got {item:?}");
    }

    /// Dart's `_matchesRadialGradient`.
    fn assert_matches_radial(item: &dyn Gradient, target: &RadialGradient) {
        let matches = item
            .as_any()
            .downcast_ref::<RadialGradient>()
            .is_some_and(|item| {
                item.center == target.center
                    && item.radius == target.radius
                    && item.tile_mode == target.tile_mode
                    && item.transform == target.transform
                    && item.focal == target.focal
                    && item.focal_radius == target.focal_radius
                    && list_color_matches(&item.colors, &target.colors)
                    && list_double_matches(item.stops.as_deref(), target.stops.as_deref())
            });
        assert!(matches, "expected {target:?}, got {item:?}");
    }

    /// Dart's `_matchesSweepGradient`.
    fn assert_matches_sweep(item: &dyn Gradient, target: &SweepGradient) {
        let matches = item
            .as_any()
            .downcast_ref::<SweepGradient>()
            .is_some_and(|item| {
                item.center == target.center
                    && item.start_angle == target.start_angle
                    && item.end_angle == target.end_angle
                    && item.tile_mode == target.tile_mode
                    && item.transform == target.transform
                    && list_color_matches(&item.colors, &target.colors)
                    && list_double_matches(item.stops.as_deref(), target.stops.as_deref())
            });
        assert!(matches, "expected {target:?}, got {item:?}");
    }

    fn colors(values: &[u32]) -> Vec<Color> {
        values.iter().map(|&value| Color::new(value)).collect()
    }

    #[test]
    fn a_linear_gradient_lerped_from_null_scales_its_colours() {
        let test_gradient = LinearGradient::new(colors(&[0x00FFFFFF, 0x11777777, 0x44444444]))
            .begin(AlignmentGeometry::BOTTOM_RIGHT)
            .end(AlignmentGeometry::xy(0.7, 1.0));
        let actual = LinearGradient::lerp(None, Some(&test_gradient), 0.25).unwrap();
        assert_matches_linear(
            &actual,
            &LinearGradient::new(colors(&[0x00FFFFFF, 0x04777777, 0x11444444]))
                .begin(AlignmentGeometry::BOTTOM_RIGHT)
                .end(AlignmentGeometry::xy(0.7, 1.0)),
        );
    }

    #[test]
    fn linear_gradients_lerp() {
        let test_gradient1 = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .begin(AlignmentGeometry::TOP_LEFT)
            .end(AlignmentGeometry::BOTTOM_LEFT);
        let test_gradient2 = LinearGradient::new(colors(&[0x44444444, 0x88888888]))
            .begin(AlignmentGeometry::TOP_RIGHT)
            .end(AlignmentGeometry::TOP_LEFT);
        let actual = LinearGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_linear(
            &actual.unwrap(),
            &LinearGradient::new(colors(&[0x3B3B3B3B, 0x77777777]))
                .begin(AlignmentGeometry::TOP_CENTER)
                .end(AlignmentGeometry::CENTER_LEFT)
                .stops(vec![0.0, 1.0]),
        );
    }

    #[test]
    fn linear_gradient_lerp_of_identical_a_and_b_returns_a() {
        assert!(LinearGradient::lerp(None, None, 0.0).is_none());
        let gradient = LinearGradient::new(colors(&[0x33333333, 0x66666666]));
        assert_eq!(
            LinearGradient::lerp(Some(&gradient), Some(&gradient), 0.5).unwrap(),
            gradient
        );
    }

    #[test]
    fn linear_gradients_lerp_with_stops() {
        let test_gradient1 = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .begin(AlignmentGeometry::TOP_LEFT)
            .end(AlignmentGeometry::BOTTOM_LEFT)
            .stops(vec![0.0, 0.5]);
        let test_gradient2 = LinearGradient::new(colors(&[0x44444444, 0x88888888]))
            .begin(AlignmentGeometry::TOP_RIGHT)
            .end(AlignmentGeometry::TOP_LEFT)
            .stops(vec![0.5, 1.0]);
        let actual = LinearGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_linear(
            &actual.unwrap(),
            &LinearGradient::new(colors(&[0x3B3B3B3B, 0x55555555, 0x77777777]))
                .begin(AlignmentGeometry::TOP_CENTER)
                .end(AlignmentGeometry::CENTER_LEFT)
                .stops(vec![0.0, 0.5, 1.0]),
        );
    }

    #[test]
    fn linear_gradients_lerp_with_unequal_numbers_of_colours() {
        let test_gradient1 = LinearGradient::new(colors(&[0x22222222, 0x66666666]));
        let test_gradient2 = LinearGradient::new(colors(&[0x44444444, 0x66666666, 0x88888888]));
        let actual = LinearGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_linear(
            &actual.unwrap(),
            &LinearGradient::new(colors(&[0x33333333, 0x55555555, 0x77777777]))
                .stops(vec![0.0, 0.5, 1.0]),
        );
    }

    #[test]
    fn linear_gradients_lerp_with_stops_and_unequal_numbers_of_colours() {
        let test_gradient1 =
            LinearGradient::new(colors(&[0x33333333, 0x66666666])).stops(vec![0.0, 0.5]);
        let test_gradient2 = LinearGradient::new(colors(&[0x44444444, 0x48484848, 0x88888888]))
            .stops(vec![0.5, 0.7, 1.0]);
        let actual = LinearGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_linear(
            &actual.unwrap(),
            &LinearGradient::new(colors(&[0x3B3B3B3B, 0x55555555, 0x57575757, 0x77777777]))
                .stops(vec![0.0, 0.5, 0.7, 1.0]),
        );
    }

    #[test]
    fn linear_gradients_lerp_with_transforms() {
        let test_gradient1 = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 4.0))
            .stops(vec![0.0, 1.0]);
        let test_gradient2 = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 2.0))
            .stops(vec![0.0, 1.0]);
        let lerp = |t| LinearGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), t);
        assert_eq!(Some(test_gradient1.clone()), lerp(0.0));
        assert_eq!(Some(test_gradient2.clone()), lerp(1.0));
        assert_eq!(Some(test_gradient2.clone()), lerp(0.5));
    }

    #[test]
    fn a_linear_gradient_prints_as_dart_does() {
        let red = Color::new(0x33333333);
        let green = Color::new(0x66666666);
        assert_eq!(
            format!(
                "{:?}",
                LinearGradient::new(vec![red, green])
                    .begin(AlignmentGeometry::TOP_LEFT)
                    .end(AlignmentGeometry::BOTTOM_LEFT)
                    .transform(GradientRotation::new(1.6))
            ),
            format!(
                "LinearGradient(begin: Alignment.topLeft, end: Alignment.bottomLeft, colors: [{red:?}, {green:?}], tileMode: TileMode.clamp, transform: GradientRotation(radians: 1.6))"
            )
        );
    }

    #[test]
    fn linear_gradients_with_different_transforms_differ() {
        let test_gradient1 = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 4.0));
        let test_gradient1_copy = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 4.0));
        let test_gradient2 = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 2.0));
        assert_eq!(test_gradient1, test_gradient1_copy);
        assert_ne!(test_gradient1, test_gradient2);
    }

    const RECT: Rect = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);

    #[test]
    #[should_panic(expected = "No TextDirection found.")]
    fn a_directional_linear_gradient_needs_a_text_direction() {
        LinearGradient::new(colors(&[0xFFFFFFFF, 0xFFFFFFFF]))
            .begin(AlignmentGeometry::TOP_START)
            .create_shader(RECT, None);
    }

    #[test]
    fn a_linear_gradient_resolves_its_alignments_with_a_text_direction() {
        let directional = LinearGradient::new(colors(&[0xFFFFFFFF, 0xFFFFFFFF]))
            .begin(AlignmentGeometry::TOP_START);
        directional.create_shader(RECT, Some(TextDirection::Rtl));
        directional.create_shader(RECT, Some(TextDirection::Ltr));
        LinearGradient::new(colors(&[0xFFFFFFFF, 0xFFFFFFFF]))
            .begin(AlignmentGeometry::TOP_LEFT)
            .create_shader(RECT, None);
    }

    #[test]
    fn a_linear_gradient_with_opacity_sets_every_alpha() {
        let test_gradient = LinearGradient::new(colors(&[0xFFFFFFFF, 0xAF777777, 0x44444444]))
            .begin(AlignmentGeometry::BOTTOM_RIGHT)
            .end(AlignmentGeometry::TOP_CENTER);
        assert_eq!(
            test_gradient.with_opacity(0.5),
            LinearGradient::new(colors(&[0x80FFFFFF, 0x80777777, 0x80444444]))
                .begin(AlignmentGeometry::BOTTOM_RIGHT)
                .end(AlignmentGeometry::TOP_CENTER)
        );
    }

    #[test]
    fn a_linear_gradient_with_opacity_keeps_its_transform() {
        let test_gradient = LinearGradient::new(colors(&[0xFFFFFFFF, 0xAF777777, 0x44444444]))
            .begin(AlignmentGeometry::BOTTOM_RIGHT)
            .end(AlignmentGeometry::TOP_CENTER)
            .transform(GradientRotation::new(1.0));
        assert_eq!(
            test_gradient.with_opacity(0.5),
            LinearGradient::new(colors(&[0x80FFFFFF, 0x80777777, 0x80444444]))
                .begin(AlignmentGeometry::BOTTOM_RIGHT)
                .end(AlignmentGeometry::TOP_CENTER)
                .transform(GradientRotation::new(1.0))
        );
    }

    #[test]
    fn a_linear_gradient_from_a_colour_replaces_its_colours_and_keeps_its_geometry() {
        let test_gradient = LinearGradient::new(colors(&[0xFFFFFFFF, 0xAF777777, 0x44444444]))
            .begin(AlignmentGeometry::BOTTOM_RIGHT)
            .end(AlignmentGeometry::TOP_CENTER)
            .stops(vec![0.0, 0.3, 1.0])
            .tile_mode(TileMode::Mirror)
            .transform(GradientRotation::new(1.0));
        assert_eq!(
            test_gradient.from_color(Color::new(0xFF00FF00)),
            LinearGradient::new(colors(&[0xFF00FF00, 0xFF00FF00, 0xFF00FF00]))
                .begin(AlignmentGeometry::BOTTOM_RIGHT)
                .end(AlignmentGeometry::TOP_CENTER)
                .stops(vec![0.0, 0.3, 1.0])
                .tile_mode(TileMode::Mirror)
                .transform(GradientRotation::new(1.0))
        );
    }

    #[test]
    fn a_radial_gradient_from_a_colour_replaces_its_colours_and_keeps_its_geometry() {
        let test_gradient = RadialGradient::new(colors(&[0xFFFFFFFF, 0x44444444]))
            .center(AlignmentGeometry::TOP_LEFT)
            .radius(0.25)
            .focal(AlignmentGeometry::CENTER_RIGHT)
            .focal_radius(0.1)
            .tile_mode(TileMode::Mirror)
            .transform(GradientRotation::new(1.0));
        assert_eq!(
            test_gradient.from_color(Color::new(0xFF00FF00)),
            RadialGradient::new(colors(&[0xFF00FF00, 0xFF00FF00]))
                .center(AlignmentGeometry::TOP_LEFT)
                .radius(0.25)
                .focal(AlignmentGeometry::CENTER_RIGHT)
                .focal_radius(0.1)
                .tile_mode(TileMode::Mirror)
                .transform(GradientRotation::new(1.0))
        );
    }

    #[test]
    fn a_sweep_gradient_from_a_colour_replaces_its_colours_and_keeps_its_geometry() {
        let test_gradient = SweepGradient::new(colors(&[0xFFFFFFFF, 0x44444444]))
            .center(AlignmentGeometry::TOP_LEFT)
            .start_angle(0.1)
            .end_angle(2.0)
            .tile_mode(TileMode::Repeat)
            .transform(GradientRotation::new(1.0));
        assert_eq!(
            test_gradient.from_color(Color::new(0xFF00FF00)),
            SweepGradient::new(colors(&[0xFF00FF00, 0xFF00FF00]))
                .center(AlignmentGeometry::TOP_LEFT)
                .start_angle(0.1)
                .end_angle(2.0)
                .tile_mode(TileMode::Repeat)
                .transform(GradientRotation::new(1.0))
        );
    }

    #[test]
    fn a_scaled_linear_gradient_keeps_its_transform() {
        let test_gradient = LinearGradient::new(colors(&[0xFFFF0000, 0xFF00FF00, 0xFF0000FF]))
            .begin(AlignmentGeometry::BOTTOM_RIGHT)
            .end(AlignmentGeometry::TOP_LEFT)
            .stops(vec![0.0, 0.5, 1.0])
            .tile_mode(TileMode::Decal)
            .transform(GradientRotation::new(PI / 4.0));
        assert_matches_linear(
            &test_gradient.scale(0.5),
            &LinearGradient::new(colors(&[0x80FF0000, 0x8000FF00, 0x800000FF]))
                .begin(AlignmentGeometry::BOTTOM_RIGHT)
                .end(AlignmentGeometry::TOP_LEFT)
                .stops(vec![0.0, 0.5, 1.0])
                .tile_mode(TileMode::Decal)
                .transform(GradientRotation::new(PI / 4.0)),
        );
    }

    #[test]
    #[should_panic(expected = "No TextDirection found.")]
    fn a_directional_radial_gradient_needs_a_text_direction() {
        RadialGradient::new(colors(&[0xFFFFFFFF, 0xFFFFFFFF]))
            .center(AlignmentGeometry::TOP_START)
            .create_shader(RECT, None);
    }

    #[test]
    fn a_radial_gradient_resolves_its_alignments_with_a_text_direction() {
        let directional = RadialGradient::new(colors(&[0xFFFFFFFF, 0xFFFFFFFF]))
            .center(AlignmentGeometry::TOP_START);
        directional.create_shader(RECT, Some(TextDirection::Rtl));
        directional.create_shader(RECT, Some(TextDirection::Ltr));
        RadialGradient::new(colors(&[0xFFFFFFFF, 0xFFFFFFFF]))
            .center(AlignmentGeometry::TOP_LEFT)
            .create_shader(RECT, None);
    }

    #[test]
    fn radial_gradients_lerp() {
        let test_gradient1 = RadialGradient::new(colors(&[0x33333333, 0x66666666]))
            .center(AlignmentGeometry::TOP_LEFT)
            .radius(20.0);
        let test_gradient2 = RadialGradient::new(colors(&[0x44444444, 0x88888888]))
            .center(AlignmentGeometry::TOP_RIGHT)
            .radius(10.0);
        let actual = RadialGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_radial(
            &actual.unwrap(),
            &RadialGradient::new(colors(&[0x3B3B3B3B, 0x77777777]))
                .center(AlignmentGeometry::TOP_CENTER)
                .radius(15.0)
                .stops(vec![0.0, 1.0]),
        );
    }

    #[test]
    fn radial_gradient_lerp_of_identical_a_and_b_returns_a() {
        assert!(RadialGradient::lerp(None, None, 0.0).is_none());
        let gradient = RadialGradient::new(colors(&[0x33333333, 0x66666666]));
        assert_eq!(
            RadialGradient::lerp(Some(&gradient), Some(&gradient), 0.5).unwrap(),
            gradient
        );
    }

    #[test]
    fn radial_gradients_lerp_with_stops() {
        let test_gradient1 = RadialGradient::new(colors(&[0x33333333, 0x66666666]))
            .center(AlignmentGeometry::TOP_LEFT)
            .radius(20.0)
            .stops(vec![0.0, 0.5]);
        let test_gradient2 = RadialGradient::new(colors(&[0x44444444, 0x88888888]))
            .center(AlignmentGeometry::TOP_RIGHT)
            .radius(10.0)
            .stops(vec![0.5, 1.0]);
        let actual =
            RadialGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5).unwrap();
        assert_matches_radial(
            &actual,
            &RadialGradient::new(colors(&[0x3B3B3B3B, 0x55555555, 0x77777777]))
                .center(AlignmentGeometry::TOP_CENTER)
                .radius(15.0)
                .stops(vec![0.0, 0.5, 1.0]),
        );
        assert!(actual.focal.is_none());
    }

    #[test]
    fn radial_gradients_lerp_with_unequal_numbers_of_colours() {
        let test_gradient1 = RadialGradient::new(colors(&[0x22222222, 0x66666666]));
        let test_gradient2 = RadialGradient::new(colors(&[0x44444444, 0x66666666, 0x88888888]));
        let actual = RadialGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_radial(
            &actual.unwrap(),
            &RadialGradient::new(colors(&[0x33333333, 0x55555555, 0x77777777]))
                .stops(vec![0.0, 0.5, 1.0]),
        );
    }

    #[test]
    fn radial_gradients_lerp_with_stops_and_unequal_numbers_of_colours() {
        let test_gradient1 =
            RadialGradient::new(colors(&[0x33333333, 0x66666666])).stops(vec![0.0, 0.5]);
        let test_gradient2 = RadialGradient::new(colors(&[0x44444444, 0x48484848, 0x88888888]))
            .stops(vec![0.5, 0.7, 1.0]);
        let actual = RadialGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_radial(
            &actual.unwrap(),
            &RadialGradient::new(colors(&[0x3B3B3B3B, 0x55555555, 0x57575757, 0x77777777]))
                .stops(vec![0.0, 0.5, 0.7, 1.0]),
        );
    }

    #[test]
    fn radial_gradients_lerp_with_transforms() {
        let test_gradient1 = RadialGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 4.0))
            .stops(vec![0.0, 1.0]);
        let test_gradient2 = RadialGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 2.0))
            .stops(vec![0.0, 1.0]);
        let lerp = |t| RadialGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), t);
        assert_eq!(Some(test_gradient1.clone()), lerp(0.0));
        assert_eq!(Some(test_gradient2.clone()), lerp(1.0));
        assert_eq!(Some(test_gradient2.clone()), lerp(0.5));
    }

    #[test]
    fn radial_gradients_lerp_with_focal() {
        let test_gradient1 = RadialGradient::new(colors(&[0x33333333, 0x66666666]))
            .center(AlignmentGeometry::TOP_LEFT)
            .focal(AlignmentGeometry::CENTER_LEFT)
            .radius(20.0)
            .focal_radius(10.0);
        let test_gradient2 = RadialGradient::new(colors(&[0x44444444, 0x88888888]))
            .center(AlignmentGeometry::TOP_RIGHT)
            .focal(AlignmentGeometry::CENTER_RIGHT)
            .radius(10.0)
            .focal_radius(5.0);
        let test_gradient3 = RadialGradient::new(colors(&[0x44444444, 0x88888888]))
            .center(AlignmentGeometry::TOP_RIGHT)
            .radius(10.0);

        let actual = RadialGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_radial(
            &actual.unwrap(),
            &RadialGradient::new(colors(&[0x3B3B3B3B, 0x77777777]))
                .center(AlignmentGeometry::TOP_CENTER)
                .focal(AlignmentGeometry::CENTER)
                .radius(15.0)
                .focal_radius(7.5)
                .stops(vec![0.0, 1.0]),
        );

        let actual2 = RadialGradient::lerp(Some(&test_gradient1), Some(&test_gradient3), 0.5);
        assert_matches_radial(
            &actual2.unwrap(),
            &RadialGradient::new(colors(&[0x3B3B3B3B, 0x77777777]))
                .center(AlignmentGeometry::TOP_CENTER)
                .focal(AlignmentGeometry::xy(-0.5, 0.0))
                .radius(15.0)
                .focal_radius(5.0)
                .stops(vec![0.0, 1.0]),
        );
    }

    #[test]
    fn a_radial_gradient_with_opacity_sets_every_alpha() {
        let test_gradient = RadialGradient::new(colors(&[0xFFFFFFFF, 0xAF777777, 0x44444444]))
            .center(AlignmentGeometry::TOP_LEFT)
            .focal(AlignmentGeometry::CENTER_LEFT)
            .radius(20.0)
            .focal_radius(10.0);
        assert_eq!(
            test_gradient.with_opacity(0.5),
            RadialGradient::new(colors(&[0x80FFFFFF, 0x80777777, 0x80444444]))
                .center(AlignmentGeometry::TOP_LEFT)
                .focal(AlignmentGeometry::CENTER_LEFT)
                .radius(20.0)
                .focal_radius(10.0)
        );
    }

    #[test]
    fn a_radial_gradient_with_opacity_keeps_its_transform() {
        let test_gradient = RadialGradient::new(colors(&[0xFFFFFFFF, 0xAF777777, 0x44444444]))
            .center(AlignmentGeometry::TOP_LEFT)
            .focal(AlignmentGeometry::CENTER_LEFT)
            .radius(20.0)
            .focal_radius(10.0)
            .transform(GradientRotation::new(1.0));
        assert_eq!(
            test_gradient.with_opacity(0.5),
            RadialGradient::new(colors(&[0x80FFFFFF, 0x80777777, 0x80444444]))
                .center(AlignmentGeometry::TOP_LEFT)
                .focal(AlignmentGeometry::CENTER_LEFT)
                .radius(20.0)
                .focal_radius(10.0)
                .transform(GradientRotation::new(1.0))
        );
    }

    #[test]
    fn a_scaled_radial_gradient_keeps_its_transform() {
        let test_gradient = RadialGradient::new(colors(&[0xFFFF0000, 0xFF00FF00, 0xFF0000FF]))
            .center(AlignmentGeometry::TOP_LEFT)
            .focal(AlignmentGeometry::CENTER_LEFT)
            .radius(20.0)
            .focal_radius(10.0)
            .stops(vec![0.0, 0.5, 1.0])
            .tile_mode(TileMode::Decal)
            .transform(GradientRotation::new(PI / 4.0));
        assert_matches_radial(
            &test_gradient.scale(0.5),
            &RadialGradient::new(colors(&[0x80FF0000, 0x8000FF00, 0x800000FF]))
                .center(AlignmentGeometry::TOP_LEFT)
                .focal(AlignmentGeometry::CENTER_LEFT)
                .radius(20.0)
                .focal_radius(10.0)
                .stops(vec![0.0, 0.5, 1.0])
                .tile_mode(TileMode::Decal)
                .transform(GradientRotation::new(PI / 4.0)),
        );
    }

    #[test]
    fn sweep_gradients_lerp() {
        let test_gradient1 = SweepGradient::new(colors(&[0x33333333, 0x66666666]))
            .center(AlignmentGeometry::TOP_LEFT)
            .end_angle(PI / 2.0);
        let test_gradient2 = SweepGradient::new(colors(&[0x44444444, 0x88888888]))
            .center(AlignmentGeometry::TOP_RIGHT)
            .start_angle(PI / 2.0)
            .end_angle(PI);
        let actual = SweepGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_sweep(
            &actual.unwrap(),
            &SweepGradient::new(colors(&[0x3B3B3B3B, 0x77777777]))
                .center(AlignmentGeometry::TOP_CENTER)
                .start_angle(PI / 4.0)
                .end_angle(PI * 3.0 / 4.0)
                .stops(vec![0.0, 1.0]),
        );
    }

    #[test]
    fn sweep_gradient_lerp_of_identical_a_and_b_returns_a() {
        assert!(SweepGradient::lerp(None, None, 0.0).is_none());
        let gradient = SweepGradient::new(colors(&[0x33333333, 0x66666666]));
        assert_eq!(
            SweepGradient::lerp(Some(&gradient), Some(&gradient), 0.5).unwrap(),
            gradient
        );
    }

    #[test]
    fn sweep_gradients_lerp_with_stops() {
        let test_gradient1 = SweepGradient::new(colors(&[0x33333333, 0x66666666]))
            .center(AlignmentGeometry::TOP_LEFT)
            .end_angle(PI / 2.0)
            .stops(vec![0.0, 0.5]);
        let test_gradient2 = SweepGradient::new(colors(&[0x44444444, 0x88888888]))
            .center(AlignmentGeometry::TOP_RIGHT)
            .start_angle(PI / 2.0)
            .end_angle(PI)
            .stops(vec![0.5, 1.0]);
        let actual = SweepGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_sweep(
            &actual.unwrap(),
            &SweepGradient::new(colors(&[0x3B3B3B3B, 0x55555555, 0x77777777]))
                .center(AlignmentGeometry::TOP_CENTER)
                .start_angle(PI / 4.0)
                .end_angle(PI * 3.0 / 4.0)
                .stops(vec![0.0, 0.5, 1.0]),
        );
    }

    #[test]
    fn sweep_gradients_lerp_with_unequal_numbers_of_colours() {
        let test_gradient1 = SweepGradient::new(colors(&[0x22222222, 0x66666666]));
        let test_gradient2 = SweepGradient::new(colors(&[0x44444444, 0x66666666, 0x88888888]));
        let actual = SweepGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_sweep(
            &actual.unwrap(),
            &SweepGradient::new(colors(&[0x33333333, 0x55555555, 0x77777777]))
                .stops(vec![0.0, 0.5, 1.0]),
        );
    }

    #[test]
    fn sweep_gradients_lerp_with_stops_and_unequal_numbers_of_colours() {
        let test_gradient1 =
            SweepGradient::new(colors(&[0x33333333, 0x66666666])).stops(vec![0.0, 0.5]);
        let test_gradient2 = SweepGradient::new(colors(&[0x44444444, 0x48484848, 0x88888888]))
            .stops(vec![0.5, 0.7, 1.0]);
        let actual = SweepGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), 0.5);
        assert_matches_sweep(
            &actual.unwrap(),
            &SweepGradient::new(colors(&[0x3B3B3B3B, 0x55555555, 0x57575757, 0x77777777]))
                .stops(vec![0.0, 0.5, 0.7, 1.0]),
        );
    }

    #[test]
    fn sweep_gradients_lerp_with_transforms() {
        let test_gradient1 = SweepGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 4.0))
            .stops(vec![0.0, 1.0]);
        let test_gradient2 = SweepGradient::new(colors(&[0x33333333, 0x66666666]))
            .transform(GradientRotation::new(PI / 2.0))
            .stops(vec![0.0, 1.0]);
        let lerp = |t| SweepGradient::lerp(Some(&test_gradient1), Some(&test_gradient2), t);
        assert_eq!(Some(test_gradient1.clone()), lerp(0.0));
        assert_eq!(Some(test_gradient2.clone()), lerp(1.0));
        assert_eq!(Some(test_gradient2.clone()), lerp(0.5));
    }

    #[test]
    fn a_scaled_sweep_gradient_scales_its_colours() {
        let test_gradient = SweepGradient::new(colors(&[0xff333333, 0xff666666]))
            .center(AlignmentGeometry::TOP_LEFT)
            .end_angle(PI / 2.0);
        assert_matches_sweep(
            &test_gradient.scale(0.5),
            &SweepGradient::new(colors(&[0x80333333, 0x80666666]))
                .center(AlignmentGeometry::TOP_LEFT)
                .end_angle(PI / 2.0),
        );
    }

    #[test]
    fn a_sweep_gradient_with_opacity_sets_every_alpha() {
        let test_gradient = SweepGradient::new(colors(&[0xFFFFFFFF, 0xAF777777, 0x44444444]))
            .center(AlignmentGeometry::TOP_LEFT)
            .end_angle(PI / 2.0);
        assert_eq!(
            test_gradient.with_opacity(0.5),
            SweepGradient::new(colors(&[0x80FFFFFF, 0x80777777, 0x80444444]))
                .center(AlignmentGeometry::TOP_LEFT)
                .end_angle(PI / 2.0)
        );
    }

    #[test]
    fn a_sweep_gradient_with_opacity_keeps_its_transform() {
        let test_gradient = SweepGradient::new(colors(&[0xFFFFFFFF, 0xAF777777, 0x44444444]))
            .center(AlignmentGeometry::TOP_LEFT)
            .end_angle(PI / 2.0)
            .transform(GradientRotation::new(1.0));
        assert_eq!(
            test_gradient.with_opacity(0.5),
            SweepGradient::new(colors(&[0x80FFFFFF, 0x80777777, 0x80444444]))
                .center(AlignmentGeometry::TOP_LEFT)
                .end_angle(PI / 2.0)
                .transform(GradientRotation::new(1.0))
        );
    }

    #[test]
    fn a_scaled_sweep_gradient_keeps_its_transform() {
        let test_gradient = SweepGradient::new(colors(&[0xFFFF0000, 0xFF00FF00, 0xFF0000FF]))
            .center(AlignmentGeometry::TOP_LEFT)
            .start_angle(PI / 4.0)
            .end_angle(PI * 3.0 / 4.0)
            .stops(vec![0.0, 0.5, 1.0])
            .tile_mode(TileMode::Decal)
            .transform(GradientRotation::new(PI / 4.0));
        assert_matches_sweep(
            &test_gradient.scale(0.5),
            &SweepGradient::new(colors(&[0x80FF0000, 0x8000FF00, 0x800000FF]))
                .center(AlignmentGeometry::TOP_LEFT)
                .start_angle(PI / 4.0)
                .end_angle(PI * 3.0 / 4.0)
                .stops(vec![0.0, 0.5, 1.0])
                .tile_mode(TileMode::Decal)
                .transform(GradientRotation::new(PI / 4.0)),
        );
    }

    #[test]
    fn gradient_lerp_interpolates_radial_gradients_both_ways() {
        let test_gradient1 = RadialGradient::new(colors(&[0x33333333, 0x66666666]))
            .center(AlignmentGeometry::TOP_LEFT)
            .radius(20.0)
            .stops(vec![0.0, 1.0]);
        let test_gradient2 = RadialGradient::new(colors(&[0x3B3B3B3B, 0x77777777]))
            .center(AlignmentGeometry::TOP_CENTER)
            .radius(15.0)
            .stops(vec![0.0, 1.0]);
        let test_gradient3 = RadialGradient::new(colors(&[0x44444444, 0x88888888]))
            .center(AlignmentGeometry::TOP_RIGHT)
            .radius(10.0)
            .stops(vec![0.0, 1.0]);
        let lerp = |a: &RadialGradient, b: &RadialGradient, t| {
            <dyn Gradient>::lerp(Some(a), Some(b), t).unwrap()
        };

        assert_matches_radial(
            &*lerp(&test_gradient1, &test_gradient3, 0.0),
            &test_gradient1,
        );
        assert_matches_radial(
            &*lerp(&test_gradient1, &test_gradient3, 0.5),
            &test_gradient2,
        );
        assert_matches_radial(
            &*lerp(&test_gradient1, &test_gradient3, 1.0),
            &test_gradient3,
        );
        assert_matches_radial(
            &*lerp(&test_gradient3, &test_gradient1, 0.0),
            &test_gradient3,
        );
        assert_matches_radial(
            &*lerp(&test_gradient3, &test_gradient1, 0.5),
            &test_gradient2,
        );
        assert_matches_radial(
            &*lerp(&test_gradient3, &test_gradient1, 1.0),
            &test_gradient1,
        );
    }

    #[test]
    fn gradient_lerp_from_a_linear_to_a_radial_gradient_fades_through_nothing() {
        let test_gradient1 = LinearGradient::new(colors(&[0x33333333, 0x66666666]))
            .begin(AlignmentGeometry::TOP_LEFT)
            .end(AlignmentGeometry::BOTTOM_RIGHT);
        let test_gradient2 = RadialGradient::new(colors(&[0x44444444, 0x88888888])).radius(20.0);
        let lerp =
            |t| <dyn Gradient>::lerp(Some(&test_gradient1), Some(&test_gradient2), t).unwrap();

        assert!(lerp(0.0).eq_gradient(&test_gradient1));
        assert!(lerp(1.0).eq_gradient(&test_gradient2));
        assert!(lerp(0.5).eq_gradient(&test_gradient2.scale(0.0)));
    }

    #[test]
    fn gradients_imply_missing_stops_and_reject_mismatched_ones() {
        let three = colors(&[0x11111111, 0x22222222, 0x33333333]);
        let test1a = LinearGradient::new(three.clone());
        let test1b = RadialGradient::new(three.clone());
        let test2a = LinearGradient::new(three.clone()).stops(vec![0.0, 1.0]);
        let test2b = RadialGradient::new(three).stops(vec![0.0, 1.0]);
        let rect = Rect::from_ltwh(1.0, 2.0, 3.0, 4.0);
        assert_eq!(test1a.create_shader(rect, None).stops().len(), 3);
        assert_eq!(test1b.create_shader(rect, None).stops().len(), 3);
        assert!(catch_unwind(AssertUnwindSafe(|| test2a.create_shader(rect, None))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| test2b.create_shader(rect, None))).is_err());
    }

    fn point(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    fn assert_maps(matrix: Matrix4, from: Point, to: Point) {
        let mapped = matrix.map_point(from);
        assert!(
            (mapped.x - to.x).abs() < 1e-3 && (mapped.y - to.y).abs() < 1e-3,
            "{from:?} mapped to {mapped:?}, not {to:?}"
        );
    }

    const TRANSFORMS_RECT: Rect = Rect::from_ltwh(0.0, 0.0, 300.0, 400.0);

    #[test]
    fn a_gradient_rotation_turns_clockwise_about_the_bounds_centre() {
        let quarter = GradientRotation::new(PI / 2.0)
            .transform(TRANSFORMS_RECT, None)
            .unwrap();
        assert_maps(quarter, point(150.0, 200.0), point(150.0, 200.0));
        assert_maps(quarter, point(0.0, 0.0), point(350.0, 50.0));

        let eighth = GradientRotation::new(PI / 4.0)
            .transform(TRANSFORMS_RECT, None)
            .unwrap();
        assert_maps(eighth, point(150.0, 200.0), point(150.0, 200.0));
        let half_root_two = std::f32::consts::FRAC_1_SQRT_2;
        assert_maps(
            eighth,
            point(0.0, 0.0),
            point(150.0 + 50.0 * half_root_two, 200.0 - 350.0 * half_root_two),
        );
    }

    #[test]
    fn each_gradient_hands_its_rotation_to_its_shader() {
        let gradient_colors = colors(&[0xFFFFFFFF, 0xFF000088]);
        let stops = vec![
            GradientStop {
                offset: 0.0,
                color: Color::new(0xFFFFFFFF).into(),
            },
            GradientStop {
                offset: 1.0,
                color: Color::new(0xFF000088).into(),
            },
        ];
        for radians in [PI / 4.0, PI / 2.0] {
            let rotation = GradientRotation::new(radians);
            let local = rotation.transform(TRANSFORMS_RECT, None).unwrap();
            assert_eq!(
                LinearGradient::new(gradient_colors.clone())
                    .transform(rotation)
                    .create_shader(TRANSFORMS_RECT, None),
                Shader::Linear {
                    start: point(0.0, 200.0),
                    end: point(300.0, 200.0),
                    stops: stops.clone(),
                    spread: SpreadMode::Pad,
                    local,
                }
            );
            assert_eq!(
                RadialGradient::new(gradient_colors.clone())
                    .center(AlignmentGeometry::TOP_CENTER)
                    .transform(rotation)
                    .create_shader(TRANSFORMS_RECT, None),
                Shader::Radial {
                    center: point(150.0, 0.0),
                    radius: 150.0,
                    stops: stops.clone(),
                    spread: SpreadMode::Pad,
                    focus: None,
                    local,
                }
            );
            assert_eq!(
                SweepGradient::new(gradient_colors.clone())
                    .transform(rotation)
                    .create_shader(TRANSFORMS_RECT, None),
                Shader::Sweep {
                    center: point(150.0, 200.0),
                    start_angle: 0.0,
                    end_angle: (PI * 2.0) as f32,
                    stops: stops.clone(),
                    spread: SpreadMode::Pad,
                    local,
                }
            );
        }
    }

    #[test]
    fn a_focal_radial_gradient_starts_from_its_focal_circle() {
        let shader = RadialGradient::new(colors(&[0xFFFFFFFF, 0xFF000088]))
            .focal(AlignmentGeometry::TOP_LEFT)
            .focal_radius(0.1)
            .tile_mode(TileMode::Mirror)
            .create_shader(TRANSFORMS_RECT, None);
        let Shader::Radial { focus, spread, .. } = shader else {
            panic!("a radial shader: {shader:?}");
        };
        assert_eq!(
            focus,
            Some(FocalCircle {
                center: point(0.0, 0.0),
                radius: 30.0,
            })
        );
        assert_eq!(spread, SpreadMode::Reflect);
    }

    #[test]
    fn a_focal_point_on_the_centre_is_a_plain_radial_gradient() {
        let shader = RadialGradient::new(colors(&[0xFFFFFFFF, 0xFF000088]))
            .focal(AlignmentGeometry::CENTER)
            .create_shader(TRANSFORMS_RECT, None);
        assert!(
            matches!(shader, Shader::Radial { focus: None, .. }),
            "{shader:?}"
        );
    }

    #[test]
    fn a_linear_gradient_spreads_its_stops_and_tiles_as_asked() {
        let shader = LinearGradient::new(colors(&[0xFF000001, 0xFF000002, 0xFF000003]))
            .tile_mode(TileMode::Repeat)
            .create_shader(TRANSFORMS_RECT, None);
        let Shader::Linear {
            stops,
            spread,
            local,
            ..
        } = shader
        else {
            panic!("a linear shader: {shader:?}");
        };
        assert_eq!(
            stops.iter().map(|stop| stop.offset).collect::<Vec<_>>(),
            [0.0, 0.5, 1.0]
        );
        assert_eq!(spread, SpreadMode::Repeat);
        assert_eq!(local, Matrix4::IDENTITY);
    }
}
