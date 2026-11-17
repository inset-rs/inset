//! flutter_test's `paints` matcher (`mock_canvas.dart`), over the display list a render object or
//! a painter records.
//!
//! Valo's display list is not a log of canvas calls: it records `drawCircle`, `drawRRect`,
//! `drawOval` and `drawPath` as one op, a path that answers its shape, and drops a draw with no
//! effect (`DisplayListBuilder::PaintResult`). So a call is told apart by its path's shape
//! ([`CanvasMethod::of`]), a rounded rect or a circle matches by its geometry within f32
//! rounding, and a draw with no effect is never there to match.
//!
//! [`paints`] answers why a pattern does not match ([`PaintMismatch`]): which step failed, the
//! call it stopped at and every call recorded, as Dart's matcher describes its failure.

use std::fmt;
use std::rc::Rc;

use waft_embedder::valo::{self, FillRule, Op, PaintStyle};
use waft_embedder::{Color, Offset, RRect, Rect, StrokeCap};

/// The canvas method a recorded op stands for, as flutter_test's `TestRecordingCanvas` names it.
///
/// Valo records `drawCircle`, `drawRRect`, `drawPath` and `drawOval` as one op; they are told
/// apart by the path (an oval, a rounded rect, any other). A `drawDRRect` is the even-odd path
/// between two contours that `draw_drrect` records, a `drawLine` the open path of two points that
/// `draw_line` records, and a `drawRRect` of a rounded rect without corners a `drawRect`
/// (`draw_rrect`). A `drawArc` is a path, which [`PaintCall::Arc`] reads by where it runs. A
/// `drawRect` or `drawRRect` whose paint has a mask filter is valo's blurred (r)rect. A transform
/// that is both, such as the identity, is a `translate` and a `scale`; [`of`](Self::of) answers
/// the translation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasMethod {
    Save,
    Restore,
    Translate,
    Scale,
    Rotate,
    Clip,
    DrawRect,
    DrawRRect,
    DrawDRRect,
    DrawCircle,
    DrawLine,
    DrawPath,
    DrawShadow,
    DrawParagraph,
    DrawPaint,
    Other,
}

impl CanvasMethod {
    /// The method `op` records.
    pub fn of(op: &Op) -> CanvasMethod {
        match op {
            Op::Save | Op::SaveLayer { .. } => CanvasMethod::Save,
            Op::Restore => CanvasMethod::Restore,
            Op::Transform(matrix) => match Transform::of(matrix) {
                Transform::Translate(..) => CanvasMethod::Translate,
                Transform::Scale(..) => CanvasMethod::Scale,
                Transform::Other(_) => CanvasMethod::Rotate,
            },
            Op::ClipPath { .. } => CanvasMethod::Clip,
            Op::DrawPath {
                path, fill_rule, ..
            } => {
                if path.as_oval().is_some() {
                    CanvasMethod::DrawCircle
                } else if path.as_round_rect().is_some() {
                    CanvasMethod::DrawRRect
                } else if is_drrect(path, *fill_rule) {
                    CanvasMethod::DrawDRRect
                } else if line_of(path).is_some() {
                    CanvasMethod::DrawLine
                } else {
                    CanvasMethod::DrawPath
                }
            }
            Op::DrawRect { .. } => CanvasMethod::DrawRect,
            // A `drawRect` or `drawRRect` whose paint has a mask filter (a box shadow's).
            Op::RRectBlur { radii, .. } if radii.iter().all(|radius| *radius == 0.0) => {
                CanvasMethod::DrawRect
            }
            Op::RRectBlur { .. } => CanvasMethod::DrawRRect,
            Op::DrawShadow { .. } => CanvasMethod::DrawShadow,
            Op::GlyphRun { .. } => CanvasMethod::DrawParagraph,
            Op::DrawPaint { .. } => CanvasMethod::DrawPaint,
            _ => CanvasMethod::Other,
        }
    }
}

/// A recorded transform, as the canvas call it stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Transform {
    Translate(f64, f64),
    Scale(f64, f64),
    /// Any other affine transform: `[a, b, c, d, tx, ty]`.
    Other([f64; 6]),
}

impl Transform {
    fn of(matrix: &valo::Matrix) -> Transform {
        let [a, b, c, d, tx, ty] = matrix.to_affine().map(f64::from);
        if b == 0.0 && c == 0.0 && a == 1.0 && d == 1.0 {
            Transform::Translate(tx, ty)
        } else if b == 0.0 && c == 0.0 && tx == 0.0 && ty == 0.0 {
            Transform::Scale(a, d)
        } else {
            Transform::Other([a, b, c, d, tx, ty])
        }
    }

    /// The scale it makes, which a translation makes by one.
    fn scale(self) -> Option<(f64, f64)> {
        match self {
            Transform::Scale(x, y) => Some((x, y)),
            Transform::Translate(0.0, 0.0) => Some((1.0, 1.0)),
            _ => None,
        }
    }
}

/// Whether `path`, filled with `fill_rule`, is what `draw_drrect` records for `drawDRRect`: the
/// area between two contours, filled even-odd. Valo keeps no record of the call, so a `drawPath`
/// of such a path reads as a `drawDRRect` too.
fn is_drrect(path: &valo::Path, fill_rule: FillRule) -> bool {
    fill_rule == FillRule::EvenOdd && path.flatten(1.0).len() == 2
}

