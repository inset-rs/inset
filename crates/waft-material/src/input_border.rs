//! Flutter counterpart: material_ui's `input_border.dart`.
//!
//! `InputBorder` is a trait over [`ShapeBorder`] whose `paint` takes the floating label's gap;
//! each border's `ShapeBorder::paint` is that `paint` with Dart's default gap arguments, so a
//! caller holding a `dyn InputBorder` names the trait: `InputBorder::paint(&*border, ..)`.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::sync::Arc;

use waft_embedder::{
    Canvas, FillRule, Offset, Paint, Path, PathBuilder, PathOperation, RRect, Radius, Rect,
    clamp_double, lerp_double, path_combine, rrect_radii_elliptical,
};
use waft_painting::{
    BorderRadius, BorderSide, BorderStyle, BoxBorder, BoxShape, EdgeInsetsGeometry, ShapeBorder,
    TextDirection, draw_rrect,
};
use waft_widgets::{MaybeWidgetStateProperty, WidgetStateProperty};

/// Defines the appearance of an `InputDecorator`'s border.
///
/// An input decorator's border is specified by `InputDecoration.border`.
///
/// The border is drawn relative to the input decorator's "container" which
/// is the optionally filled area above the decorator's helper, error,
/// and counter.
///
/// Input border's are decorated with a line whose weight and color are defined
/// by [`border_side`](Self::border_side). The input decorator's renderer animates the input
/// border's appearance in response to state changes, like gaining or losing the focus,
/// by creating new copies of its input border with [`copy_with`](Self::copy_with).
///
/// See also:
///
///  * [`UnderlineInputBorder`], the default `InputDecorator` border which
///    draws a horizontal line at the bottom of the input decorator's container.
///  * [`OutlineInputBorder`], an `InputDecorator` border which draws a
///    rounded rectangle around the input decorator's container.
///  * `InputDecoration`, which is used to configure an `InputDecorator`.
pub trait InputBorder: ShapeBorder {
    /// Defines the border line's color and weight.
    ///
    /// The `InputDecorator` creates copies of its input border, using [`copy_with`](Self::copy_with),
    /// based on the current theme and `InputDecorator.isFocused`.
    fn border_side(&self) -> BorderSide;

    /// Creates a copy of this input border with the specified `border_side`.
    fn copy_with(&self, border_side: Option<BorderSide>) -> Box<dyn InputBorder>;

    /// True if this border will enclose the `InputDecorator`'s container.
    ///
    /// This property affects the alignment of container's contents. For example
    /// when an input decorator is configured with an [`OutlineInputBorder`] its
    /// label is centered with its container.
    fn is_outline(&self) -> bool;

    /// Paint this input border on `canvas`.
    ///
    /// The `rect` parameter bounds the `InputDecorator`'s container.
    ///
    /// The additional `gap` parameters reflect the state of the `InputDecorator`'s
    /// floating label. When an input decorator gains the focus, its label
    /// animates upwards, to make room for the input child. The `gap_start` and
    /// `gap_extent` parameters define a floating label width interval, and
    /// `gap_percentage` defines the animation's progress (0.0 to 1.0).
    ///
    /// Dart's defaults are `gap_extent: 0.0` and `gap_percentage: 0.0`, which the border's
    /// `ShapeBorder::paint` passes.
    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        gap_start: Option<f64>,
        gap_extent: f64,
        gap_percentage: f64,
        text_direction: Option<TextDirection>,
    );

    /// A heap clone typed as [`InputBorder`].
    fn clone_input_border(&self) -> Box<dyn InputBorder>;

    /// Dart's `border is WidgetStateProperty<InputBorder>`: this border as the property it
    /// also is, for the borders that implement `WidgetStateProperty` (`WidgetStateInputBorder`
    /// and the deprecated `MaterialState` input borders).
    fn as_widget_state_property(&self) -> Option<&dyn WidgetStateProperty<Box<dyn InputBorder>>> {
        None
    }
}

impl dyn InputBorder {
    /// No input border.
    ///
    /// Use this value with `InputDecoration.border` to specify that no border
    /// should be drawn. The `InputDecoration.collapsed` constructor sets
    /// its border to this value.
    pub fn none() -> Box<dyn InputBorder> {
        Box::new(NoInputBorder)
    }
}

impl Clone for Box<dyn InputBorder> {
    fn clone(&self) -> Self {
        self.clone_input_border()
    }
}

impl PartialEq for dyn InputBorder {
    fn eq(&self, other: &dyn InputBorder) -> bool {
        self.eq_shape(other)
    }
}

impl<T: InputBorder + 'static> From<T> for Box<dyn InputBorder> {
    fn from(border: T) -> Box<dyn InputBorder> {
        Box::new(border)
    }
}

/// Dart's `T?` is `InputBorder?`: a `WidgetStateInputBorder` resolves to a border, which
/// `resolve_as` wraps in `Some` for an `Option<Box<dyn InputBorder>>`.
impl MaybeWidgetStateProperty<Box<dyn InputBorder>> for Box<dyn InputBorder> {
    fn as_widget_state_property(&self) -> Option<&dyn WidgetStateProperty<Box<dyn InputBorder>>> {
        InputBorder::as_widget_state_property(&**self)
    }

    fn from_resolved(value: Box<dyn InputBorder>) -> Box<dyn InputBorder> {
        value
    }
}

/// Dart's `Path()..addRect(rect)`.
fn rect_path(rect: Rect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rect(rect.into());
    path.build()
}

/// Dart's `Path()..addRRect(rrect)`.
fn rrect_path(rrect: RRect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rrect_radii_elliptical(rrect.outer_rect(), rrect_radii_elliptical(rrect));
    path.build()
}

