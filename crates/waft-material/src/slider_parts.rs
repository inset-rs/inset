//! Flutter counterpart: material_ui's `slider_parts.dart`.
//!
//! [`SliderTickMarkShape`] and [`SliderTrackShape`] are abstract classes users subclass, so
//! traits held as [`SliderTickMarkShapeRef`] and [`SliderTrackShapeRef`]; the mixin
//! [`BaseSliderTrackShape`] is a trait with provided bodies that a track's
//! [`SliderTrackShape`] methods call. A parameter Dart declares optional with no default in
//! the abstract signature is an `Option`, so an implementation applies its own default as
//! Dart's override does. A shape's `paint` takes the `App`: the animations and the parent box
//! live there, and the `Tween` and `ColorTween` Dart makes on each paint are arena objects
//! that [`evaluate_tween`] and [`evaluate_color_tween`] make and evaluate.

use std::any::Any;
use std::fmt::Debug;
use std::rc::Rc;

use waft_animation::{Animatable, AnyAnimation, ColorTween, Tween};
use waft_embedder::{
    Canvas, ClipOp, Color, FillRule, Offset, Paint, PaintStyle, Path, PathBuilder, RRect, Radius,
    Rect, Size, Stroke, TextDirection, rrect_radii_elliptical,
};
use waft_foundation::App;
use waft_painting::{BorderRadius, ClipContext, PaintingBinding, TextPainter, draw_rrect};
use waft_rendering::{AnyRenderBox, PaintingContext, RenderView};
use waft_widgets::WidgetStates;

use crate::colors::Colors;

use crate::slider_theme::SliderThemeData;
use crate::slider_value_indicator_shape::SliderComponentShape;

/// Dart's `ColorTween(begin: begin, end: end).evaluate(animation)`.
pub(crate) fn evaluate_color_tween(
    app: &mut App,
    begin: Option<Color>,
    end: Option<Color>,
    animation: AnyAnimation<f64>,
) -> Option<Color> {
    let tween = ColorTween::new(app, begin, end);
    tween.evaluate(app, animation)
}

/// Dart's `Tween<double>(begin: begin, end: end).evaluate(animation)`.
pub(crate) fn evaluate_tween(
    app: &mut App,
    begin: f64,
    end: f64,
    animation: AnyAnimation<f64>,
) -> f64 {
    let tween = Tween::new(app, Some(begin), Some(end));
    tween.evaluate(app, animation)
}

/// Dart's `canvas.drawShadow(path, color, elevation, transparentOccluder)`, with the device
/// pixel ratio of the view `parent_box` paints into, which the engine reads from the implicit
/// view; 1 outside a view, as the engine's is without metrics.
pub(crate) fn draw_shadow(
    app: &App,
    canvas: &mut Canvas,
    parent_box: AnyRenderBox,
    path: &Path,
    color: Color,
    elevation: f64,
    transparent_occluder: bool,
) {
    let device_pixel_ratio = parent_box
        .as_object()
        .owner(app)
        .and_then(|owner| owner.root_node(app))
        .and_then(|root| root.downcast::<RenderView>(app))
        .map_or(1.0, |view| view.configuration(app).device_pixel_ratio);
    canvas.draw_shadow(
        path.as_valo(),
        path.fill_type,
        color.into(),
        elevation as f32,
        transparent_occluder,
        device_pixel_ratio as f32,
    );
}

/// Base class for `Slider` tick mark shapes.
///
/// Create a subclass of this if you would like a custom slider tick mark shape.
///
/// The tick mark painting can be skipped by specifying
/// [`no_tick_mark`](<dyn SliderTickMarkShape>::no_tick_mark) for
/// `SliderThemeData::tick_mark_shape`.
///
/// Equality is [`eq_shape`](Self::eq_shape): identity, as Dart's default `==`, unless the
/// shape overrides it.
///
/// See also:
///
///  * [`RoundSliderTickMarkShape`], which is the default `Slider`'s tick mark
///    shape that paints a solid circle.
///  * [`SliderTrackShape`], which can be used to create custom shapes for the
///    `Slider`'s track.
///  * [`SliderComponentShape`], which can be used to create custom shapes for
///    the `Slider`'s thumb, overlay, and value indicator and the
///    `RangeSlider`'s overlay.
pub trait SliderTickMarkShape: Any + Debug {
    /// Returns the preferred size of the shape.
    ///
    /// It is used to help position the tick marks within the slider.
    ///
    /// the `slider_theme` argument is the theme assigned to the `Slider` that this
    /// shape belongs to.
    ///
    /// The `is_enabled` argument is false when `Slider::on_changed` is null and true
    /// otherwise. When true, the slider will respond to input.
    fn get_preferred_size(&self, slider_theme: &SliderThemeData, is_enabled: bool) -> Size;

    /// Paints the slider track.
    ///
    /// The `context` argument is the same as the one that includes the `Slider`'s
    /// render box.
    ///
    /// The `center` argument is the offset for where this shape's center should be
    /// painted. This offset is relative to the origin of the `context` canvas.
    ///
    /// The `parent_box` argument is the `RenderBox` of the `Slider`. Its attributes,
    /// such as size, can be used to assist in painting this shape.
    ///
    /// the `slider_theme` argument is the theme assigned to the `Slider` that this
    /// shape belongs to.
    ///
    /// The `enable_animation` argument is an animation triggered when the `Slider`
    /// is enabled, and it reverses when the slider is disabled. The `Slider` is
    /// enabled when `Slider::on_changed` is not null. Use this to paint
    /// intermediate frames for this shape when the slider changes enabled state.
    ///
    /// The `is_enabled` argument is false when `Slider::on_changed` is null and true
    /// otherwise. When true, the slider will respond to input.
    ///
    /// The `text_direction` argument can be used to determine how the tick marks
    /// are painting depending on whether they are on an active track segment or
    /// not. The track segment between the start of the slider and the thumb is
    /// the active track segment. The track segment between the thumb and the end
    /// of the slider is the inactive track segment. In LTR text direction, the
    /// start of the slider is on the left, and in RTL text direction, the start
    /// of the slider is on the right.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        thumb_center: Offset,
        is_enabled: bool,
        text_direction: TextDirection,
    );

    /// Downcast support for Dart's `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: identity unless a shape overrides it.
    fn eq_shape(&self, other: &dyn SliderTickMarkShape) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

/// What a field or parameter Dart types as `SliderTickMarkShape` becomes.
pub type SliderTickMarkShapeRef = Rc<dyn SliderTickMarkShape>;

impl PartialEq for dyn SliderTickMarkShape {
    fn eq(&self, other: &dyn SliderTickMarkShape) -> bool {
        self.eq_shape(other)
    }
}

thread_local! {
    static NO_TICK_MARK: SliderTickMarkShapeRef = Rc::new(EmptySliderTickMarkShape);
}

impl dyn SliderTickMarkShape {
    /// Special instance of [`SliderTickMarkShape`] to skip the tick mark painting.
    ///
    /// Each call hands out the same instance, as Dart's `static final` field is one object.
    ///
    /// See also:
    ///
    ///  * `SliderThemeData::tick_mark_shape`, which is the shape that the `Slider`
    ///    uses when painting tick marks.
    pub fn no_tick_mark() -> SliderTickMarkShapeRef {
        NO_TICK_MARK.with(Rc::clone)
    }
}