/// The two ends of `path` if it is what `draw_line` records: one open contour of two points.
fn line_of(path: &valo::Path) -> Option<(Offset, Offset)> {
    let contours = path.flatten(1.0);
    match contours.as_slice() {
        [contour] if !contour.closed && contour.points.len() == 2 => {
            let point = |p: valo::Point| Offset::new(f64::from(p.x), f64::from(p.y));
            Some((point(contour.points[0]), point(contour.points[1])))
        }
        _ => None,
    }
}

/// One recorded op, as the canvas call it stands for with the arguments a pattern reads: what a
/// [`PaintMismatch`] lists, and what a [`PaintCall::something`] predicate is handed.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordedCall {
    Save,
    Restore,
    Translate {
        x: f64,
        y: f64,
    },
    Scale {
        x: f64,
        y: f64,
    },
    /// A transform that is neither a translation nor a scale: `[a, b, c, d, tx, ty]`.
    Transform {
        affine: [f64; 6],
    },
    Clip,
    Rect {
        rect: Rect,
        color: Color,
    },
    /// `drawCircle`, or a `drawOval` of a circle's box.
    Circle {
        center: Offset,
        radius: f64,
        color: Color,
        stroke_width: Option<f64>,
    },
    RRect {
        bounds: Rect,
        color: Color,
        stroke_width: Option<f64>,
    },
    DRRect {
        color: Color,
    },
    Line {
        p1: Offset,
        p2: Offset,
        color: Color,
        stroke_width: Option<f64>,
    },
    Path {
        bounds: Rect,
        color: Color,
        stroke_width: Option<f64>,
    },
    Shadow {
        elevation: f64,
        color: Color,
    },
    Paragraph,
    Paint {
        color: Color,
    },
    Other,
}

impl RecordedCall {
    /// What `op` records.
    pub fn of(op: &Op) -> RecordedCall {
        match op {
            Op::Save | Op::SaveLayer { .. } => RecordedCall::Save,
            Op::Restore => RecordedCall::Restore,
            Op::Transform(matrix) => match Transform::of(matrix) {
                Transform::Translate(x, y) => RecordedCall::Translate { x, y },
                Transform::Scale(x, y) => RecordedCall::Scale { x, y },
                Transform::Other(affine) => RecordedCall::Transform { affine },
            },
            Op::ClipPath { .. } => RecordedCall::Clip,
            Op::DrawRect { rect, paint, .. } => RecordedCall::Rect {
                rect: rect_of(*rect),
                color: paint.color.into(),
            },
            Op::RRectBlur {
                rect, radii, paint, ..
            } if radii.iter().all(|radius| *radius == 0.0) => RecordedCall::Rect {
                rect: rect_of(*rect),
                color: paint.color.into(),
            },
            Op::RRectBlur { rect, paint, .. } => RecordedCall::RRect {
                bounds: rect_of(*rect),
                color: paint.color.into(),
                stroke_width: None,
            },
            Op::DrawPath {
                path,
                fill_rule,
                paint,
                ..
            } => {
                let color = paint.color.into();
                let stroke_width = stroke_width_of(&paint.style);
                if let Some(oval) = path.as_oval() {
                    RecordedCall::Circle {
                        center: Offset::new(
                            f64::from(oval.x + oval.width / 2.0),
                            f64::from(oval.y + oval.height / 2.0),
                        ),
                        radius: f64::from(oval.width / 2.0),
                        color,
                        stroke_width,
                    }
                } else if path.as_round_rect().is_some() {
                    RecordedCall::RRect {
                        bounds: rect_of(path.bounds()),
                        color,
                        stroke_width,
                    }
                } else if is_drrect(path, *fill_rule) {
                    RecordedCall::DRRect { color }
                } else if let Some((p1, p2)) = line_of(path) {
                    RecordedCall::Line {
                        p1,
                        p2,
                        color,
                        stroke_width,
                    }
                } else {
                    RecordedCall::Path {
                        bounds: rect_of(path.bounds()),
                        color,
                        stroke_width,
                    }
                }
            }
            Op::DrawShadow {
                elevation, color, ..
            } => RecordedCall::Shadow {
                elevation: f64::from(*elevation),
                color: (*color).into(),
            },
            Op::GlyphRun { .. } => RecordedCall::Paragraph,
            Op::DrawPaint { paint, .. } => RecordedCall::Paint {
                color: paint.color.into(),
            },
            _ => RecordedCall::Other,
        }
    }
}

fn stroke_width_of(style: &PaintStyle) -> Option<f64> {
    match style {
        PaintStyle::Stroke(stroke) => Some(f64::from(stroke.width)),
        PaintStyle::Fill => None,
    }
}

fn rect_of(rect: valo::Rect) -> Rect {
    Rect::from_ltwh(
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.width),
        f64::from(rect.height),
    )
}

/// Dart's `style:` and `strokeWidth:` of a `paints` step: filled, or stroked at the width given,
/// any width for `None`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaintedStyle {
    Fill,
    Stroke(Option<f64>),
}

impl PaintedStyle {
    fn matches(self, style: &PaintStyle) -> bool {
        match (self, style) {
            (PaintedStyle::Fill, PaintStyle::Fill) => true,
            (PaintedStyle::Stroke(width), PaintStyle::Stroke(stroke)) => {
                width.is_none_or(|width| f32_is(f64::from(stroke.width), width))
            }
            _ => false,
        }
    }
}

