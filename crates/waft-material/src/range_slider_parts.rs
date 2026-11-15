//! Flutter counterpart: material_ui's `range_slider_parts.dart`.
//!
//! The range slider's shapes are abstract classes users subclass, so traits, each held as an
//! `Rc` (`RangeSliderThumbShapeRef` and the others); the mixin [`BaseRangeSliderTrackShape`]
//! is a trait with a provided body that a track's [`RangeSliderTrackShape`] method calls. A
//! parameter Dart declares optional with no default in the abstract signature is an
//! `Option`, so an implementation applies its own default as Dart's override does. A shape's
//! `paint` takes the `App`, as `slider_parts.rs` says. `RangeSlider` itself is
//! `range_slider.dart`, not ported yet.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_animation::AnyAnimation;
use waft_embedder::{
    Canvas, ClipOp, Color, FillRule, Offset, Paint, PaintStyle, Path, PathBuilder, RRect, Radius,
    Rect, Size, Stroke, TextDirection, rrect_radii_elliptical,
};
use waft_foundation::App;
use waft_painting::{
    BorderRadius, ClipContext, PaintingBinding, TextPainter, debug_disable_shadows, draw_rrect,
};
use waft_rendering::{AnyRenderBox, PaintingContext};
use waft_widgets::WidgetStates;

use crate::colors::Colors;
use crate::slider_parts::{evaluate_color_tween, evaluate_tween};
use crate::slider_theme::{SliderThemeData, Thumb};

/// Base class for `RangeSlider` thumb shapes.
///
/// Equality is [`eq_shape`](Self::eq_shape): identity, as Dart's default `==`, unless the
/// shape overrides it.
///
/// See also:
///
///  * [`RoundRangeSliderThumbShape`] for the default `RangeSlider`'s thumb shape
///    that paints a solid circle.
///  * [`RangeSliderTickMarkShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s tick marks.
///  * [`RangeSliderTrackShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s track.
///  * [`RangeSliderValueIndicatorShape`], which can be used to create custom
///    shapes for the `RangeSlider`'s value indicator.
///  * `SliderComponentShape`, which can be used to create custom shapes for
///    the `Slider`'s thumb, overlay, and value indicator and the
///    `RangeSlider`'s overlay.
pub trait RangeSliderThumbShape: Any + Debug {
    /// Returns the preferred size of the shape, based on the given conditions.
    ///
    /// The `is_discrete` argument is true if `RangeSlider::divisions` is non-null.
    /// When true, the slider will render tick marks on top of the track.
    ///
    /// The `is_enabled` argument is false when `RangeSlider::on_changed` is null and
    /// true otherwise. When true, the slider will respond to input.
    fn get_preferred_size(&self, is_enabled: bool, is_discrete: bool) -> Size;

    /// Paints the thumb shape based on the state passed to it.
    ///
    /// The `context` argument represents the `RangeSlider`'s render box.
    ///
    /// The `center` argument is the offset for where this shape's center should be
    /// painted. This offset is relative to the origin of the `context` canvas.
    ///
    /// The `activation_animation` argument is an animation triggered when the user
    /// begins to interact with the `RangeSlider`. It reverses when the user stops
    /// interacting with the slider.
    ///
    /// The `enable_animation` argument is an animation triggered when the
    /// `RangeSlider` is enabled, and it reverses when the slider is disabled. The
    /// `RangeSlider` is enabled when `RangeSlider::on_changed` is not null. Use
    /// this to paint intermediate frames for this shape when the slider changes
    /// enabled state.
    ///
    /// The `is_discrete` argument is true if `RangeSlider::divisions` is non-null.
    /// When true, the slider will render tick marks on top of the track.
    ///
    /// The `is_enabled` argument is false when `RangeSlider::on_changed` is null and
    /// true otherwise. When true, the slider will respond to input.
    ///
    /// If the `is_on_top` argument is true, this thumb is painted on top of the
    /// other slider thumb because this thumb is the one that was most recently
    /// selected.
    ///
    /// The `slider_theme` argument is the theme assigned to the `RangeSlider` that
    /// this shape belongs to.
    ///
    /// The `text_direction` argument can be used to determine how the orientation
    /// of either slider thumb should be changed, such as drawing different
    /// shapes for the left and right thumb.
    ///
    /// The `thumb` argument is the specifier for which of the two thumbs this
    /// method should paint (start or end).
    ///
    /// The `is_pressed` argument can be used to give the selected thumb
    /// additional selected or pressed state visual feedback, such as a larger
    /// shadow.
    ///
    /// `None` is Dart's argument left out.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        enable_animation: AnyAnimation<f64>,
        is_discrete: Option<bool>,
        is_enabled: Option<bool>,
        is_on_top: Option<bool>,
        text_direction: Option<TextDirection>,
        slider_theme: &SliderThemeData,
        thumb: Option<Thumb>,
        is_pressed: Option<bool>,
    );

    /// Downcast support for Dart's `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: identity unless a shape overrides it.
    fn eq_shape(&self, other: &dyn RangeSliderThumbShape) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

/// What a field or parameter Dart types as `RangeSliderThumbShape` becomes.
pub type RangeSliderThumbShapeRef = Rc<dyn RangeSliderThumbShape>;

impl PartialEq for dyn RangeSliderThumbShape {
    fn eq(&self, other: &dyn RangeSliderThumbShape) -> bool {
        self.eq_shape(other)
    }
}

/// Base class for `RangeSlider` value indicator shapes.
///
/// Equality is [`eq_shape`](Self::eq_shape): identity, as Dart's default `==`, unless the
/// shape overrides it.
///
/// See also:
///
///  * `PaddleRangeSliderValueIndicatorShape` for the default `RangeSlider`'s
///    value indicator shape that paints a custom path with text in it.
///  * [`RangeSliderTickMarkShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s tick marks.
///  * [`RangeSliderThumbShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s thumb.
///  * [`RangeSliderTrackShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s track.
///  * `SliderComponentShape`, which can be used to create custom shapes for
///    the `Slider`'s thumb, overlay, and value indicator and the
///    `RangeSlider`'s overlay.
pub trait RangeSliderValueIndicatorShape: Any + Debug {
    /// Returns the preferred size of the shape, based on the given conditions.
    ///
    /// The `is_enabled` argument is false when `RangeSlider::on_changed` is null and
    /// true otherwise. When true, the slider will respond to input.
    ///
    /// The `is_discrete` argument is true if `RangeSlider::divisions` is non-null.
    /// When true, the slider will render tick marks on top of the track.
    ///
    /// The `label_painter` argument helps determine the width of the shape. It is
    /// variable width because it is derived from a formatted string.
    ///
    /// The `text_scale_factor` argument can be used to determine whether the
    /// component should paint larger or smaller, depending on whether
    /// `text_scale_factor` is greater than 1 for larger, and between 0 and 1 for
    /// smaller. It's usually computed from `MediaQueryData::text_scaler`.
    fn get_preferred_size(
        &self,
        is_enabled: bool,
        is_discrete: bool,
        label_painter: &TextPainter,
        text_scale_factor: f64,
    ) -> Size;