/// Dart's `Path.addArc(oval, startAngle, sweepAngle)`: a new contour along the oval inscribed
/// in `oval`, from `start_angle` clockwise through `sweep_angle` radians. An empty oval or a
/// zero sweep adds nothing, as Skia's `addArc` does.
fn add_arc(path: &mut PathBuilder, oval: Rect, start_angle: f64, sweep_angle: f64) {
    if oval.is_empty() || sweep_angle == 0.0 {
        return;
    }
    let center = oval.center();
    let radius_x = oval.width() / 2.0;
    let radius_y = oval.height() / 2.0;
    path.move_to(Offset::new(
        center.dx() + radius_x * start_angle.cos(),
        center.dy() + radius_y * start_angle.sin(),
    ));
    path.ellipse(
        center,
        [radius_x as f32, radius_y as f32],
        0.0,
        start_angle as f32,
        sweep_angle as f32,
    );
}

/// Dart's `Canvas.drawLine`: a stroked two-point path.
fn draw_line(canvas: &mut Canvas, p1: Offset, p2: Offset, paint: &Paint) {
    let mut path = PathBuilder::new();
    path.move_to(p1);
    path.line_to(p2);
    canvas.draw_path(&path.build(), FillRule::NonZero, paint);
}

// Used to create the InputBorder.none singleton.
#[derive(Clone, Copy)]
struct NoInputBorder;

impl ShapeBorder for NoInputBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::ZERO
    }

    fn scale(&self, _t: f64) -> Box<dyn ShapeBorder> {
        Box::new(NoInputBorder)
    }

    fn get_outer_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        rect_path(rect)
    }

    fn get_inner_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        rect_path(rect)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _text_direction: Option<TextDirection>,
    ) {
        canvas.draw_rect(rect, paint);
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        InputBorder::paint(self, canvas, rect, None, 0.0, 0.0, text_direction);
    }

    fn clone_box(&self) -> Box<dyn ShapeBorder> {
        Box::new(NoInputBorder)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    // Dart's `InputBorder.none` is the one `const _NoInputBorder()`, so every no-border is
    // identical to every other.
    fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
        other.as_any().is::<NoInputBorder>()
    }
}

impl InputBorder for NoInputBorder {
    fn border_side(&self) -> BorderSide {
        BorderSide::NONE
    }

    fn copy_with(&self, _border_side: Option<BorderSide>) -> Box<dyn InputBorder> {
        Box::new(NoInputBorder)
    }

    fn is_outline(&self) -> bool {
        false
    }

    fn paint(
        &self,
        _canvas: &mut Canvas,
        _rect: Rect,
        _gap_start: Option<f64>,
        _gap_extent: f64,
        _gap_percentage: f64,
        _text_direction: Option<TextDirection>,
    ) {
        // Do not paint.
    }

    fn clone_input_border(&self) -> Box<dyn InputBorder> {
        Box::new(NoInputBorder)
    }
}

impl Debug for NoInputBorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_NoInputBorder()")
    }
}

/// Draws a horizontal line at the bottom of an `InputDecorator`'s container and
/// defines the container's shape.
///
/// The input decorator's "container" is the optionally filled area above the
/// decorator's helper, error, and counter.
///
/// See also:
///
///  * [`OutlineInputBorder`], an `InputDecorator` border which draws a
///    rounded rectangle around the input decorator's container.
///  * `InputDecoration`, which is used to configure an `InputDecorator`.
#[derive(Clone, Copy, PartialEq)]
pub struct UnderlineInputBorder {
    /// Defines the border line's color and weight.
    pub border_side: BorderSide,

    /// The radii of the border's rounded rectangle corners.
    ///
    /// When this border is used with a filled input decorator, see
    /// `InputDecoration.filled`, the border radius defines the shape
    /// of the background fill as well as the bottom left and right
    /// edges of the underline itself.
    ///
    /// By default the top right and top left corners have a circular radius
    /// of 4.0.
    pub border_radius: BorderRadius,
}

impl Default for UnderlineInputBorder {
    /// Creates an underline border for an `InputDecorator`.
    ///
    /// The [`border_side`](Self::border_side) parameter defaults to `BorderSide()`.
    /// Applications typically do not specify a `border_side` parameter
    /// because the input decorator substitutes its own, using `copy_with`, based
    /// on the current theme and `InputDecorator.isFocused`.
    ///
    /// The [`border_radius`](Self::border_radius) parameter defaults to a value where the top
    /// left and right corners have a circular radius of 4.0.
    fn default() -> UnderlineInputBorder {
        UnderlineInputBorder {
            border_side: BorderSide::default(),
            border_radius: BorderRadius::only(
                Radius::circular(4.0),
                Radius::circular(4.0),
                Radius::ZERO,
                Radius::ZERO,
            ),
        }
    }
}

impl UnderlineInputBorder {
    /// Creates an underline border for an `InputDecorator`, with Dart's defaults.
    pub fn new() -> UnderlineInputBorder {
        UnderlineInputBorder::default()
    }

    /// Dart `UnderlineInputBorder(borderSide:)`.
    pub fn border_side(mut self, border_side: BorderSide) -> UnderlineInputBorder {
        self.border_side = border_side;
        self
    }