/// The predicate of [`PaintCall::something`]: `Ok(true)` stops the step at the call handed,
/// `Ok(false)` goes on to the next, and `Err` fails the pattern with its message.
pub type SomethingPredicate = Rc<dyn Fn(&RecordedCall) -> Result<bool, String>>;

/// One step of Dart's `paints` pattern (`paints..circle()..rect()`), each argument checked only
/// when given.
#[derive(Clone)]
pub enum PaintCall {
    /// `..save()`.
    Save,
    /// `..restore()`.
    Restore,
    /// `..translate(x:, y:)`.
    Translate { x: Option<f64>, y: Option<f64> },
    /// `..scale(x:, y:)`.
    Scale { x: Option<f64>, y: Option<f64> },
    /// `..rotate(angle:)`.
    Rotate { angle: Option<f64> },
    /// `..clipRect()`, `..clipRRect()` or `..clipPath()`: valo records each as a path clip.
    Clip,
    /// `..path(color:, style:, strokeWidth:, includes:, excludes:)`: `includes` are points the
    /// path's fill covers and `excludes` points it does not.
    Path {
        color: Option<Color>,
        style: Option<PaintedStyle>,
        includes: Vec<Offset>,
        excludes: Vec<Offset>,
    },
    /// `..line(p1:, p2:, color:, strokeWidth:)`.
    Line {
        p1: Option<Offset>,
        p2: Option<Offset>,
        color: Option<Color>,
        stroke_width: Option<f64>,
    },
    /// `..arc(rect:, color:, strokeWidth:, strokeCap:)`: a stroked path that runs along the oval
    /// inscribed in `rect`. Valo records an arc as a path, so this step looks at the next path.
    Arc {
        rect: Option<Rect>,
        color: Option<Color>,
        stroke_width: Option<f64>,
        stroke_cap: Option<StrokeCap>,
    },
    /// `..rect(rect:, color:)`.
    Rect {
        rect: Option<Rect>,
        color: Option<Color>,
    },
    /// `..rrect(rrect:, color:, style:, strokeWidth:, hasMaskFilter:)`.
    RRect {
        rrect: Option<RRect>,
        color: Option<Color>,
        style: Option<PaintedStyle>,
        has_mask_filter: Option<bool>,
    },
    /// `..drrect(outer:, inner:, color:, style:, strokeWidth:)`.
    DRRect {
        outer: Option<RRect>,
        inner: Option<RRect>,
        color: Option<Color>,
        style: Option<PaintedStyle>,
    },
    /// `..circle(x:, y:, radius:, color:, style:, strokeWidth:)`.
    Circle {
        x: Option<f64>,
        y: Option<f64>,
        radius: Option<f64>,
        color: Option<Color>,
        style: Option<PaintedStyle>,
    },
    /// `..shadow(elevation:, color:)`.
    Shadow {
        elevation: Option<f64>,
        color: Option<Color>,
    },
    /// `..paragraph()`.
    Paragraph,
    /// `..saveRestore()`: a `save` and the `restore` that balances it, with whatever lies
    /// between.
    SaveRestore,
    /// `..something(predicate)`.
    Something(SomethingPredicate),
}

/// How an op reads as a call of a step's method. Valo records some calls of two methods alike
/// (a rounded rect without corners and a rect, a straight path and a line); such an op could be
/// either, and a step takes it only when its arguments match, reading it otherwise as the other
/// method's call, which the step skips as Dart's skips another method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    Is,
    Could,
    Not,
}

impl PaintCall {
    /// `..translate(x:, y:)`.
    pub fn translate(x: Option<f64>, y: Option<f64>) -> PaintCall {
        PaintCall::Translate { x, y }
    }

    /// `..scale(x:, y:)`.
    pub fn scale(x: Option<f64>, y: Option<f64>) -> PaintCall {
        PaintCall::Scale { x, y }
    }

    /// `..rotate(angle:)`.
    pub fn rotate(angle: Option<f64>) -> PaintCall {
        PaintCall::Rotate { angle }
    }

    /// `..path(color:)`.
    pub fn path(color: Option<Color>) -> PaintCall {
        PaintCall::Path {
            color,
            style: None,
            includes: Vec::new(),
            excludes: Vec::new(),
        }
    }

    /// `..path(includes:, excludes:)`.
    pub fn path_covering(includes: Vec<Offset>, excludes: Vec<Offset>) -> PaintCall {
        PaintCall::Path {
            color: None,
            style: None,
            includes,
            excludes,
        }
    }

    /// `..line(p1:, p2:, strokeWidth:)`.
    pub fn line(p1: Offset, p2: Offset, stroke_width: Option<f64>) -> PaintCall {
        PaintCall::Line {
            p1: Some(p1),
            p2: Some(p2),
            color: None,
            stroke_width,
        }
    }

    /// `..arc(rect:)`.
    pub fn arc(rect: Option<Rect>) -> PaintCall {
        PaintCall::Arc {
            rect,
            color: None,
            stroke_width: None,
            stroke_cap: None,
        }
    }

    /// `..rect(color:)`.
    pub fn rect(color: Option<Color>) -> PaintCall {
        PaintCall::Rect { rect: None, color }
    }

    /// `..rect(rect:, color:)`.
    pub fn rect_at(rect: Rect, color: Option<Color>) -> PaintCall {
        PaintCall::Rect {
            rect: Some(rect),
            color,
        }
    }

    /// `..rrect(color:)`.
    pub fn rrect(color: Option<Color>) -> PaintCall {
        PaintCall::RRect {
            rrect: None,
            color,
            style: None,
            has_mask_filter: None,
        }
    }

