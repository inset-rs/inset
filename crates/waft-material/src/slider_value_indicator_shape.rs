//! Flutter counterpart: material_ui's `slider_value_indicator_shape.dart`.
//!
//! [`SliderComponentShape`] is an abstract class users subclass, so a trait; a theme holds a
//! [`SliderComponentShapeRef`]. Its `paint` takes the `App` (the animations, the parent box
//! and the text painter's fonts live there) and the label painter by `&mut`, which the slider
//! lends from its render object for the call. The value indicator shapes' `getPreferredSize`
//! overrides add Dart's optional `labelPainter` and `textScaleFactor`, which a trait method
//! cannot: each such shape has an inherent `get_preferred_size` that takes them, and its trait
//! method calls that one without them, failing as Dart's `labelPainter!` does.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::Debug;
use std::rc::Rc;

use waft_animation::AnyAnimation;
use waft_embedder::{
    Canvas, Color, FillRule, Offset, Paint, PaintStyle, PathBuilder, RRect, Radius, Rect, Size,
    Stroke, TextDirection, clamp_double, rrect_radii_elliptical,
};
use waft_foundation::App;
use waft_painting::{ClipContext, PaintingBinding, TextPainter};
use waft_rendering::{AnyRenderBox, PaintingContext};

use crate::range_slider_parts::RangeSliderValueIndicatorShape;
use crate::slider_parts::{evaluate_color_tween, evaluate_tween};
use crate::slider_theme::{SliderThemeData, Thumb};

/// Base class for slider thumb, thumb overlay, and value indicator shapes.
///
/// Create a subclass of this if you would like a custom shape.
///
/// All shapes are painted to the same canvas and ordering is important.
/// The overlay is painted first, then the value indicator, then the thumb.
///
/// The thumb painting can be skipped by specifying [`no_thumb`](<dyn SliderComponentShape>::no_thumb)
/// for `SliderThemeData::thumb_shape`.
///
/// The overlay painting can be skipped by specifying
/// [`no_overlay`](<dyn SliderComponentShape>::no_overlay) for `SliderThemeData::overlay_shape`.
///
/// Equality is [`eq_shape`](Self::eq_shape): identity, as Dart's default `==`, unless the
/// shape overrides it.
///
/// See also:
///
///  * `RoundSliderThumbShape`, which is the default `Slider`'s thumb shape that
///    paints a solid circle.
///  * [`RoundSliderOverlayShape`], which is the default `Slider` and
///    `RangeSlider`'s overlay shape that paints a transparent circle.
///  * [`PaddleSliderValueIndicatorShape`], which is the default `Slider`'s value
///    indicator shape that paints a custom path with text in it.
pub trait SliderComponentShape: Any + Debug {
    /// Returns the preferred size of the shape, based on the given conditions.
    fn get_preferred_size(&self, is_enabled: bool, is_discrete: bool) -> Size;

    /// Paints the shape, taking into account the state passed to it.
    ///
    /// The `context` argument is the same as the one that includes the `Slider`'s
    /// render box.
    ///
    /// The `center` argument is the offset for where this shape's center should be
    /// painted. This offset is relative to the origin of the `context` canvas.
    ///
    /// The `activation_animation` argument is an animation triggered when the user
    /// begins to interact with the slider. It reverses when the user stops interacting
    /// with the slider.
    ///
    /// The `enable_animation` argument is an animation triggered when the `Slider`
    /// is enabled, and it reverses when the slider is disabled. The `Slider` is
    /// enabled when `Slider::on_changed` is not null. Use this to paint
    /// intermediate frames for this shape when the slider changes enabled state.
    ///
    /// The `is_discrete` argument is true if `Slider::divisions` is non-null. When
    /// true, the slider will render tick marks on top of the track.
    ///
    /// If the `label_painter` argument is non-null, then `TextPainter::paint`
    /// should be called on the `label_painter` with the location that the label
    /// should appear. If the `label_painter` argument is null, then no label was
    /// supplied to the `Slider`.
    ///
    /// The `parent_box` argument is the `RenderBox` of the `Slider`. Its attributes,
    /// such as size, can be used to assist in painting this shape.
    ///
    /// the `slider_theme` argument is the theme assigned to the `Slider` that this
    /// shape belongs to.
    ///
    /// The `text_direction` argument can be used to determine how any extra text
    /// or graphics (besides the text painted by the `label_painter`) should be
    /// positioned. The `label_painter` already has the `text_direction` set.
    ///
    /// The `value` argument is the current parametric value (from 0.0 to 1.0) of
    /// the slider.
    ///
    /// The `text_scale_factor` argument can be used to determine whether the
    /// component should paint larger or smaller, depending on whether
    /// `text_scale_factor` is greater than 1 for larger, and between 0 and 1 for
    /// smaller. It's usually computed from `MediaQueryData::text_scaler`.
    ///
    /// The `size_with_overflow` argument can be used to determine the bounds the
    /// drawing of the components that are outside of the regular slider bounds.
    /// It's the size of the box, whose center is aligned with the slider's
    /// bounds, that the value indicators must be drawn within. Typically, it is
    /// bigger than the slider.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        enable_animation: AnyAnimation<f64>,
        is_discrete: bool,
        label_painter: &mut TextPainter,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        text_direction: TextDirection,
        value: f64,
        text_scale_factor: f64,
        size_with_overflow: Size,
    );

    /// Downcast support for Dart's `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: identity unless a shape overrides it.
    fn eq_shape(&self, other: &dyn SliderComponentShape) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