    /// Dart `UnderlineInputBorder(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> UnderlineInputBorder {
        self.border_radius = border_radius;
        self
    }

    /// Creates a copy of this border with the given fields replaced.
    pub fn copy_with(
        &self,
        border_side: Option<BorderSide>,
        border_radius: Option<BorderRadius>,
    ) -> UnderlineInputBorder {
        UnderlineInputBorder {
            border_side: border_side.unwrap_or(self.border_side),
            border_radius: border_radius.unwrap_or(self.border_radius),
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> UnderlineInputBorder {
        UnderlineInputBorder::new().border_side(self.border_side.scale(t))
    }
}

impl ShapeBorder for UnderlineInputBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::only(0.0, 0.0, 0.0, self.border_side.width)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(UnderlineInputBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<UnderlineInputBorder>()) {
            return Some(Box::new(UnderlineInputBorder {
                border_side: BorderSide::lerp(a.border_side, self.border_side, t),
                border_radius: BorderRadius::lerp(
                    Some(a.border_radius),
                    Some(self.border_radius),
                    t,
                )
                .expect("both radii are given"),
            }));
        }
        if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<UnderlineInputBorder>()) {
            return Some(Box::new(UnderlineInputBorder {
                border_side: BorderSide::lerp(self.border_side, b.border_side, t),
                border_radius: BorderRadius::lerp(
                    Some(self.border_radius),
                    Some(b.border_radius),
                    t,
                )
                .expect("both radii are given"),
            }));
        }
        if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        rect_path(Rect::from_ltwh(
            rect.left,
            rect.top,
            rect.width(),
            (rect.height() - self.border_side.width).max(0.0),
        ))
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        rrect_path(self.border_radius.resolve(text_direction).to_rrect(rect))
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        draw_rrect(
            canvas,
            self.border_radius.resolve(text_direction).to_rrect(rect),
            paint,
        );
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        InputBorder::paint(self, canvas, rect, None, 0.0, 0.0, text_direction);
    }

    fn clone_box(&self) -> Box<dyn ShapeBorder> {
        Box::new(*self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
        other
            .as_any()
            .downcast_ref::<UnderlineInputBorder>()
            .is_some_and(|other| other == self)
    }
}

impl InputBorder for UnderlineInputBorder {
    fn border_side(&self) -> BorderSide {
        self.border_side
    }

    fn copy_with(&self, border_side: Option<BorderSide>) -> Box<dyn InputBorder> {
        Box::new(UnderlineInputBorder::copy_with(self, border_side, None))
    }

    fn is_outline(&self) -> bool {
        false
    }

    /// Draw a horizontal line at the bottom of `rect`.
    ///
    /// The [`border_side`](UnderlineInputBorder::border_side) defines the line's color and
    /// weight. The `text_direction` `gap` and `text_direction` parameters are ignored.
    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        _gap_start: Option<f64>,
        _gap_extent: f64,
        _gap_percentage: f64,
        text_direction: Option<TextDirection>,
    ) {
        if self.border_side.style == BorderStyle::None {
            return;
        }

        if self.border_radius.bottom_left != Radius::ZERO
            || self.border_radius.bottom_right != Radius::ZERO
        {
            // This prevents the border from leaking the color due to anti-aliasing rounding errors.
            let updated_border_radius = BorderRadius::only(
                Radius::ZERO,
                Radius::ZERO,
                self.border_radius
                    .bottom_left
                    .clamp(None, Some(Radius::circular(rect.height() / 2.0))),
                self.border_radius
                    .bottom_right
                    .clamp(None, Some(Radius::circular(rect.height() / 2.0))),
            );

            <dyn BoxBorder>::paint_non_uniform_border(
                canvas,
                rect,
                Some(updated_border_radius),
                text_direction,
                BoxShape::Rectangle,
                BorderSide::NONE,
                BorderSide::NONE,
                self.border_side
                    .copy_with(None, None, None, Some(BorderSide::STROKE_ALIGN_INSIDE)),
                BorderSide::NONE,
                self.border_side.color,
            );
        } else {
            let align_inside_offset = Offset::new(0.0, self.border_side.width / 2.0);
            draw_line(
                canvas,
                rect.bottom_left() - align_inside_offset,
                rect.bottom_right() - align_inside_offset,
                &self.border_side.to_paint(),
            );
        }
    }

    fn clone_input_border(&self) -> Box<dyn InputBorder> {
        Box::new(*self)
    }
}

impl Debug for UnderlineInputBorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnderlineInputBorder()")
    }
}

/// Draws a rounded rectangle around an `InputDecorator`'s container.
///
/// When the input decorator's label is floating, for example because its
/// input child has the focus, the label appears in a gap in the border outline.
///
/// The input decorator's "container" is the optionally filled area above the
/// decorator's helper, error, and counter.
///
/// See also:
///
///  * [`UnderlineInputBorder`], the default `InputDecorator` border which
///    draws a horizontal line at the bottom of the input decorator's container.
///  * [`ShapedInputBorder`], an `InputDecorator` border which draws a custom
///    [`ShapeBorder`] around the input decorator's container.
///  * `InputDecoration`, which is used to configure an `InputDecorator`.
#[derive(Clone, Copy, PartialEq)]
pub struct OutlineInputBorder {
    /// Defines the border line's color and weight.
    pub border_side: BorderSide,

    /// Horizontal padding on either side of the border's
    /// `InputDecoration.labelText` width gap.
    ///
    /// This value is used by the [`paint`](InputBorder::paint) method to compute the actual
    /// gap width.
    pub gap_padding: f64,

    /// The radii of the border's rounded rectangle corners.
    ///
    /// The corner radii must be circular, i.e. their [`Radius::x`] and [`Radius::y`]
    /// values must be the same.
    pub border_radius: BorderRadius,
}

impl Default for OutlineInputBorder {
    /// Creates a rounded rectangle outline border for an `InputDecorator`.
    ///
    /// If the [`border_side`](Self::border_side) parameter is [`BorderSide::NONE`], it will not
    /// draw a border. However, it will still define a shape (which you can see if
    /// `InputDecoration.filled` is true).
    ///
    /// If an application does not specify a `border_side` parameter of
    /// value [`BorderSide::NONE`], the input decorator substitutes its own, using
    /// `copy_with`, based on the current theme and `InputDecorator.isFocused`.
    ///
    /// The [`border_radius`](Self::border_radius) parameter defaults to a value where all four
    /// corners have a circular radius of 4.0. The corner radii must be circular, i.e.
    /// their [`Radius::x`] and [`Radius::y`] values must be the same.
    ///
    /// See also:
    ///
    ///  * `InputDecoration.floatingLabelBehavior`, which should be set to
    ///    `FloatingLabelBehavior.never` when the `border_side` is
    ///    [`BorderSide::NONE`]. If left as `FloatingLabelBehavior.auto`, the label
    ///    will extend beyond the container as if the border were still being
    ///    drawn.
    fn default() -> OutlineInputBorder {
        OutlineInputBorder {
            border_side: BorderSide::default(),
            gap_padding: 4.0,
            border_radius: BorderRadius::all(Radius::circular(4.0)),
        }
    }
}