    /// `..rrect(rrect:, color:)`.
    pub fn rrect_at(rrect: RRect, color: Option<Color>) -> PaintCall {
        PaintCall::RRect {
            rrect: Some(rrect),
            color,
            style: None,
            has_mask_filter: None,
        }
    }

    /// `..drrect(color:)`.
    pub fn drrect(color: Option<Color>) -> PaintCall {
        PaintCall::DRRect {
            outer: None,
            inner: None,
            color,
            style: None,
        }
    }

    /// `..drrect(outer:, inner:, color:)`: the ring between `outer` and `inner`.
    pub fn drrect_between(outer: RRect, inner: RRect, color: Option<Color>) -> PaintCall {
        PaintCall::DRRect {
            outer: Some(outer),
            inner: Some(inner),
            color,
            style: None,
        }
    }

    /// `..circle(color:)`.
    pub fn circle(color: Option<Color>) -> PaintCall {
        PaintCall::Circle {
            x: None,
            y: None,
            radius: None,
            color,
            style: None,
        }
    }

    /// `..circle(color:, style: PaintingStyle.stroke, strokeWidth:)`.
    pub fn stroked_circle(color: Color, stroke_width: f64) -> PaintCall {
        PaintCall::Circle {
            x: None,
            y: None,
            radius: None,
            color: Some(color),
            style: Some(PaintedStyle::Stroke(Some(stroke_width))),
        }
    }

    /// `..shadow(color:)`.
    pub fn shadow(color: Option<Color>) -> PaintCall {
        PaintCall::Shadow {
            elevation: None,
            color,
        }
    }

    /// `..shadow(elevation:, color:)`.
    pub fn shadow_with_elevation(elevation: f64, color: Option<Color>) -> PaintCall {
        PaintCall::Shadow {
            elevation: Some(elevation),
            color,
        }
    }

    /// `..something(predicate)`.
    pub fn something(
        predicate: impl Fn(&RecordedCall) -> Result<bool, String> + 'static,
    ) -> PaintCall {
        PaintCall::Something(Rc::new(predicate))
    }

    /// Whether `op` is a call of this step's method, whatever its arguments: [`Reading::Is`]
    /// when the op is what only that method records, [`Reading::Could`] when valo records that
    /// method and another one alike.
    fn reading(&self, op: &Op) -> Reading {
        let method = CanvasMethod::of(op);
        let is = |yes: bool| if yes { Reading::Is } else { Reading::Not };
        match self {
            PaintCall::Save | PaintCall::SaveRestore => is(method == CanvasMethod::Save),
            PaintCall::Restore => is(method == CanvasMethod::Restore),
            PaintCall::Translate { .. } => is(method == CanvasMethod::Translate),
            PaintCall::Scale { .. } => is(scale_of(op).is_some()),
            PaintCall::Rotate { .. } => is(method == CanvasMethod::Rotate),
            PaintCall::Clip => is(method == CanvasMethod::Clip),
            // A `drawPath` of a straight run of two points is recorded as a `drawLine`.
            PaintCall::Path { .. } if method == CanvasMethod::DrawLine => Reading::Could,
            PaintCall::Path { .. } | PaintCall::Arc { .. } => is(method == CanvasMethod::DrawPath),
            PaintCall::Line { .. } => is(method == CanvasMethod::DrawLine),
            // `draw_rrect` records a rounded rect without corners (a rectangular
            // `PhysicalModel`'s fill, Dart's `drawRRect`) as a `drawRect`.
            PaintCall::Rect { .. } if method == CanvasMethod::DrawRect => Reading::Could,
            PaintCall::Rect { .. } => Reading::Not,
            // `draw_rrect` records a rounded rect without corners as a `drawRect`, one whose
            // corners make an oval as a `drawCircle`, and one whose corners are not circular as a
            // `drawPath`.
            PaintCall::RRect { rrect: None, .. } => match method {
                CanvasMethod::DrawRRect => Reading::Is,
                CanvasMethod::DrawRect | CanvasMethod::DrawCircle | CanvasMethod::DrawPath => {
                    Reading::Could
                }
                _ => Reading::Not,
            },
            // Recorded as another method's call, the rounded rect is that method's too.
            PaintCall::RRect {
                rrect: Some(rrect), ..
            } => match drawn_rrect_method(*rrect) {
                drawn if drawn != method => Reading::Not,
                CanvasMethod::DrawRRect => Reading::Is,
                _ => Reading::Could,
            },
            PaintCall::DRRect { .. } => is(method == CanvasMethod::DrawDRRect),
            PaintCall::Circle { .. } => is(method == CanvasMethod::DrawCircle),
            PaintCall::Shadow { .. } => is(method == CanvasMethod::DrawShadow),
            PaintCall::Paragraph => is(method == CanvasMethod::DrawParagraph),
            PaintCall::Something(_) => Reading::Is,
        }
    }

    /// Whether this step takes `op`: a call of its method, or, of a record two methods share,
    /// one with the step's arguments; [`matches`](Self::matches) then checks the arguments.
    fn takes(&self, op: &Op) -> bool {
        match self.reading(op) {
            Reading::Is => true,
            Reading::Could => self.matches(op),
            Reading::Not => false,
        }
    }