/// Base class for slider track shapes.
///
/// The slider's thumb moves along the track. A discrete slider's tick marks
/// are drawn after the track, but before the thumb, and are aligned with the
/// track.
///
/// The [`get_preferred_rect`](Self::get_preferred_rect) helps position the slider thumb and
/// tick marks relative to the track.
///
/// Equality is [`eq_shape`](Self::eq_shape): identity, as Dart's default `==`, unless the
/// shape overrides it.
///
/// See also:
///
///  * [`RoundedRectSliderTrackShape`] for the default `Slider`'s track shape that
///    paints a stadium-like track.
///  * [`SliderTickMarkShape`], which can be used to create custom shapes for the
///    `Slider`'s tick marks.
///  * [`SliderComponentShape`], which can be used to create custom shapes for
///    the `Slider`'s thumb, overlay, and value indicator and the
///    `RangeSlider`'s overlay.
pub trait SliderTrackShape: Any + Debug {
    /// Returns the preferred bounds of the shape.
    ///
    /// It is used to provide horizontal boundaries for the thumb's position, and
    /// to help position the slider thumb and tick marks relative to the track.
    ///
    /// The `parent_box` argument can be used to help determine the preferredRect relative to
    /// attributes of the render box of the slider itself, such as size.
    ///
    /// The `offset` argument is relative to the caller's bounding box. It can be used to
    /// convert gesture coordinates from global to slider-relative coordinates; Dart's
    /// default is `Offset.zero`.
    ///
    /// the `slider_theme` argument is the theme assigned to the `Slider` that this
    /// shape belongs to.
    ///
    /// The `is_enabled` argument is false when `Slider::on_changed` is null and true
    /// otherwise. When true, the slider will respond to input. `None` is Dart's argument
    /// left out.
    ///
    /// The `is_discrete` argument is true if `Slider::divisions` is non-null. When
    /// true, the slider will render tick marks on top of the track. `None` is Dart's
    /// argument left out.
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect;

    /// Paints the track shape based on the state passed to it.
    ///
    /// The `context` argument is the same as the one that includes the `Slider`'s
    /// render box.
    ///
    /// The `offset` argument the offset of the origin of the `parent_box` to the
    /// origin of its `context` canvas. This shape must be painted relative to
    /// this offset. See `PaintingContextCallback`.
    ///
    /// The `parent_box` argument is the `RenderBox` of the `Slider`. Its attributes,
    /// such as size, can be used to assist in painting this shape.
    ///
    /// the `slider_theme` argument is the theme assigned to the `Slider` that this
    /// shape belongs to.
    ///
    /// The `enable_animation` argument is an animation triggered when the `Slider`
    /// is enabled, and it reverses when the slider is disabled. The `Slider` is
    /// enabled when `Slider::on_changed` is not null. Use this to paint
    /// intermediate frames for this shape when the slider changes enabled state.
    ///
    /// The `thumb_center` argument is the offset of the center of the thumb
    /// relative to the origin of the `PaintingContext::canvas`. It can be used as
    /// the point that divides the track into 2 segments.
    ///
    /// The `secondary_offset` argument is the offset of the secondary value
    /// relative to the origin of the `PaintingContext::canvas`.
    ///
    /// If not null, the track is divided into 3 segments.
    ///
    /// The `is_enabled` argument is false when `Slider::on_changed` is null and true
    /// otherwise. When true, the slider will respond to input. `None` is Dart's argument
    /// left out.
    ///
    /// The `is_discrete` argument is true if `Slider::divisions` is non-null. When
    /// true, the slider will render tick marks on top of the track. `None` is Dart's
    /// argument left out.
    ///
    /// The `text_direction` argument can be used to determine how the track
    /// segments are painted depending on whether they are active or not.
    ///
    /// The track segment between the start of the slider and the thumb is the
    /// active track segment. The track segment between the thumb and the end of the
    /// slider is the inactive track segment. In `TextDirection::Ltr`, the start of
    /// the slider is on the left, and in `TextDirection::Rtl`, the start of the
    /// slider is on the right.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        thumb_center: Offset,
        secondary_offset: Option<Offset>,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
        text_direction: TextDirection,
    );

    /// Whether the track shape is rounded.
    ///
    /// This is used to determine the correct position of the thumb in relation to the track.
    fn is_rounded(&self) -> bool {
        false
    }

    /// Downcast support for Dart's `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: identity unless a shape overrides it.
    fn eq_shape(&self, other: &dyn SliderTrackShape) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

/// What a field or parameter Dart types as `SliderTrackShape` becomes.
pub type SliderTrackShapeRef = Rc<dyn SliderTrackShape>;

impl PartialEq for dyn SliderTrackShape {
    fn eq(&self, other: &dyn SliderTrackShape) -> bool {
        self.eq_shape(other)
    }
}

/// Base track shape that provides an implementation of
/// [`get_preferred_rect`](Self::get_preferred_rect) for default sizing.
///
/// The height is set from `SliderThemeData::track_height` and the width of the
/// parent box less the larger of the widths of `SliderThemeData::thumb_shape` and
/// `SliderThemeData::overlay_shape`.
///
/// A track that mixes it in calls these bodies from its [`SliderTrackShape`] methods.
///
/// See also:
///
///  * [`RectangularSliderTrackShape`], which is a track shape with sharp
///    rectangular edges
///  * [`RoundedRectSliderTrackShape`], which is a track shape with round
///    stadium-like edges.
pub trait BaseSliderTrackShape {
    /// Returns a rect that represents the track bounds that fits within the
    /// `Slider`.
    ///
    /// The width is the width of the `Slider` or `RangeSlider`, but padded by
    /// the max of the overlay and thumb radius. The height is defined by the
    /// `SliderThemeData::track_height`.
    ///
    /// The [`Rect`] is centered both horizontally and vertically within the slider
    /// bounds.
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect {
        let is_enabled = is_enabled.unwrap_or(false);
        let is_discrete = is_discrete.unwrap_or(false);
        let thumb_width = slider_theme
            .thumb_shape
            .as_ref()
            .expect("a slider theme with a thumb shape")
            .get_preferred_size(is_enabled, is_discrete)
            .width();
        let overlay_width = slider_theme
            .overlay_shape
            .as_ref()
            .expect("a slider theme with an overlay shape")
            .get_preferred_size(is_enabled, is_discrete)
            .width();
        let mut track_height = slider_theme
            .track_height
            .expect("a slider theme with a track height");
        debug_assert!(overlay_width >= 0.0);
        debug_assert!(track_height >= 0.0);

        // If the track colors are transparent, then override only the track height
        // to maintain overall Slider width.
        if slider_theme.active_track_color == Some(Colors::TRANSPARENT)
            && slider_theme.inactive_track_color == Some(Colors::TRANSPARENT)
        {
            track_height = 0.0;
        }

        let parent_size = parent_box.size(app);
        let track_left = offset.dx()
            + if slider_theme.padding.is_none() {
                (overlay_width / 2.0).max(thumb_width / 2.0)
            } else {
                0.0
            };
        let track_top = offset.dy() + (parent_size.height() - track_height) / 2.0;
        let track_right = track_left + parent_size.width()
            - if slider_theme.padding.is_none() {
                thumb_width.max(overlay_width)
            } else {
                0.0
            };
        let track_bottom = track_top + track_height;
        // If the parentBox's size less than slider's size the trackRight will be less than trackLeft, so switch them.
        Rect::from_ltrb(
            track_left.min(track_right),
            track_top,
            track_left.max(track_right),
            track_bottom,
        )
    }

    /// Whether the track shape is rounded. This is used to determine the correct
    /// position of the thumb in relation to the track. Defaults to false.
    fn is_rounded(&self) -> bool {
        false
    }
}