impl OutlineInputBorder {
    /// Creates a rounded rectangle outline border for an `InputDecorator`, with Dart's
    /// defaults.
    pub fn new() -> OutlineInputBorder {
        OutlineInputBorder::default()
    }

    /// Dart `OutlineInputBorder(borderSide:)`.
    pub fn border_side(mut self, border_side: BorderSide) -> OutlineInputBorder {
        self.border_side = border_side;
        self
    }

    /// Dart `OutlineInputBorder(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> OutlineInputBorder {
        self.border_radius = border_radius;
        self
    }

    /// Dart `OutlineInputBorder(gapPadding:)`.
    pub fn gap_padding(mut self, gap_padding: f64) -> OutlineInputBorder {
        debug_assert!(gap_padding >= 0.0);
        self.gap_padding = gap_padding;
        self
    }

    // The label text's gap can extend into the corners (even both the top left
    // and the top right corner). To avoid the more complicated problem of finding
    // how far the gap penetrates into an elliptical corner, just require them
    // to be circular.
    //
    // This can't be checked by the constructor because const constructor.
    fn corners_are_circular(border_radius: BorderRadius) -> bool {
        border_radius.top_left.x == border_radius.top_left.y
            && border_radius.bottom_left.x == border_radius.bottom_left.y
            && border_radius.top_right.x == border_radius.top_right.y
            && border_radius.bottom_right.x == border_radius.bottom_right.y
    }

    /// Creates a copy of this border with the given fields replaced.
    pub fn copy_with(
        &self,
        border_side: Option<BorderSide>,
        border_radius: Option<BorderRadius>,
        gap_padding: Option<f64>,
    ) -> OutlineInputBorder {
        OutlineInputBorder::new()
            .border_side(border_side.unwrap_or(self.border_side))
            .border_radius(border_radius.unwrap_or(self.border_radius))
            .gap_padding(gap_padding.unwrap_or(self.gap_padding))
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> OutlineInputBorder {
        OutlineInputBorder::new()
            .border_side(self.border_side.scale(t))
            .border_radius(self.border_radius * t)
            .gap_padding(self.gap_padding * t)
    }

    fn gap_border_path(
        &self,
        center: RRect,
        outer_width: f64,
        start: f64,
        extent: f64,
    ) -> Arc<Path> {
        // When the corner radii on any side add up to be greater than the
        // given height, each radius has to be scaled to not exceed the
        // size of the width/height of the RRect.
        let scaled_rrect = center.scale_radii();

        let tl_corner = Rect::from_ltwh(
            scaled_rrect.left,
            scaled_rrect.top,
            scaled_rrect.tl_radius_x * 2.0,
            scaled_rrect.tl_radius_y * 2.0,
        );
        let tr_corner = Rect::from_ltwh(
            scaled_rrect.right - scaled_rrect.tr_radius_x * 2.0,
            scaled_rrect.top,
            scaled_rrect.tr_radius_x * 2.0,
            scaled_rrect.tr_radius_y * 2.0,
        );
        let br_corner = Rect::from_ltwh(
            scaled_rrect.right - scaled_rrect.br_radius_x * 2.0,
            scaled_rrect.bottom - scaled_rrect.br_radius_y * 2.0,
            scaled_rrect.br_radius_x * 2.0,
            scaled_rrect.br_radius_y * 2.0,
        );
        let bl_corner = Rect::from_ltwh(
            scaled_rrect.left,
            scaled_rrect.bottom - scaled_rrect.bl_radius_y * 2.0,
            scaled_rrect.bl_radius_x * 2.0,
            scaled_rrect.bl_radius_y * 2.0,
        );

        // This assumes that the radius is circular (x and y radius are equal).
        // Currently, BorderRadius only supports circular radii.
        const CORNER_ARC_SWEEP: f64 = PI / 2.0;
        let mut path = PathBuilder::new();

        // Top left corner
        if scaled_rrect.tl_radius() != Radius::ZERO {
            let tl_corner_arc_sweep =
                clamp_double(1.0 - start / scaled_rrect.tl_radius_x, 0.0, 1.0).acos();
            add_arc(&mut path, tl_corner, PI, tl_corner_arc_sweep);
        } else {
            // Because the path is painted with Paint.strokeCap = StrokeCap.butt, horizontal coordinate is moved
            // based on strokeOffset to respect strokeAlign.
            path.move_to(Offset::new(
                scaled_rrect.left + self.border_side.stroke_offset() / 2.0,
                scaled_rrect.top,
            ));
        }

        // Draw top border from top left corner to gap start.
        if start > scaled_rrect.tl_radius_x {
            path.line_to(Offset::new(start, scaled_rrect.top));
        }

        // Draw top border from gap end to top right corner and draw top right corner.
        const TR_CORNER_ARC_START: f64 = (3.0 * PI) / 2.0;
        const TR_CORNER_ARC_SWEEP: f64 = CORNER_ARC_SWEEP;
        if start + extent < outer_width - scaled_rrect.tr_radius_x {
            path.move_to(Offset::new(start + extent, scaled_rrect.top));
            path.line_to(Offset::new(
                scaled_rrect.right - scaled_rrect.tr_radius_x,
                scaled_rrect.top,
            ));
            if scaled_rrect.tr_radius() != Radius::ZERO {
                add_arc(
                    &mut path,
                    tr_corner,
                    TR_CORNER_ARC_START,
                    TR_CORNER_ARC_SWEEP,
                );
            }
        } else if start + extent < outer_width {
            let dx = outer_width - (start + extent);
            let sweep = clamp_double(1.0 - dx / scaled_rrect.tr_radius_x, 0.0, 1.0).asin();
            add_arc(
                &mut path,
                tr_corner,
                TR_CORNER_ARC_START + sweep,
                TR_CORNER_ARC_SWEEP - sweep,
            );
        }

        // Draw right border and bottom right corner.
        if scaled_rrect.br_radius() != Radius::ZERO {
            path.move_to(Offset::new(
                scaled_rrect.right,
                scaled_rrect.top + scaled_rrect.tr_radius_y,
            ));
        }
        path.line_to(Offset::new(
            scaled_rrect.right,
            scaled_rrect.bottom - scaled_rrect.br_radius_y,
        ));
        if scaled_rrect.br_radius() != Radius::ZERO {
            add_arc(&mut path, br_corner, 0.0, CORNER_ARC_SWEEP);
        }

        // Draw bottom border and bottom left corner.
        path.line_to(Offset::new(
            scaled_rrect.left + scaled_rrect.bl_radius_x,
            scaled_rrect.bottom,
        ));
        if scaled_rrect.bl_radius() != Radius::ZERO {
            add_arc(&mut path, bl_corner, PI / 2.0, CORNER_ARC_SWEEP);
        }

        // Draw left border
        path.line_to(Offset::new(
            scaled_rrect.left,
            scaled_rrect.top + scaled_rrect.tl_radius_y,
        ));

        path.build()
    }
}