    /// Determines the best offset to keep this shape on the screen.
    ///
    /// Override this method when the center of the value indicator should be
    /// shifted from the vertical center of the thumb.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
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
        let _ = (
            app,
            parent_box,
            center,
            label_painter,
            activation_animation,
            text_scale_factor,
            size_with_overflow,
        );
        0.0
    }

    /// Paints the value indicator shape based on the state passed to it.
    ///
    /// The `context` argument represents the `RangeSlider`'s render box.
    ///
    /// The `center` argument is the offset for where this shape's center should be
    /// painted. This offset is relative to the origin of the `context` canvas.
    ///
    /// The `activation_animation` argument is an animation triggered when the user
    /// begins to interact with the `RangeSlider`. It reverses when the user stops
    /// interacting with the slider.
    ///
    /// The `enable_animation` argument is an animation triggered when the
    /// `RangeSlider` is enabled, and it reverses when the slider is disabled. The
    /// `RangeSlider` is enabled when `RangeSlider::on_changed` is not null. Use
    /// this to paint intermediate frames for this shape when the slider changes
    /// enabled state.
    ///
    /// The `is_discrete` argument is true if `RangeSlider::divisions` is non-null.
    /// When true, the slider will render tick marks on top of the track.
    ///
    /// The `is_on_top` argument is the top-most value indicator between the two value
    /// indicators, which is always the indicator for the most recently selected thumb. In
    /// the default case, this is used to paint a stroke around the top indicator
    /// for better visibility between the two indicators.
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
    ///
    /// The `parent_box` argument is the `RenderBox` of the `RangeSlider`. Its
    /// attributes, such as size, can be used to assist in painting this shape.
    ///
    /// The `slider_theme` argument is the theme assigned to the `RangeSlider` that
    /// this shape belongs to.
    ///
    /// The `text_direction` argument can be used to determine how any extra text
    /// or graphics, besides the text painted by the `label_painter` should be
    /// positioned. The `label_painter` argument already has the `text_direction`
    /// set.
    ///
    /// The `value` argument is the current parametric value (from 0.0 to 1.0) of
    /// the slider.
    ///
    /// The `thumb` argument is the specifier for which of the two thumbs this
    /// method should paint (start or end).
    ///
    /// `None` is Dart's argument left out.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        activation_animation: AnyAnimation<f64>,
        enable_animation: AnyAnimation<f64>,
        is_discrete: Option<bool>,
        is_on_top: Option<bool>,
        label_painter: &mut TextPainter,
        text_scale_factor: Option<f64>,
        size_with_overflow: Option<Size>,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        text_direction: Option<TextDirection>,
        value: Option<f64>,
        thumb: Option<Thumb>,
    );

    /// Downcast support for Dart's `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: identity unless a shape overrides it.
    fn eq_shape(&self, other: &dyn RangeSliderValueIndicatorShape) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

/// What a field or parameter Dart types as `RangeSliderValueIndicatorShape` becomes.
pub type RangeSliderValueIndicatorShapeRef = Rc<dyn RangeSliderValueIndicatorShape>;

impl PartialEq for dyn RangeSliderValueIndicatorShape {
    fn eq(&self, other: &dyn RangeSliderValueIndicatorShape) -> bool {
        self.eq_shape(other)
    }
}

/// Base class for `RangeSlider` tick mark shapes.
///
/// This is a simplified version of `SliderComponentShape` with a
/// [`SliderThemeData`] passed when getting the preferred size.
///
/// Equality is [`eq_shape`](Self::eq_shape): identity, as Dart's default `==`, unless the
/// shape overrides it.
///
/// See also:
///
///  * [`RoundRangeSliderTickMarkShape`] for the default `RangeSlider`'s tick mark
///    shape that paints a solid circle.
///  * [`RangeSliderThumbShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s thumb.
///  * [`RangeSliderTrackShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s track.
///  * [`RangeSliderValueIndicatorShape`], which can be used to create custom
///    shapes for the `RangeSlider`'s value indicator.
///  * `SliderComponentShape`, which can be used to create custom shapes for
///    the `Slider`'s thumb, overlay, and value indicator and the
///    `RangeSlider`'s overlay.
pub trait RangeSliderTickMarkShape: Any + Debug {
    /// Returns the preferred size of the shape.
    ///
    /// It is used to help position the tick marks within the slider.
    ///
    /// The `slider_theme` argument is the theme assigned to the `RangeSlider` that
    /// this shape belongs to.
    ///
    /// The `is_enabled` argument is false when `RangeSlider::on_changed` is null and
    /// true otherwise. When true, the slider will respond to input. `None` is Dart's
    /// argument left out.
    fn get_preferred_size(&self, slider_theme: &SliderThemeData, is_enabled: Option<bool>) -> Size;

    /// Paints the slider track.
    ///
    /// The `context` argument represents the `RangeSlider`'s render box.
    ///
    /// The `center` argument is the offset for where this shape's center should be
    /// painted. This offset is relative to the origin of the `context` canvas.
    ///
    /// The `parent_box` argument is the `RenderBox` of the `RangeSlider`. Its
    /// attributes, such as size, can be used to assist in painting this shape.
    ///
    /// The `slider_theme` argument is the theme assigned to the `RangeSlider` that
    /// this shape belongs to.
    ///
    /// The `enable_animation` argument is an animation triggered when the
    /// `RangeSlider` is enabled, and it reverses when the slider is disabled. The
    /// `RangeSlider` is enabled when `RangeSlider::on_changed` is not null. Use
    /// this to paint intermediate frames for this shape when the slider changes
    /// enabled state.
    ///
    /// The `is_enabled` argument is false when `RangeSlider::on_changed` is null and
    /// true otherwise. When true, the slider will respond to input. `None` is Dart's
    /// argument left out.
    ///
    /// The `text_direction` argument can be used to determine how the tick marks
    /// are painted depending on whether they are on an active track segment or not.
    ///
    /// The track segment between the two thumbs is the active track segment. The
    /// track segments between the thumb and each end of the slider are the inactive
    /// track segments. In `TextDirection::Ltr`, the start of the slider is on the
    /// left, and in `TextDirection::Rtl`, the start of the slider is on the right.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        center: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: Option<bool>,
        text_direction: TextDirection,
    );

    /// Downcast support for Dart's `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: identity unless a shape overrides it.
    fn eq_shape(&self, other: &dyn RangeSliderTickMarkShape) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

/// What a field or parameter Dart types as `RangeSliderTickMarkShape` becomes.
pub type RangeSliderTickMarkShapeRef = Rc<dyn RangeSliderTickMarkShape>;

impl PartialEq for dyn RangeSliderTickMarkShape {
    fn eq(&self, other: &dyn RangeSliderTickMarkShape) -> bool {
        self.eq_shape(other)
    }
}