/// What a field or parameter Dart types as `SliderComponentShape` becomes.
pub type SliderComponentShapeRef = Rc<dyn SliderComponentShape>;

impl PartialEq for dyn SliderComponentShape {
    fn eq(&self, other: &dyn SliderComponentShape) -> bool {
        self.eq_shape(other)
    }
}

thread_local! {
    static NO_THUMB: SliderComponentShapeRef = Rc::new(EmptySliderComponentShape);
    static NO_OVERLAY: SliderComponentShapeRef = Rc::new(EmptySliderComponentShape);
}

impl dyn SliderComponentShape {
    /// Special instance of [`SliderComponentShape`] to skip the thumb drawing.
    ///
    /// Each call hands out the same instance, as Dart's `static final` field is one object.
    ///
    /// See also:
    ///
    ///  * `SliderThemeData::thumb_shape`, which is the shape that the `Slider`
    ///    uses when painting the thumb.
    pub fn no_thumb() -> SliderComponentShapeRef {
        NO_THUMB.with(Rc::clone)
    }

    /// Special instance of [`SliderComponentShape`] to skip the overlay drawing.
    ///
    /// Each call hands out the same instance, as Dart's `static final` field is one object.
    ///
    /// See also:
    ///
    ///  * `SliderThemeData::overlay_shape`, which is the shape that the `Slider`
    ///    uses when painting the overlay.
    pub fn no_overlay() -> SliderComponentShapeRef {
        NO_OVERLAY.with(Rc::clone)
    }
}

/// A special version of [`SliderComponentShape`] that has a zero size and paints
/// nothing.
///
/// This class is used to create a special instance of a [`SliderComponentShape`]
/// that will not paint any component shape. A static reference is stored in
/// [`no_thumb`](<dyn SliderComponentShape>::no_thumb) and
/// [`no_overlay`](<dyn SliderComponentShape>::no_overlay). When this value
/// is specified for `SliderThemeData::thumb_shape`, the thumb painting is
/// skipped. When this value is specified for `SliderThemeData::overlay_shape`,
/// the overlay painting is skipped.
#[derive(Debug)]
struct EmptySliderComponentShape;

impl SliderComponentShape for EmptySliderComponentShape {
    fn get_preferred_size(&self, _is_enabled: bool, _is_discrete: bool) -> Size {
        Size::ZERO
    }

    fn paint(
        &self,
        _app: &mut App,
        _context: &mut PaintingContext,
        _center: Offset,
        _activation_animation: AnyAnimation<f64>,
        _enable_animation: AnyAnimation<f64>,
        _is_discrete: bool,
        _label_painter: &mut TextPainter,
        _parent_box: AnyRenderBox,
        _slider_theme: &SliderThemeData,
        _text_direction: TextDirection,
        _value: f64,
        _text_scale_factor: f64,
        _size_with_overflow: Size,
    ) {
        // no-op.
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The default shape of a `Slider`'s thumb overlay.
///
/// The shape of the overlay is a circle with the same center as the thumb, but
/// with a larger radius. It animates to full size when the thumb is pressed,
/// and animates back down to size 0 when it is released. It is painted behind
/// the thumb, and is expected to extend beyond the bounds of the thumb so that
/// it is visible.
///
/// The overlay color is defined by `SliderThemeData::overlay_color`.
///
/// See also:
///
///  * `Slider`, which includes an overlay defined by this shape.
///  * `SliderTheme`, which can be used to configure the overlay shape of all
///    sliders in a widget subtree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundSliderOverlayShape {
    /// The preferred radius of the round thumb shape when enabled.
    ///
    /// If it is not provided, then half of the `SliderThemeData::track_height` is
    /// used.
    pub overlay_radius: f64,
}

impl RoundSliderOverlayShape {
    /// Create a slider thumb overlay that draws a circle.
    pub const fn new() -> RoundSliderOverlayShape {
        RoundSliderOverlayShape {
            overlay_radius: 24.0,
        }
    }