/// A `Slider` track that's a simple rectangle.
///
/// It paints a solid colored rectangle, vertically centered in the
/// `parent_box`. The track rectangle extends to the bounds of the `parent_box`,
/// but is padded by the `RoundSliderOverlayShape` radius. The height is defined
/// by the `SliderThemeData::track_height`. The color is determined by the
/// `Slider`'s enabled state and the track segment's active state which are
/// defined by:
///   `SliderThemeData::active_track_color`,
///   `SliderThemeData::inactive_track_color`,
///   `SliderThemeData::disabled_active_track_color`,
///   `SliderThemeData::disabled_inactive_track_color`.
///
/// The track segment between the start of the slider and the thumb is the
/// active track segment. The track segment between the thumb and the end of the
/// slider is the inactive track segment. In `TextDirection::Ltr`, the start of
/// the slider is on the left, and in `TextDirection::Rtl`, the start of the
/// slider is on the right.
///
/// See also:
///
///  * `Slider`, for the component that is meant to display this shape.
///  * [`SliderThemeData`], where an instance of this class is set to inform the
///    slider of the visual details of the its track.
///  * [`SliderTrackShape`], which can be used to create custom shapes for the
///    `Slider`'s track.
///  * [`RoundedRectSliderTrackShape`], for a similar track with rounded edges.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectangularSliderTrackShape;

impl RectangularSliderTrackShape {
    /// Creates a slider track that draws 2 rectangles.
    pub const fn new() -> RectangularSliderTrackShape {
        RectangularSliderTrackShape
    }
}

impl BaseSliderTrackShape for RectangularSliderTrackShape {}

impl SliderTrackShape for RectangularSliderTrackShape {
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect {
        BaseSliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            is_enabled,
            is_discrete,
        )
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        thumb_center: Offset,
        secondary_offset: Option<Offset>,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
        text_direction: TextDirection,
    ) {
        let is_discrete = is_discrete.unwrap_or(false);
        let is_enabled = is_enabled.unwrap_or(false);
        debug_assert!(slider_theme.disabled_active_track_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_track_color.is_some());
        debug_assert!(slider_theme.active_track_color.is_some());
        debug_assert!(slider_theme.inactive_track_color.is_some());
        debug_assert!(slider_theme.thumb_shape.is_some());
        // If the slider [SliderThemeData.trackHeight] is less than or equal to 0,
        // then it makes no difference whether the track is painted or not,
        // therefore the painting can be a no-op.
        if slider_theme
            .track_height
            .expect("a slider theme with a track height")
            <= 0.0
        {
            return;
        }

        // Assign the track segment paints, which are left: active, right: inactive,
        // but reversed for right to left text.
        let active_paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_active_track_color,
                slider_theme.active_track_color,
                enable_animation,
            )
            .expect("both track colors are set")
            .into(),
        );
        let inactive_paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_inactive_track_color,
                slider_theme.inactive_track_color,
                enable_animation,
            )
            .expect("both track colors are set")
            .into(),
        );
        let (left_track_paint, right_track_paint) = match text_direction {
            TextDirection::Ltr => (&active_paint, &inactive_paint),
            TextDirection::Rtl => (&inactive_paint, &active_paint),
        };

        let track_rect = SliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            Some(is_enabled),
            Some(is_discrete),
        );

        let left_track_segment = Rect::from_ltrb(
            track_rect.left,
            track_rect.top,
            thumb_center.dx(),
            track_rect.bottom,
        );
        if !left_track_segment.is_empty() {
            context
                .canvas()
                .draw_rect(left_track_segment, left_track_paint);
        }
        let right_track_segment = Rect::from_ltrb(
            thumb_center.dx(),
            track_rect.top,
            track_rect.right,
            track_rect.bottom,
        );
        if !right_track_segment.is_empty() {
            context
                .canvas()
                .draw_rect(right_track_segment, right_track_paint);
        }

        let show_secondary_track =
            secondary_offset.is_some_and(|secondary_offset| match text_direction {
                TextDirection::Rtl => secondary_offset.dx() < thumb_center.dx(),
                TextDirection::Ltr => secondary_offset.dx() > thumb_center.dx(),
            });

        if let Some(secondary_offset) = secondary_offset.filter(|_| show_secondary_track) {
            let secondary_track_paint = Paint::from_color(
                evaluate_color_tween(
                    app,
                    slider_theme.disabled_secondary_active_track_color,
                    slider_theme.secondary_active_track_color,
                    enable_animation,
                )
                .expect("both secondary track colors are set")
                .into(),
            );
            let secondary_track_segment = match text_direction {
                TextDirection::Rtl => Rect::from_ltrb(
                    secondary_offset.dx(),
                    track_rect.top,
                    thumb_center.dx(),
                    track_rect.bottom,
                ),
                TextDirection::Ltr => Rect::from_ltrb(
                    thumb_center.dx(),
                    track_rect.top,
                    secondary_offset.dx(),
                    track_rect.bottom,
                ),
            };
            if !secondary_track_segment.is_empty() {
                context
                    .canvas()
                    .draw_rect(secondary_track_segment, &secondary_track_paint);
            }
        }
    }

    fn is_rounded(&self) -> bool {
        BaseSliderTrackShape::is_rounded(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderTrackShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// Dart's `_TrackSegment`: a horizontal span of the track, and whether it is wide enough to
/// be painted.
#[derive(Clone, Copy)]
struct TrackSegment {
    draw: bool,
    left: f64,
    right: f64,
}

/// The default shape of a `Slider`'s track.
///
/// It paints a solid colored rectangle with rounded edges, vertically centered
/// in the `parent_box`. The track rectangle extends to the bounds of the
/// `parent_box`, but is padded by the larger of `RoundSliderOverlayShape`'s
/// radius and [`RoundSliderThumbShape`]'s radius. The height is defined by the
/// `SliderThemeData::track_height`. The color is determined by the `Slider`'s
/// enabled state and the track segment's active state which are defined by:
///   `SliderThemeData::active_track_color`,
///   `SliderThemeData::inactive_track_color`,
///   `SliderThemeData::disabled_active_track_color`,
///   `SliderThemeData::disabled_inactive_track_color`.
///
/// The track segment between the start of the slider and the thumb is the
/// active track segment. The track segment between the thumb and the end of the
/// slider is the inactive track segment. In `TextDirection::Ltr`, the start of
/// the slider is on the left, and in `TextDirection::Rtl`, the start of the
/// slider is on the right.
///
/// See also:
///
///  * `Slider`, for the component that is meant to display this shape.
///  * [`SliderThemeData`], where an instance of this class is set to inform the
///    slider of the visual details of the its track.
///  * [`SliderTrackShape`], which can be used to create custom shapes for the
///    `Slider`'s track.
///  * [`RectangularSliderTrackShape`], for a similar track with sharp edges.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedRectSliderTrackShape;

impl RoundedRectSliderTrackShape {
    /// Create a slider track that draws two rectangles with rounded outer edges.
    pub const fn new() -> RoundedRectSliderTrackShape {
        RoundedRectSliderTrackShape
    }

    /// Dart's `paint` with its extra `additionalActiveTrackHeight` argument, which the
    /// [`SliderTrackShape::paint`] override passes as Dart's default, 2.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    pub fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        text_direction: TextDirection,
        thumb_center: Offset,
        secondary_offset: Option<Offset>,
        is_discrete: bool,
        is_enabled: bool,
        additional_active_track_height: f64,
    ) {
        debug_assert!(slider_theme.disabled_active_track_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_track_color.is_some());
        debug_assert!(slider_theme.active_track_color.is_some());
        debug_assert!(slider_theme.inactive_track_color.is_some());
        debug_assert!(slider_theme.thumb_shape.is_some());
        // If the slider [SliderThemeData.trackHeight] is less than or equal to 0,
        // then it makes no difference whether the track is painted or not,
        // therefore the painting can be a no-op.
        let Some(track_height) = slider_theme.track_height.filter(|height| *height > 0.0) else {
            return;
        };

        // Assign the track segment paints, which are leading: active and
        // trailing: inactive.
        let active_paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_active_track_color,
                slider_theme.active_track_color,
                enable_animation,
            )
            .expect("both track colors are set")
            .into(),
        );
        let inactive_paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_inactive_track_color,
                slider_theme.inactive_track_color,
                enable_animation,
            )
            .expect("both track colors are set")
            .into(),
        );

        let track_rect = SliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            Some(is_enabled),
            Some(is_discrete),
        );
        let track_radius = Radius::circular(track_rect.height() / 2.0);
        let active_track_radius =
            Radius::circular((track_rect.height() + additional_active_track_height) / 2.0);
        let is_ltr = text_direction == TextDirection::Ltr;

        // Both segments extend past the thumb center by half of the track height.
        let half_track_height = track_height / 2.0;
        let left_track = TrackSegment {
            draw: thumb_center.dx() > track_rect.left + half_track_height,
            left: track_rect.left,
            right: thumb_center.dx() + half_track_height,
        };
        let right_track = TrackSegment {
            draw: thumb_center.dx() < track_rect.right - half_track_height,
            left: thumb_center.dx() - half_track_height,
            right: track_rect.right,
        };
        let (inactive_track, active_track) = if is_ltr {
            (right_track, left_track)
        } else {
            (left_track, right_track)
        };

        // The segment painted last has its rounded end drawn on top of the other one,
        // so paint the inactive segment first to keep the active one on top in both
        // text directions.
        if inactive_track.draw {
            draw_rrect(
                context.canvas(),
                RRect::from_ltrbr(
                    inactive_track.left,
                    track_rect.top,
                    inactive_track.right,
                    track_rect.bottom,
                    track_radius,
                ),
                &inactive_paint,
            );
        }
        if active_track.draw {
            draw_rrect(
                context.canvas(),
                RRect::from_ltrbr(
                    active_track.left,
                    track_rect.top - (additional_active_track_height / 2.0),
                    active_track.right,
                    track_rect.bottom + (additional_active_track_height / 2.0),
                    active_track_radius,
                ),
                &active_paint,
            );
        }

        let show_secondary_track = secondary_offset.is_some_and(|secondary_offset| {
            if is_ltr {
                secondary_offset.dx() > thumb_center.dx()
            } else {
                secondary_offset.dx() < thumb_center.dx()
            }
        });

        if let Some(secondary_offset) = secondary_offset.filter(|_| show_secondary_track) {
            let secondary_track_paint = Paint::from_color(
                evaluate_color_tween(
                    app,
                    slider_theme.disabled_secondary_active_track_color,
                    slider_theme.secondary_active_track_color,
                    enable_animation,
                )
                .expect("both secondary track colors are set")
                .into(),
            );
            if is_ltr {
                draw_rrect(
                    context.canvas(),
                    RRect::from_ltrb_and_corners(
                        thumb_center.dx(),
                        track_rect.top,
                        secondary_offset.dx(),
                        track_rect.bottom,
                        Radius::ZERO,
                        track_radius,
                        track_radius,
                        Radius::ZERO,
                    ),
                    &secondary_track_paint,
                );
            } else {
                draw_rrect(
                    context.canvas(),
                    RRect::from_ltrb_and_corners(
                        secondary_offset.dx(),
                        track_rect.top,
                        thumb_center.dx(),
                        track_rect.bottom,
                        track_radius,
                        Radius::ZERO,
                        Radius::ZERO,
                        track_radius,
                    ),
                    &secondary_track_paint,
                );
            }
        }
    }
}