/// Base class for `RangeSlider` track shapes.
///
/// The slider's thumbs move along the track. A discrete slider's tick marks
/// are drawn after the track, but before the thumb, and are aligned with the
/// track.
///
/// The [`get_preferred_rect`](Self::get_preferred_rect) helps position the slider thumbs and
/// tick marks relative to the track.
///
/// Equality is [`eq_shape`](Self::eq_shape): identity, as Dart's default `==`, unless the
/// shape overrides it.
///
/// See also:
///
///  * [`RoundedRectRangeSliderTrackShape`] for the default `RangeSlider`'s track
///    shape that paints a stadium-like track.
///  * [`RangeSliderTickMarkShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s tick marks.
///  * [`RangeSliderThumbShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s thumb.
///  * [`RangeSliderValueIndicatorShape`], which can be used to create custom
///    shapes for the `RangeSlider`'s value indicator.
///  * `SliderComponentShape`, which can be used to create custom shapes for
///    the `Slider`'s thumb, overlay, and value indicator and the
///    `RangeSlider`'s overlay.
pub trait RangeSliderTrackShape: Any + Debug {
    /// Returns the preferred bounds of the shape.
    ///
    /// It is used to provide horizontal boundaries for the position of the
    /// thumbs, and to help position the slider thumbs and tick marks relative to
    /// the track.
    ///
    /// The `parent_box` argument can be used to help determine the preferredRect
    /// relative to attributes of the render box of the slider itself, such as
    /// size.
    ///
    /// The `offset` argument is relative to the caller's bounding box. It can be
    /// used to convert gesture coordinates from global to slider-relative
    /// coordinates; Dart's default is `Offset.zero`.
    ///
    /// The `slider_theme` argument is the theme assigned to the `RangeSlider` that
    /// this shape belongs to.
    ///
    /// The `is_enabled` argument is false when `RangeSlider::on_changed` is null and
    /// true otherwise. When true, the slider will respond to input. `None` is Dart's
    /// argument left out.
    ///
    /// The `is_discrete` argument is true if `RangeSlider::divisions` is non-null.
    /// When true, the slider will render tick marks on top of the track. `None` is
    /// Dart's argument left out.
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
    /// The `offset` argument is the offset of the origin of the `parent_box` to
    /// the origin of its `context` canvas. This shape must be painted relative
    /// to this offset. See `PaintingContextCallback`.
    ///
    /// The `parent_box` argument is the `RenderBox` of the `RangeSlider`. Its
    /// attributes, such as size, can be used to assist in painting this shape.
    ///
    /// The `slider_theme` argument is the theme assigned to the `RangeSlider` that
    /// this shape belongs to.
    ///
    /// The `enable_animation` argument is an animation triggered when the
    /// `RangeSlider` is enabled, and it reverses when the slider is disabled. The
    /// `RangeSlider` is enabled when `RangeSlider::on_changed` is not null. Use
    /// this to paint intermediate frames for this shape when the slider changes
    /// enabled state.
    ///
    /// The `start_thumb_center` argument is the offset of the center of the start
    /// thumb relative to the origin of the `PaintingContext::canvas`. It can be
    /// used as one point that divides the track between inactive and active.
    ///
    /// The `end_thumb_center` argument is the offset of the center of the end
    /// thumb relative to the origin of the `PaintingContext::canvas`. It can be
    /// used as one point that divides the track between inactive and active.
    ///
    /// The `is_enabled` argument is false when `RangeSlider::on_changed` is null and
    /// true otherwise. When true, the slider will respond to input; Dart's default is
    /// false.
    ///
    /// The `is_discrete` argument is true if `RangeSlider::divisions` is non-null.
    /// When true, the slider will render tick marks on top of the track; Dart's default
    /// is false.
    ///
    /// The `text_direction` argument can be used to determine how the track
    /// segments are painted depending on whether they are on an active track
    /// segment or not.
    ///
    /// The track segment between the two thumbs is the active track segment. The
    /// track segments between the thumb and each end of the slider are the inactive
    /// track segments. In `TextDirection::Ltr`, the start of the slider is on the
    /// left, and in `TextDirection::Rtl`, the start of the slider is on the right.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: bool,
        is_discrete: bool,
        text_direction: TextDirection,
    );

    /// Whether the track shape is rounded. This is used to determine the correct
    /// position of the thumbs in relation to the track. Defaults to false.
    fn is_rounded(&self) -> bool {
        false
    }

    /// Downcast support for Dart's `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: identity unless a shape overrides it.
    fn eq_shape(&self, other: &dyn RangeSliderTrackShape) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

/// What a field or parameter Dart types as `RangeSliderTrackShape` becomes.
pub type RangeSliderTrackShapeRef = Rc<dyn RangeSliderTrackShape>;

impl PartialEq for dyn RangeSliderTrackShape {
    fn eq(&self, other: &dyn RangeSliderTrackShape) -> bool {
        self.eq_shape(other)
    }
}

/// Base range slider track shape that provides an implementation of
/// [`get_preferred_rect`](Self::get_preferred_rect) for default sizing.
///
/// The height is set from `SliderThemeData::track_height` and the width of the
/// parent box less the larger of the widths of `SliderThemeData::range_thumb_shape` and
/// `SliderThemeData::overlay_shape`.
///
/// A track that mixes it in calls this body from its [`RangeSliderTrackShape`] method.
///
/// See also:
///
///  * [`RectangularRangeSliderTrackShape`], which is a track shape with sharp
///    rectangular edges
pub trait BaseRangeSliderTrackShape {
    /// Returns a rect that represents the track bounds that fits within the
    /// `Slider`.
    ///
    /// The width is the width of the `RangeSlider`, but padded by the max
    /// of the overlay and thumb radius. The height is defined by the
    /// `SliderThemeData::track_height`.
    ///
    /// The [`Rect`] is centered both horizontally and vertically within the slider
    /// bounds.
    ///
    /// `None` for `is_enabled` or `is_discrete` is Dart's default, false.
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
        debug_assert!(slider_theme.range_thumb_shape.is_some());
        debug_assert!(slider_theme.overlay_shape.is_some());
        debug_assert!(slider_theme.track_height.is_some());
        let thumb_size = slider_theme
            .range_thumb_shape
            .as_ref()
            .expect("the slider theme has a range thumb shape")
            .get_preferred_size(is_enabled, is_discrete);
        let overlay_width = slider_theme
            .overlay_shape
            .as_ref()
            .expect("the slider theme has an overlay shape")
            .get_preferred_size(is_enabled, is_discrete)
            .width();
        let mut track_height = slider_theme
            .track_height
            .expect("the slider theme has a track height");
        debug_assert!(overlay_width >= 0.0);
        debug_assert!(track_height >= 0.0);

        // If the track colors are transparent, then override only the track height
        // to maintain overall Slider width.
        if slider_theme.active_track_color == Some(Colors::TRANSPARENT)
            && slider_theme.inactive_track_color == Some(Colors::TRANSPARENT)
        {
            track_height = 0.0;
        }