impl ShapeBorder for OutlineInputBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::all(self.border_side.stroke_inset())
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(OutlineInputBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(outline) = a.and_then(|a| a.as_any().downcast_ref::<OutlineInputBorder>()) {
            return Some(Box::new(
                OutlineInputBorder::new()
                    .border_radius(
                        BorderRadius::lerp(
                            Some(outline.border_radius),
                            Some(self.border_radius),
                            t,
                        )
                        .expect("both radii are given"),
                    )
                    .border_side(BorderSide::lerp(outline.border_side, self.border_side, t))
                    .gap_padding(outline.gap_padding),
            ));
        }
        if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(outline) = b.and_then(|b| b.as_any().downcast_ref::<OutlineInputBorder>()) {
            return Some(Box::new(
                OutlineInputBorder::new()
                    .border_radius(
                        BorderRadius::lerp(
                            Some(self.border_radius),
                            Some(outline.border_radius),
                            t,
                        )
                        .expect("both radii are given"),
                    )
                    .border_side(BorderSide::lerp(self.border_side, outline.border_side, t))
                    .gap_padding(outline.gap_padding),
            ));
        }
        if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        rrect_path(
            self.border_radius
                .resolve(text_direction)
                .to_rrect(rect)
                .deflate(self.border_side.stroke_inset()),
        )
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        rrect_path(self.border_radius.resolve(text_direction).to_rrect(rect))
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        draw_rrect(
            canvas,
            self.border_radius.resolve(text_direction).to_rrect(rect),
            paint,
        );
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        InputBorder::paint(self, canvas, rect, None, 0.0, 0.0, text_direction);
    }

    fn clone_box(&self) -> Box<dyn ShapeBorder> {
        Box::new(*self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
        other
            .as_any()
            .downcast_ref::<OutlineInputBorder>()
            .is_some_and(|other| other == self)
    }
}

impl InputBorder for OutlineInputBorder {
    fn border_side(&self) -> BorderSide {
        self.border_side
    }

    fn copy_with(&self, border_side: Option<BorderSide>) -> Box<dyn InputBorder> {
        Box::new(OutlineInputBorder::copy_with(self, border_side, None, None))
    }

    fn is_outline(&self) -> bool {
        true
    }

    /// Draw a rounded rectangle around `rect` using
    /// [`border_radius`](OutlineInputBorder::border_radius).
    ///
    /// The [`border_side`](OutlineInputBorder::border_side) defines the line's color and
    /// weight.
    ///
    /// The top side of the rounded rectangle may be interrupted by a single gap
    /// if `gap_extent` is non-null. In that case the gap begins at
    /// `gap_start - gap_padding` (assuming that the `text_direction` is [`TextDirection::Ltr`]).
    /// The gap's width is `(gap_padding + gap_extent + gap_padding) * gap_percentage`.
    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        gap_start: Option<f64>,
        gap_extent: f64,
        gap_percentage: f64,
        text_direction: Option<TextDirection>,
    ) {
        debug_assert!((0.0..=1.0).contains(&gap_percentage));
        debug_assert!(OutlineInputBorder::corners_are_circular(self.border_radius));

        let paint = self.border_side.to_paint();
        let outer = self.border_radius.to_rrect(rect);
        let center = outer.inflate(self.border_side.stroke_offset() / 2.0);
        match gap_start {
            Some(gap_start) if gap_extent > 0.0 && gap_percentage != 0.0 => {
                let extent = lerp_double(
                    Some(0.0),
                    Some(gap_extent + self.gap_padding * 2.0),
                    gap_percentage,
                )
                .expect("both ends are given");
                let start = match text_direction.expect("a gap needs a text direction") {
                    TextDirection::Rtl => gap_start + self.gap_padding - extent,
                    TextDirection::Ltr => gap_start - self.gap_padding,
                };
                let path = self.gap_border_path(center, outer.width(), start.max(0.0), extent);
                canvas.draw_path(&path, FillRule::NonZero, &paint);
            }
            _ => draw_rrect(canvas, center, &paint),
        }
    }

    fn clone_input_border(&self) -> Box<dyn InputBorder> {
        Box::new(*self)
    }
}

impl Debug for OutlineInputBorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OutlineInputBorder()")
    }
}

/// Draws a custom shape around an `InputDecorator`'s container.
///
/// This border allows any [`ShapeBorder`] to be used as an input decorator border.
/// This provides maximum flexibility for custom border shapes while maintaining
/// the gap functionality for floating labels.
///
/// When the input decorator's label is floating, for example because its
/// input child has the focus, the label appears in a gap in the border outline.
///
/// The input decorator's "container" is the optionally filled area above the
/// decorator's helper, error, and counter.
///
/// See also:
///
///  * [`OutlineInputBorder`], a traditional rounded rectangle border.
///  * [`UnderlineInputBorder`], the default `InputDecorator` border which
///    draws a horizontal line at the bottom of the input decorator's container.
///  * `RoundedSuperellipseBorder`, which can be used with this border for iOS-style shapes.
///  * `InputDecoration`, which is used to configure an `InputDecorator`.
pub struct ShapedInputBorder {
    /// Defines the border line's color and weight.
    pub border_side: BorderSide,