impl BaseSliderTrackShape for RoundedRectSliderTrackShape {}

impl SliderTrackShape for RoundedRectSliderTrackShape {
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect {
        BaseSliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            is_enabled,
            is_discrete,
        )
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        thumb_center: Offset,
        secondary_offset: Option<Offset>,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
        text_direction: TextDirection,
    ) {
        RoundedRectSliderTrackShape::paint(
            self,
            app,
            context,
            offset,
            parent_box,
            slider_theme,
            enable_animation,
            text_direction,
            thumb_center,
            secondary_offset,
            is_discrete.unwrap_or(false),
            is_enabled.unwrap_or(false),
            2.0,
        );
    }

    fn is_rounded(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderTrackShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The default shape of each `Slider` tick mark.
///
/// Tick marks are only displayed if the slider is discrete, which can be done
/// by setting the `Slider::divisions` to an integer value.
///
/// It paints a solid circle, centered in the on the track.
/// The color is determined by the `Slider`'s enabled state and track's active
/// states. These colors are defined in:
///   `SliderThemeData::active_track_color`,
///   `SliderThemeData::inactive_track_color`,
///   `SliderThemeData::disabled_active_track_color`,
///   `SliderThemeData::disabled_inactive_track_color`.
///
/// See also:
///
///  * `Slider`, which includes tick marks defined by this shape.
///  * `SliderTheme`, which can be used to configure the tick mark shape of all
///    sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundSliderTickMarkShape {
    /// The preferred radius of the round tick mark.
    ///
    /// If it is not provided, then 1/4 of the `SliderThemeData::track_height` is used.
    pub tick_mark_radius: Option<f64>,
}

impl RoundSliderTickMarkShape {
    /// Create a slider tick mark that draws a circle.
    pub const fn new() -> RoundSliderTickMarkShape {
        RoundSliderTickMarkShape {
            tick_mark_radius: None,
        }
    }

    /// Dart `RoundSliderTickMarkShape(tickMarkRadius:)`.
    pub const fn tick_mark_radius(mut self, tick_mark_radius: f64) -> RoundSliderTickMarkShape {
        self.tick_mark_radius = Some(tick_mark_radius);
        self
    }
}