    /// Whether `op`, a call of this step's method, has the arguments given, compared exactly as
    /// Dart's predicates do, at the f32 precision valo records in.
    fn matches(&self, op: &Op) -> bool {
        match (self, op) {
            (PaintCall::Translate { x, y }, Op::Transform(matrix)) => {
                let Transform::Translate(tx, ty) = Transform::of(matrix) else {
                    return false;
                };
                x.is_none_or(|x| near(tx, x)) && y.is_none_or(|y| near(ty, y))
            }
            (PaintCall::Scale { x, y }, op) => scale_of(op).is_some_and(|(sx, sy)| {
                x.is_none_or(|x| near(sx, x)) && y.is_none_or(|y| near(sy, y))
            }),
            (PaintCall::Rotate { angle }, Op::Transform(matrix)) => {
                let [a, b, ..] = matrix.to_affine().map(f64::from);
                angle.is_none_or(|angle| near(b.atan2(a), angle))
            }
            (
                PaintCall::Path {
                    color,
                    style,
                    includes,
                    excludes,
                },
                Op::DrawPath { path, paint, .. },
            ) => {
                let covers = |point: &Offset| path.contains((*point).into(), FillRule::NonZero);
                color_matches(paint.color, *color)
                    && style_matches(*style, &paint.style)
                    && includes.iter().all(covers)
                    && !excludes.iter().any(covers)
            }
            (
                PaintCall::Line {
                    p1,
                    p2,
                    color,
                    stroke_width,
                },
                Op::DrawPath { path, paint, .. },
            ) => {
                let Some((start, end)) = line_of(path) else {
                    return false;
                };
                color_matches(paint.color, *color)
                    && p1.is_none_or(|p1| same_point(start, p1))
                    && p2.is_none_or(|p2| same_point(end, p2))
                    && stroke_width.is_none_or(|width| {
                        stroke_width_of(&paint.style).is_some_and(|w| f32_is(w, width))
                    })
            }
            (
                PaintCall::Arc {
                    rect,
                    color,
                    stroke_width,
                    stroke_cap,
                },
                Op::DrawPath { path, paint, .. },
            ) => {
                let PaintStyle::Stroke(stroke) = &paint.style else {
                    return false;
                };
                color_matches(paint.color, *color)
                    && stroke_width.is_none_or(|width| f32_is(f64::from(stroke.width), width))
                    && stroke_cap.is_none_or(|cap| stroke.cap == cap)
                    && rect.is_none_or(|rect| runs_along_oval(path, rect))
            }
            (
                PaintCall::Rect { rect, color },
                Op::DrawRect {
                    rect: painted,
                    paint,
                    ..
                },
            ) => {
                color_matches(paint.color, *color)
                    && rect.is_none_or(|rect| valo::Rect::from(rect) == *painted)
            }
            (
                PaintCall::DRRect {
                    outer,
                    inner,
                    color,
                    style,
                },
                Op::DrawPath { path, paint, .. },
            ) => {
                color_matches(paint.color, *color)
                    && style_matches(*style, &paint.style)
                    && outer
                        .zip(*inner)
                        .is_none_or(|(outer, inner)| same_ring(path, outer, inner))
            }
            (
                PaintCall::RRect {
                    rrect,
                    color,
                    style,
                    has_mask_filter,
                },
                Op::DrawPath { path, paint, .. },
            ) => {
                color_matches(paint.color, *color)
                    && style_matches(*style, &paint.style)
                    && mask_matches(*has_mask_filter, paint)
                    && rrect.is_none_or(|rrect| same_round_rect(path, rrect))
            }
            (
                PaintCall::RRect {
                    rrect,
                    color,
                    style,
                    has_mask_filter,
                },
                Op::DrawRect { rect, paint, .. },
            ) => {
                color_matches(paint.color, *color)
                    && style_matches(*style, &paint.style)
                    && mask_matches(*has_mask_filter, paint)
                    && rrect
                        .is_none_or(|rrect| same_rect(valo::Rect::from(rrect.outer_rect()), *rect))
            }
            (
                PaintCall::RRect {
                    rrect,
                    color,
                    style,
                    has_mask_filter,
                },
                Op::RRectBlur {
                    rect, radii, paint, ..
                },
            ) => {
                color_matches(paint.color, *color)
                    && style_matches(*style, &paint.style)
                    && mask_matches(*has_mask_filter, paint)
                    && rrect.is_none_or(|rrect| same_blurred_round_rect(*rect, *radii, rrect))
            }
            (
                PaintCall::Rect { rect, color },
                Op::RRectBlur {
                    rect: painted,
                    paint,
                    ..
                },
            ) => {
                color_matches(paint.color, *color)
                    && rect.is_none_or(|rect| valo::Rect::from(rect) == *painted)
            }
            (
                PaintCall::Circle {
                    x,
                    y,
                    radius,
                    color,
                    style,
                },
                Op::DrawPath { path, paint, .. },
            ) => {
                let Some(oval) = path.as_oval() else {
                    return false;
                };
                color_matches(paint.color, *color)
                    && style_matches(*style, &paint.style)
                    && x.is_none_or(|x| f32_is(f64::from(oval.x + oval.width / 2.0), x))
                    && y.is_none_or(|y| f32_is(f64::from(oval.y + oval.height / 2.0), y))
                    && radius.is_none_or(|radius| f32_is(f64::from(oval.width / 2.0), radius))
            }
            (
                PaintCall::Shadow { elevation, color },
                Op::DrawShadow {
                    color: shadow,
                    elevation: shadow_elevation,
                    ..
                },
            ) => {
                color_matches(*shadow, *color)
                    && elevation
                        .is_none_or(|elevation| f32_is(f64::from(*shadow_elevation), elevation))
            }
            (PaintCall::Save, _)
            | (PaintCall::Restore, _)
            | (PaintCall::Clip, _)
            | (PaintCall::Paragraph, _) => true,
            _ => false,
        }
    }
}