    /// Horizontal padding on either side of the border's
    /// `InputDecoration.labelText` width gap.
    ///
    /// This value is used by the [`paint`](InputBorder::paint) method to compute the actual
    /// gap width.
    pub gap_padding: f64,

    /// The shape of the border.
    pub shape: Box<dyn ShapeBorder>,
}

impl ShapedInputBorder {
    /// Creates a shaped outline border for an `InputDecorator`.
    ///
    /// The `shape` parameter defines the custom border shape. It can be any
    /// [`ShapeBorder`] such as `RoundedSuperellipseBorder`, `StadiumBorder`,
    /// `BeveledRectangleBorder`, or a custom shape.
    ///
    /// If the [`border_side`](Self::border_side) parameter is [`BorderSide::NONE`], it will
    /// not draw a border. However, it will still define a shape (which you can see if
    /// `InputDecoration.filled` is true).
    ///
    /// If an application does not specify a `border_side` parameter of
    /// value [`BorderSide::NONE`], the input decorator substitutes its own, using
    /// `copy_with`, based on the current theme and `InputDecorator.isFocused`.
    ///
    /// See also:
    ///
    ///  * `InputDecoration.floatingLabelBehavior`, which should be set to
    ///    `FloatingLabelBehavior.never` when the `border_side` is
    ///    [`BorderSide::NONE`]. If left as `FloatingLabelBehavior.auto`, the label
    ///    will extend beyond the container as if the border were still being
    ///    drawn.
    pub fn new(shape: impl Into<Box<dyn ShapeBorder>>) -> ShapedInputBorder {
        ShapedInputBorder {
            border_side: BorderSide::default(),
            gap_padding: 4.0,
            shape: shape.into(),
        }
    }

    /// Dart `ShapedInputBorder(borderSide:)`.
    pub fn border_side(mut self, border_side: BorderSide) -> ShapedInputBorder {
        self.border_side = border_side;
        self
    }

    /// Dart `ShapedInputBorder(gapPadding:)`.
    pub fn gap_padding(mut self, gap_padding: f64) -> ShapedInputBorder {
        debug_assert!(gap_padding >= 0.0);
        self.gap_padding = gap_padding;
        self
    }

    /// Creates a copy of this border with the given fields replaced.
    pub fn copy_with(
        &self,
        border_side: Option<BorderSide>,
        shape: Option<Box<dyn ShapeBorder>>,
        gap_padding: Option<f64>,
    ) -> ShapedInputBorder {
        ShapedInputBorder::new(shape.unwrap_or_else(|| self.shape.clone_box()))
            .border_side(border_side.unwrap_or(self.border_side))
            .gap_padding(gap_padding.unwrap_or(self.gap_padding))
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> ShapedInputBorder {
        ShapedInputBorder::new(self.shape.scale(t))
            .border_side(self.border_side.scale(t))
            .gap_padding(self.gap_padding * t)
    }

    fn gap_border_path(
        &self,
        rect: Rect,
        start: f64,
        extent: f64,
        text_direction: Option<TextDirection>,
    ) -> Arc<Path> {
        // Create a continuous path for the border with a gap in the top edge.
        let outer_path = self.shape.get_outer_path(rect, text_direction);

        // If there's no meaningful gap, return the full outline.
        if start <= 0.0 && extent <= 0.0 {
            return outer_path;
        }

        // Create a rectangle that represents the gap area.
        // The gap is on the top edge, so we create a rect that covers the gap region.
        let gap_left = start;
        let gap_right = start + extent;

        // Create a path that excludes the gap area by combining with a difference operation.
        // We'll subtract a small rectangle at the top where the gap should be.
        let gap_rect = rect_path(Rect::from_ltrb(
            clamp_double(gap_left, rect.left, rect.right),
            rect.top - 1.0, // Extend slightly beyond to ensure clean cut.
            clamp_double(gap_right, rect.left, rect.right),
            rect.top + 1.0, // Small height to only affect top edge.
        ));

        path_combine(PathOperation::Difference, &outer_path, &gap_rect)
    }
}

impl Clone for ShapedInputBorder {
    fn clone(&self) -> ShapedInputBorder {
        ShapedInputBorder {
            border_side: self.border_side,
            gap_padding: self.gap_padding,
            shape: self.shape.clone_box(),
        }
    }
}

impl PartialEq for ShapedInputBorder {
    fn eq(&self, other: &ShapedInputBorder) -> bool {
        other.border_side == self.border_side
            && other.shape.eq_shape(&*self.shape)
            && other.gap_padding == self.gap_padding
    }
}

impl ShapeBorder for ShapedInputBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::all(self.border_side.width)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(ShapedInputBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<ShapedInputBorder>()) {
            return Some(Box::new(
                ShapedInputBorder::new(
                    <dyn ShapeBorder>::lerp(Some(&*a.shape), Some(&*self.shape), t)
                        .expect("two shapes lerp to a shape"),
                )
                .border_side(BorderSide::lerp(a.border_side, self.border_side, t))
                .gap_padding(a.gap_padding),
            ));
        }
        if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<ShapedInputBorder>()) {
            return Some(Box::new(
                ShapedInputBorder::new(
                    <dyn ShapeBorder>::lerp(Some(&*self.shape), Some(&*b.shape), t)
                        .expect("two shapes lerp to a shape"),
                )
                .border_side(BorderSide::lerp(self.border_side, b.border_side, t))
                .gap_padding(b.gap_padding),
            ));
        }
        if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        self.shape
            .get_inner_path(rect.deflate(self.border_side.width), text_direction)
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        self.shape.get_outer_path(rect, text_direction)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        if self.shape.prefer_paint_interior() {
            self.shape
                .paint_interior(canvas, rect, paint, text_direction);
        } else {
            // Fallback for shapes that don't support paintInterior.
            canvas.draw_path(
                &self.shape.get_outer_path(rect, text_direction),
                FillRule::NonZero,
                paint,
            );
        }
    }

    fn prefer_paint_interior(&self) -> bool {
        self.shape.prefer_paint_interior()
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        InputBorder::paint(self, canvas, rect, None, 0.0, 0.0, text_direction);
    }

    fn clone_box(&self) -> Box<dyn ShapeBorder> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
        other
            .as_any()
            .downcast_ref::<ShapedInputBorder>()
            .is_some_and(|other| other == self)
    }
}