    /// Dart `RoundSliderOverlayShape(overlayRadius:)`.
    pub const fn overlay_radius(mut self, overlay_radius: f64) -> RoundSliderOverlayShape {
        self.overlay_radius = overlay_radius;
        self
    }
}

impl Default for RoundSliderOverlayShape {
    fn default() -> RoundSliderOverlayShape {
        RoundSliderOverlayShape::new()
    }
}

impl SliderComponentShape for RoundSliderOverlayShape {
    fn get_preferred_size(&self, _is_enabled: bool, _is_discrete: bool) -> Size {
        Size::from_radius(self.overlay_radius)
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        _enable_animation: AnyAnimation<f64>,
        _is_discrete: bool,
        _label_painter: &mut TextPainter,
        _parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        _text_direction: TextDirection,
        _value: f64,
        _text_scale_factor: f64,
        _size_with_overflow: Size,
    ) {
        let radius = evaluate_tween(app, 0.0, self.overlay_radius, activation_animation);
        let color = slider_theme
            .overlay_color
            .as_ref()
            .expect("a slider theme with an overlay color")
            .color();
        context
            .canvas()
            .draw_circle(center, radius as f32, &Paint::from_color(color.into()));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderComponentShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The default shape of a `Slider`'s value indicator.
///
/// See also:
///
///  * `Slider`, which includes a value indicator defined by this shape.
///  * `SliderTheme`, which can be used to configure the slider value indicator
///    of all sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectangularSliderValueIndicatorShape;

impl RectangularSliderValueIndicatorShape {
    const PATH_PAINTER: RectangularSliderValueIndicatorPathPainter =
        RectangularSliderValueIndicatorPathPainter;

    /// Create a slider value indicator that resembles a rectangular tooltip.
    pub const fn new() -> RectangularSliderValueIndicatorShape {
        RectangularSliderValueIndicatorShape
    }

    /// Dart's `getPreferredSize` with its optional `labelPainter` and `textScaleFactor`,
    /// which this shape needs.
    pub fn get_preferred_size(
        &self,
        _is_enabled: bool,
        _is_discrete: bool,
        label_painter: Option<&TextPainter>,
        text_scale_factor: Option<f64>,
    ) -> Size {
        debug_assert!(label_painter.is_some());
        debug_assert!(text_scale_factor.is_some_and(|factor| factor >= 0.0));
        Self::PATH_PAINTER.get_preferred_size(
            label_painter.expect("a label painter"),
            text_scale_factor.expect("a text scale factor"),
        )
    }
}

impl SliderComponentShape for RectangularSliderValueIndicatorShape {
    fn get_preferred_size(&self, is_enabled: bool, is_discrete: bool) -> Size {
        RectangularSliderValueIndicatorShape::get_preferred_size(
            self,
            is_enabled,
            is_discrete,
            None,
            None,
        )
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        _enable_animation: AnyAnimation<f64>,
        _is_discrete: bool,
        label_painter: &mut TextPainter,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        _text_direction: TextDirection,
        _value: f64,
        text_scale_factor: f64,
        size_with_overflow: Size,
    ) {
        let scale = activation_animation.value(app);
        Self::PATH_PAINTER.paint(
            app,
            parent_box,
            context.canvas(),
            center,
            scale,
            label_painter,
            text_scale_factor,
            size_with_overflow,
            slider_theme
                .value_indicator_color
                .expect("a slider theme with a value indicator color"),
            slider_theme.value_indicator_stroke_color,
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderComponentShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The default shape of a `RangeSlider`'s value indicators.
///
/// See also:
///
///  * `RangeSlider`, which includes value indicators defined by this shape.
///  * `SliderTheme`, which can be used to configure the range slider value
///    indicator of all sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectangularRangeSliderValueIndicatorShape;

impl RectangularRangeSliderValueIndicatorShape {
    const PATH_PAINTER: RectangularSliderValueIndicatorPathPainter =
        RectangularSliderValueIndicatorPathPainter;

    /// Create a range slider value indicator that resembles a rectangular tooltip.
    pub const fn new() -> RectangularRangeSliderValueIndicatorShape {
        RectangularRangeSliderValueIndicatorShape
    }
}

impl RangeSliderValueIndicatorShape for RectangularRangeSliderValueIndicatorShape {
    fn get_preferred_size(
        &self,
        _is_enabled: bool,
        _is_discrete: bool,
        label_painter: &TextPainter,
        text_scale_factor: f64,
    ) -> Size {
        debug_assert!(text_scale_factor >= 0.0);
        Self::PATH_PAINTER.get_preferred_size(label_painter, text_scale_factor)
    }

    fn get_horizontal_shift(
        &self,
        app: &App,
        parent_box: Option<AnyRenderBox>,
        center: Option<Offset>,
        label_painter: Option<&TextPainter>,
        activation_animation: Option<AnyAnimation<f64>>,
        text_scale_factor: Option<f64>,
        size_with_overflow: Option<Size>,
    ) -> f64 {
        Self::PATH_PAINTER.get_horizontal_shift(
            app,
            parent_box.expect("a parent box"),
            center.expect("a center"),
            label_painter.expect("a label painter"),
            text_scale_factor.expect("a text scale factor"),
            size_with_overflow.expect("a size with overflow"),
            activation_animation
                .expect("an activation animation")
                .value(app),
        )
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        _enable_animation: AnyAnimation<f64>,
        _is_discrete: Option<bool>,
        is_on_top: Option<bool>,
        label_painter: &mut TextPainter,
        text_scale_factor: Option<f64>,
        size_with_overflow: Option<Size>,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        _text_direction: Option<TextDirection>,
        _value: Option<f64>,
        _thumb: Option<Thumb>,
    ) {
        let scale = activation_animation.value(app);
        Self::PATH_PAINTER.paint(
            app,
            parent_box,
            context.canvas(),
            center,
            scale,
            label_painter,
            text_scale_factor.expect("a text scale factor"),
            size_with_overflow.expect("a size with overflow"),
            slider_theme
                .value_indicator_color
                .expect("a slider theme with a value indicator color"),
            if is_on_top.expect("whether this indicator is on top") {
                slider_theme.overlapping_shape_stroke_color
            } else {
                slider_theme.value_indicator_stroke_color
            },
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn RangeSliderValueIndicatorShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// Dart's `_RectangularSliderValueIndicatorPathPainter`.
struct RectangularSliderValueIndicatorPathPainter;

impl RectangularSliderValueIndicatorPathPainter {
    const TRIANGLE_HEIGHT: f64 = 8.0;
    const LABEL_PADDING: f64 = 16.0;
    const PREFERRED_HEIGHT: f64 = 32.0;
    const MIN_LABEL_WIDTH: f64 = 16.0;
    const BOTTOM_TIP_Y_OFFSET: f64 = 14.0;
    const PREFERRED_HALF_HEIGHT: f64 = Self::PREFERRED_HEIGHT / 2.0;
    const UPPER_RECT_RADIUS: f64 = 4.0;

    fn get_preferred_size(&self, label_painter: &TextPainter, text_scale_factor: f64) -> Size {
        Size::new(
            self.upper_rectangle_width(label_painter, 1.0, text_scale_factor),
            label_painter.height() + Self::LABEL_PADDING,
        )
    }

    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn get_horizontal_shift(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        center: Offset,
        label_painter: &TextPainter,
        text_scale_factor: f64,
        size_with_overflow: Size,
        scale: f64,
    ) -> f64 {
        debug_assert!(!size_with_overflow.is_empty());

        const EDGE_PADDING: f64 = 8.0;
        let rectangle_width = self.upper_rectangle_width(label_painter, scale, text_scale_factor);

        // Value indicator draws on the Overlay and by using the global Offset
        // we are making sure we use the bounds of the Overlay instead of the Slider.
        let global_center = parent_box.local_to_global(app, center, None);

        // The rectangle must be shifted towards the center so that it minimizes the
        // chance of it rendering outside the bounds of the render box. If the shift
        // is negative, then the lobe is shifted from right to left, and if it is
        // positive, then the lobe is shifted from left to right.
        let overflow_left = 0.0_f64.max(rectangle_width / 2.0 - global_center.dx() + EDGE_PADDING);
        let overflow_right = 0.0_f64.max(
            rectangle_width / 2.0
                - (size_with_overflow.width() - global_center.dx() - EDGE_PADDING),
        );

        if rectangle_width < size_with_overflow.width() {
            overflow_left - overflow_right
        } else if overflow_left - overflow_right > 0.0 {
            overflow_left - (EDGE_PADDING * text_scale_factor)
        } else {
            -overflow_right + (EDGE_PADDING * text_scale_factor)
        }
    }

    fn upper_rectangle_width(
        &self,
        label_painter: &TextPainter,
        scale: f64,
        text_scale_factor: f64,
    ) -> f64 {
        let unscaled_width = (Self::MIN_LABEL_WIDTH * text_scale_factor).max(label_painter.width())
            + Self::LABEL_PADDING * 2.0;
        unscaled_width * scale
    }

    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        parent_box: AnyRenderBox,
        canvas: &mut Canvas,
        center: Offset,
        scale: f64,
        label_painter: &mut TextPainter,
        text_scale_factor: f64,
        size_with_overflow: Size,
        background_paint_color: Color,
        stroke_paint_color: Option<Color>,
    ) {
        if scale == 0.0 {
            // Zero scale essentially means "do not draw anything", so it's safe to just return.
            return;
        }
        debug_assert!(!size_with_overflow.is_empty());

        let rectangle_width = self.upper_rectangle_width(label_painter, scale, text_scale_factor);
        let horizontal_shift = self.get_horizontal_shift(
            app,
            parent_box,
            center,
            label_painter,
            text_scale_factor,
            size_with_overflow,
            scale,
        );

        let rect_height = label_painter.height() + Self::LABEL_PADDING;
        let upper_rect = Rect::from_ltwh(
            -rectangle_width / 2.0 + horizontal_shift,
            -Self::TRIANGLE_HEIGHT - rect_height,
            rectangle_width,
            rect_height,
        );

        let mut triangle_path = PathBuilder::new();
        // Dart's `lineTo` on an empty path starts its contour at the origin.
        triangle_path.move_to((0.0, 0.0));
        triangle_path
            .line_to((-Self::TRIANGLE_HEIGHT as f32, -Self::TRIANGLE_HEIGHT as f32))
            .line_to((Self::TRIANGLE_HEIGHT as f32, -Self::TRIANGLE_HEIGHT as f32))
            .close();
        let fill_paint = Paint::from_color(background_paint_color.into());
        let upper_rrect =
            RRect::from_rect_and_radius(upper_rect, Radius::circular(Self::UPPER_RECT_RADIUS));
        triangle_path.rrect_radii_elliptical(
            upper_rrect.outer_rect(),
            rrect_radii_elliptical(upper_rrect),
        );
        let triangle_path = triangle_path.build();

        canvas.save();
        // Prepare the canvas for the base of the tooltip, which is relative to the
        // center of the thumb.
        canvas.translate(
            center.dx() as f32,
            (center.dy() - Self::BOTTOM_TIP_Y_OFFSET) as f32,
        );
        canvas.scale(scale as f32, scale as f32);
        if let Some(stroke_paint_color) = stroke_paint_color {
            let stroke_paint = Paint {
                color: stroke_paint_color.into(),
                style: PaintStyle::Stroke(Stroke::new(1.0)),
                ..Paint::default()
            };
            canvas.draw_path(&triangle_path, FillRule::NonZero, &stroke_paint);
        }
        canvas.draw_path(&triangle_path, FillRule::NonZero, &fill_paint);

        // The label text is centered within the value indicator.
        let bottom_tip_to_upper_rect_translate_y =
            -Self::PREFERRED_HALF_HEIGHT / 2.0 - upper_rect.height();
        canvas.translate(0.0, bottom_tip_to_upper_rect_translate_y as f32);
        let box_center = Offset::new(horizontal_shift, upper_rect.height() / 2.0);
        let half_label_painter_offset =
            Offset::new(label_painter.width() / 2.0, label_painter.height() / 2.0);
        let label_offset = box_center - half_label_painter_offset;
        let fonts = PaintingBinding::instance(app).fonts(app);
        label_painter.paint(app.get_mut(fonts), canvas, label_offset);
        canvas.restore();
    }
}

/// A variant shape of a `Slider`'s value indicator . The value indicator is in
/// the shape of an upside-down pear.
///
/// See also:
///
///  * `Slider`, which includes a value indicator defined by this shape.
///  * `SliderTheme`, which can be used to configure the slider value indicator
///    of all sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PaddleSliderValueIndicatorShape;

impl PaddleSliderValueIndicatorShape {
    const PATH_PAINTER: PaddleSliderValueIndicatorPathPainter =
        PaddleSliderValueIndicatorPathPainter;

    /// Create a slider value indicator in the shape of an upside-down pear.
    pub const fn new() -> PaddleSliderValueIndicatorShape {
        PaddleSliderValueIndicatorShape
    }

    /// Dart's `getPreferredSize` with its optional `labelPainter` and `textScaleFactor`,
    /// which this shape needs.
    pub fn get_preferred_size(
        &self,
        _is_enabled: bool,
        _is_discrete: bool,
        label_painter: Option<&TextPainter>,
        text_scale_factor: Option<f64>,
    ) -> Size {
        debug_assert!(label_painter.is_some());
        debug_assert!(text_scale_factor.is_some_and(|factor| factor >= 0.0));
        Self::PATH_PAINTER.get_preferred_size(
            label_painter.expect("a label painter"),
            text_scale_factor.expect("a text scale factor"),
        )
    }
}

impl SliderComponentShape for PaddleSliderValueIndicatorShape {
    fn get_preferred_size(&self, is_enabled: bool, is_discrete: bool) -> Size {
        PaddleSliderValueIndicatorShape::get_preferred_size(
            self,
            is_enabled,
            is_discrete,
            None,
            None,
        )
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        enable_animation: AnyAnimation<f64>,
        _is_discrete: bool,
        label_painter: &mut TextPainter,
        _parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        _text_direction: TextDirection,
        _value: f64,
        text_scale_factor: f64,
        size_with_overflow: Size,
    ) {
        debug_assert!(!size_with_overflow.is_empty());
        let paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_thumb_color,
                slider_theme.value_indicator_color,
                enable_animation,
            )
            .expect("both value indicator colors are set")
            .into(),
        );
        let scale = activation_animation.value(app);
        Self::PATH_PAINTER.paint(
            app,
            context.canvas(),
            center,
            paint,
            scale,
            label_painter,
            text_scale_factor,
            size_with_overflow,
            slider_theme.value_indicator_stroke_color,
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderComponentShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// A variant shape of a `RangeSlider`'s value indicators. The value indicator
/// is in the shape of an upside-down pear.
///
/// See also:
///
///  * `RangeSlider`, which includes value indicators defined by this shape.
///  * `SliderTheme`, which can be used to configure the range slider value
///    indicator of all sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PaddleRangeSliderValueIndicatorShape;

impl PaddleRangeSliderValueIndicatorShape {
    const PATH_PAINTER: PaddleSliderValueIndicatorPathPainter =
        PaddleSliderValueIndicatorPathPainter;

    /// Create a slider value indicator in the shape of an upside-down pear.
    pub const fn new() -> PaddleRangeSliderValueIndicatorShape {
        PaddleRangeSliderValueIndicatorShape
    }
}

impl RangeSliderValueIndicatorShape for PaddleRangeSliderValueIndicatorShape {
    fn get_preferred_size(
        &self,
        _is_enabled: bool,
        _is_discrete: bool,
        label_painter: &TextPainter,
        text_scale_factor: f64,
    ) -> Size {
        debug_assert!(text_scale_factor >= 0.0);
        Self::PATH_PAINTER.get_preferred_size(label_painter, text_scale_factor)
    }

    fn get_horizontal_shift(
        &self,
        app: &App,
        _parent_box: Option<AnyRenderBox>,
        center: Option<Offset>,
        label_painter: Option<&TextPainter>,
        activation_animation: Option<AnyAnimation<f64>>,
        text_scale_factor: Option<f64>,
        size_with_overflow: Option<Size>,
    ) -> f64 {
        Self::PATH_PAINTER.get_horizontal_shift(
            center.expect("a center"),
            label_painter.expect("a label painter"),
            activation_animation
                .expect("an activation animation")
                .value(app),
            text_scale_factor.expect("a text scale factor"),
            size_with_overflow.expect("a size with overflow"),
        )
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        enable_animation: AnyAnimation<f64>,
        _is_discrete: Option<bool>,
        is_on_top: Option<bool>,
        label_painter: &mut TextPainter,
        text_scale_factor: Option<f64>,
        size_with_overflow: Option<Size>,
        _parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        _text_direction: Option<TextDirection>,
        _value: Option<f64>,
        _thumb: Option<Thumb>,
    ) {
        let is_on_top = is_on_top.unwrap_or(false);
        let size_with_overflow = size_with_overflow.expect("a size with overflow");
        debug_assert!(!size_with_overflow.is_empty());
        let paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_thumb_color,
                slider_theme.value_indicator_color,
                enable_animation,
            )
            .expect("both value indicator colors are set")
            .into(),
        );
        let scale = activation_animation.value(app);
        // Add a stroke of 1dp around the top paddle.
        Self::PATH_PAINTER.paint(
            app,
            context.canvas(),
            center,
            paint,
            scale,
            label_painter,
            text_scale_factor.expect("a text scale factor"),
            size_with_overflow,
            if is_on_top {
                slider_theme.overlapping_shape_stroke_color
            } else {
                slider_theme.value_indicator_stroke_color
            },
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn RangeSliderValueIndicatorShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// Dart's `_PaddleSliderValueIndicatorPathPainter`.
struct PaddleSliderValueIndicatorPathPainter;

impl PaddleSliderValueIndicatorPathPainter {
    // These constants define the shape of the default value indicator.
    // The value indicator changes shape based on the size of
    // the label: The top lobe spreads horizontally, and the
    // top arc on the neck moves down to keep it merging smoothly
    // with the top lobe as it expands.

    // Radius of the top lobe of the value indicator.
    const TOP_LOBE_RADIUS: f64 = 16.0;
    const MIN_LABEL_WIDTH: f64 = 16.0;
    // Radius of the bottom lobe of the value indicator.
    const BOTTOM_LOBE_RADIUS: f64 = 10.0;
    const LABEL_PADDING: f64 = 8.0;
    const DISTANCE_BETWEEN_TOP_BOTTOM_CENTERS: f64 = 40.0;
    const MIDDLE_NECK_WIDTH: f64 = 3.0;
    const BOTTOM_NECK_RADIUS: f64 = 4.5;
    // The base of the triangle between the top lobe center and the centers of
    // the two top neck arcs.
    const NECK_TRIANGLE_BASE: f64 = Self::TOP_NECK_RADIUS + Self::MIDDLE_NECK_WIDTH / 2.0;
    const RIGHT_BOTTOM_NECK_CENTER_X: f64 =
        Self::MIDDLE_NECK_WIDTH / 2.0 + Self::BOTTOM_NECK_RADIUS;
    const RIGHT_BOTTOM_NECK_ANGLE_START: f64 = PI;
    const TOP_LOBE_CENTER: Offset = Offset::new(0.0, -Self::DISTANCE_BETWEEN_TOP_BOTTOM_CENTERS);
    const TOP_NECK_RADIUS: f64 = 13.0;
    // The length of the hypotenuse of the triangle formed by the center
    // of the left top lobe arc and the center of the top left neck arc.
    // Used to calculate the position of the center of the arc.
    const NECK_TRIANGLE_HYPOTENUSE: f64 = Self::TOP_LOBE_RADIUS + Self::TOP_NECK_RADIUS;
    // Some convenience values to help readability.
    const TWO_SEVENTY_DEGREES: f64 = 3.0 * PI / 2.0;
    const NINETY_DEGREES: f64 = PI / 2.0;
    const THIRTY_DEGREES: f64 = PI / 6.0;
    const PREFERRED_HEIGHT: f64 = Self::DISTANCE_BETWEEN_TOP_BOTTOM_CENTERS
        + Self::TOP_LOBE_RADIUS
        + Self::BOTTOM_LOBE_RADIUS;
    // Set to true if you want a rectangle to be drawn around the label bubble.
    // This helps with building tests that check that the label draws in the right
    // place (because it prints the rect in the failed test output). It should not
    // be checked in while set to "true".
    const DEBUGGING_LABEL_LOCATION: bool = false;

    fn get_preferred_size(&self, label_painter: &TextPainter, text_scale_factor: f64) -> Size {
        debug_assert!(text_scale_factor >= 0.0);
        let width = (Self::MIN_LABEL_WIDTH * text_scale_factor).max(label_painter.width())
            + Self::LABEL_PADDING * 2.0 * text_scale_factor;
        Size::new(width, Self::PREFERRED_HEIGHT * text_scale_factor)
    }

    // Adds an arc to the path that has the attributes passed in. This is
    // a convenience to make adding arcs have less boilerplate.
    fn add_arc(
        path: &mut PathBuilder,
        center: Offset,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    ) {
        debug_assert!(center.is_finite());
        path.arc(
            center,
            radius as f32,
            start_angle as f32,
            (end_angle - start_angle) as f32,
        );
    }

    fn get_horizontal_shift(
        &self,
        center: Offset,
        label_painter: &TextPainter,
        scale: f64,
        text_scale_factor: f64,
        size_with_overflow: Size,
    ) -> f64 {
        debug_assert!(!size_with_overflow.is_empty());
        let inverse_text_scale = if text_scale_factor != 0.0 {
            1.0 / text_scale_factor
        } else {
            0.0
        };
        let label_half_width = label_painter.width() / 2.0;
        let half_width_needed = 0.0_f64.max(
            inverse_text_scale * label_half_width - (Self::TOP_LOBE_RADIUS - Self::LABEL_PADDING),
        );
        let shift = self.get_ideal_offset(
            half_width_needed,
            text_scale_factor * scale,
            center,
            size_with_overflow.width(),
        );
        shift * text_scale_factor
    }

    // Determines the "best" offset to keep the bubble within the slider. The
    // calling code will bound that with the available movement in the paddle shape.
    fn get_ideal_offset(
        &self,
        half_width_needed: f64,
        scale: f64,
        center: Offset,
        width_with_overflow: f64,
    ) -> f64 {
        const EDGE_MARGIN: f64 = 8.0;
        let top_lobe_rect = Rect::from_ltwh(
            -Self::TOP_LOBE_RADIUS - half_width_needed,
            -Self::TOP_LOBE_RADIUS - Self::DISTANCE_BETWEEN_TOP_BOTTOM_CENTERS,
            2.0 * (Self::TOP_LOBE_RADIUS + half_width_needed),
            2.0 * Self::TOP_LOBE_RADIUS,
        );
        // We can just multiply by scale instead of a transform, since we're scaling
        // around (0, 0).
        let top_left = (top_lobe_rect.top_left() * scale) + center;
        let bottom_right = (top_lobe_rect.bottom_right() * scale) + center;
        let mut shift = 0.0;

        if top_left.dx() < EDGE_MARGIN {
            shift = EDGE_MARGIN - top_left.dx();
        }

        let end_global = width_with_overflow;
        if bottom_right.dx() > end_global - EDGE_MARGIN {
            shift = end_global - EDGE_MARGIN - bottom_right.dx();
        }

        shift = if scale == 0.0 { 0.0 } else { shift / scale };
        if shift < 0.0 {
            // Shifting to the left.
            shift = shift.max(-half_width_needed);
        } else {
            // Shifting to the right.
            shift = shift.min(half_width_needed);
        }
        shift
    }

    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        canvas: &mut Canvas,
        center: Offset,
        paint: Paint,
        scale: f64,
        label_painter: &mut TextPainter,
        text_scale_factor: f64,
        size_with_overflow: Size,
        stroke_paint_color: Option<Color>,
    ) {
        if scale == 0.0 {
            // Zero scale essentially means "do not draw anything", so it's safe to just return. Otherwise,
            // our math below will attempt to divide by zero and send needless NaNs to the engine.
            return;
        }
        debug_assert!(!size_with_overflow.is_empty());

        // The entire value indicator should scale with the size of the label,
        // to keep it large enough to encompass the label text.
        let overall_scale = scale * text_scale_factor;
        let inverse_text_scale = if text_scale_factor != 0.0 {
            1.0 / text_scale_factor
        } else {
            0.0
        };
        let label_half_width = label_painter.width() / 2.0;

        canvas.save();
        canvas.translate(center.dx() as f32, center.dy() as f32);
        canvas.scale(overall_scale as f32, overall_scale as f32);

        let bottom_neck_triangle_hypotenuse =
            Self::BOTTOM_NECK_RADIUS + Self::BOTTOM_LOBE_RADIUS / overall_scale;
        let right_bottom_neck_center_y = -(bottom_neck_triangle_hypotenuse.powi(2)
            - Self::RIGHT_BOTTOM_NECK_CENTER_X.powi(2))
        .sqrt();
        let right_bottom_neck_angle_end =
            PI + (right_bottom_neck_center_y / Self::RIGHT_BOTTOM_NECK_CENTER_X).atan();
        let mut path = PathBuilder::new();
        path.move_to((
            (Self::MIDDLE_NECK_WIDTH / 2.0) as f32,
            right_bottom_neck_center_y as f32,
        ));
        Self::add_arc(
            &mut path,
            Offset::new(Self::RIGHT_BOTTOM_NECK_CENTER_X, right_bottom_neck_center_y),
            Self::BOTTOM_NECK_RADIUS,
            Self::RIGHT_BOTTOM_NECK_ANGLE_START,
            right_bottom_neck_angle_end,
        );
        Self::add_arc(
            &mut path,
            Offset::ZERO,
            Self::BOTTOM_LOBE_RADIUS / overall_scale,
            right_bottom_neck_angle_end - PI,
            2.0 * PI - right_bottom_neck_angle_end,
        );
        Self::add_arc(
            &mut path,
            Offset::new(
                -Self::RIGHT_BOTTOM_NECK_CENTER_X,
                right_bottom_neck_center_y,
            ),
            Self::BOTTOM_NECK_RADIUS,
            PI - right_bottom_neck_angle_end,
            0.0,
        );

        // This is the needed extra width for the label. It is only positive when
        // the label exceeds the minimum size contained by the round top lobe.
        let half_width_needed = 0.0_f64.max(
            inverse_text_scale * label_half_width - (Self::TOP_LOBE_RADIUS - Self::LABEL_PADDING),
        );

        let shift = self.get_ideal_offset(
            half_width_needed,
            overall_scale,
            center,
            size_with_overflow.width(),
        );
        let left_width_needed = half_width_needed - shift;
        let right_width_needed = half_width_needed + shift;

        // The parameter that describes how far along the transition from round to
        // stretched we are.
        let left_amount = 0.0_f64.max(1.0_f64.min(left_width_needed / Self::NECK_TRIANGLE_BASE));
        let right_amount = 0.0_f64.max(1.0_f64.min(right_width_needed / Self::NECK_TRIANGLE_BASE));
        // The angle between the top neck arc's center and the top lobe's center
        // and vertical. The base amount is chosen so that the neck is smooth,
        // even when the lobe is shifted due to its size.
        let left_theta = (1.0 - left_amount) * Self::THIRTY_DEGREES;
        let right_theta = (1.0 - right_amount) * Self::THIRTY_DEGREES;
        // The center of the top left neck arc.
        let left_top_neck_center = Offset::new(
            -Self::NECK_TRIANGLE_BASE,
            Self::TOP_LOBE_CENTER.dy() + left_theta.cos() * Self::NECK_TRIANGLE_HYPOTENUSE,
        );
        let neck_right_center = Offset::new(
            Self::NECK_TRIANGLE_BASE,
            Self::TOP_LOBE_CENTER.dy() + right_theta.cos() * Self::NECK_TRIANGLE_HYPOTENUSE,
        );
        let left_neck_arc_angle = Self::NINETY_DEGREES - left_theta;
        let right_neck_arc_angle = PI + Self::NINETY_DEGREES - right_theta;
        // The distance between the end of the bottom neck arc and the beginning of
        // the top neck arc. We use this to shrink/expand it based on the scale
        // factor of the value indicator.
        let neck_stretch_baseline = 0.0_f64.max(
            right_bottom_neck_center_y - left_top_neck_center.dy().max(neck_right_center.dy()),
        );
        let t = inverse_text_scale.powf(3.0);
        let stretch = clamp_double(neck_stretch_baseline * t, 0.0, 10.0 * neck_stretch_baseline);
        let neck_stretch = Offset::new(0.0, neck_stretch_baseline - stretch);

        if cfg!(debug_assertions) && Self::DEBUGGING_LABEL_LOCATION {
            let left_center =
                Self::TOP_LOBE_CENTER - Offset::new(left_width_needed, 0.0) + neck_stretch;
            let right_center =
                Self::TOP_LOBE_CENTER + Offset::new(right_width_needed, 0.0) + neck_stretch;
            let value_rect = Rect::from_ltrb(
                left_center.dx() - Self::TOP_LOBE_RADIUS,
                left_center.dy() - Self::TOP_LOBE_RADIUS,
                right_center.dx() + Self::TOP_LOBE_RADIUS,
                right_center.dy() + Self::TOP_LOBE_RADIUS,
            );
            let outline_paint = Paint {
                color: Color::new(0xffff0000).into(),
                style: PaintStyle::Stroke(Stroke::new(1.0)),
                ..Paint::default()
            };
            canvas.draw_rect(value_rect, &outline_paint);
        }

        Self::add_arc(
            &mut path,
            left_top_neck_center + neck_stretch,
            Self::TOP_NECK_RADIUS,
            0.0,
            -left_neck_arc_angle,
        );
        Self::add_arc(
            &mut path,
            Self::TOP_LOBE_CENTER - Offset::new(left_width_needed, 0.0) + neck_stretch,
            Self::TOP_LOBE_RADIUS,
            Self::NINETY_DEGREES + left_theta,
            Self::TWO_SEVENTY_DEGREES,
        );
        Self::add_arc(
            &mut path,
            Self::TOP_LOBE_CENTER + Offset::new(right_width_needed, 0.0) + neck_stretch,
            Self::TOP_LOBE_RADIUS,
            Self::TWO_SEVENTY_DEGREES,
            Self::TWO_SEVENTY_DEGREES + PI - right_theta,
        );
        Self::add_arc(
            &mut path,
            neck_right_center + neck_stretch,
            Self::TOP_NECK_RADIUS,
            right_neck_arc_angle,
            PI,
        );
        let path = path.build();

        if let Some(stroke_paint_color) = stroke_paint_color {
            let stroke_paint = Paint {
                color: stroke_paint_color.into(),
                style: PaintStyle::Stroke(Stroke::new(1.0)),
                ..Paint::default()
            };
            canvas.draw_path(&path, FillRule::NonZero, &stroke_paint);
        }

        canvas.draw_path(&path, FillRule::NonZero, &paint);

        // Draw the label.
        canvas.save();
        canvas.translate(
            shift as f32,
            (-Self::DISTANCE_BETWEEN_TOP_BOTTOM_CENTERS + neck_stretch.dy()) as f32,
        );
        canvas.scale(inverse_text_scale as f32, inverse_text_scale as f32);
        let fonts = PaintingBinding::instance(app).fonts(app);
        label_painter.paint(
            app.get_mut(fonts),
            canvas,
            Offset::ZERO - Offset::new(label_half_width, label_painter.height() / 2.0),
        );
        canvas.restore();
        canvas.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_slider_overlay_shape_prefers_its_radius() {
        assert_eq!(
            RoundSliderOverlayShape::new().get_preferred_size(true, false),
            Size::new(48.0, 48.0)
        );
        assert_eq!(
            RoundSliderOverlayShape::new()
                .overlay_radius(10.0)
                .get_preferred_size(false, true),
            Size::new(20.0, 20.0)
        );
    }

    #[test]
    fn no_thumb_and_no_overlay_are_empty_distinct_instances() {
        let no_thumb = <dyn SliderComponentShape>::no_thumb();
        let no_overlay = <dyn SliderComponentShape>::no_overlay();
        assert_eq!(no_thumb.get_preferred_size(true, true), Size::ZERO);
        assert_eq!(no_overlay.get_preferred_size(true, true), Size::ZERO);
        assert!(Rc::ptr_eq(
            &no_thumb,
            &<dyn SliderComponentShape>::no_thumb()
        ));
        assert!(*no_thumb != *no_overlay);
    }

    #[test]
    fn built_in_value_indicator_shapes_compare_by_value() {
        let paddle: SliderComponentShapeRef = Rc::new(PaddleSliderValueIndicatorShape::new());
        let same: SliderComponentShapeRef = Rc::new(PaddleSliderValueIndicatorShape::new());
        let rectangular: SliderComponentShapeRef =
            Rc::new(RectangularSliderValueIndicatorShape::new());
        assert!(*paddle == *same);
        assert!(*paddle != *rectangular);
        assert!(
            rectangular
                .as_any()
                .is::<RectangularSliderValueIndicatorShape>()
        );
    }

    /// Dart's `labelPainter!` throws when a call through the base signature leaves it out.
    #[test]
    #[should_panic]
    fn a_value_indicator_preferred_size_needs_its_label_painter() {
        let shape: SliderComponentShapeRef = Rc::new(RectangularSliderValueIndicatorShape::new());
        shape.get_preferred_size(true, false);
    }
}