/// Dart's `hasMaskFilter:`: whether the paint blurs what it draws.
fn mask_matches(expected: Option<bool>, paint: &valo::Paint) -> bool {
    expected.is_none_or(|expected| paint.mask_blur.is_some() == expected)
}

fn style_matches(expected: Option<PaintedStyle>, style: &PaintStyle) -> bool {
    expected.is_none_or(|expected| expected.matches(style))
}

fn same_point(a: Offset, b: Offset) -> bool {
    f32_is(a.dx(), b.dx()) && f32_is(a.dy(), b.dy())
}

/// Whether `path` runs along the oval inscribed in `rect`, sampled at every tenth of each
/// contour's length: an arc that `draw_arc(rect, ..)` drew.
fn runs_along_oval(path: &valo::Path, rect: Rect) -> bool {
    let center = rect.center();
    let (radius_x, radius_y) = (rect.width() / 2.0, rect.height() / 2.0);
    let contours = path.measure(0.01);
    !contours.is_empty()
        && contours.iter().all(|contour| {
            (0..=10).all(|step| {
                let point = contour
                    .sample(contour.length() * step as f32 / 10.0)
                    .position;
                let x = (f64::from(point.x) - center.dx()) / radius_x;
                let y = (f64::from(point.y) - center.dy()) / radius_y;
                (x * x + y * y - 1.0).abs() < 1e-3
            })
        })
}

/// The scale `op` makes, if it is a transform that only scales, the identity included.
fn scale_of(op: &Op) -> Option<(f64, f64)> {
    match op {
        Op::Transform(matrix) => Transform::of(matrix).scale(),
        _ => None,
    }
}

/// Whether `path` is the rounded rect `draw_rrect` records for `rrect`: Dart's
/// `_RRectPaintPredicate`, each side and radius within its 0.0001. A rounded rect whose corners
/// make an oval is recorded as that oval.
fn same_round_rect(path: &valo::Path, rrect: RRect) -> bool {
    let mut canvas = waft_embedder::Canvas::new();
    waft_painting::draw_rrect(&mut canvas, rrect, &waft_embedder::Paint::default());
    let expected = canvas.build();
    let Some(Op::DrawPath { path: expected, .. }) = expected.ops().first() else {
        return false;
    };
    match (round_rect_of(path), round_rect_of(expected)) {
        (Some(recorded), Some(expected)) => same_corners(recorded, expected),
        _ => *path == **expected,
    }
}

/// Whether the blurred (r)rect over `rect` with circular `radii` is what `draw_rrect` records
/// for `rrect` with a mask filter, within Dart's `_RRectPaintPredicate` tolerance.
fn same_blurred_round_rect(rect: valo::Rect, radii: [f32; 4], rrect: RRect) -> bool {
    let mut canvas = waft_embedder::Canvas::new();
    waft_painting::draw_rrect(&mut canvas, rrect, &waft_embedder::Paint::default());
    let expected = match canvas.build().ops().first() {
        Some(Op::DrawPath { path, .. }) => round_rect_of(path),
        Some(Op::DrawRect { rect, .. }) => Some((*rect, [[0.0; 2]; 4])),
        _ => None,
    };
    expected.is_some_and(|expected| {
        same_corners((rect, radii.map(|radius| [radius, radius])), expected)
    })
}

/// Whether two rounded rects' sides and radii are within Dart's `_RRectPaintPredicate`
/// tolerance of each other.
fn same_corners(
    (rect, radii): (valo::Rect, [[f32; 2]; 4]),
    (expected_rect, expected_radii): (valo::Rect, [[f32; 2]; 4]),
) -> bool {
    same_rect(rect, expected_rect)
        && radii
            .iter()
            .flatten()
            .zip(expected_radii.iter().flatten())
            .all(|(a, b)| within_rrect_eps(*a, *b))
}

/// The rect and the per-corner radii of `path` if it was built as a rounded rect or an oval.
fn round_rect_of(path: &valo::Path) -> Option<(valo::Rect, [[f32; 2]; 4])> {
    path.as_round_rect().or_else(|| {
        path.as_oval()
            .map(|oval| (oval, [[oval.width / 2.0, oval.height / 2.0]; 4]))
    })
}

/// Whether each side of `a` is within Dart's `_RRectPaintPredicate` tolerance of `b`'s.
fn same_rect(a: valo::Rect, b: valo::Rect) -> bool {
    [
        (a.x, b.x),
        (a.y, b.y),
        (a.right(), b.right()),
        (a.bottom(), b.bottom()),
    ]
    .iter()
    .all(|(a, b)| within_rrect_eps(*a, *b))
}

fn within_rrect_eps(a: f32, b: f32) -> bool {
    const EPS: f64 = 0.0001;
    (f64::from(a) - f64::from(b)).abs() <= EPS
}

/// Whether `path` is the ring `draw_drrect` records between `outer` and `inner`.
fn same_ring(path: &valo::Path, outer: RRect, inner: RRect) -> bool {
    let mut canvas = waft_embedder::Canvas::new();
    waft_painting::draw_drrect(&mut canvas, outer, inner, &waft_embedder::Paint::default());
    let ring = canvas.build();
    matches!(ring.ops().first(), Some(Op::DrawPath { path: expected, .. }) if **expected == *path)
}