impl InputBorder for ShapedInputBorder {
    fn border_side(&self) -> BorderSide {
        self.border_side
    }

    fn copy_with(&self, border_side: Option<BorderSide>) -> Box<dyn InputBorder> {
        Box::new(ShapedInputBorder::copy_with(self, border_side, None, None))
    }

    fn is_outline(&self) -> bool {
        true
    }

    /// Draw the custom shape around `rect`.
    ///
    /// The [`border_side`](ShapedInputBorder::border_side) defines the line's color and
    /// weight.
    ///
    /// The top side of the border may be interrupted by a single gap
    /// if `gap_extent` is non-null. In that case the gap begins at
    /// `gap_start - gap_padding` (assuming that the `text_direction` is [`TextDirection::Ltr`]).
    /// The gap's width is `(gap_padding + gap_extent + gap_padding) * gap_percentage`.
    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        gap_start: Option<f64>,
        gap_extent: f64,
        gap_percentage: f64,
        text_direction: Option<TextDirection>,
    ) {
        debug_assert!((0.0..=1.0).contains(&gap_percentage));

        let paint = self.border_side.to_paint();
        let deflated_rect = rect.deflate(self.border_side.width / 2.0);

        match gap_start {
            Some(gap_start) if gap_extent > 0.0 && gap_percentage != 0.0 => {
                let extent = lerp_double(
                    Some(0.0),
                    Some(gap_extent + self.gap_padding * 2.0),
                    gap_percentage,
                )
                .expect("both ends are given");
                let start = match text_direction.expect("a gap needs a text direction") {
                    TextDirection::Rtl => gap_start + self.gap_padding - extent,
                    TextDirection::Ltr => gap_start - self.gap_padding,
                };
                let path =
                    self.gap_border_path(deflated_rect, start.max(0.0), extent, text_direction);
                canvas.draw_path(&path, FillRule::NonZero, &paint);
            }
            // Draw the shape without a gap.
            _ => match self.shape.as_outlined_border() {
                Some(outlined_shape) => {
                    // Create a copy with our border side.
                    let shaped_border = outlined_shape.copy_with(Some(self.border_side));
                    shaped_border.paint(canvas, deflated_rect, text_direction);
                }
                None => canvas.draw_path(
                    &self.shape.get_outer_path(deflated_rect, text_direction),
                    FillRule::NonZero,
                    &paint,
                ),
            },
        }
    }

    fn clone_input_border(&self) -> Box<dyn InputBorder> {
        Box::new(self.clone())
    }
}

impl Debug for ShapedInputBorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ShapedInputBorder()")
    }
}

/// Dart's `shape as InputBorder` on a border that `ShapeBorder::lerp` gave back: the input
/// border it is, when it is one of this crate's input borders.
///
/// A `dyn ShapeBorder` cannot be asked for a supertrait it does not name, so a border of
/// another crate's `InputBorder` type answers `None`.
#[allow(deprecated)]
pub(crate) fn input_border_of(shape: &dyn ShapeBorder) -> Option<Box<dyn InputBorder>> {
    let any = shape.as_any();
    if let Some(border) = any.downcast_ref::<NoInputBorder>() {
        return Some(border.clone_input_border());
    }
    if let Some(border) = any.downcast_ref::<UnderlineInputBorder>() {
        return Some(border.clone_input_border());
    }
    if let Some(border) = any.downcast_ref::<OutlineInputBorder>() {
        return Some(border.clone_input_border());
    }
    if let Some(border) = any.downcast_ref::<ShapedInputBorder>() {
        return Some(border.clone_input_border());
    }
    if let Some(border) = any.downcast_ref::<crate::material_state::WidgetStateInputBorder>() {
        return Some(border.clone_input_border());
    }
    if let Some(border) =
        any.downcast_ref::<crate::material_state::MaterialStateOutlineInputBorder>()
    {
        return Some(border.clone_input_border());
    }
    if let Some(border) =
        any.downcast_ref::<crate::material_state::MaterialStateUnderlineInputBorder>()
    {
        return Some(border.clone_input_border());
    }
    None
}

#[cfg(test)]
mod tests {
    use waft_embedder::valo::{Op, Point};
    use waft_foundation::TargetPlatform;
    use waft_painting::{RoundedRectangleBorder, RoundedSuperellipseBorder, StadiumBorder};
    use waft_widgets::{Center, IntoWidget};

    use super::*;
    use crate::colors::Colors;
    use crate::input_decorator::{FloatingLabelBehavior, InputDecoration};
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::text_field::TextField;
    use crate::theme_data::ThemeData;

    fn superellipse(radius: f64) -> RoundedSuperellipseBorder {
        RoundedSuperellipseBorder::new(
            BorderSide::NONE,
            Some(BorderRadius::all(Radius::circular(radius)).into()),
        )
    }

    fn width(width: f64) -> BorderSide {
        BorderSide {
            width,
            ..BorderSide::default()
        }
    }

    fn blue(width: f64) -> BorderSide {
        BorderSide {
            color: Colors::BLUE.primary(),
            width,
            ..BorderSide::default()
        }
    }

    #[test]
    fn shaped_input_border_constructor() {
        let input_border = ShapedInputBorder::new(RoundedRectangleBorder::default());
        assert_eq!(input_border.border_side, BorderSide::default());
        assert!(
            input_border
                .shape
                .eq_shape(&RoundedRectangleBorder::default())
        );
        assert_eq!(input_border.gap_padding, 4.0);
        assert!(input_border.is_outline());
    }