        let track_left = offset.dx()
            + if slider_theme.padding.is_none() {
                (overlay_width / 2.0).max(thumb_size.width() / 2.0)
            } else {
                thumb_size.width() / 2.0
            };
        let track_top = offset.dy() + (parent_box.size(app).height() - track_height) / 2.0;
        let track_right = track_left + parent_box.size(app).width()
            - if slider_theme.padding.is_none() {
                thumb_size.width().max(overlay_width)
            } else {
                thumb_size.width()
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
}

/// A `RangeSlider` track that's a simple rectangle.
///
/// It paints a solid colored rectangle, vertically centered in the
/// `parent_box`. The track rectangle extends to the bounds of the `parent_box`,
/// but is padded by the `RoundSliderOverlayShape` radius. The height is
/// defined by the `SliderThemeData::track_height`. The color is determined by the
/// `Slider`'s enabled state and the track segment's active state which are
/// defined by:
///   `SliderThemeData::active_track_color`,
///   `SliderThemeData::inactive_track_color`,
///   `SliderThemeData::disabled_active_track_color`,
///   `SliderThemeData::disabled_inactive_track_color`.
///
/// The track segment between the two thumbs is the active track segment. The
/// track segments between the thumb and each end of the slider are the inactive
/// track segments. In `TextDirection::Ltr`, the start of the slider is on the
/// left, and in `TextDirection::Rtl`, the start of the slider is on the right.
///
/// See also:
///
///  * `RangeSlider`, for the component that is meant to display this shape.
///  * [`SliderThemeData`], where an instance of this class is set to inform the
///    slider of the visual details of the its track.
///  * [`RangeSliderTrackShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s track.
///  * [`RoundedRectRangeSliderTrackShape`], for a similar track with rounded
///    edges.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectangularRangeSliderTrackShape;

impl RectangularRangeSliderTrackShape {
    /// Create a slider track with rectangular outer edges.
    ///
    /// The middle track segment is the selected range and is active, and the two
    /// outer track segments are inactive.
    pub const fn new() -> RectangularRangeSliderTrackShape {
        RectangularRangeSliderTrackShape
    }
}

impl BaseRangeSliderTrackShape for RectangularRangeSliderTrackShape {}

impl RangeSliderTrackShape for RectangularRangeSliderTrackShape {
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect {
        BaseRangeSliderTrackShape::get_preferred_rect(
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
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: bool,
        is_discrete: bool,
        text_direction: TextDirection,
    ) {
        debug_assert!(slider_theme.disabled_active_track_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_track_color.is_some());
        debug_assert!(slider_theme.active_track_color.is_some());
        debug_assert!(slider_theme.inactive_track_color.is_some());
        debug_assert!(slider_theme.range_thumb_shape.is_some());
        // Assign the track segment paints, which are left: active, right: inactive,
        // but reversed for right to left text.
        let active_paint = fill_paint(
            evaluate_color_tween(
                app,
                slider_theme.disabled_active_track_color,
                slider_theme.active_track_color,
                enable_animation,
            )
            .expect("the active track colors are set"),
        );
        let inactive_paint = fill_paint(
            evaluate_color_tween(
                app,
                slider_theme.disabled_inactive_track_color,
                slider_theme.inactive_track_color,
                enable_animation,
            )
            .expect("the inactive track colors are set"),
        );

        let (left_thumb_offset, right_thumb_offset) = match text_direction {
            TextDirection::Ltr => (start_thumb_center, end_thumb_center),
            TextDirection::Rtl => (end_thumb_center, start_thumb_center),
        };

        let track_rect = RangeSliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            Some(is_enabled),
            Some(is_discrete),
        );
        let canvas = context.canvas();
        let left_track_segment = Rect::from_ltrb(
            track_rect.left,
            track_rect.top,
            left_thumb_offset.dx(),
            track_rect.bottom,
        );
        if !left_track_segment.is_empty() {
            canvas.draw_rect(left_track_segment, &inactive_paint);
        }
        let middle_track_segment = Rect::from_ltrb(
            left_thumb_offset.dx(),
            track_rect.top,
            right_thumb_offset.dx(),
            track_rect.bottom,
        );
        if !middle_track_segment.is_empty() {
            canvas.draw_rect(middle_track_segment, &active_paint);
        }
        let right_track_segment = Rect::from_ltrb(
            right_thumb_offset.dx(),
            track_rect.top,
            track_rect.right,
            track_rect.bottom,
        );
        if !right_track_segment.is_empty() {
            canvas.draw_rect(right_track_segment, &inactive_paint);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn RangeSliderTrackShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The default shape of a `RangeSlider`'s track.
///
/// It paints a solid colored rectangle with rounded edges, vertically centered
/// in the `parent_box`. The track rectangle extends to the bounds of the
/// `parent_box`, but is padded by the larger of `RoundSliderOverlayShape`'s
/// radius and [`RoundRangeSliderThumbShape`]'s radius. The height is defined by
/// the `SliderThemeData::track_height`. The color is determined by the
/// `RangeSlider`'s enabled state and the track segment's active state which are
/// defined by:
///   `SliderThemeData::active_track_color`,
///   `SliderThemeData::inactive_track_color`,
///   `SliderThemeData::disabled_active_track_color`,
///   `SliderThemeData::disabled_inactive_track_color`.
///
/// The track segment between the two thumbs is the active track segment. The
/// track segments between the thumb and each end of the slider are the inactive
/// track segments. In `TextDirection::Ltr`, the start of the slider is on the
/// left, and in `TextDirection::Rtl`, the start of the slider is on the right.
///
/// See also:
///
///  * `RangeSlider`, for the component that is meant to display this shape.
///  * [`SliderThemeData`], where an instance of this class is set to inform the
///    slider of the visual details of the its track.
///  * [`RangeSliderTrackShape`], which can be used to create custom shapes for
///    the `RangeSlider`'s track.
///  * [`RectangularRangeSliderTrackShape`], for a similar track with sharp edges.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedRectRangeSliderTrackShape;

impl RoundedRectRangeSliderTrackShape {
    /// Create a slider track with rounded outer edges.
    ///
    /// The middle track segment is the selected range and is active, and the two
    /// outer track segments are inactive.
    pub const fn new() -> RoundedRectRangeSliderTrackShape {
        RoundedRectRangeSliderTrackShape
    }

    /// Dart's `paint` override with its extra `additionalActiveTrackHeight` (Dart's
    /// default 2), which a call through [`RangeSliderTrackShape`] cannot pass.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    pub fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: bool,
        is_discrete: bool,
        text_direction: TextDirection,
        additional_active_track_height: f64,
    ) {
        debug_assert!(slider_theme.disabled_active_track_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_track_color.is_some());
        debug_assert!(slider_theme.active_track_color.is_some());
        debug_assert!(slider_theme.inactive_track_color.is_some());
        debug_assert!(slider_theme.range_thumb_shape.is_some());

        let Some(track_height) = slider_theme.track_height.filter(|height| *height > 0.0) else {
            return;
        };

        // Assign the track segment paints, which are left: active, right: inactive,
        // but reversed for right to left text.
        let active_paint = fill_paint(
            evaluate_color_tween(
                app,
                slider_theme.disabled_active_track_color,
                slider_theme.active_track_color,
                enable_animation,
            )
            .expect("the active track colors are set"),
        );
        let inactive_paint = fill_paint(
            evaluate_color_tween(
                app,
                slider_theme.disabled_inactive_track_color,
                slider_theme.inactive_track_color,
                enable_animation,
            )
            .expect("the inactive track colors are set"),
        );

        let (left_thumb_offset, right_thumb_offset) = match text_direction {
            TextDirection::Ltr => (start_thumb_center, end_thumb_center),
            TextDirection::Rtl => (end_thumb_center, start_thumb_center),
        };
        let thumb_size = slider_theme
            .range_thumb_shape
            .as_ref()
            .expect("the slider theme has a range thumb shape")
            .get_preferred_size(is_enabled, is_discrete);
        let thumb_radius = thumb_size.width() / 2.0;
        debug_assert!(thumb_radius > 0.0);

        let track_rect = RangeSliderTrackShape::get_preferred_rect(
            self,
            app,
            parent_box,
            offset,
            slider_theme,
            Some(is_enabled),
            Some(is_discrete),
        );

        let track_radius = Radius::circular(track_rect.height() / 2.0);

        let canvas = context.canvas();
        draw_rrect(
            canvas,
            RRect::from_ltrb_and_corners(
                track_rect.left,
                track_rect.top,
                left_thumb_offset.dx(),
                track_rect.bottom,
                track_radius,
                Radius::ZERO,
                Radius::ZERO,
                track_radius,
            ),
            &inactive_paint,
        );
        draw_rrect(
            canvas,
            RRect::from_ltrb_and_corners(
                right_thumb_offset.dx(),
                track_rect.top,
                track_rect.right,
                track_rect.bottom,
                Radius::ZERO,
                track_radius,
                track_radius,
                Radius::ZERO,
            ),
            &inactive_paint,
        );
        draw_rrect(
            canvas,
            RRect::from_ltrbr(
                left_thumb_offset.dx() - (track_height / 2.0),
                track_rect.top - (additional_active_track_height / 2.0),
                right_thumb_offset.dx() + (track_height / 2.0),
                track_rect.bottom + (additional_active_track_height / 2.0),
                track_radius,
            ),
            &active_paint,
        );
    }
}

impl BaseRangeSliderTrackShape for RoundedRectRangeSliderTrackShape {}

impl RangeSliderTrackShape for RoundedRectRangeSliderTrackShape {
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect {
        BaseRangeSliderTrackShape::get_preferred_rect(
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
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: bool,
        is_discrete: bool,
        text_direction: TextDirection,
    ) {
        RoundedRectRangeSliderTrackShape::paint(
            self,
            app,
            context,
            offset,
            parent_box,
            slider_theme,
            enable_animation,
            start_thumb_center,
            end_thumb_center,
            is_enabled,
            is_discrete,
            text_direction,
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
    fn eq_shape(&self, other: &dyn RangeSliderTrackShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The default shape of each `RangeSlider` tick mark.
///
/// Tick marks are only displayed if the slider is discrete, which can be done
/// by setting the `RangeSlider::divisions` to an integer value.
///
/// It paints a solid circle, centered on the track.
/// The color is determined by the `Slider`'s enabled state and track's active
/// states. These colors are defined in:
///   `SliderThemeData::active_track_color`,
///   `SliderThemeData::inactive_track_color`,
///   `SliderThemeData::disabled_active_track_color`,
///   `SliderThemeData::disabled_inactive_track_color`.
///
/// See also:
///
///  * `RangeSlider`, which includes tick marks defined by this shape.
///  * `SliderTheme`, which can be used to configure the tick mark shape of all
///    sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundRangeSliderTickMarkShape {
    /// The preferred radius of the round tick mark.
    ///
    /// If it is not provided, then 1/4 of the `SliderThemeData::track_height` is used.
    pub tick_mark_radius: Option<f64>,
}

impl RoundRangeSliderTickMarkShape {
    /// Create a range slider tick mark that draws a circle.
    pub const fn new() -> RoundRangeSliderTickMarkShape {
        RoundRangeSliderTickMarkShape {
            tick_mark_radius: None,
        }
    }

    /// Dart `RoundRangeSliderTickMarkShape(tickMarkRadius:)`.
    pub const fn tick_mark_radius(
        mut self,
        tick_mark_radius: f64,
    ) -> RoundRangeSliderTickMarkShape {
        self.tick_mark_radius = Some(tick_mark_radius);
        self
    }
}

impl RangeSliderTickMarkShape for RoundRangeSliderTickMarkShape {
    fn get_preferred_size(
        &self,
        slider_theme: &SliderThemeData,
        _is_enabled: Option<bool>,
    ) -> Size {
        debug_assert!(slider_theme.track_height.is_some());
        Size::from_radius(self.tick_mark_radius.unwrap_or_else(|| {
            slider_theme
                .track_height
                .expect("the slider theme has a track height")
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
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: Option<bool>,
        text_direction: TextDirection,
    ) {
        let is_enabled = is_enabled.unwrap_or(false);
        debug_assert!(slider_theme.disabled_active_tick_mark_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_tick_mark_color.is_some());
        debug_assert!(slider_theme.active_tick_mark_color.is_some());
        debug_assert!(slider_theme.inactive_tick_mark_color.is_some());

        let has_gap = slider_theme
            .track_gap
            .is_some_and(|track_gap| track_gap > 0.0);
        let under_thumb =
            start_thumb_center.dx() == center.dx() || end_thumb_center.dx() == center.dx();
        if has_gap && under_thumb {
            return;
        }
        let is_between_thumbs = match text_direction {
            TextDirection::Ltr => {
                start_thumb_center.dx() < center.dx() && center.dx() < end_thumb_center.dx()
            }
            TextDirection::Rtl => {
                end_thumb_center.dx() < center.dx() && center.dx() < start_thumb_center.dx()
            }
        };
        let begin = if is_between_thumbs {
            slider_theme.disabled_active_tick_mark_color
        } else {
            slider_theme.disabled_inactive_tick_mark_color
        };
        let end = if is_between_thumbs {
            slider_theme.active_tick_mark_color
        } else {
            slider_theme.inactive_tick_mark_color
        };
        let paint = fill_paint(
            evaluate_color_tween(app, begin, end, enable_animation)
                .expect("the tick mark colors are set"),
        );

        // The tick marks are tiny circles that are the same height as the track.
        let tick_mark_radius =
            RangeSliderTickMarkShape::get_preferred_size(self, slider_theme, Some(is_enabled))
                .width()
                / 2.0;
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
    fn eq_shape(&self, other: &dyn RangeSliderTickMarkShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The default shape of a `RangeSlider`'s thumbs.
///
/// There is a shadow for the resting and pressed state.
///
/// See also:
///
///  * `RangeSlider`, which includes thumbs defined by this shape.
///  * `SliderTheme`, which can be used to configure the thumb shapes of all
///    range sliders in a widget subtree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundRangeSliderThumbShape {
    /// The preferred radius of the round thumb shape when the slider is enabled.
    ///
    /// If it is not provided, then the Material Design default of 10 is used.
    pub enabled_thumb_radius: f64,

    /// The preferred radius of the round thumb shape when the slider is disabled.
    ///
    /// If no disabledRadius is provided, then it is equal to the
    /// [`enabled_thumb_radius`](Self::enabled_thumb_radius).
    pub disabled_thumb_radius: Option<f64>,

    /// The resting elevation adds shadow to the unpressed thumb.
    ///
    /// The default is 1.
    pub elevation: f64,

    /// The pressed elevation adds shadow to the pressed thumb.
    ///
    /// The default is 6.
    pub pressed_elevation: f64,
}

impl RoundRangeSliderThumbShape {
    /// Create a slider thumb that draws a circle.
    pub const fn new() -> RoundRangeSliderThumbShape {
        RoundRangeSliderThumbShape {
            enabled_thumb_radius: 10.0,
            disabled_thumb_radius: None,
            elevation: 1.0,
            pressed_elevation: 6.0,
        }
    }

    /// Dart `RoundRangeSliderThumbShape(enabledThumbRadius:)`.
    pub const fn enabled_thumb_radius(
        mut self,
        enabled_thumb_radius: f64,
    ) -> RoundRangeSliderThumbShape {
        self.enabled_thumb_radius = enabled_thumb_radius;
        self
    }

    /// Dart `RoundRangeSliderThumbShape(disabledThumbRadius:)`.
    pub const fn disabled_thumb_radius(
        mut self,
        disabled_thumb_radius: f64,
    ) -> RoundRangeSliderThumbShape {
        self.disabled_thumb_radius = Some(disabled_thumb_radius);
        self
    }

    /// Dart `RoundRangeSliderThumbShape(elevation:)`.
    pub const fn elevation(mut self, elevation: f64) -> RoundRangeSliderThumbShape {
        self.elevation = elevation;
        self
    }

    /// Dart `RoundRangeSliderThumbShape(pressedElevation:)`.
    pub const fn pressed_elevation(mut self, pressed_elevation: f64) -> RoundRangeSliderThumbShape {
        self.pressed_elevation = pressed_elevation;
        self
    }

    /// Dart's `_disabledThumbRadius`.
    fn effective_disabled_thumb_radius(&self) -> f64 {
        self.disabled_thumb_radius
            .unwrap_or(self.enabled_thumb_radius)
    }
}

impl Default for RoundRangeSliderThumbShape {
    fn default() -> RoundRangeSliderThumbShape {
        RoundRangeSliderThumbShape::new()
    }
}

impl RangeSliderThumbShape for RoundRangeSliderThumbShape {
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
        _is_discrete: Option<bool>,
        _is_enabled: Option<bool>,
        is_on_top: Option<bool>,
        _text_direction: Option<TextDirection>,
        slider_theme: &SliderThemeData,
        _thumb: Option<Thumb>,
        is_pressed: Option<bool>,
    ) {
        debug_assert!(slider_theme.show_value_indicator.is_some());
        debug_assert!(slider_theme.overlapping_shape_stroke_color.is_some());
        let radius = evaluate_tween(
            app,
            self.effective_disabled_thumb_radius(),
            self.enabled_thumb_radius,
            enable_animation,
        );

        // Add a stroke of 1dp around the circle if this thumb would overlap
        // the other thumb.
        if is_on_top.unwrap_or(false) {
            let stroke_paint = stroke_paint(
                slider_theme
                    .overlapping_shape_stroke_color
                    .expect("the slider theme has an overlapping shape stroke color"),
            );
            context
                .canvas()
                .draw_circle(center, radius as f32, &stroke_paint);
        }

        let color = evaluate_color_tween(
            app,
            slider_theme.disabled_thumb_color,
            slider_theme.thumb_color,
            enable_animation,
        )
        .expect("the thumb colors are set");

        let evaluated_elevation =
            if is_pressed.expect("the range slider says whether it is pressed") {
                evaluate_tween(
                    app,
                    self.elevation,
                    self.pressed_elevation,
                    activation_animation,
                )
            } else {
                self.elevation
            };
        let mut shadow_path = PathBuilder::new();
        shadow_path.arc(center, radius as f32, 0.0, std::f32::consts::TAU);
        let shadow_path = Path::new(shadow_path.build());

        let mut paint_shadows = true;
        if cfg!(debug_assertions) && debug_disable_shadows() {
            debug_draw_shadow(context.canvas(), &shadow_path, evaluated_elevation);
            paint_shadows = false;
        }

        if paint_shadows {
            let device_pixel_ratio = implicit_view_device_pixel_ratio(app);
            context.canvas().draw_shadow(
                shadow_path.as_valo(),
                shadow_path.fill_type,
                Colors::BLACK.into(),
                evaluated_elevation as f32,
                true,
                device_pixel_ratio as f32,
            );
        }

        context
            .canvas()
            .draw_circle(center, radius as f32, &fill_paint(color));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn RangeSliderThumbShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The device pixel ratio the engine's `Canvas.drawShadow` reads from the implicit view:
/// the first view the platform reports, 1 when it reports none, as the engine's is without
/// metrics.
fn implicit_view_device_pixel_ratio(app: &App) -> f64 {
    app.platform()
        .views()
        .first()
        .map_or(1.0, |view| view.metrics().device_pixel_ratio)
}

/// Decides which thumbs (if any) should be selected.
///
/// The default finds the closest thumb, but if the thumbs are close to each
/// other, it waits for movement defined by `dx` to determine the selected
/// thumb.
///
/// Override `SliderThemeData::thumb_selector` for custom thumb selection.
///
/// Its arguments are Dart's `textDirection`, `values`, `tapValue`, `thumbSize`,
/// `trackSize` and `dx`, in that order; two selectors are equal only when they are one `Rc`.
pub type RangeThumbSelector =
    Rc<dyn Fn(TextDirection, RangeValues, f64, Size, Size, f64) -> Option<Thumb>>;

/// Object for representing range slider thumb values.
///
/// This object is passed into `RangeSlider::values` to set its values, and it
/// is emitted in `RangeSlider::on_changed`, `RangeSlider::on_change_start`, and
/// `RangeSlider::on_change_end` when the values change.
#[derive(Clone, Copy, PartialEq)]
pub struct RangeValues {
    /// The value of the start thumb.
    ///
    /// For LTR text direction, the start is the left thumb, and for RTL text
    /// direction, the start is the right thumb.
    pub start: f64,

    /// The value of the end thumb.
    ///
    /// For LTR text direction, the end is the right thumb, and for RTL text
    /// direction, the end is the left thumb.
    pub end: f64,
}

impl RangeValues {
    /// Creates pair of start and end values.
    pub const fn new(start: f64, end: f64) -> RangeValues {
        RangeValues { start, end }
    }
}

impl Debug for RangeValues {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RangeValues({:?}, {:?})", self.start, self.end)
    }
}

/// Object for setting range slider label values that appear in the value
/// indicator for each thumb.
///
/// Used in combination with `SliderThemeData::show_value_indicator` to display
/// labels above the thumbs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RangeLabels {
    /// The label of the start thumb.
    ///
    /// For LTR text direction, the start is the left thumb, and for RTL text
    /// direction, the start is the right thumb.
    pub start: String,

    /// The label of the end thumb.
    ///
    /// For LTR text direction, the end is the right thumb, and for RTL text
    /// direction, the end is the left thumb.
    pub end: String,
}

impl RangeLabels {
    /// Creates pair of start and end labels.
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> RangeLabels {
        RangeLabels {
            start: start.into(),
            end: end.into(),
        }
    }
}

impl Debug for RangeLabels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RangeLabels({}, {})", self.start, self.end)
    }
}

/// Dart's `_debugDrawShadow`.
fn debug_draw_shadow(canvas: &mut Canvas, path: &Path, elevation: f64) {
    if elevation > 0.0 {
        canvas.draw_path(
            path.as_valo(),
            path.fill_type,
            &Paint {
                color: Colors::BLACK.into(),
                style: PaintStyle::Stroke(Stroke::new((elevation * 2.0) as f32)),
                ..Paint::default()
            },
        );
    }
}

/// Dart's `Paint()..color = color`.
fn fill_paint(color: Color) -> Paint {
    Paint {
        color: color.into(),
        ..Paint::default()
    }
}

/// Dart's `Paint()..color = color..strokeWidth = 1.0..style = PaintingStyle.stroke`.
fn stroke_paint(color: Color) -> Paint {
    Paint {
        color: color.into(),
        style: PaintStyle::Stroke(Stroke::new(1.0)),
        ..Paint::default()
    }
}

/// The gapped shape of a `RangeSlider`'s track.
///
/// The [`GappedRangeSliderTrackShape`] consists of active and inactive
/// tracks. The active track uses the `SliderThemeData::active_track_color` and the
/// inactive tracks uses the `SliderThemeData::inactive_track_color`.
///
/// The track shape uses circular corner radius for the edge corners and a corner radius
/// of 2 pixels for the inside corners.
///
/// Between the active and inactive tracks there are gaps of size
/// `SliderThemeData::track_gap`. If the `SliderThemeData::thumb_shape` is
/// [`HandleRangeSliderThumbShape`] and the thumb is pressed, the thumb's width is reduced;
/// as a result, the track gaps size in [`GappedRangeSliderTrackShape`] is also reduced.
///
/// If `SliderThemeData::track_gap` is null, then the track gaps size defaults to 6 pixels.
///
/// If `ThemeData::use_material3` is true and `RangeSlider::year2023` is false, then the
/// `RangeSlider` will use [`GappedRangeSliderTrackShape`] as the default track shape.
///
/// See also:
///
///  * `RangeSlider`, which includes a track defined by this shape.
///  * `SliderTheme`, which can be used to configure the track shape of all
///    range sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GappedRangeSliderTrackShape;

impl GappedRangeSliderTrackShape {
    /// Create a range slider track that draws 3 rounded rectangles with rounded outer edges.
    pub const fn new() -> GappedRangeSliderTrackShape {
        GappedRangeSliderTrackShape
    }

    /// Dart's `paint` override with its extra `additionalActiveTrackHeight` (Dart's
    /// default 2), which a call through [`RangeSliderTrackShape`] cannot pass.
    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    pub fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        parent_box: AnyRenderBox,
        slider_theme: &SliderThemeData,
        enable_animation: AnyAnimation<f64>,
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: bool,
        is_discrete: bool,
        text_direction: TextDirection,
        additional_active_track_height: f64,
    ) {
        let _ = additional_active_track_height;
        debug_assert!(slider_theme.disabled_active_track_color.is_some());
        debug_assert!(slider_theme.disabled_inactive_track_color.is_some());
        debug_assert!(slider_theme.active_track_color.is_some());
        debug_assert!(slider_theme.inactive_track_color.is_some());
        debug_assert!(slider_theme.range_thumb_shape.is_some());

        let Some(track_height) = slider_theme.track_height.filter(|height| *height > 0.0) else {
            return;
        };

        let active_paint = fill_paint(
            evaluate_color_tween(
                app,
                slider_theme.disabled_active_track_color,
                slider_theme.active_track_color,
                enable_animation,
            )
            .expect("the active track colors are set"),
        );
        let inactive_paint = fill_paint(
            evaluate_color_tween(
                app,
                slider_theme.disabled_inactive_track_color,
                slider_theme.inactive_track_color,
                enable_animation,
            )
            .expect("the inactive track colors are set"),
        );

        let track_rect = RangeSliderTrackShape::get_preferred_rect(
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

        let (left_thumb_offset, right_thumb_offset) = match text_direction {
            TextDirection::Ltr => (start_thumb_center, end_thumb_center),
            TextDirection::Rtl => (end_thumb_center, start_thumb_center),
        };

        let thumb_size = slider_theme
            .range_thumb_shape
            .as_ref()
            .expect("the slider theme has a range thumb shape")
            .get_preferred_size(is_enabled, is_discrete);
        let thumb_radius = thumb_size.width() / 2.0;
        debug_assert!(thumb_radius > 0.0);
        let track_gap = slider_theme
            .track_gap
            .expect("the slider theme has a track gap");

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
            left_thumb_offset.dx() - track_gap,
            track_rect.bottom,
            track_corner_radius,
            TRACK_INSIDE_CORNER_RADIUS,
            TRACK_INSIDE_CORNER_RADIUS,
            track_corner_radius,
        );

        let right_rrect = RRect::from_ltrb_and_corners(
            right_thumb_offset.dx() + track_gap,
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
        let draw_left_track = start_thumb_center.dx() > (left_rrect.left + (track_height / 2.0));
        let draw_right_track = end_thumb_center.dx() < (right_rrect.right - (track_height / 2.0));

        if draw_left_track {
            draw_rrect(canvas, left_rrect, &inactive_paint);
        }
        if draw_right_track {
            draw_rrect(canvas, right_rrect, &inactive_paint);
        }

        if left_thumb_offset.dx() + track_gap < right_thumb_offset.dx() - track_gap {
            draw_rrect(
                canvas,
                RRect::from_ltrbr(
                    left_thumb_offset.dx() + track_gap,
                    track_rect.top,
                    right_thumb_offset.dx() - track_gap,
                    track_rect.bottom,
                    TRACK_INSIDE_CORNER_RADIUS,
                ),
                &active_paint,
            );
        }

        canvas.restore();

        const STOP_INDICATOR_RADIUS: f64 = 2.0;
        let stop_indicator_trailing_space = track_height / 2.0;
        let start_stop_indicator_offset = Offset::new(
            track_rect.center_left().dx() + stop_indicator_trailing_space,
            track_rect.center().dy(),
        );
        let end_stop_indicator_offset = Offset::new(
            track_rect.center_right().dx() - stop_indicator_trailing_space,
            track_rect.center().dy(),
        );

        let show_start_stop_indicator = start_thumb_center.dx() > start_stop_indicator_offset.dx();
        if show_start_stop_indicator && !is_discrete {
            let stop_indicator_rect =
                Rect::from_circle(start_stop_indicator_offset, STOP_INDICATOR_RADIUS);
            canvas.draw_circle(
                stop_indicator_rect.center(),
                STOP_INDICATOR_RADIUS as f32,
                &active_paint,
            );
        }

        let show_end_stop_indicator = end_thumb_center.dx() < end_stop_indicator_offset.dx();
        if show_end_stop_indicator && !is_discrete {
            let stop_indicator_rect =
                Rect::from_circle(end_stop_indicator_offset, STOP_INDICATOR_RADIUS);
            canvas.draw_circle(
                stop_indicator_rect.center(),
                STOP_INDICATOR_RADIUS as f32,
                &active_paint,
            );
        }
    }
}

impl BaseRangeSliderTrackShape for GappedRangeSliderTrackShape {}

impl RangeSliderTrackShape for GappedRangeSliderTrackShape {
    fn get_preferred_rect(
        &self,
        app: &App,
        parent_box: AnyRenderBox,
        offset: Offset,
        slider_theme: &SliderThemeData,
        is_enabled: Option<bool>,
        is_discrete: Option<bool>,
    ) -> Rect {
        BaseRangeSliderTrackShape::get_preferred_rect(
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
        start_thumb_center: Offset,
        end_thumb_center: Offset,
        is_enabled: bool,
        is_discrete: bool,
        text_direction: TextDirection,
    ) {
        GappedRangeSliderTrackShape::paint(
            self,
            app,
            context,
            offset,
            parent_box,
            slider_theme,
            enable_animation,
            start_thumb_center,
            end_thumb_center,
            is_enabled,
            is_discrete,
            text_direction,
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
    fn eq_shape(&self, other: &dyn RangeSliderTrackShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The bar shape of `RangeSlider`'s thumbs.
///
/// When the range slider is enabled, the `ColorScheme::primary` color is used for the
/// thumb. When the slider is disabled, the `ColorScheme::on_surface` color with an
/// opacity of 0.38 is used for the thumb.
///
/// The thumb bar shape width is reduced when the thumb is pressed.
///
/// If `SliderThemeData::thumb_size` is null, then the thumb size is 4 pixels for the width
/// and 44 pixels for the height.
///
/// If `ThemeData::use_material3` is true and `RangeSlider::year2023` is false, then the
/// `RangeSlider` will use [`HandleRangeSliderThumbShape`] as the default track shape.
///
/// See also:
///
///  * `RangeSlider`, which includes thumbs defined by this shape.
///  * `SliderTheme`, which can be used to configure the thumbs shape of all
///    range sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HandleRangeSliderThumbShape;

impl HandleRangeSliderThumbShape {
    /// Create a range slider thumb that draws a bar.
    pub const fn new() -> HandleRangeSliderThumbShape {
        HandleRangeSliderThumbShape
    }
}

impl RangeSliderThumbShape for HandleRangeSliderThumbShape {
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
        _is_discrete: Option<bool>,
        _is_enabled: Option<bool>,
        _is_on_top: Option<bool>,
        _text_direction: Option<TextDirection>,
        slider_theme: &SliderThemeData,
        _thumb: Option<Thumb>,
        _is_pressed: Option<bool>,
    ) {
        debug_assert!(slider_theme.show_value_indicator.is_some());
        debug_assert!(slider_theme.overlapping_shape_stroke_color.is_some());
        debug_assert!(slider_theme.disabled_thumb_color.is_some());
        debug_assert!(slider_theme.thumb_color.is_some());
        debug_assert!(slider_theme.thumb_size.is_some());

        let color = evaluate_color_tween(
            app,
            slider_theme.disabled_thumb_color,
            slider_theme.thumb_color,
            enable_animation,
        )
        .expect("the thumb colors are set");

        let thumb_size = slider_theme
            .thumb_size
            .as_ref()
            .expect("the slider theme has a thumb size")
            .resolve(&WidgetStates::new())
            .expect("the thumb size resolves for no state"); // This is resolved in the paint method.
        let rrect = RRect::from_rect_and_radius(
            Rect::from_center(center, thumb_size.width(), thumb_size.height()),
            Radius::circular(thumb_size.shortest_side() / 2.0),
        );

        draw_rrect(context.canvas(), rrect, &fill_paint(color));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Value equality, as Dart's canonicalized `const` instances are identical.
    fn eq_shape(&self, other: &dyn RangeSliderThumbShape) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
}

/// The rounded rectangle shape of a `RangeSlider`'s value indicators.
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
/// If `ThemeData::use_material3` is true and `RangeSlider::year2023` is false, then the
/// `RangeSlider` will use [`RoundedRectRangeSliderValueIndicatorShape`] as the default value
/// indicators shape.
///
/// See also:
///
///  * `RangeSlider`, which includes value indicators defined by this shape.
///  * `SliderTheme`, which can be used to configure the range slider value indicators
///    of all range sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedRectRangeSliderValueIndicatorShape;

impl RoundedRectRangeSliderValueIndicatorShape {
    const PATH_PAINTER: RoundedRectSliderValueIndicatorPathPainter =
        RoundedRectSliderValueIndicatorPathPainter;

    /// Create range slider value indicators that resembles a rounded rectangle.
    pub const fn new() -> RoundedRectRangeSliderValueIndicatorShape {
        RoundedRectRangeSliderValueIndicatorShape
    }
}

impl RangeSliderValueIndicatorShape for RoundedRectRangeSliderValueIndicatorShape {
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
        debug_assert!(text_scale_factor.is_some());
        debug_assert!(size_with_overflow.is_some());
        debug_assert!(slider_theme.value_indicator_color.is_some());

        let scale = activation_animation.value(app);
        Self::PATH_PAINTER.paint(
            app,
            context,
            parent_box,
            center,
            scale,
            label_painter,
            text_scale_factor.expect("the range slider passes its text scale factor"),
            size_with_overflow.expect("the range slider passes its size with overflow"),
            slider_theme
                .value_indicator_color
                .expect("the slider theme has a value indicator color"),
            if is_on_top.expect("the range slider says which indicator is on top") {
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

/// The shape of a Material 3 `RangeSlider`'s value indicators.
///
/// If the `SliderThemeData::value_indicator_color` is null, then the shape uses the
/// `ColorScheme::primary` color to draw the value indicator.
///
/// If the `SliderThemeData::value_indicator_text_style` is null, then the indicator label
/// text style defaults to `TextTheme::label_medium` with the color set to
/// `ColorScheme::on_primary`. If the `ThemeData::use_material3` is set to false, then the
/// indicator label text style defaults to `TextTheme::body_large` with the color set to
/// `ColorScheme::on_inverse_surface`.
///
/// If the `SliderThemeData::value_indicator_stroke_color` is provided, then the value
/// indicator is drawn with a stroke border with the color provided.
///
/// See also:
///
///  * `RangeSlider`, which includes value indicators defined by this shape.
///  * `SliderTheme`, which can be used to configure the range slider value indicators
///    of all range sliders in a widget subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DropRangeSliderValueIndicatorShape;

impl DropRangeSliderValueIndicatorShape {
    const PATH_PAINTER: DropSliderValueIndicatorPathPainter = DropSliderValueIndicatorPathPainter;

    /// Create a range slider value indicator that resembles a drop shape.
    pub const fn new() -> DropRangeSliderValueIndicatorShape {
        DropRangeSliderValueIndicatorShape
    }
}

impl RangeSliderValueIndicatorShape for DropRangeSliderValueIndicatorShape {
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
            context,
            parent_box,
            center,
            scale,
            label_painter,
            text_scale_factor.expect("the range slider passes its text scale factor"),
            size_with_overflow.expect("the range slider passes its size with overflow"),
            slider_theme
                .value_indicator_color
                .expect("the slider theme has a value indicator color"),
            if is_on_top.expect("the range slider says which indicator is on top") {
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

/// Dart's private `_RoundedRectSliderValueIndicatorPathPainter`, which this file repeats
/// from `slider_parts.dart`.
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
        let overflow_left = (rectangle_width / 2.0 - global_center.dx() + EDGE_PADDING).max(0.0);
        let overflow_right = (rectangle_width / 2.0
            - (size_with_overflow.width() - global_center.dx() - EDGE_PADDING))
            .max(0.0);

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
        context: &mut PaintingContext,
        parent_box: AnyRenderBox,
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

        let fill_paint = fill_paint(background_paint_color);

        let fonts = PaintingBinding::instance(app).fonts(app);
        let canvas = context.canvas();
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
            draw_rrect(canvas, rrect, &stroke_paint(stroke_paint_color));
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
        label_painter.paint(app.get_mut(fonts), canvas, label_offset);
        canvas.restore();
    }
}

/// Dart's private `_DropSliderValueIndicatorPathPainter`, which this file repeats from
/// `slider_parts.dart`.
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
        let overflow_left = (rectangle_width / 2.0 - global_center.dx() + EDGE_PADDING).max(0.0);
        let overflow_right = (rectangle_width / 2.0
            - (size_with_overflow.width() - global_center.dx() - EDGE_PADDING))
            .max(0.0);

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
        .expect("both radii are given")
    }

    #[expect(clippy::too_many_arguments, reason = "Dart's parameters, in its order")]
    fn paint(
        &self,
        app: &mut App,
        context: &mut PaintingContext,
        parent_box: AnyRenderBox,
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

        let fill_paint = fill_paint(background_paint_color);

        let fonts = PaintingBinding::instance(app).fonts(app);
        let canvas = context.canvas();
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
        // Dart's `Path()..lineTo` starts at the origin; valo's `line_to` on an empty path would
        // start at its point.
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
            canvas.draw_path(
                &triangle_path,
                FillRule::NonZero,
                &stroke_paint(stroke_paint_color),
            );
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
        label_painter.paint(app.get_mut(fonts), canvas, label_offset);
        canvas.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_values_compare_by_their_start_and_end() {
        assert_eq!(RangeValues::new(0.25, 0.75), RangeValues::new(0.25, 0.75));
        assert_ne!(RangeValues::new(0.25, 0.75), RangeValues::new(0.25, 0.5));
        assert_ne!(RangeValues::new(0.25, 0.75), RangeValues::new(0.5, 0.75));
    }

    #[test]
    fn range_values_print_as_dart_to_string_does() {
        assert_eq!(
            format!("{:?}", RangeValues::new(0.0, 1.5)),
            "RangeValues(0.0, 1.5)"
        );
    }

    #[test]
    fn range_labels_compare_by_their_start_and_end() {
        assert_eq!(RangeLabels::new("a", "b"), RangeLabels::new("a", "b"));
        assert_ne!(RangeLabels::new("a", "b"), RangeLabels::new("a", "c"));
        assert_ne!(RangeLabels::new("a", "b"), RangeLabels::new("c", "b"));
    }

    #[test]
    fn range_labels_print_as_dart_to_string_does() {
        assert_eq!(
            format!("{:?}", RangeLabels::new("start", "end")),
            "RangeLabels(start, end)"
        );
    }

    /// slider_theme_test 'Track shape isRounded defaults', for the range tracks.
    #[test]
    fn track_shape_is_rounded_defaults() {
        assert!(!RangeSliderTrackShape::is_rounded(
            &RectangularRangeSliderTrackShape::new()
        ));
        assert!(RangeSliderTrackShape::is_rounded(
            &RoundedRectRangeSliderTrackShape::new()
        ));
        assert!(RangeSliderTrackShape::is_rounded(
            &GappedRangeSliderTrackShape::new()
        ));
    }

    #[test]
    fn the_round_thumb_is_sized_by_its_radius_for_its_enabled_state() {
        let shape = RoundRangeSliderThumbShape::new();
        assert_eq!(shape.get_preferred_size(true, false), Size::new(20.0, 20.0));
        assert_eq!(
            shape.get_preferred_size(false, false),
            Size::new(20.0, 20.0)
        );
        let shape = shape.enabled_thumb_radius(12.0).disabled_thumb_radius(6.0);
        assert_eq!(shape.get_preferred_size(true, true), Size::new(24.0, 24.0));
        assert_eq!(shape.get_preferred_size(false, true), Size::new(12.0, 12.0));
    }

    #[test]
    fn the_handle_thumb_is_four_by_forty_four() {
        assert_eq!(
            HandleRangeSliderThumbShape::new().get_preferred_size(true, false),
            Size::new(4.0, 44.0)
        );
    }

    #[test]
    fn the_round_tick_mark_defaults_to_a_quarter_of_the_track_height() {
        let theme = SliderThemeData::new().track_height(8.0);
        assert_eq!(
            RoundRangeSliderTickMarkShape::new().get_preferred_size(&theme, None),
            Size::new(4.0, 4.0)
        );
        assert_eq!(
            RoundRangeSliderTickMarkShape::new()
                .tick_mark_radius(5.0)
                .get_preferred_size(&theme, Some(true)),
            Size::new(10.0, 10.0)
        );
    }

    #[test]
    fn built_in_shapes_compare_by_value() {
        let a: RangeSliderThumbShapeRef = Rc::new(RoundRangeSliderThumbShape::new());
        let b: RangeSliderThumbShapeRef = Rc::new(RoundRangeSliderThumbShape::new());
        let c: RangeSliderThumbShapeRef = Rc::new(RoundRangeSliderThumbShape::new().elevation(2.0));
        assert!(*a == *b);
        assert!(*a != *c);
        let handle: RangeSliderThumbShapeRef = Rc::new(HandleRangeSliderThumbShape::new());
        assert!(*a != *handle);
    }
}