/// The method valo records `draw_rrect(rrect)` as: `drawRect` for a rounded rect without corners,
/// `drawCircle` for one whose corners make an oval, `drawRRect` for the rest.
fn drawn_rrect_method(rrect: RRect) -> CanvasMethod {
    let mut canvas = waft_embedder::Canvas::new();
    waft_painting::draw_rrect(&mut canvas, rrect, &waft_embedder::Paint::default());
    canvas
        .build()
        .ops()
        .first()
        .map_or(CanvasMethod::Other, CanvasMethod::of)
}

/// Dart's exact `==` on a value valo recorded as an `f32`: equal within that value's rounding.
fn f32_is(recorded: f64, expected: f64) -> bool {
    (recorded - expected).abs() <= f64::from(f32::EPSILON) * expected.abs().max(1.0)
}

/// Equal within what a canvas transform keeps in f32 and Waft's `Matrix4` rounds, where Dart's
/// are f64.
fn near(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-4
}

fn color_matches(painted: valo::Color, expected: Option<Color>) -> bool {
    expected.is_none_or(|expected| colors_match(painted, expected))
}

/// flutter_test's `_colorsMatch`: each channel within 1/255.
pub fn colors_match(painted: valo::Color, expected: Color) -> bool {
    const LIMIT: f64 = 1.0 / 255.0;
    (f64::from(painted.a) - expected.a).abs() < LIMIT
        && (f64::from(painted.r) - expected.r).abs() < LIMIT
        && (f64::from(painted.g) - expected.g).abs() < LIMIT
        && (f64::from(painted.b) - expected.b).abs() < LIMIT
}

/// Why a `paints` pattern did not match: the step that failed, the call it stopped at, and
/// every call recorded.
#[derive(Clone, Debug)]
pub struct PaintMismatch {
    /// The index of the step that failed.
    pub step: usize,
    /// That step, as written.
    pub expected: String,
    /// The call it failed on, with its index in [`log`](Self::log); `None` when the calls ran
    /// out first.
    pub found: Option<(usize, RecordedCall)>,
    /// What went wrong, in Dart's words where it has them.
    pub reason: String,
    /// Every call recorded, in order.
    pub log: Vec<RecordedCall>,
}

impl fmt::Display for PaintMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "the paints pattern failed at step {} ({}): {}",
            self.step, self.expected, self.reason
        )?;
        match &self.found {
            Some((index, call)) => writeln!(f, "found: #{index} {call:?}")?,
            None => writeln!(f, "found: no call left")?,
        }
        writeln!(f, "log:")?;
        for (index, call) in self.log.iter().enumerate() {
            writeln!(f, "  #{index} {call:?}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for PaintCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaintCall::Save => f.write_str("save"),
            PaintCall::Restore => f.write_str("restore"),
            PaintCall::Translate { x, y } => write!(f, "translate(x: {x:?}, y: {y:?})"),
            PaintCall::Scale { x, y } => write!(f, "scale(x: {x:?}, y: {y:?})"),
            PaintCall::Rotate { angle } => write!(f, "rotate(angle: {angle:?})"),
            PaintCall::Clip => f.write_str("clip"),
            PaintCall::Path {
                color,
                style,
                includes,
                excludes,
            } => write!(
                f,
                "path(color: {color:?}, style: {style:?}, includes: {includes:?}, \
                 excludes: {excludes:?})"
            ),
            PaintCall::Line {
                p1,
                p2,
                color,
                stroke_width,
            } => write!(
                f,
                "line(p1: {p1:?}, p2: {p2:?}, color: {color:?}, stroke_width: {stroke_width:?})"
            ),
            PaintCall::Arc {
                rect,
                color,
                stroke_width,
                stroke_cap,
            } => write!(
                f,
                "arc(rect: {rect:?}, color: {color:?}, stroke_width: {stroke_width:?}, \
                 stroke_cap: {stroke_cap:?})"
            ),
            PaintCall::Rect { rect, color } => write!(f, "rect(rect: {rect:?}, color: {color:?})"),
            PaintCall::RRect {
                rrect,
                color,
                style,
                has_mask_filter,
            } => write!(
                f,
                "rrect(rrect: {rrect:?}, color: {color:?}, style: {style:?}, \
                 has_mask_filter: {has_mask_filter:?})"
            ),
            PaintCall::DRRect {
                outer,
                inner,
                color,
                style,
            } => write!(
                f,
                "drrect(outer: {outer:?}, inner: {inner:?}, color: {color:?}, style: {style:?})"
            ),
            PaintCall::Circle {
                x,
                y,
                radius,
                color,
                style,
            } => write!(
                f,
                "circle(x: {x:?}, y: {y:?}, radius: {radius:?}, color: {color:?}, \
                 style: {style:?})"
            ),
            PaintCall::Shadow { elevation, color } => {
                write!(f, "shadow(elevation: {elevation:?}, color: {color:?})")
            }
            PaintCall::Paragraph => f.write_str("paragraph"),
            PaintCall::SaveRestore => f.write_str("saveRestore"),
            PaintCall::Something(_) => f.write_str("something"),
        }
    }
}