    #[test]
    fn shaped_input_border_with_rounded_superellipse_border() {
        let input_border = ShapedInputBorder::new(superellipse(12.0));
        assert_eq!(input_border.border_side, BorderSide::default());
        assert!(input_border.shape.eq_shape(&superellipse(12.0)));
        assert_eq!(input_border.gap_padding, 4.0);
        assert!(input_border.is_outline());
    }

    #[test]
    fn shaped_input_border_copy_with_and_equality() {
        let input_border = ShapedInputBorder::new(RoundedRectangleBorder::default());
        let copy = input_border.copy_with(None, None, None);
        assert!(input_border == copy);

        let copy2 = input_border.copy_with(
            Some(blue(1.0)),
            Some(Box::new(StadiumBorder::new(BorderSide::NONE))),
            Some(8.0),
        );
        assert_eq!(copy2.border_side, blue(1.0));
        assert!(copy2.shape.eq_shape(&StadiumBorder::new(BorderSide::NONE)));
        assert_eq!(copy2.gap_padding, 8.0);
        assert!(input_border != copy2);
    }

    #[test]
    fn shaped_input_border_scale() {
        let input_border = ShapedInputBorder::new(superellipse(10.0)).border_side(width(2.0));
        let scaled = input_border.scale(2.0);
        assert_eq!(scaled.border_side.width, 4.0);
        assert!(scaled.shape.eq_shape(&superellipse(20.0)));
        assert_eq!(scaled.gap_padding, 8.0);
    }

    #[test]
    fn shaped_input_border_lerp() {
        let input_border1 = ShapedInputBorder::new(superellipse(4.0)).gap_padding(2.0);
        let input_border2 = ShapedInputBorder::new(superellipse(12.0))
            .border_side(width(3.0))
            .gap_padding(6.0);

        let lerped = <dyn ShapeBorder>::lerp(Some(&input_border1), Some(&input_border2), 0.5)
            .expect("two shaped input borders lerp");
        let lerped = lerped
            .as_any()
            .downcast_ref::<ShapedInputBorder>()
            .expect("a shaped input border");
        assert_eq!(lerped.border_side.width, 2.0);
        assert!(lerped.shape.eq_shape(&superellipse(8.0)));
        // gapPadding is not interpolated, it takes the value from the source
        assert_eq!(lerped.gap_padding, 2.0);
    }

    #[test]
    fn shaped_input_border_dimensions() {
        let input_border =
            ShapedInputBorder::new(RoundedRectangleBorder::default()).border_side(width(4.0));
        assert_eq!(input_border.dimensions(), EdgeInsetsGeometry::all(4.0));
    }

    fn pump_text_field(
        border: fn() -> ShapedInputBorder,
        floating_label_behavior: FloatingLabelBehavior,
    ) {
        with_main_thread_stack(move || {
            let mut tester = WidgetTester::on(TargetPlatform::Android);
            tester.install_platform_fonts();
            let theme = ThemeData::new(&tester.cell.borrow());
            let app = material_app(
                theme,
                Scaffold::new()
                    .body(
                        Center::new().child(
                            TextField::new().decoration(Some(
                                InputDecoration::new()
                                    .border(border())
                                    .label_text("Test Label")
                                    .floating_label_behavior(floating_label_behavior),
                            )),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(app);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn shaped_input_border_paint_with_rounded_superellipse_border() {
        pump_text_field(
            || ShapedInputBorder::new(superellipse(12.0)).border_side(blue(2.0)),
            FloatingLabelBehavior::Auto,
        );
    }

    #[test]
    fn shaped_input_border_paint_with_gap() {
        pump_text_field(
            || ShapedInputBorder::new(superellipse(12.0)).border_side(blue(2.0)),
            FloatingLabelBehavior::Always,
        );
    }

    #[test]
    fn shaped_input_border_paint_with_stadium_border() {
        pump_text_field(
            || ShapedInputBorder::new(StadiumBorder::new(BorderSide::NONE)).border_side(blue(2.0)),
            FloatingLabelBehavior::Auto,
        );
    }

    #[test]
    fn shaped_input_border_paint_interior() {
        let input_border = ShapedInputBorder::new(superellipse(12.0));
        assert!(input_border.prefer_paint_interior());

        let mut canvas = Canvas::new();
        input_border.paint_interior(
            &mut canvas,
            Rect::from_ltrb(10.0, 20.0, 30.0, 40.0),
            &Paint::default(),
            None,
        );
        canvas.build();
    }

    /// The gap Dart's `paint` cuts for a floating label: the shape's area less a rectangle two
    /// pixels high over the gap's span of the top edge, whose outline the border strokes.
    #[test]
    fn a_shaped_input_border_cuts_its_gap_from_the_top_edge() {
        let input_border = ShapedInputBorder::new(RoundedRectangleBorder::default());
        let mut canvas = Canvas::new();
        InputBorder::paint(
            &input_border,
            &mut canvas,
            Rect::from_ltrb(0.0, 0.0, 200.0, 50.0),
            Some(40.0),
            60.0,
            1.0,
            Some(TextDirection::Ltr),
        );
        let list = canvas.build();
        let Some(Op::DrawPath { path, .. }) = list
            .ops()
            .iter()
            .find(|op| matches!(op, Op::DrawPath { .. }))
        else {
            panic!("the border is drawn as a path");
        };
        // The deflated rect's top is at 0.5; the gap runs from 40 - 4 to 36 + 60 + 2 * 4.
        let contains = |x, y| path.contains(Point::new(x, y), FillRule::EvenOdd);
        assert!(contains(20.0, 1.0), "the top edge before the gap");
        assert!(!contains(70.0, 1.0), "the gap");
        assert!(contains(70.0, 10.0), "below the gap");
        assert!(contains(150.0, 1.0), "the top edge after the gap");
    }
}