impl SliderTickMarkShape for RoundSliderTickMarkShape {
    fn get_preferred_size(&self, slider_theme: &SliderThemeData, _is_enabled: bool) -> Size {
        debug_assert!(slider_theme.track_height.is_some());
        // The tick marks are tiny circles. If no radius is provided, then the
        // radius is defaulted to be a fraction of the
        // [SliderThemeData.trackHeight]. The fraction is 1/4.
        Size::from_radius(self.tick_mark_radius.unwrap_or_else(|| {
            slider_theme
                .track_height
                .expect("a slider theme with a track height")
                / 4.0
        }))
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        _parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        thumb_center: Offset,
        is_enabled: bool,
        text_direction: TextDirection,
    ) {
        debug_assert!(slider_theme.disabled_active_tick_mark_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_tick_mark_color.is_some());
        debug_assert!(slider_theme.active_tick_mark_color.is_some());
        debug_assert!(slider_theme.inactive_tick_mark_color.is_some());
        // The paint color of the tick mark depends on its position relative
        // to the thumb and the text direction.
        let x_offset = center.dx() - thumb_center.dx();
        let (begin, end) = match text_direction {
            TextDirection::Ltr if x_offset > 0.0 => (
                slider_theme.disabled_inactive_tick_mark_color,
                slider_theme.inactive_tick_mark_color,
            ),
            TextDirection::Rtl if x_offset < 0.0 => (
                slider_theme.disabled_inactive_tick_mark_color,
                slider_theme.inactive_tick_mark_color,
            ),
            TextDirection::Ltr | TextDirection::Rtl => (
                slider_theme.disabled_active_tick_mark_color,
                slider_theme.active_tick_mark_color,
            ),
        };
        let paint = Paint::from_color(
            evaluate_color_tween(app, begin, end, enable_animation)
                .expect("both tick mark colors are set")
                .into(),
        );

        // The tick marks are tiny circles that are the same height as the track.
        let tick_mark_radius = self.get_preferred_size(slider_theme, is_enabled).width() / 2.0;
        if tick_mark_radius > 0.0 {
            context
                .canvas()
                .draw_circle(center, tick_mark_radius as f32, &paint);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderTickMarkShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// A special version of [`SliderTickMarkShape`] that has a zero size and paints
/// nothing.
///
/// This class is used to create a special instance of a [`SliderTickMarkShape`]
/// that will not paint any tick mark shape. A static reference is stored in
/// [`no_tick_mark`](<dyn SliderTickMarkShape>::no_tick_mark). When this value is specified for
/// `SliderThemeData::tick_mark_shape`, the tick mark painting is skipped.
#[derive(Debug)]
struct EmptySliderTickMarkShape;

impl SliderTickMarkShape for EmptySliderTickMarkShape {
    fn get_preferred_size(&self, _slider_theme: &SliderThemeData, _is_enabled: bool) -> Size {
        Size::ZERO
    }

    fn paint(
        &self,
        _app: &mut App,
        _context: &mut PaintingContext,
        _center: Offset,
        _parent_box: AnyRenderBox,
        _slider_theme: &SliderThemeData,
        _enable_animation: AnyAnimation<f64>,
        _thumb_center: Offset,
        _is_enabled: bool,
        _text_direction: TextDirection,
    ) {
        // no-op.
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The default shape of a `Slider`'s thumb.
///
/// There is a shadow for the resting, pressed, hovered, and focused state.
///
/// See also:
///
///  * `Slider`, which includes a thumb defined by this shape.
///  * `SliderTheme`, which can be used to configure the thumb shape of all
///    sliders in a widget subtree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundSliderThumbShape {
    /// The preferred radius of the round thumb shape when the slider is enabled.
    ///
    /// If it is not provided, then the Material Design default of 10 is used.
    pub enabled_thumb_radius: f64,

    /// The preferred radius of the round thumb shape when the slider is disabled.
    ///
    /// If no disabledRadius is provided, then it is equal to the
    /// [`enabled_thumb_radius`](Self::enabled_thumb_radius)
    pub disabled_thumb_radius: Option<f64>,

    /// The resting elevation adds shadow to the unpressed thumb.
    ///
    /// The default is 1.
    ///
    /// Use 0 for no shadow. The higher the value, the larger the shadow. For
    /// example, a value of 12 will create a very large shadow.
    pub elevation: f64,

    /// The pressed elevation adds shadow to the pressed thumb.
    ///
    /// The default is 6.
    ///
    /// Use 0 for no shadow. The higher the value, the larger the shadow. For
    /// example, a value of 12 will create a very large shadow.
    pub pressed_elevation: f64,
}

impl RoundSliderThumbShape {
    /// Create a slider thumb that draws a circle.
    pub const fn new() -> RoundSliderThumbShape {
        RoundSliderThumbShape {
            enabled_thumb_radius: 10.0,
            disabled_thumb_radius: None,
            elevation: 1.0,
            pressed_elevation: 6.0,
        }
    }

    /// Dart `RoundSliderThumbShape(enabledThumbRadius:)`.
    pub const fn enabled_thumb_radius(
        mut self,
        enabled_thumb_radius: f64,
    ) -> RoundSliderThumbShape {
        self.enabled_thumb_radius = enabled_thumb_radius;
        self
    }

    /// Dart `RoundSliderThumbShape(disabledThumbRadius:)`.
    pub const fn disabled_thumb_radius(
        mut self,
        disabled_thumb_radius: f64,
    ) -> RoundSliderThumbShape {
        self.disabled_thumb_radius = Some(disabled_thumb_radius);
        self
    }

    /// Dart `RoundSliderThumbShape(elevation:)`.
    pub const fn elevation(mut self, elevation: f64) -> RoundSliderThumbShape {
        self.elevation = elevation;
        self
    }

    /// Dart `RoundSliderThumbShape(pressedElevation:)`.
    pub const fn pressed_elevation(mut self, pressed_elevation: f64) -> RoundSliderThumbShape {
        self.pressed_elevation = pressed_elevation;
        self
    }

    /// Dart's `_disabledThumbRadius`.
    fn effective_disabled_thumb_radius(&self) -> f64 {
        self.disabled_thumb_radius
            .unwrap_or(self.enabled_thumb_radius)
    }
}

impl Default for RoundSliderThumbShape {
    fn default() -> RoundSliderThumbShape {
        RoundSliderThumbShape::new()
    }
}

impl SliderComponentShape for RoundSliderThumbShape {
    fn get_preferred_size(&self, is_enabled: bool, _is_discrete: bool) -> Size {
        Size::from_radius(if is_enabled {
            self.enabled_thumb_radius
        } else {
            self.effective_disabled_thumb_radius()
        })
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        enable_animation: AnyAnimation<f64>,
        _is_discrete: bool,
        _label_painter: &mut TextPainter,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        _text_direction: TextDirection,
        _value: f64,
        _text_scale_factor: f64,
        _size_with_overflow: Size,
    ) {
        debug_assert!(slider_theme.disabled_thumb_color.is_some());
        debug_assert!(slider_theme.thumb_color.is_some());

        let color = evaluate_color_tween(
            app,
            slider_theme.disabled_thumb_color,
            slider_theme.thumb_color,
            enable_animation,
        )
        .expect("both thumb colors are set");
        let radius = evaluate_tween(
            app,
            self.effective_disabled_thumb_radius(),
            self.enabled_thumb_radius,
            enable_animation,
        );

        let evaluated_elevation = evaluate_tween(
            app,
            self.elevation,
            self.pressed_elevation,
            activation_animation,
        );
        let mut path = PathBuilder::new();
        path.arc(center, radius as f32, 0.0, std::f32::consts::TAU);
        let path = Path::new(path.build());

        draw_shadow(
            app,
            context.canvas(),
            parent_box,
            &path,
            Colors::BLACK,
            evaluated_elevation,
            true,
        );

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

/// The default shape of a Material 3 `Slider`'s value indicator.
///
/// See also:
///
///  * `Slider`, which includes a value indicator defined by this shape.
///  * `SliderTheme`, which can be used to configure the slider value indicator
///    of all sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DropSliderValueIndicatorShape;

impl DropSliderValueIndicatorShape {
    const PATH_PAINTER: DropSliderValueIndicatorPathPainter = DropSliderValueIndicatorPathPainter;

    /// Create a slider value indicator that resembles a drop shape.
    pub const fn new() -> DropSliderValueIndicatorShape {
        DropSliderValueIndicatorShape
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

impl SliderComponentShape for DropSliderValueIndicatorShape {
    fn get_preferred_size(&self, is_enabled: bool, is_discrete: bool) -> Size {
        DropSliderValueIndicatorShape::get_preferred_size(self, is_enabled, is_discrete, None, None)
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

/// Dart's `_DropSliderValueIndicatorPathPainter`.
struct DropSliderValueIndicatorPathPainter;

impl DropSliderValueIndicatorPathPainter {
    const TRIANGLE_HEIGHT: f64 = 10.0;
    const LABEL_PADDING: f64 = 8.0;
    const PREFERRED_HEIGHT: f64 = 32.0;
    const MIN_LABEL_WIDTH: f64 = 20.0;
    const MIN_RECT_HEIGHT: f64 = 28.0;
    const RECT_Y_OFFSET: f64 = 6.0;
    const BOTTOM_TIP_Y_OFFSET: f64 = 16.0;
    const PREFERRED_HALF_HEIGHT: f64 = Self::PREFERRED_HEIGHT / 2.0;
    const UPPER_RECT_RADIUS: f64 = 4.0;

    fn get_preferred_size(&self, label_painter: &TextPainter, text_scale_factor: f64) -> Size {
        let width = Self::MIN_LABEL_WIDTH.max(label_painter.width())
            + Self::LABEL_PADDING * 2.0 * text_scale_factor;
        Size::new(width, Self::PREFERRED_HEIGHT * text_scale_factor)
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
        let rectangle_width = self.upper_rectangle_width(label_painter, scale);

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

    fn upper_rectangle_width(&self, label_painter: &TextPainter, scale: f64) -> f64 {
        let unscaled_width = Self::MIN_LABEL_WIDTH.max(label_painter.width()) + Self::LABEL_PADDING;
        unscaled_width * scale
    }

    fn adjust_border_radius(&self, rect: Rect) -> BorderRadius {
        const RECTNESS: f64 = 0.0;
        BorderRadius::lerp(
            Some(BorderRadius::all(Radius::circular(Self::UPPER_RECT_RADIUS))),
            Some(BorderRadius::all(Radius::circular(
                rect.shortest_side() / 2.0,
            ))),
            1.0 - RECTNESS,
        )
        .expect("both radii are set")
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
        let rectangle_width = self.upper_rectangle_width(label_painter, scale);
        let horizontal_shift = self.get_horizontal_shift(
            app,
            parent_box,
            center,
            label_painter,
            text_scale_factor,
            size_with_overflow,
            scale,
        );
        let upper_rect = Rect::from_ltwh(
            -rectangle_width / 2.0 + horizontal_shift,
            -Self::RECT_Y_OFFSET - Self::MIN_RECT_HEIGHT,
            rectangle_width,
            Self::MIN_RECT_HEIGHT,
        );

        let fill_paint = Paint::from_color(background_paint_color.into());

        canvas.save();
        canvas.translate(
            center.dx() as f32,
            (center.dy() - Self::BOTTOM_TIP_Y_OFFSET) as f32,
        );
        canvas.scale(scale as f32, scale as f32);

        let adjusted_border_radius = self.adjust_border_radius(upper_rect);
        let border_rect = adjusted_border_radius
            .resolve(label_painter.text_direction())
            .to_rrect(upper_rect);
        let mut triangle_path = PathBuilder::new();
        // Dart's `lineTo` on an empty path starts its contour at the origin.
        triangle_path.move_to((0.0, 0.0));
        triangle_path
            .line_to((-Self::TRIANGLE_HEIGHT as f32, -Self::TRIANGLE_HEIGHT as f32))
            .line_to((Self::TRIANGLE_HEIGHT as f32, -Self::TRIANGLE_HEIGHT as f32))
            .close();
        triangle_path.rrect_radii_elliptical(
            border_rect.outer_rect(),
            rrect_radii_elliptical(border_rect),
        );
        let triangle_path = triangle_path.build();

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
        let box_center = Offset::new(horizontal_shift, upper_rect.height() / 1.75);
        let half_label_painter_offset =
            Offset::new(label_painter.width() / 2.0, label_painter.height() / 2.0);
        let label_offset = box_center - half_label_painter_offset;
        let fonts = PaintingBinding::instance(app).fonts(app);
        label_painter.paint(app.get_mut(fonts), canvas, label_offset);
        canvas.restore();
    }
}

/// The bar shape of a `Slider`'s thumb.
///
/// When the slider is enabled, the `ColorScheme::primary` color is used for the
/// thumb. When the slider is disabled, the `ColorScheme::on_surface` color with an
/// opacity of 0.38 is used for the thumb.
///
/// The thumb bar shape width is reduced when the thumb is pressed.
///
/// If `SliderThemeData::thumb_size` is null, then the thumb size is 4 pixels for the width
/// and 44 pixels for the height.
///
/// This is the default thumb shape for `Slider`. If `ThemeData::use_material3` is false,
/// then the default thumb shape is [`RoundSliderThumbShape`].
///
/// See also:
///
///  * `Slider`, which includes an overlay defined by this shape.
///  * `SliderTheme`, which can be used to configure the overlay shape of all
///    sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HandleThumbShape;

impl HandleThumbShape {
    /// Create a slider thumb that draws a bar.
    pub const fn new() -> HandleThumbShape {
        HandleThumbShape
    }
}

impl SliderComponentShape for HandleThumbShape {
    fn get_preferred_size(&self, _is_enabled: bool, _is_discrete: bool) -> Size {
        Size::new(4.0, 44.0)
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        _activation_animation: AnyAnimation<f64>,
        enable_animation: AnyAnimation<f64>,
        _is_discrete: bool,
        _label_painter: &mut TextPainter,
        _parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        _text_direction: TextDirection,
        _value: f64,
        _text_scale_factor: f64,
        _size_with_overflow: Size,
    ) {
        debug_assert!(slider_theme.disabled_thumb_color.is_some());
        debug_assert!(slider_theme.thumb_color.is_some());
        debug_assert!(slider_theme.thumb_size.is_some());

        let color = evaluate_color_tween(
            app,
            slider_theme.disabled_thumb_color,
            slider_theme.thumb_color,
            enable_animation,
        )
        .expect("both thumb colors are set");

        let thumb_size = slider_theme
            .thumb_size
            .as_ref()
            .expect("a slider theme with a thumb size")
            .resolve(&WidgetStates::new())
            .expect("a thumb size for no states"); // This is resolved in the paint method.
        let rrect = RRect::from_rect_and_radius(
            Rect::from_center(center, thumb_size.width(), thumb_size.height()),
            Radius::circular(thumb_size.shortest_side() / 2.0),
        );
        draw_rrect(context.canvas(), rrect, &Paint::from_color(color.into()));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderComponentShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The gapped shape of a `Slider`'s track.
///
/// The [`GappedSliderTrackShape`] consists of active and inactive
/// tracks. The active track uses the `SliderThemeData::active_track_color` and the
/// inactive track uses the `SliderThemeData::inactive_track_color`.
///
/// The track shape uses circular corner radius for the edge corners and a corner radius
/// of 2 pixels for the inside corners.
///
/// Between the active and inactive tracks there is a gap of size
/// `SliderThemeData::track_gap`. If the `SliderThemeData::thumb_shape` is
/// [`HandleThumbShape`] and the thumb is pressed, the thumb's width is reduced; as a result,
/// the track gap size in [`GappedSliderTrackShape`] is also reduced.
///
/// If `SliderThemeData::track_gap` is null, then the track gap size defaults to 6 pixels.
///
/// This is the default track shape for `Slider`. If `ThemeData::use_material3` is false,
/// then the default track shape is [`RoundedRectSliderTrackShape`].
///
/// See also:
///
///  * `Slider`, which includes an overlay defined by this shape.
///  * `SliderTheme`, which can be used to configure the overlay shape of all
///    sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GappedSliderTrackShape;

impl GappedSliderTrackShape {
    /// Create a slider track that draws two rectangles with rounded outer edges.
    pub const fn new() -> GappedSliderTrackShape {
        GappedSliderTrackShape
    }

    /// Dart's `paint` with its extra `additionalActiveTrackHeight` argument, which the
    /// [`SliderTrackShape::paint`] override passes as Dart's default, 2.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    pub fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        text_direction: TextDirection,
        thumb_center: Offset,
        secondary_offset: Option<Offset>,
        is_discrete: bool,
        is_enabled: bool,
        _additional_active_track_height: f64,
    ) {
        debug_assert!(slider_theme.disabled_active_track_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_track_color.is_some());
        debug_assert!(slider_theme.active_track_color.is_some());
        debug_assert!(slider_theme.inactive_track_color.is_some());
        debug_assert!(slider_theme.thumb_shape.is_some());
        debug_assert!(slider_theme.track_gap.is_some());
        debug_assert!(
            slider_theme
                .track_gap
                .is_some_and(|gap| !gap.is_sign_negative())
        );
        // If the slider [SliderThemeData.trackHeight] is less than or equal to 0,
        // then it makes no difference whether the track is painted or not,
        // therefore the painting can be a no-op.
        let Some(track_height) = slider_theme.track_height.filter(|height| *height > 0.0) else {
            return;
        };

        // Assign the track segment paints, which are left: active, right: inactive,
        // but reversed for right to left text.
        let active_paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_active_track_color,
                slider_theme.active_track_color,
                enable_animation,
            )
            .expect("both track colors are set")
            .into(),
        );
        let inactive_paint = Paint::from_color(
            evaluate_color_tween(
                app,
                slider_theme.disabled_inactive_track_color,
                slider_theme.inactive_track_color,
                enable_animation,
            )
            .expect("both track colors are set")
            .into(),
        );
        let (left_track_paint, right_track_paint) = match text_direction {
            TextDirection::Ltr => (&active_paint, &inactive_paint),
            TextDirection::Rtl => (&inactive_paint, &active_paint),
        };

        // Gap, starting from the middle of the thumb.
        let track_gap = slider_theme
            .track_gap
            .expect("a slider theme with a track gap");

        let track_rect = SliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            Some(is_enabled),
            Some(is_discrete),
        );

        let track_corner_radius = Radius::circular(track_rect.shortest_side() / 2.0);
        const TRACK_INSIDE_CORNER_RADIUS: Radius = Radius::circular(2.0);

        let track_rrect = RRect::from_rect_and_corners(
            track_rect,
            track_corner_radius,
            track_corner_radius,
            track_corner_radius,
            track_corner_radius,
        );

        let left_rrect = RRect::from_ltrb_and_corners(
            track_rect.left,
            track_rect.top,
            track_rect.left.max(thumb_center.dx() - track_gap),
            track_rect.bottom,
            track_corner_radius,
            TRACK_INSIDE_CORNER_RADIUS,
            TRACK_INSIDE_CORNER_RADIUS,
            track_corner_radius,
        );

        let right_rrect = RRect::from_ltrb_and_corners(
            thumb_center.dx() + track_gap,
            track_rect.top,
            track_rect.right,
            track_rect.bottom,
            TRACK_INSIDE_CORNER_RADIUS,
            track_corner_radius,
            track_corner_radius,
            TRACK_INSIDE_CORNER_RADIUS,
        );

        let canvas = context.canvas();
        canvas.save();
        canvas.clip_rrect_radii_elliptical(
            track_rrect.outer_rect(),
            rrect_radii_elliptical(track_rrect),
            ClipOp::Intersect,
            true,
        );
        let draw_left_track = thumb_center.dx() > (left_rrect.left + (track_height / 2.0));
        let draw_right_track = thumb_center.dx() < (right_rrect.right - (track_height / 2.0));
        if draw_left_track {
            draw_rrect(canvas, left_rrect, left_track_paint);
        }
        if draw_right_track {
            draw_rrect(canvas, right_rrect, right_track_paint);
        }

        let is_ltr = text_direction == TextDirection::Ltr;
        let show_secondary_track = secondary_offset.is_some_and(|secondary_offset| match is_ltr {
            true => secondary_offset.dx() > thumb_center.dx() + track_gap,
            false => secondary_offset.dx() < thumb_center.dx() - track_gap,
        });

        if let Some(secondary_offset) = secondary_offset.filter(|_| show_secondary_track) {
            let secondary_track_paint = Paint::from_color(
                evaluate_color_tween(
                    app,
                    slider_theme.disabled_secondary_active_track_color,
                    slider_theme.secondary_active_track_color,
                    enable_animation,
                )
                .expect("both secondary track colors are set")
                .into(),
            );
            let canvas = context.canvas();
            if is_ltr {
                draw_rrect(
                    canvas,
                    RRect::from_ltrb_and_corners(
                        thumb_center.dx() + track_gap,
                        track_rect.top,
                        secondary_offset.dx(),
                        track_rect.bottom,
                        TRACK_INSIDE_CORNER_RADIUS,
                        track_corner_radius,
                        track_corner_radius,
                        TRACK_INSIDE_CORNER_RADIUS,
                    ),
                    &secondary_track_paint,
                );
            } else {
                draw_rrect(
                    canvas,
                    RRect::from_ltrb_and_corners(
                        secondary_offset.dx(),
                        track_rect.top,
                        thumb_center.dx() - track_gap,
                        track_rect.bottom,
                        track_corner_radius,
                        TRACK_INSIDE_CORNER_RADIUS,
                        TRACK_INSIDE_CORNER_RADIUS,
                        track_corner_radius,
                    ),
                    &secondary_track_paint,
                );
            }
        }
        let canvas = context.canvas();
        canvas.restore();

        const STOP_INDICATOR_RADIUS: f64 = 2.0;
        let stop_indicator_trailing_space = track_height / 2.0;
        let stop_indicator_offset = Offset::new(
            if text_direction == TextDirection::Ltr {
                track_rect.center_right().dx() - stop_indicator_trailing_space
            } else {
                track_rect.center_left().dx() + stop_indicator_trailing_space
            },
            track_rect.center().dy(),
        );

        let show_stop_indicator = if text_direction == TextDirection::Ltr {
            thumb_center.dx() < stop_indicator_offset.dx()
        } else {
            thumb_center.dx() > stop_indicator_offset.dx()
        };
        if show_stop_indicator && !is_discrete {
            let stop_indicator_rect =
                Rect::from_circle(stop_indicator_offset, STOP_INDICATOR_RADIUS);
            canvas.draw_circle(
                stop_indicator_rect.center(),
                STOP_INDICATOR_RADIUS as f32,
                &active_paint,
            );
        }
    }
}

impl BaseSliderTrackShape for GappedSliderTrackShape {}

impl SliderTrackShape for GappedSliderTrackShape {
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect {
        BaseSliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            is_enabled,
            is_discrete,
        )
    }

    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        thumb_center: Offset,
        secondary_offset: Option<Offset>,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
        text_direction: TextDirection,
    ) {
        GappedSliderTrackShape::paint(
            self,
            app,
            context,
            offset,
            parent_box,
            slider_theme,
            enable_animation,
            text_direction,
            thumb_center,
            secondary_offset,
            is_discrete.unwrap_or(false),
            is_enabled.unwrap_or(false),
            2.0,
        );
    }

    fn is_rounded(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn SliderTrackShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The rounded rectangle shape of a `Slider`'s value indicator.
///
/// If the `SliderThemeData::value_indicator_color` is null, then the shape uses the
/// `ColorScheme::inverse_surface` color to draw the value indicator.
///
/// If the `SliderThemeData::value_indicator_text_style` is null, then the indicator label
/// text style defaults to `TextTheme::label_medium` with the color set to
/// `ColorScheme::on_inverse_surface`. If the `ThemeData::use_material3` is set to false, then
/// the indicator label text style defaults to `TextTheme::body_large` with the color set to
/// `ColorScheme::on_inverse_surface`.
///
/// If the `SliderThemeData::value_indicator_stroke_color` is provided, then the value
/// indicator is drawn with a stroke border with the color provided.
///
/// This is the default value indicator shape for `Slider`. If `ThemeData::use_material3` is
/// false, then the default value indicator shape is `RectangularSliderValueIndicatorShape`.
///
/// See also:
///
///  * `Slider`, which includes a value indicator defined by this shape.
///  * `SliderTheme`, which can be used to configure the slider value indicator
///    of all sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedRectSliderValueIndicatorShape;

impl RoundedRectSliderValueIndicatorShape {
    const PATH_PAINTER: RoundedRectSliderValueIndicatorPathPainter =
        RoundedRectSliderValueIndicatorPathPainter;

    /// Create a slider value indicator that resembles a rounded rectangle.
    pub const fn new() -> RoundedRectSliderValueIndicatorShape {
        RoundedRectSliderValueIndicatorShape
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

impl SliderComponentShape for RoundedRectSliderValueIndicatorShape {
    fn get_preferred_size(&self, is_enabled: bool, is_discrete: bool) -> Size {
        RoundedRectSliderValueIndicatorShape::get_preferred_size(
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

/// Dart's `_RoundedRectSliderValueIndicatorPathPainter`.
struct RoundedRectSliderValueIndicatorPathPainter;

impl RoundedRectSliderValueIndicatorPathPainter {
    const LABEL_PADDING: f64 = 10.0;
    const PREFERRED_HEIGHT: f64 = 32.0;
    const MIN_LABEL_WIDTH: f64 = 16.0;
    const RECT_Y_OFFSET: f64 = 10.0;
    const BOTTOM_TIP_Y_OFFSET: f64 = 16.0;
    const PREFERRED_HALF_HEIGHT: f64 = Self::PREFERRED_HEIGHT / 2.0;

    fn get_preferred_size(&self, label_painter: &TextPainter, text_scale_factor: f64) -> Size {
        let width = Self::MIN_LABEL_WIDTH.max(label_painter.width())
            + (Self::LABEL_PADDING * 2.0) * text_scale_factor;
        Size::new(width, Self::PREFERRED_HEIGHT * text_scale_factor)
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
        let rectangle_width = self.upper_rectangle_width(label_painter, scale);

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

    fn upper_rectangle_width(&self, label_painter: &TextPainter, scale: f64) -> f64 {
        let unscaled_width =
            Self::MIN_LABEL_WIDTH.max(label_painter.width()) + (Self::LABEL_PADDING * 2.0);
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

        let rectangle_width = self.upper_rectangle_width(label_painter, scale);
        let horizontal_shift = self.get_horizontal_shift(
            app,
            parent_box,
            center,
            label_painter,
            text_scale_factor,
            size_with_overflow,
            scale,
        );

        let upper_rect = Rect::from_ltwh(
            -rectangle_width / 2.0 + horizontal_shift,
            -Self::RECT_Y_OFFSET - Self::PREFERRED_HEIGHT,
            rectangle_width,
            Self::PREFERRED_HEIGHT,
        );

        let fill_paint = Paint::from_color(background_paint_color.into());

        canvas.save();
        // Prepare the canvas for the base of the tooltip, which is relative to the
        // center of the thumb.
        canvas.translate(
            center.dx() as f32,
            (center.dy() - Self::BOTTOM_TIP_Y_OFFSET) as f32,
        );
        canvas.scale(scale as f32, scale as f32);

        let rrect =
            RRect::from_rect_and_radius(upper_rect, Radius::circular(upper_rect.height() / 2.0));
        if let Some(stroke_paint_color) = stroke_paint_color {
            let stroke_paint = Paint {
                color: stroke_paint_color.into(),
                style: PaintStyle::Stroke(Stroke::new(1.0)),
                ..Paint::default()
            };
            draw_rrect(canvas, rrect, &stroke_paint);
        }

        draw_rrect(canvas, rrect, &fill_paint);

        // The label text is centered within the value indicator.
        let bottom_tip_to_upper_rect_translate_y =
            -Self::PREFERRED_HALF_HEIGHT / 2.0 - upper_rect.height();
        canvas.translate(0.0, bottom_tip_to_upper_rect_translate_y as f32);
        let box_center = Offset::new(horizontal_shift, upper_rect.height() / 2.3);
        let half_label_painter_offset =
            Offset::new(label_painter.width() / 2.0, label_painter.height() / 2.0);
        let label_offset = box_center - half_label_painter_offset;
        let fonts = PaintingBinding::instance(app).fonts(app);
        label_painter.paint(app.get_mut(fonts), canvas, label_offset);
        canvas.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slider_value_indicator_shape::{RoundSliderOverlayShape, SliderComponentShapeRef};

    #[test]
    fn track_shape_is_rounded_defaults() {
        assert!(!SliderTrackShape::is_rounded(
            &RectangularSliderTrackShape::new()
        ));
        assert!(SliderTrackShape::is_rounded(
            &RoundedRectSliderTrackShape::new()
        ));
        assert!(SliderTrackShape::is_rounded(&GappedSliderTrackShape::new()));
    }

    #[test]
    fn round_slider_thumb_shape_prefers_its_enabled_or_disabled_radius() {
        let shape = RoundSliderThumbShape::new();
        assert_eq!(shape.get_preferred_size(true, false), Size::new(20.0, 20.0));
        assert_eq!(
            shape.get_preferred_size(false, false),
            Size::new(20.0, 20.0)
        );
        let shape = shape.enabled_thumb_radius(7.0).disabled_thumb_radius(4.0);
        assert_eq!(shape.get_preferred_size(true, false), Size::new(14.0, 14.0));
        assert_eq!(shape.get_preferred_size(false, false), Size::new(8.0, 8.0));
    }

    #[test]
    fn handle_thumb_shape_prefers_four_by_forty_four() {
        assert_eq!(
            HandleThumbShape::new().get_preferred_size(true, true),
            Size::new(4.0, 44.0)
        );
    }

    #[test]
    fn round_tick_mark_defaults_to_a_quarter_of_the_track_height() {
        let theme = SliderThemeData::new().track_height(8.0);
        assert_eq!(
            RoundSliderTickMarkShape::new().get_preferred_size(&theme, true),
            Size::new(4.0, 4.0)
        );
        assert_eq!(
            RoundSliderTickMarkShape::new()
                .tick_mark_radius(5.0)
                .get_preferred_size(&theme, true),
            Size::new(10.0, 10.0)
        );
        assert_eq!(
            <dyn SliderTickMarkShape>::no_tick_mark().get_preferred_size(&theme, true),
            Size::ZERO
        );
    }

    #[test]
    fn built_in_shapes_compare_by_value_and_the_empty_one_by_identity() {
        let a: SliderTrackShapeRef = Rc::new(RoundedRectSliderTrackShape::new());
        let b: SliderTrackShapeRef = Rc::new(RoundedRectSliderTrackShape::new());
        let c: SliderTrackShapeRef = Rc::new(GappedSliderTrackShape::new());
        assert!(*a == *b);
        assert!(*a != *c);

        let thumb: SliderComponentShapeRef =
            Rc::new(RoundSliderThumbShape::new().enabled_thumb_radius(7.0));
        let same: SliderComponentShapeRef =
            Rc::new(RoundSliderThumbShape::new().enabled_thumb_radius(7.0));
        let other: SliderComponentShapeRef = Rc::new(RoundSliderThumbShape::new());
        let overlay: SliderComponentShapeRef = Rc::new(RoundSliderOverlayShape::new());
        assert!(*thumb == *same);
        assert!(*thumb != *other);
        assert!(*thumb != *overlay);

        let tick: SliderTickMarkShapeRef = Rc::new(RoundSliderTickMarkShape::new());
        let same_tick: SliderTickMarkShapeRef = Rc::new(RoundSliderTickMarkShape::new());
        assert!(*tick == *same_tick);
        assert!(Rc::ptr_eq(
            &<dyn SliderTickMarkShape>::no_tick_mark(),
            &<dyn SliderTickMarkShape>::no_tick_mark()
        ));
        assert!(*tick != *<dyn SliderTickMarkShape>::no_tick_mark());
    }
}