/// Dart's `expect(render, paints..a..b..)` over `ops`: each step is the next call of its method
/// after the last one matched, and has the arguments given; other methods' calls between are
/// skipped. [`PaintCall::SaveRestore`] skips a balanced `save`/`restore`, and
/// [`PaintCall::Something`] walks on until its predicate answers.
pub fn paints(ops: &[Op], pattern: &[PaintCall]) -> Result<(), Box<PaintMismatch>> {
    let log: Vec<RecordedCall> = ops.iter().map(RecordedCall::of).collect();
    let mismatch = |step: usize, found: Option<usize>, reason: &str| {
        Box::new(PaintMismatch {
            step,
            expected: format!("{:?}", pattern[step]),
            found: found.map(|index| (index, log[index].clone())),
            reason: reason.to_string(),
            log: log.clone(),
        })
    };
    if ops.is_empty() || pattern.is_empty() {
        return Err(Box::new(PaintMismatch {
            step: 0,
            expected: format!("{pattern:?}"),
            found: None,
            reason: if ops.is_empty() {
                "it painted nothing"
            } else {
                "it painted something, but the pattern has no steps"
            }
            .to_string(),
            log,
        }));
    }
    let mut index = 0;
    for (step, expected) in pattern.iter().enumerate() {
        match expected {
            PaintCall::SaveRestore => {
                index = next_call(ops, index, expected)
                    .ok_or_else(|| mismatch(step, None, "no save was called"))?;
                let mut depth = 1;
                while depth > 0 {
                    index += 1;
                    match ops.get(index).map(CanvasMethod::of) {
                        None => return Err(mismatch(step, None, "no restore balances the save")),
                        Some(CanvasMethod::Save) => depth += 1,
                        Some(CanvasMethod::Restore) => depth -= 1,
                        Some(_) => {}
                    }
                }
                index += 1;
            }
            PaintCall::Something(predicate) => loop {
                let Some(call) = log.get(index) else {
                    return Err(mismatch(step, None, "nothing matched the predicate"));
                };
                let answer =
                    predicate(call).map_err(|reason| mismatch(step, Some(index), &reason))?;
                index += 1;
                if answer {
                    break;
                }
            },
            _ => {
                let found = next_call(ops, index, expected)
                    .ok_or_else(|| mismatch(step, None, "the method was not called"))?;
                if !expected.matches(&ops[found]) {
                    return Err(mismatch(step, Some(found), "the call has other arguments"));
                }
                index = found + 1;
            }
        }
    }
    Ok(())
}

/// The index of the first op at or after `from` that `step` takes as a call of its method.
fn next_call(ops: &[Op], from: usize, step: &PaintCall) -> Option<usize> {
    (from..ops.len()).find(|&index| step.takes(&ops[index]))
}

/// [`paints`], failing the case with the mismatch's description.
#[track_caller]
pub fn assert_paints(ops: &[Op], pattern: &[PaintCall]) {
    if let Err(mismatch) = paints(ops, pattern) {
        panic!("{mismatch}");
    }
}

/// `ops` is exactly `pattern`: each step matches the call at its own position, of its method
/// and with its arguments, and no call is left over. For a whole recording compared call for
/// call; pass [`draws`] of the ops to leave out the canvas state and the clips.
pub fn paints_exactly(ops: &[Op], pattern: &[PaintCall]) -> Result<(), Box<PaintMismatch>> {
    let log: Vec<RecordedCall> = ops.iter().map(RecordedCall::of).collect();
    let mismatch = |step: usize, found: Option<usize>, expected: String, reason: &str| {
        Box::new(PaintMismatch {
            step,
            expected,
            found: found.map(|index| (index, log[index].clone())),
            reason: reason.to_string(),
            log: log.clone(),
        })
    };
    for (step, expected) in pattern.iter().enumerate() {
        let written = format!("{expected:?}");
        let Some(op) = ops.get(step) else {
            return Err(mismatch(step, None, written, "it painted fewer calls"));
        };
        let fits = match expected {
            PaintCall::Something(predicate) => predicate(&log[step])
                .map_err(|reason| mismatch(step, Some(step), written.clone(), &reason))?,
            PaintCall::SaveRestore => {
                return Err(mismatch(
                    step,
                    Some(step),
                    written,
                    "saveRestore spans calls",
                ));
            }
            _ => expected.takes(op) && expected.matches(op),
        };
        if !fits {
            return Err(mismatch(step, Some(step), written, "the call differs"));
        }
    }
    if ops.len() > pattern.len() {
        return Err(Box::new(PaintMismatch {
            step: pattern.len(),
            expected: "nothing more".to_string(),
            found: Some((pattern.len(), log[pattern.len()].clone())),
            reason: "it painted more calls".to_string(),
            log,
        }));
    }
    Ok(())
}

/// The ops of `ops` that draw, without the canvas state (`save`, `restore`, transforms) and the
/// clips: what [`paints_exactly`] compares a list of draws with.
pub fn draws(ops: &[Op]) -> Vec<Op> {
    ops.iter()
        .filter(|op| {
            !matches!(
                CanvasMethod::of(op),
                CanvasMethod::Save
                    | CanvasMethod::Restore
                    | CanvasMethod::Translate
                    | CanvasMethod::Scale
                    | CanvasMethod::Rotate
                    | CanvasMethod::Clip
            )
        })
        .cloned()
        .collect()
}

/// Dart's `paintsExactlyCountTimes(method, count)`: how many calls of `method` the ops record.
pub fn count_calls(ops: &[Op], method: CanvasMethod) -> usize {
    ops.iter()
        .filter(|op| CanvasMethod::of(op) == method)
        .count()
}
