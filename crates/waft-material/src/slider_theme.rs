//! Flutter counterpart: material_ui's `slider_theme.dart`.
//!
//! The shapes a [`SliderThemeData`] carries are trait objects (`slider_parts.rs`,
//! `slider_value_indicator_shape.rs`, `range_slider_parts.rs`); its `==` compares them with
//! their `eq_shape`, which is identity unless a shape overrides it, as the built-in ones do.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Color, Size, lerp_double};
use waft_foundation::App;
use waft_painting::{AnyColor, EdgeInsetsGeometry, TextStyle};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::colors::Colors;
use crate::range_slider_parts::{
    RangeSliderThumbShapeRef, RangeSliderTickMarkShapeRef, RangeSliderTrackShapeRef,
    RangeSliderValueIndicatorShapeRef, RangeThumbSelector, RoundRangeSliderThumbShape,
    RoundRangeSliderTickMarkShape, RoundedRectRangeSliderTrackShape,
};
use crate::slider::SliderInteraction;
use crate::slider_parts::{
    RoundSliderThumbShape, RoundSliderTickMarkShape, RoundedRectSliderTrackShape,
    SliderTickMarkShapeRef, SliderTrackShapeRef,
};
use crate::slider_value_indicator_shape::{
    PaddleRangeSliderValueIndicatorShape, PaddleSliderValueIndicatorShape, RoundSliderOverlayShape,
    SliderComponentShapeRef,
};
use crate::theme::Theme;

/// Applies a slider theme to descendant `Slider` widgets.
///
/// A slider theme describes the colors and shape choices of the slider
/// components.
///
/// Descendant widgets obtain the current theme's [`SliderThemeData`] object using
/// [`SliderTheme::of`]. When a widget uses [`SliderTheme::of`], it is automatically
/// rebuilt if the theme later changes.
///
/// The slider is as big as the largest of
/// the `SliderComponentShape::get_preferred_size` of the thumb shape,
/// the `SliderComponentShape::get_preferred_size` of the overlay shape,
/// and the `SliderTickMarkShape::get_preferred_size` of the tick mark shape.
///
/// See also:
///
///  * [`SliderThemeData`], which describes the actual configuration of a slider
///    theme.
///  * `SliderComponentShape`, which can be used to create custom shapes for
///    the `Slider`'s thumb, overlay, and value indicator and the
///    `RangeSlider`'s overlay.
///  * `SliderTrackShape`, which can be used to create custom shapes for the
///    `Slider`'s track.
///  * `SliderTickMarkShape`, which can be used to create custom shapes for the
///    `Slider`'s tick marks.
///  * `RangeSliderThumbShape`, which can be used to create custom shapes for
///    the `RangeSlider`'s thumb.
///  * `RangeSliderValueIndicatorShape`, which can be used to create custom
///    shapes for the `RangeSlider`'s value indicator.
///  * `RangeSliderTrackShape`, which can be used to create custom shapes for
///    the `RangeSlider`'s track.
///  * `RangeSliderTickMarkShape`, which can be used to create custom shapes for
///    the `RangeSlider`'s tick marks.
#[derive(Debug)]
pub struct SliderTheme {
    pub key: Option<KeyRef>,
    /// Specifies the color and shape values for descendant slider widgets.
    pub data: SliderThemeData,
    pub child: WidgetRef,
}

impl SliderTheme {
    /// Applies the given theme [`data`](Self::data) to `child`.
    pub fn new<K>(data: SliderThemeData, child: impl IntoWidget<K>) -> SliderTheme {
        SliderTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `SliderTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliderTheme {
        self.key = Some(key);
        self
    }

    /// Returns the data from the closest [`SliderTheme`] instance that encloses
    /// the given context.
    ///
    /// Defaults to the ambient `ThemeData::slider_theme` if there is no
    /// [`SliderTheme`] in the given build context.
    ///
    /// ```text
    /// SliderTheme::new(
    ///     SliderTheme::of(app, context).copy_with().active_track_color(Color::new(0xff804040)),
    ///     Slider::new(rocket_thrust, Some(on_changed)),
    /// )
    /// ```
    ///
    /// See also:
    ///
    ///  * [`SliderThemeData`], which describes the actual configuration of a slider
    ///    theme.
    pub fn of(app: &mut App, context: BuildContext) -> SliderThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<SliderTheme>(app) {
            Some(inherited_theme) => inherited_theme.data.clone(),
            None => Theme::of(app, context).slider_theme.clone(),
        }
    }
}

impl InheritedTheme for SliderTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        SliderTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for SliderTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &SliderTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Describes the conditions under which the value indicator on a `Slider`
/// will be shown. Used in `Slider::show_value_indicator` and
/// [`SliderThemeData::show_value_indicator`].
///
/// See also:
///
///  * `Slider`, a Material Design slider widget.
///  * [`SliderThemeData`], which describes the actual configuration of a slider
///    theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShowValueIndicator {
    /// The value indicator will only be shown while dragging for discrete sliders (sliders
    /// where `Slider::divisions` is non-null).
    OnlyForDiscrete,

    /// The value indicator will only be shown while dragging for continuous sliders (sliders
    /// where `Slider::divisions` is null).
    OnlyForContinuous,

    /// The value indicator is shown while dragging.
    #[deprecated(note = "Use ShowValueIndicator::OnDrag. \
                This feature was deprecated after v3.28.0-1.0.pre.")]
    Always,

    /// The value indicator is shown while dragging.
    OnDrag,

    /// The value indicator is always displayed as long as the slider has a
    /// `Slider::on_changed` callback.
    AlwaysVisible,

    /// The value indicator will never be shown.
    Never,
}

/// Identifier for a thumb.
///
/// There are 2 thumbs in a `RangeSlider`, [`start`](Thumb::Start) and [`end`](Thumb::End).
///
/// For `TextDirection::Ltr`, the [`start`](Thumb::Start) thumb is the left-most thumb and the
/// [`end`](Thumb::End) thumb is the right-most thumb. For `TextDirection::Rtl` the
/// [`start`](Thumb::Start) thumb is the right-most thumb, and the [`end`](Thumb::End) thumb is
/// the left-most thumb.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Thumb {
    /// Left-most thumb for `TextDirection::Ltr`, otherwise, right-most thumb.
    Start,

    /// Right-most thumb for `TextDirection::Ltr`, otherwise, left-most thumb.
    End,
}

/// Overrides the default values of visual properties for descendant
/// `Slider` widgets.
///
/// Descendant widgets obtain the current [`SliderThemeData`] object with
/// [`SliderTheme::of`]. Instances of [`SliderThemeData`] can
/// be customized with [`SliderThemeData::copy_with`].
///
/// Typically a [`SliderThemeData`] is specified as part of the overall
/// `Theme` with `ThemeData::slider_theme`.
///
/// This theme is for both the `Slider` and the `RangeSlider`. The properties
/// that are only for the `Slider` are: [`tick_mark_shape`](Self::tick_mark_shape),
/// [`thumb_shape`](Self::thumb_shape), [`track_shape`](Self::track_shape), and
/// [`value_indicator_shape`](Self::value_indicator_shape). The properties that are only for
/// the `RangeSlider` are [`range_tick_mark_shape`](Self::range_tick_mark_shape),
/// [`range_thumb_shape`](Self::range_thumb_shape), [`range_track_shape`](Self::range_track_shape),
/// [`range_value_indicator_shape`](Self::range_value_indicator_shape),
/// [`overlapping_shape_stroke_color`](Self::overlapping_shape_stroke_color),
/// [`min_thumb_separation`](Self::min_thumb_separation), and
/// [`thumb_selector`](Self::thumb_selector).
/// All other properties are used by both the `Slider` and the `RangeSlider`.
///
/// The parts of a slider are:
///
///  * The "thumb", which is a shape that slides horizontally when the user
///    drags it.
///  * The "track", which is the line that the slider thumb slides along.
///  * The "tick marks", which are regularly spaced marks that are drawn when
///    using discrete divisions.
///  * The "value indicator", which appears when the user is dragging the thumb
///    to indicate the value being selected.
///  * The "overlay", which appears around the thumb, and is shown when the
///    thumb is pressed, focused, or hovered. It is painted underneath the
///    thumb, so it must extend beyond the bounds of the thumb itself to
///    actually be visible.
///  * The "active" side of the slider is the side between the thumb and the
///    minimum value.
///  * The "inactive" side of the slider is the side between the thumb and the
///    maximum value.
///  * The `Slider` is disabled when it is not accepting user input. See
///    `Slider` for details on when this happens.
///
/// The thumb, track, tick marks, value indicator, and overlay can be customized
/// by creating subclasses of `SliderTrackShape`,
/// `SliderComponentShape`, and/or `SliderTickMarkShape`. See
/// `RoundSliderThumbShape`, `RectangularSliderTrackShape`,
/// `RoundSliderTickMarkShape`, `RectangularSliderValueIndicatorShape`, and
/// `RoundSliderOverlayShape` for examples.
///
/// The track painting can be skipped by specifying 0 for [`track_height`](Self::track_height).
/// The thumb painting can be skipped by specifying
/// `SliderComponentShape::no_thumb` for [`thumb_shape`](Self::thumb_shape).
/// The overlay painting can be skipped by specifying
/// `SliderComponentShape::no_overlay` for [`overlay_shape`](Self::overlay_shape).
/// The tick mark painting can be skipped by specifying
/// `SliderTickMarkShape::no_tick_mark` for [`tick_mark_shape`](Self::tick_mark_shape).
/// The value indicator painting can be skipped by specifying the
/// appropriate [`ShowValueIndicator`] for [`show_value_indicator`](Self::show_value_indicator).
///
/// [`overlay_color`](Self::overlay_color) is an [`AnyColor`]: the slider resolves it by state
/// when it is a `WidgetStateColor`, as Dart's `WidgetStateProperty.resolveAs` does.
///
/// See also:
///
///  * [`SliderTheme`] widget, which can override the slider theme of its
///    children.
///  * `Theme` widget, which performs a similar function to [`SliderTheme`],
///    but for overall themes.
///  * `ThemeData`, which has a default [`SliderThemeData`].
#[derive(Clone, Default)]
pub struct SliderThemeData {
    /// The height of the `Slider` track.
    pub track_height: Option<f64>,

    /// The color of the `Slider` track between the `Slider::min` position and the
    /// current thumb position.
    pub active_track_color: Option<Color>,

    /// The color of the `Slider` track between the current thumb position and the
    /// `Slider::max` position.
    pub inactive_track_color: Option<Color>,

    /// The color of the `Slider` track between the current thumb position and the
    /// `Slider::secondary_track_value` position.
    pub secondary_active_track_color: Option<Color>,

    /// The color of the `Slider` track between the `Slider::min` position and the
    /// current thumb position when the `Slider` is disabled.
    pub disabled_active_track_color: Option<Color>,

    /// The color of the `Slider` track between the current thumb position and the
    /// `Slider::secondary_track_value` position when the `Slider` is disabled.
    pub disabled_secondary_active_track_color: Option<Color>,

    /// The color of the `Slider` track between the current thumb position and the
    /// `Slider::max` position when the `Slider` is disabled.
    pub disabled_inactive_track_color: Option<Color>,

    /// The color of the track's tick marks that are drawn between the `Slider::min`
    /// position and the current thumb position.
    pub active_tick_mark_color: Option<Color>,

    /// The color of the track's tick marks that are drawn between the current
    /// thumb position and the `Slider::max` position.
    pub inactive_tick_mark_color: Option<Color>,

    /// The color of the track's tick marks that are drawn between the `Slider::min`
    /// position and the current thumb position when the `Slider` is disabled.
    pub disabled_active_tick_mark_color: Option<Color>,

    /// The color of the track's tick marks that are drawn between the current
    /// thumb position and the `Slider::max` position when the `Slider` is
    /// disabled.
    pub disabled_inactive_tick_mark_color: Option<Color>,

    /// The color given to the [`thumb_shape`](Self::thumb_shape) to draw itself with.
    pub thumb_color: Option<Color>,

    /// The color given to the perimeter of the top [`range_thumb_shape`](Self::range_thumb_shape)
    /// when the thumbs are overlapping and the top
    /// [`range_value_indicator_shape`](Self::range_value_indicator_shape) when the value
    /// indicators are overlapping.
    pub overlapping_shape_stroke_color: Option<Color>,

    /// The color given to the [`thumb_shape`](Self::thumb_shape) to draw itself with when the
    /// `Slider` is disabled.
    pub disabled_thumb_color: Option<Color>,

    /// The color of the overlay drawn around the slider thumb when it is
    /// pressed, focused, or hovered.
    ///
    /// This is typically a semi-transparent color.
    pub overlay_color: Option<AnyColor>,

    /// The color given to the [`value_indicator_shape`](Self::value_indicator_shape) to draw
    /// itself with.
    pub value_indicator_color: Option<Color>,

    /// The color given to the [`value_indicator_shape`](Self::value_indicator_shape) stroke.
    pub value_indicator_stroke_color: Option<Color>,

    /// The shape that will be used to draw the `Slider`'s overlay.
    ///
    /// Both the [`overlay_color`](Self::overlay_color) and a non default
    /// [`overlay_shape`](Self::overlay_shape) may be specified. The default
    /// [`overlay_shape`](Self::overlay_shape) refers to the [`overlay_color`](Self::overlay_color).
    ///
    /// The default value is `RoundSliderOverlayShape`.
    pub overlay_shape: Option<SliderComponentShapeRef>,

    /// The shape that will be used to draw the `Slider`'s tick marks.
    ///
    /// The `SliderTickMarkShape::get_preferred_size` is used to help determine the
    /// location of each tick mark on the track. The slider's minimum size will
    /// be at least this big.
    ///
    /// The default value is `RoundSliderTickMarkShape`.
    ///
    /// See also:
    ///
    ///  * `RoundRangeSliderTickMarkShape`, which is the default tick mark
    ///    shape for the range slider.
    pub tick_mark_shape: Option<SliderTickMarkShapeRef>,

    /// The shape that will be used to draw the `Slider`'s thumb.
    ///
    /// The default value is `RoundSliderThumbShape`.
    ///
    /// See also:
    ///
    ///  * `RoundRangeSliderThumbShape`, which is the default thumb shape for
    ///    the `RangeSlider`.
    pub thumb_shape: Option<SliderComponentShapeRef>,

    /// The shape that will be used to draw the `Slider`'s track.
    ///
    /// The `SliderTrackShape::get_preferred_rect` method is used to map
    /// slider-relative gesture coordinates to the correct thumb position on the
    /// track. It is also used to horizontally position tick marks, when the
    /// slider is discrete.
    ///
    /// The default value is `RoundedRectSliderTrackShape`.
    ///
    /// See also:
    ///
    ///  * `RoundedRectRangeSliderTrackShape`, which is the default track
    ///    shape for the `RangeSlider`.
    pub track_shape: Option<SliderTrackShapeRef>,

    /// The shape that will be used to draw the `Slider`'s value
    /// indicator.
    ///
    /// The default value is `PaddleSliderValueIndicatorShape`.
    ///
    /// See also:
    ///
    ///  * `PaddleRangeSliderValueIndicatorShape`, which is the default value
    ///    indicator shape for the `RangeSlider`.
    pub value_indicator_shape: Option<SliderComponentShapeRef>,

    /// The shape that will be used to draw the `RangeSlider`'s tick marks.
    ///
    /// The `RangeSliderTickMarkShape::get_preferred_size` is used to help determine
    /// the location of each tick mark on the track. The slider's minimum size
    /// will be at least this big.
    ///
    /// The default value is `RoundRangeSliderTickMarkShape`.
    ///
    /// See also:
    ///
    ///  * `RoundSliderTickMarkShape`, which is the default tick mark shape
    ///    for the `Slider`.
    pub range_tick_mark_shape: Option<RangeSliderTickMarkShapeRef>,

    /// The shape that will be used for the `RangeSlider`'s thumbs.
    ///
    /// By default the same shape is used for both thumbs, but strokes the top
    /// thumb when it overlaps the bottom thumb. The top thumb is always the last
    /// selected thumb.
    ///
    /// The default value is `RoundRangeSliderThumbShape`.
    ///
    /// See also:
    ///
    ///  * `RoundSliderThumbShape`, which is the default thumb shape for the
    ///    `Slider`.
    pub range_thumb_shape: Option<RangeSliderThumbShapeRef>,

    /// The shape that will be used to draw the `RangeSlider`'s track.
    ///
    /// The `SliderTrackShape::get_preferred_rect` method is used to map
    /// slider-relative gesture coordinates to the correct thumb position on the
    /// track. It is also used to horizontally position the tick marks, when the
    /// slider is discrete.
    ///
    /// The default value is `RoundedRectRangeSliderTrackShape`.
    ///
    /// See also:
    ///
    ///  * `RoundedRectSliderTrackShape`, which is the default track
    ///    shape for the `Slider`.
    pub range_track_shape: Option<RangeSliderTrackShapeRef>,

    /// The shape that will be used for the `RangeSlider`'s value indicators.
    ///
    /// The default shape uses the same value indicator for each thumb, but
    /// strokes the top value indicator when it overlaps the bottom value
    /// indicator. The top indicator corresponds to the top thumb, which is always
    /// the most recently selected thumb.
    ///
    /// The default value is `PaddleRangeSliderValueIndicatorShape`.
    ///
    /// See also:
    ///
    ///  * `PaddleSliderValueIndicatorShape`, which is the default value
    ///    indicator shape for the `Slider`.
    pub range_value_indicator_shape: Option<RangeSliderValueIndicatorShapeRef>,

    /// Whether the value indicator should be shown for different types of
    /// sliders.
    ///
    /// By default, [`show_value_indicator`](Self::show_value_indicator) is set to
    /// [`ShowValueIndicator::OnlyForDiscrete`]. The value indicator is only shown
    /// when the thumb is being touched.
    pub show_value_indicator: Option<ShowValueIndicator>,

    /// The text style for the text on the value indicator.
    pub value_indicator_text_style: Option<TextStyle>,

    /// Limits the thumb's separation distance.
    ///
    /// Use this only if you want to control the visual appearance of the thumbs
    /// in terms of a logical pixel value. This can be done when you want a
    /// specific look for thumbs when they are close together. To limit with the
    /// real values, rather than logical pixels, the values can be restricted by
    /// the parent.
    pub min_thumb_separation: Option<f64>,

    /// Determines which thumb should be selected when the slider is interacted
    /// with.
    ///
    /// If null, the default thumb selector finds the closest thumb, excluding
    /// taps that are between the thumbs and not within any one touch target.
    /// When the selection is within the touch target bounds of both thumbs, no
    /// thumb is selected until the selection is moved.
    ///
    /// Override this for custom thumb selection.
    pub thumb_selector: Option<RangeThumbSelector>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If specified, overrides the default value of `Slider::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// Allowed way for the user to interact with the `Slider`.
    ///
    /// If specified, overrides the default value of `Slider::allowed_interaction`.
    pub allowed_interaction: Option<SliderInteraction>,

    /// Determines the padding around the `Slider`.
    ///
    /// If specified, this padding overrides the default vertical padding of
    /// the `Slider`, defaults to the height of the overlay shape, and the
    /// horizontal padding, defaults to the width of the thumb shape or
    /// overlay shape, whichever is larger.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The size of the `HandleThumbShape` thumb.
    ///
    /// If [`thumb_shape`](Self::thumb_shape) is `HandleThumbShape`, this property is used to
    /// set the size of the thumb. Otherwise, the default thumb size is 4 pixels for the
    /// width and 44 pixels for the height.
    pub thumb_size: Option<WidgetStatePropertyRef<Option<Size>>>,

    /// The size of the gap between the active and inactive tracks of the
    /// `GappedSliderTrackShape`.
    ///
    /// If [`track_shape`](Self::track_shape) is `GappedSliderTrackShape`, this property
    /// is used to set the gap between the active and inactive tracks. Otherwise,
    /// the default gap size is 6.0 pixels.
    ///
    /// The Slider defaults to `GappedSliderTrackShape` when the track shape is
    /// not specified, and the [`track_gap`](Self::track_gap) can be used to adjust the gap
    /// size.
    ///
    /// If `Slider::year2023` is true or `ThemeData::use_material3` is false, then
    /// the Slider track shape defaults to `RoundedRectSliderTrackShape` and the
    /// [`track_gap`](Self::track_gap) is ignored. In this case, set the track shape to
    /// `GappedSliderTrackShape` to use the [`track_gap`](Self::track_gap).
    ///
    /// Defaults to 6.0 pixels of gap between the active and inactive tracks.
    pub track_gap: Option<f64>,

    /// Overrides the default value of `Slider::year2023` and `RangeSlider::year2023`.
    ///
    /// When true, the `Slider` and `RangeSlider` will use the 2023 Material Design 3
    /// appearance. Defaults to true.
    ///
    /// If this is set to false, the `Slider` and `RangeSlider` will use the latest Material
    /// Design 3 appearance, which was introduced in December 2023.
    ///
    /// If `ThemeData::use_material3` is false, then this property is ignored.
    ///
    /// Deprecated in Flutter after v3.27.0-0.2.pre: set this flag to false to opt into the
    /// 2024 slider appearance. Its setter carries the deprecation.
    pub year2023: Option<bool>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `SliderThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait SliderThemeDataGetters for SliderThemeData, fields: slider_theme_data {
        track_height: Option<f64>,
        active_track_color: Option<Color>,
        inactive_track_color: Option<Color>,
        secondary_active_track_color: Option<Color>,
        disabled_active_track_color: Option<Color>,
        disabled_secondary_active_track_color: Option<Color>,
        disabled_inactive_track_color: Option<Color>,
        active_tick_mark_color: Option<Color>,
        inactive_tick_mark_color: Option<Color>,
        disabled_active_tick_mark_color: Option<Color>,
        disabled_inactive_tick_mark_color: Option<Color>,
        thumb_color: Option<Color>,
        overlapping_shape_stroke_color: Option<Color>,
        disabled_thumb_color: Option<Color>,
        overlay_color: Option<AnyColor>,
        value_indicator_color: Option<Color>,
        value_indicator_stroke_color: Option<Color>,
        overlay_shape: Option<SliderComponentShapeRef>,
        tick_mark_shape: Option<SliderTickMarkShapeRef>,
        thumb_shape: Option<SliderComponentShapeRef>,
        track_shape: Option<SliderTrackShapeRef>,
        value_indicator_shape: Option<SliderComponentShapeRef>,
        range_tick_mark_shape: Option<RangeSliderTickMarkShapeRef>,
        range_thumb_shape: Option<RangeSliderThumbShapeRef>,
        range_track_shape: Option<RangeSliderTrackShapeRef>,
        range_value_indicator_shape: Option<RangeSliderValueIndicatorShapeRef>,
        show_value_indicator: Option<ShowValueIndicator>,
        value_indicator_text_style: Option<TextStyle>,
        min_thumb_separation: Option<f64>,
        thumb_selector: Option<RangeThumbSelector>,
        mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
        allowed_interaction: Option<SliderInteraction>,
        padding: Option<EdgeInsetsGeometry>,
        thumb_size: Option<WidgetStatePropertyRef<Option<Size>>>,
        track_gap: Option<f64>,
        year2023: Option<bool>,
    }
}

macro_rules! slider_theme_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `SliderThemeData(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> SliderThemeData {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl SliderThemeData {
    /// Create a [`SliderThemeData`] given a set of exact values.
    ///
    /// This will rarely be used directly. It is used by [`lerp`](Self::lerp) to
    /// create intermediate themes based on two themes.
    ///
    /// The simplest way to create a SliderThemeData is to use
    /// [`copy_with`](Self::copy_with) on the one you get from [`SliderTheme::of`], or create an
    /// entirely new one with [`SliderThemeData::from_primary_colors`].
    pub fn new() -> SliderThemeData {
        SliderThemeData::default()
    }

    /// Generates a SliderThemeData from three main colors.
    ///
    /// Usually these are the primary, dark and light colors from
    /// a `ThemeData`.
    ///
    /// The opacities of these colors will be overridden with the Material Design
    /// defaults when assigning them to the slider theme component colors.
    ///
    /// This is used to generate the default slider theme for a `ThemeData`.
    pub fn from_primary_colors(
        primary_color: Color,
        primary_color_dark: Color,
        primary_color_light: Color,
        value_indicator_text_style: TextStyle,
    ) -> SliderThemeData {
        // These are Material Design defaults, and are used to derive
        // component Colors (with opacity) from base colors.
        const ACTIVE_TRACK_ALPHA: i32 = 0xff;
        const INACTIVE_TRACK_ALPHA: i32 = 0x3d; // 24% opacity
        const SECONDARY_ACTIVE_TRACK_ALPHA: i32 = 0x8a; // 54% opacity
        const DISABLED_ACTIVE_TRACK_ALPHA: i32 = 0x52; // 32% opacity
        const DISABLED_INACTIVE_TRACK_ALPHA: i32 = 0x1f; // 12% opacity
        const DISABLED_SECONDARY_ACTIVE_TRACK_ALPHA: i32 = 0x1f; // 12% opacity
        const ACTIVE_TICK_MARK_ALPHA: i32 = 0x8a; // 54% opacity
        const INACTIVE_TICK_MARK_ALPHA: i32 = 0x8a; // 54% opacity
        const DISABLED_ACTIVE_TICK_MARK_ALPHA: i32 = 0x1f; // 12% opacity
        const DISABLED_INACTIVE_TICK_MARK_ALPHA: i32 = 0x1f; // 12% opacity
        const THUMB_ALPHA: i32 = 0xff;
        const DISABLED_THUMB_ALPHA: i32 = 0x52; // 32% opacity
        const OVERLAY_ALPHA: i32 = 0x1f; // 12% opacity
        const VALUE_INDICATOR_ALPHA: i32 = 0xff;

        SliderThemeData::new()
            .track_height(2.0)
            .active_track_color(primary_color.with_alpha(ACTIVE_TRACK_ALPHA))
            .inactive_track_color(primary_color.with_alpha(INACTIVE_TRACK_ALPHA))
            .secondary_active_track_color(primary_color.with_alpha(SECONDARY_ACTIVE_TRACK_ALPHA))
            .disabled_active_track_color(primary_color_dark.with_alpha(DISABLED_ACTIVE_TRACK_ALPHA))
            .disabled_inactive_track_color(
                primary_color_dark.with_alpha(DISABLED_INACTIVE_TRACK_ALPHA),
            )
            .disabled_secondary_active_track_color(
                primary_color_dark.with_alpha(DISABLED_SECONDARY_ACTIVE_TRACK_ALPHA),
            )
            .active_tick_mark_color(primary_color_light.with_alpha(ACTIVE_TICK_MARK_ALPHA))
            .inactive_tick_mark_color(primary_color.with_alpha(INACTIVE_TICK_MARK_ALPHA))
            .disabled_active_tick_mark_color(
                primary_color_light.with_alpha(DISABLED_ACTIVE_TICK_MARK_ALPHA),
            )
            .disabled_inactive_tick_mark_color(
                primary_color_dark.with_alpha(DISABLED_INACTIVE_TICK_MARK_ALPHA),
            )
            .thumb_color(primary_color.with_alpha(THUMB_ALPHA))
            .overlapping_shape_stroke_color(Colors::WHITE)
            .disabled_thumb_color(primary_color_dark.with_alpha(DISABLED_THUMB_ALPHA))
            .overlay_color(primary_color.with_alpha(OVERLAY_ALPHA))
            .value_indicator_color(primary_color.with_alpha(VALUE_INDICATOR_ALPHA))
            .value_indicator_stroke_color(primary_color.with_alpha(VALUE_INDICATOR_ALPHA))
            .overlay_shape(Rc::new(RoundSliderOverlayShape::new()))
            .tick_mark_shape(Rc::new(RoundSliderTickMarkShape::new()))
            .thumb_shape(Rc::new(RoundSliderThumbShape::new()))
            .track_shape(Rc::new(RoundedRectSliderTrackShape::new()))
            .value_indicator_shape(Rc::new(PaddleSliderValueIndicatorShape::new()))
            .range_tick_mark_shape(Rc::new(RoundRangeSliderTickMarkShape::new()))
            .range_thumb_shape(Rc::new(RoundRangeSliderThumbShape::new()))
            .range_track_shape(Rc::new(RoundedRectRangeSliderTrackShape::new()))
            .range_value_indicator_shape(Rc::new(PaddleRangeSliderValueIndicatorShape::new()))
            .value_indicator_text_style(value_indicator_text_style)
            .show_value_indicator(ShowValueIndicator::OnlyForDiscrete)
    }

    slider_theme_setters!(
        track_height: f64 => "trackHeight",
        active_track_color: Color => "activeTrackColor",
        inactive_track_color: Color => "inactiveTrackColor",
        secondary_active_track_color: Color => "secondaryActiveTrackColor",
        disabled_active_track_color: Color => "disabledActiveTrackColor",
        disabled_inactive_track_color: Color => "disabledInactiveTrackColor",
        disabled_secondary_active_track_color: Color => "disabledSecondaryActiveTrackColor",
        active_tick_mark_color: Color => "activeTickMarkColor",
        inactive_tick_mark_color: Color => "inactiveTickMarkColor",
        disabled_active_tick_mark_color: Color => "disabledActiveTickMarkColor",
        disabled_inactive_tick_mark_color: Color => "disabledInactiveTickMarkColor",
        thumb_color: Color => "thumbColor",
        overlapping_shape_stroke_color: Color => "overlappingShapeStrokeColor",
        disabled_thumb_color: Color => "disabledThumbColor",
    );

    /// Dart `SliderThemeData(overlayColor:)`.
    pub fn overlay_color(mut self, overlay_color: impl Into<AnyColor>) -> SliderThemeData {
        self.overlay_color = Some(overlay_color.into());
        self
    }

    slider_theme_setters!(
        value_indicator_color: Color => "valueIndicatorColor",
        value_indicator_stroke_color: Color => "valueIndicatorStrokeColor",
        overlay_shape: SliderComponentShapeRef => "overlayShape",
        tick_mark_shape: SliderTickMarkShapeRef => "tickMarkShape",
        thumb_shape: SliderComponentShapeRef => "thumbShape",
        track_shape: SliderTrackShapeRef => "trackShape",
        value_indicator_shape: SliderComponentShapeRef => "valueIndicatorShape",
        range_tick_mark_shape: RangeSliderTickMarkShapeRef => "rangeTickMarkShape",
        range_thumb_shape: RangeSliderThumbShapeRef => "rangeThumbShape",
        range_track_shape: RangeSliderTrackShapeRef => "rangeTrackShape",
        range_value_indicator_shape: RangeSliderValueIndicatorShapeRef => "rangeValueIndicatorShape",
        show_value_indicator: ShowValueIndicator => "showValueIndicator",
        value_indicator_text_style: TextStyle => "valueIndicatorTextStyle",
        min_thumb_separation: f64 => "minThumbSeparation",
        thumb_selector: RangeThumbSelector => "thumbSelector",
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>> => "mouseCursor",
        allowed_interaction: SliderInteraction => "allowedInteraction",
        thumb_size: WidgetStatePropertyRef<Option<Size>> => "thumbSize",
        track_gap: f64 => "trackGap",
    );

    /// Dart `SliderThemeData(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> SliderThemeData {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `SliderThemeData(year2023:)`.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 slider appearance. \
                Defaults to true. In the future, this flag will default to false. Use \
                SliderThemeData to customize individual properties. This feature was \
                deprecated after v3.27.0-0.2.pre."
    )]
    pub fn year2023(mut self, year2023: bool) -> SliderThemeData {
        self.year2023 = Some(year2023);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().track_height(4.0)`).
    pub fn copy_with(&self) -> SliderThemeData {
        self.clone()
    }

    /// Linearly interpolate between two slider themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: &SliderThemeData, b: &SliderThemeData, t: f64) -> SliderThemeData {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        fn pick<T: Clone>(a: &Option<T>, b: &Option<T>, t: f64) -> Option<T> {
            if t < 0.5 { a.clone() } else { b.clone() }
        }
        SliderThemeData {
            track_height: lerp_double(a.track_height, b.track_height, t),
            active_track_color: Color::lerp(a.active_track_color, b.active_track_color, t),
            inactive_track_color: Color::lerp(a.inactive_track_color, b.inactive_track_color, t),
            secondary_active_track_color: Color::lerp(
                a.secondary_active_track_color,
                b.secondary_active_track_color,
                t,
            ),
            disabled_active_track_color: Color::lerp(
                a.disabled_active_track_color,
                b.disabled_active_track_color,
                t,
            ),
            disabled_inactive_track_color: Color::lerp(
                a.disabled_inactive_track_color,
                b.disabled_inactive_track_color,
                t,
            ),
            disabled_secondary_active_track_color: Color::lerp(
                a.disabled_secondary_active_track_color,
                b.disabled_secondary_active_track_color,
                t,
            ),
            active_tick_mark_color: Color::lerp(
                a.active_tick_mark_color,
                b.active_tick_mark_color,
                t,
            ),
            inactive_tick_mark_color: Color::lerp(
                a.inactive_tick_mark_color,
                b.inactive_tick_mark_color,
                t,
            ),
            disabled_active_tick_mark_color: Color::lerp(
                a.disabled_active_tick_mark_color,
                b.disabled_active_tick_mark_color,
                t,
            ),
            disabled_inactive_tick_mark_color: Color::lerp(
                a.disabled_inactive_tick_mark_color,
                b.disabled_inactive_tick_mark_color,
                t,
            ),
            thumb_color: Color::lerp(a.thumb_color, b.thumb_color, t),
            overlapping_shape_stroke_color: Color::lerp(
                a.overlapping_shape_stroke_color,
                b.overlapping_shape_stroke_color,
                t,
            ),
            disabled_thumb_color: Color::lerp(a.disabled_thumb_color, b.disabled_thumb_color, t),
            overlay_color: AnyColor::lerp(a.overlay_color.as_ref(), b.overlay_color.as_ref(), t)
                .map(AnyColor::new),
            value_indicator_color: Color::lerp(a.value_indicator_color, b.value_indicator_color, t),
            value_indicator_stroke_color: Color::lerp(
                a.value_indicator_stroke_color,
                b.value_indicator_stroke_color,
                t,
            ),
            overlay_shape: pick(&a.overlay_shape, &b.overlay_shape, t),
            tick_mark_shape: pick(&a.tick_mark_shape, &b.tick_mark_shape, t),
            thumb_shape: pick(&a.thumb_shape, &b.thumb_shape, t),
            track_shape: pick(&a.track_shape, &b.track_shape, t),
            value_indicator_shape: pick(&a.value_indicator_shape, &b.value_indicator_shape, t),
            range_tick_mark_shape: pick(&a.range_tick_mark_shape, &b.range_tick_mark_shape, t),
            range_thumb_shape: pick(&a.range_thumb_shape, &b.range_thumb_shape, t),
            range_track_shape: pick(&a.range_track_shape, &b.range_track_shape, t),
            range_value_indicator_shape: pick(
                &a.range_value_indicator_shape,
                &b.range_value_indicator_shape,
                t,
            ),
            show_value_indicator: pick(&a.show_value_indicator, &b.show_value_indicator, t),
            value_indicator_text_style: TextStyle::lerp(
                a.value_indicator_text_style.as_ref(),
                b.value_indicator_text_style.as_ref(),
                t,
            ),
            min_thumb_separation: lerp_double(a.min_thumb_separation, b.min_thumb_separation, t),
            thumb_selector: pick(&a.thumb_selector, &b.thumb_selector, t),
            mouse_cursor: pick(&a.mouse_cursor, &b.mouse_cursor, t),
            allowed_interaction: pick(&a.allowed_interaction, &b.allowed_interaction, t),
            padding: EdgeInsetsGeometry::lerp(a.padding, b.padding, t),
            thumb_size: <dyn WidgetStateProperty<Option<Size>>>::lerp_optional(
                a.thumb_size.clone(),
                b.thumb_size.clone(),
                t,
                Size::lerp,
            ),
            track_gap: lerp_double(a.track_gap, b.track_gap, t),
            year2023: pick(&a.year2023, &b.year2023, t),
        }
    }
}

/// Dart's `==` on a `RangeThumbSelector`: a closure, equal only to itself.
fn same_thumb_selector(a: &Option<RangeThumbSelector>, b: &Option<RangeThumbSelector>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl PartialEq for SliderThemeData {
    fn eq(&self, other: &SliderThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.track_height == self.track_height
            && other.active_track_color == self.active_track_color
            && other.inactive_track_color == self.inactive_track_color
            && other.secondary_active_track_color == self.secondary_active_track_color
            && other.disabled_active_track_color == self.disabled_active_track_color
            && other.disabled_inactive_track_color == self.disabled_inactive_track_color
            && other.disabled_secondary_active_track_color
                == self.disabled_secondary_active_track_color
            && other.active_tick_mark_color == self.active_tick_mark_color
            && other.inactive_tick_mark_color == self.inactive_tick_mark_color
            && other.disabled_active_tick_mark_color == self.disabled_active_tick_mark_color
            && other.disabled_inactive_tick_mark_color == self.disabled_inactive_tick_mark_color
            && other.thumb_color == self.thumb_color
            && other.overlapping_shape_stroke_color == self.overlapping_shape_stroke_color
            && other.disabled_thumb_color == self.disabled_thumb_color
            && other.overlay_color == self.overlay_color
            && other.value_indicator_color == self.value_indicator_color
            && other.value_indicator_stroke_color == self.value_indicator_stroke_color
            && other.overlay_shape == self.overlay_shape
            && other.tick_mark_shape == self.tick_mark_shape
            && other.thumb_shape == self.thumb_shape
            && other.track_shape == self.track_shape
            && other.value_indicator_shape == self.value_indicator_shape
            && other.range_tick_mark_shape == self.range_tick_mark_shape
            && other.range_thumb_shape == self.range_thumb_shape
            && other.range_track_shape == self.range_track_shape
            && other.range_value_indicator_shape == self.range_value_indicator_shape
            && other.show_value_indicator == self.show_value_indicator
            && other.value_indicator_text_style == self.value_indicator_text_style
            && other.min_thumb_separation == self.min_thumb_separation
            && same_thumb_selector(&other.thumb_selector, &self.thumb_selector)
            && other.mouse_cursor == self.mouse_cursor
            && other.allowed_interaction == self.allowed_interaction
            && other.padding == self.padding
            && other.thumb_size == self.thumb_size
            && other.track_gap == self.track_gap
            && other.year2023 == self.year2023
    }
}

impl Debug for SliderThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("SliderThemeData");
        macro_rules! property {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some(value) = &self.$field {
                        properties.field(stringify!($field), value);
                    }
                )*
            };
        }
        property!(
            track_height,
            active_track_color,
            inactive_track_color,
            secondary_active_track_color,
            disabled_active_track_color,
            disabled_inactive_track_color,
            disabled_secondary_active_track_color,
            active_tick_mark_color,
            inactive_tick_mark_color,
            disabled_active_tick_mark_color,
            disabled_inactive_tick_mark_color,
            thumb_color,
            overlapping_shape_stroke_color,
            disabled_thumb_color,
            overlay_color,
            value_indicator_color,
            value_indicator_stroke_color,
            overlay_shape,
            tick_mark_shape,
            thumb_shape,
            track_shape,
            value_indicator_shape,
            range_tick_mark_shape,
            range_thumb_shape,
            range_track_shape,
            range_value_indicator_shape,
            show_value_indicator,
            value_indicator_text_style,
            min_thumb_separation,
        );
        if self.thumb_selector.is_some() {
            properties.field("thumb_selector", &"RangeThumbSelector");
        }
        property!(
            mouse_cursor,
            allowed_interaction,
            padding,
            thumb_size,
            track_gap,
            year2023
        );
        properties.finish()
    }
}

/// A callback that formats a numeric value from a `Slider` or `RangeSlider` widget.
///
/// See also:
///
///  * `Slider::semantic_formatter_callback`, which shows an example use case.
///  * `RangeSlider::semantic_formatter_callback`, which shows an example use case.
pub type SemanticFormatterCallback = Rc<dyn Fn(f64) -> String>;

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use waft_embedder::{Offset, RRect, Radius, TargetPlatform, TextDirection};
    use waft_foundation::ValueChanged;
    use waft_rendering::{RenderBox, RenderObject};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{Center, SizedBox, StatefulBuilder, WidgetStatePropertyAll};

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::material::Material;
    use crate::range_slider_parts::RangeSliderTrackShape;
    use crate::range_slider_parts::RectangularRangeSliderTrackShape;
    use crate::scaffold::Scaffold;
    use crate::slider::Slider;
    use crate::slider::tests::{
        active_cursor, default_theme, is_cursor, noop, overlay_render, paints_circle,
        render_paints, slider_rect, slider_render,
    };
    use crate::slider_parts::{RectangularSliderTrackShape, SliderTrackShape};
    use crate::test_support::{PaintInspection, is_same_color_as, material_app};
    use crate::theme_data::ThemeData;
    use waft_rendering::testing::{PaintCall, assert_paints};

    // 'SliderThemeData copyWith, ==, hashCode basics'.
    #[test]
    fn slider_theme_data_copy_with_and_equality_basics() {
        assert_eq!(SliderThemeData::new(), SliderThemeData::new().copy_with());
    }

    // 'SliderThemeData lerp special cases'.
    #[test]
    fn slider_theme_data_lerp_special_cases() {
        // A theme with a `thumb_size`: lerping it makes a new property, which compares by
        // identity, so only Dart's `identical` shortcut hands back an equal theme.
        let data = SliderThemeData::new().thumb_size(Rc::new(WidgetStatePropertyAll::new(Some(
            Size::new(20.0, 20.0),
        ))));
        let lerp = SliderThemeData::lerp(&data, &data, 0.5);
        assert!(Rc::ptr_eq(
            lerp.thumb_size.as_ref().expect("lerped"),
            data.thumb_size.as_ref().expect("given")
        ));
        assert_eq!(lerp, data);
    }

    fn custom_colors() -> (Color, Color, Color, Color) {
        (
            Color::new(0xcafefeed),
            Color::new(0xdeadbeef),
            Color::new(0xdecaface),
            Color::new(0xfeedcafe),
        )
    }

    /// Dart's `ThemeData.fallback().textTheme.bodyLarge!.copyWith(color: color)`.
    fn fallback_body_large(tester: &mut WidgetTester, color: Color) -> TextStyle {
        let fallback = ThemeData::fallback(&tester.cell.borrow(), None);
        fallback
            .text_theme
            .body_large
            .expect("the fallback theme has a body large style")
            .copy_with()
            .color(color)
    }

    // 'SliderThemeData generates correct opacities for fromPrimaryColors'.
    #[test]
    fn from_primary_colors_generates_the_opacities() {
        test_widgets(|tester| {
            let (custom_color1, custom_color2, custom_color3, custom_color4) = custom_colors();
            let slider_theme = SliderThemeData::from_primary_colors(
                custom_color1,
                custom_color2,
                custom_color3,
                fallback_body_large(tester, custom_color4),
            );

            assert_eq!(
                slider_theme.active_track_color,
                Some(custom_color1.with_alpha(0xff))
            );
            assert_eq!(
                slider_theme.inactive_track_color,
                Some(custom_color1.with_alpha(0x3d))
            );
            assert_eq!(
                slider_theme.secondary_active_track_color,
                Some(custom_color1.with_alpha(0x8a))
            );
            assert_eq!(
                slider_theme.disabled_active_track_color,
                Some(custom_color2.with_alpha(0x52))
            );
            assert_eq!(
                slider_theme.disabled_inactive_track_color,
                Some(custom_color2.with_alpha(0x1f))
            );
            assert_eq!(
                slider_theme.disabled_secondary_active_track_color,
                Some(custom_color2.with_alpha(0x1f))
            );
            assert_eq!(
                slider_theme.active_tick_mark_color,
                Some(custom_color3.with_alpha(0x8a))
            );
            assert_eq!(
                slider_theme.inactive_tick_mark_color,
                Some(custom_color1.with_alpha(0x8a))
            );
            assert_eq!(
                slider_theme.disabled_active_tick_mark_color,
                Some(custom_color3.with_alpha(0x1f))
            );
            assert_eq!(
                slider_theme.disabled_inactive_tick_mark_color,
                Some(custom_color2.with_alpha(0x1f))
            );
            assert_eq!(
                slider_theme.thumb_color,
                Some(custom_color1.with_alpha(0xff))
            );
            assert_eq!(
                slider_theme.disabled_thumb_color,
                Some(custom_color2.with_alpha(0x52))
            );
            assert_eq!(
                slider_theme.overlay_color,
                Some(AnyColor::new(custom_color1.with_alpha(0x1f)))
            );
            assert_eq!(
                slider_theme.value_indicator_color,
                Some(custom_color1.with_alpha(0xff))
            );
            assert_eq!(
                slider_theme.value_indicator_stroke_color,
                Some(custom_color1.with_alpha(0xff))
            );
            assert_eq!(
                slider_theme
                    .value_indicator_text_style
                    .and_then(|style| style.color)
                    .map(|color| color.color()),
                Some(custom_color4)
            );
        });
    }

    // 'SliderThemeData generates correct shapes for fromPrimaryColors'.
    #[test]
    fn from_primary_colors_generates_the_shapes() {
        test_widgets(|tester| {
            use crate::range_slider_parts::{
                RoundRangeSliderThumbShape, RoundRangeSliderTickMarkShape,
                RoundedRectRangeSliderTrackShape,
            };
            use crate::slider_parts::{
                RoundSliderThumbShape, RoundSliderTickMarkShape, RoundedRectSliderTrackShape,
            };
            let (custom_color1, custom_color2, custom_color3, custom_color4) = custom_colors();
            let slider_theme = SliderThemeData::from_primary_colors(
                custom_color1,
                custom_color2,
                custom_color3,
                fallback_body_large(tester, custom_color4),
            );

            fn is<T: PartialEq + 'static>(any: &dyn std::any::Any, expected: T) -> bool {
                any.downcast_ref::<T>() == Some(&expected)
            }
            assert!(is(
                slider_theme.overlay_shape.unwrap().as_any(),
                RoundSliderOverlayShape::new()
            ));
            assert!(is(
                slider_theme.tick_mark_shape.unwrap().as_any(),
                RoundSliderTickMarkShape::new()
            ));
            assert!(is(
                slider_theme.thumb_shape.unwrap().as_any(),
                RoundSliderThumbShape::new()
            ));
            assert!(is(
                slider_theme.track_shape.unwrap().as_any(),
                RoundedRectSliderTrackShape::new()
            ));
            assert!(is(
                slider_theme.value_indicator_shape.unwrap().as_any(),
                PaddleSliderValueIndicatorShape::new()
            ));
            assert!(is(
                slider_theme.range_tick_mark_shape.unwrap().as_any(),
                RoundRangeSliderTickMarkShape::new()
            ));
            assert!(is(
                slider_theme.range_thumb_shape.unwrap().as_any(),
                RoundRangeSliderThumbShape::new()
            ));
            assert!(is(
                slider_theme.range_track_shape.unwrap().as_any(),
                RoundedRectRangeSliderTrackShape::new()
            ));
            assert!(is(
                slider_theme.range_value_indicator_shape.unwrap().as_any(),
                PaddleRangeSliderValueIndicatorShape::new()
            ));
        });
    }

    // 'SliderThemeData lerps correctly'.
    #[test]
    fn slider_theme_data_lerps() {
        test_widgets(|tester| {
            let slider_theme_black = SliderThemeData::from_primary_colors(
                Colors::BLACK,
                Colors::BLACK,
                Colors::BLACK,
                fallback_body_large(tester, Colors::BLACK),
            )
            .copy_with()
            .track_height(2.0);
            let slider_theme_white = SliderThemeData::from_primary_colors(
                Colors::WHITE,
                Colors::WHITE,
                Colors::WHITE,
                fallback_body_large(tester, Colors::WHITE),
            )
            .copy_with()
            .track_height(6.0);
            let lerp = SliderThemeData::lerp(&slider_theme_black, &slider_theme_white, 0.5);
            let middle_grey = Color::new(0xff7f7f7f);
            let same = |color: Option<Color>, alpha: i32| {
                is_same_color_as(color.expect("lerped"), middle_grey.with_alpha(alpha))
            };

            assert_eq!(lerp.track_height, Some(4.0));
            assert!(same(lerp.active_track_color, 0xff));
            assert!(same(lerp.inactive_track_color, 0x3d));
            assert!(same(lerp.secondary_active_track_color, 0x8a));
            assert!(same(lerp.disabled_active_track_color, 0x52));
            assert!(same(lerp.disabled_inactive_track_color, 0x1f));
            assert!(same(lerp.disabled_secondary_active_track_color, 0x1f));
            assert!(same(lerp.active_tick_mark_color, 0x8a));
            assert!(same(lerp.inactive_tick_mark_color, 0x8a));
            assert!(same(lerp.disabled_active_tick_mark_color, 0x1f));
            assert!(same(lerp.disabled_inactive_tick_mark_color, 0x1f));
            assert!(same(lerp.thumb_color, 0xff));
            assert!(same(lerp.disabled_thumb_color, 0x52));
            assert!(same(lerp.overlay_color.map(|color| color.color()), 0x1f));
            assert!(same(lerp.value_indicator_color, 0xff));
            assert!(same(lerp.value_indicator_stroke_color, 0xff));
            assert!(same(
                lerp.value_indicator_text_style
                    .and_then(|style| style.color)
                    .map(|color| color.color()),
                0xff
            ));
        });
    }

    // 'Track shape isRounded defaults'.
    #[test]
    fn track_shapes_report_whether_they_are_rounded() {
        use crate::range_slider_parts::RoundedRectRangeSliderTrackShape;
        use crate::slider_parts::RoundedRectSliderTrackShape;
        assert!(!SliderTrackShape::is_rounded(
            &RectangularSliderTrackShape::new()
        ));
        assert!(SliderTrackShape::is_rounded(
            &RoundedRectSliderTrackShape::new()
        ));
        assert!(!RangeSliderTrackShape::is_rounded(
            &RectangularRangeSliderTrackShape::new()
        ));
        assert!(RangeSliderTrackShape::is_rounded(
            &RoundedRectRangeSliderTrackShape::new()
        ));
    }

    /// Dart's `_buildApp`: a slider at `value` under `slider_theme` in a scaffold's body.
    fn build_app(
        tester: &WidgetTester,
        slider_theme: SliderThemeData,
        value: f64,
        secondary_track_value: Option<f64>,
        enabled: bool,
    ) -> WidgetRef {
        let mut slider = Slider::new(value, enabled.then(noop).flatten()).label(format!("{value}"));
        if let Some(secondary_track_value) = secondary_track_value {
            slider = slider.secondary_track_value(secondary_track_value);
        }
        material_app(
            default_theme(tester),
            Scaffold::new()
                .body(Center::new().child(SliderTheme::new(slider_theme, slider)))
                .into_widget(),
        )
    }

    // 'Slider parameters overrides theme properties'.
    #[test]
    fn slider_parameters_override_theme_properties() {
        test_widgets(|tester| {
            const ACTIVE_TRACK_COLOR: Color = Color::new(0xffff0001);
            const INACTIVE_TRACK_COLOR: Color = Color::new(0xffff0002);
            const SECONDARY_ACTIVE_TRACK_COLOR: Color = Color::new(0xffff0003);
            const THUMB_COLOR: Color = Color::new(0xffff0004);

            let theme = ThemeData::builder()
                .platform(TargetPlatform::Android)
                .primary_swatch(Colors::BLUE)
                .slider_theme(
                    SliderThemeData::new()
                        .active_track_color(Color::new(0xff000001))
                        .inactive_tick_mark_color(Color::new(0xff000002))
                        .secondary_active_track_color(Color::new(0xff000003))
                        .thumb_color(Color::new(0xff000004)),
                )
                .build(&tester.cell.borrow());
            let value = 0.45;
            tester.pump_widget(material_app(
                theme,
                waft_widgets::Directionality::new(
                    TextDirection::Ltr,
                    Material::new().child(
                        Center::new().child(
                            Slider::new(value, noop())
                                .active_color(ACTIVE_TRACK_COLOR)
                                .inactive_color(INACTIVE_TRACK_COLOR)
                                .secondary_active_color(SECONDARY_ACTIVE_TRACK_COLOR)
                                .thumb_color(THUMB_COLOR)
                                .secondary_track_value(0.75)
                                .label(format!("{value}")),
                        ),
                    ),
                )
                .into_widget(),
            ));

            // Test Slider parameters.
            assert!(render_paints(
                tester,
                slider_render(tester),
                &[
                    PaintCall::rrect(Some(INACTIVE_TRACK_COLOR)),
                    PaintCall::rrect(Some(ACTIVE_TRACK_COLOR)),
                    PaintCall::rrect(Some(SECONDARY_ACTIVE_TRACK_COLOR)),
                ],
            ));
            assert!(paints_circle(tester, slider_render(tester), THUMB_COLOR));
        });
    }

    /// Dart's 'Slider uses ThemeData slider theme if present' and 'Slider overrides ThemeData
    /// theme if SliderTheme present', which pump the same app.
    fn uses_the_slider_theme_given(tester: &mut WidgetTester) {
        let theme = ThemeData::builder()
            .platform(TargetPlatform::Android)
            .primary_swatch(Colors::RED)
            .build(&tester.cell.borrow());
        let slider_theme = theme.slider_theme.clone();
        let purple = Colors::PURPLE.primary();
        let custom_theme = slider_theme
            .copy_with()
            .active_track_color(purple)
            .inactive_track_color(purple.with_alpha(0x3d))
            .secondary_active_track_color(purple.with_alpha(0x8a));

        let app = build_app(tester, slider_theme, 0.5, Some(0.75), false);
        tester.pump_widget(app);

        let rrect = |color: Option<Color>| PaintCall::rrect(color);
        assert!(render_paints(
            tester,
            slider_render(tester),
            &[
                rrect(custom_theme.disabled_active_track_color),
                rrect(custom_theme.disabled_inactive_track_color),
                rrect(custom_theme.disabled_secondary_active_track_color),
            ],
        ));
    }

    // 'Slider uses ThemeData slider theme if present'.
    #[test]
    fn a_slider_uses_the_theme_data_slider_theme() {
        test_widgets(uses_the_slider_theme_given);
    }

    // 'Slider overrides ThemeData theme if SliderTheme present'.
    #[test]
    fn a_slider_theme_overrides_the_theme_data_slider_theme() {
        test_widgets(uses_the_slider_theme_given);
    }

    // 'The mouse cursor is themeable'.
    #[test]
    fn the_mouse_cursor_is_themeable() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let cursor: MouseCursorRef = SystemMouseCursors::TEXT.into();
            let app = build_app(
                tester,
                theme
                    .slider_theme
                    .copy_with()
                    .mouse_cursor(Rc::new(WidgetStatePropertyAll::new(Some(cursor)))),
                0.0,
                None,
                true,
            );
            tester.pump_widget(app);

            tester.pump_and_settle();
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, slider_rect(tester).center(), false);
            tester.pump_and_settle();
            assert!(is_cursor(&active_cursor(tester), SystemMouseCursors::TEXT));
        });
    }

    // 'SliderTheme.allowedInteraction is themeable'.
    #[test]
    fn the_allowed_interaction_is_themeable() {
        test_widgets(|tester| {
            let value = Rc::new(Cell::new(0.0));
            let theme = default_theme(tester);
            let build = |theme_null: bool, slider_null: bool| {
                let value = Rc::clone(&value);
                let mut slider_theme = theme.slider_theme.copy_with();
                if !theme_null {
                    slider_theme = slider_theme.allowed_interaction(SliderInteraction::SlideOnly);
                }
                material_app(
                    theme.clone(),
                    Scaffold::new()
                        .body(Center::new().child(SliderTheme::new(
                            slider_theme,
                            StatefulBuilder::new(move |_app, _context, set_state| {
                                let on_changed_value = Rc::clone(&value);
                                let mut slider = Slider::new(
                                    value.get(),
                                    Some(Rc::new(move |app: &mut App, new_value: f64| {
                                        let value = Rc::clone(&on_changed_value);
                                        set_state(app, &mut |_app| value.set(new_value));
                                    })),
                                );
                                if !slider_null {
                                    slider = slider.allowed_interaction(SliderInteraction::TapOnly);
                                }
                                slider.into_widget()
                            }),
                        )))
                        .into_widget(),
                )
            };
            let mut gesture = TestPointer::touch(1);
            let tap = |tester: &mut WidgetTester, gesture: &mut TestPointer| {
                value.set(0.0);
                tester.down(gesture, slider_rect(tester).center());
                tester.pump();
                let tapped = value.get();
                tester.up(gesture);
                tapped
            };
            let slide = |tester: &mut WidgetTester, gesture: &mut TestPointer| {
                value.set(0.0);
                tester.down(gesture, slider_rect(tester).center());
                tester.pump();
                let at = gesture.position + Offset::new(50.0, 0.0);
                tester.move_to(gesture, at, true);
                let slid = value.get();
                tester.up(gesture);
                slid
            };

            // when theme and parameter are specified, parameter is used [tapOnly].
            tester.pump_widget(build(false, false));
            // tap is allowed.
            assert_eq!(tap(tester, &mut gesture), 0.5); // changes
            // slide isn't allowed.
            assert_eq!(slide(tester, &mut gesture), 0.0); // no change

            // when only parameter is specified, parameter is used [tapOnly].
            tester.pump_widget(build(true, false));
            // tap is allowed.
            assert_eq!(tap(tester, &mut gesture), 0.5); // changes
            // slide isn't allowed.
            assert_eq!(slide(tester, &mut gesture), 0.0); // no change

            // when theme is specified but parameter is null, theme is used [slideOnly].
            tester.pump_widget(build(false, true));
            // tap isn't allowed.
            assert_eq!(tap(tester, &mut gesture), 0.0); // no change
            // slide isn't allowed.
            assert!(slide(tester, &mut gesture) > 0.0); // changes

            // when both theme and parameter are null, default is used [tapAndSlide].
            tester.pump_widget(build(true, true));
            // tap is allowed.
            assert_eq!(tap(tester, &mut gesture), 0.5);
            // slide is allowed.
            assert!(slide(tester, &mut gesture) > 0.0); // changes
        });
    }

    // 'Opt into 2024 Slider appearance with SliderThemeData.year2023'.
    #[test]
    #[expect(deprecated, reason = "Dart's case sets the deprecated `year2023`")]
    fn slider_theme_data_year2023_opts_into_the_2024_appearance() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .slider_theme(SliderThemeData::new().year2023(false))
                .build(&tester.cell.borrow());
            let color_scheme = theme.color_scheme.clone();
            let value = 0.45;
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(Center::new().child(Slider::new(value, noop())))
                    .into_widget(),
            ));

            // Test default track shape.
            let outer = Radius::circular(8.0);
            let inner = Radius::circular(2.0);
            assert!(render_paints(
                tester,
                slider_render(tester),
                &[
                    // Active track.
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            24.0, 292.0, 356.4, 308.0, outer, inner, inner, outer,
                        ),
                        Some(color_scheme.primary),
                    ),
                    // Inctive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrb_and_corners(
                            368.4, 292.0, 776.0, 308.0, inner, outer, outer, inner,
                        ),
                        Some(color_scheme.secondary_container()),
                    ),
                ],
            ));
        });
    }

    // 'Slider.year2023 overrides SliderThemeData.year2023'.
    #[test]
    #[expect(deprecated, reason = "Dart's case sets the deprecated `year2023`")]
    fn slider_year2023_overrides_the_theme_year2023() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .slider_theme(SliderThemeData::new().year2023(false))
                .build(&tester.cell.borrow());
            let color_scheme = theme.color_scheme.clone();
            let value = 0.45;
            let app_theme = default_theme(tester);
            tester.pump_widget(material_app(
                app_theme,
                Material::new()
                    .child(Center::new().child(crate::theme::Theme::new(
                        theme,
                        Slider::new(value, noop()).year2023(true),
                    )))
                    .into_widget(),
            ));

            // Test default track shape.
            let active_track_corner_radius = Radius::circular(3.0);
            let inactive_track_corner_radius = Radius::circular(2.0);
            assert!(render_paints(
                tester,
                slider_render(tester),
                &[
                    // Active track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(360.4, 298.0, 776.0, 302.0, inactive_track_corner_radius,),
                        Some(color_scheme.surface_container_highest()),
                    ),
                    // Inctive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(24.0, 297.0, 364.4, 303.0, active_track_corner_radius,),
                        Some(color_scheme.primary),
                    ),
                ],
            ));
        });
    }

    /// The colours Dart's 'Slider defaults' cases expect.
    struct DefaultColors {
        inactive_track: Color,
        active_track: Color,
        secondary_active_track: Color,
        disabled_active_track: Color,
        disabled_inactive_track: Color,
        disabled_secondary_active_track: Color,
        shadow: Color,
        thumb: Color,
        disabled_thumb: Color,
        active_tick_mark: Color,
        inactive_tick_mark: Color,
        disabled_active_tick_mark: Color,
        disabled_inactive_tick_mark: Color,
    }

    /// The body Dart's 'Slider defaults' (Material 3 and Material 2) share: `build_app` makes
    /// the app for `divisions` and `enabled`.
    fn expect_slider_defaults(
        tester: &mut WidgetTester,
        colors: &DefaultColors,
        build_app: &dyn Fn(Option<i32>, bool) -> WidgetRef,
    ) {
        const TRACK_HEIGHT: f64 = 4.0;
        tester.pump_widget(build_app(None, true));

        // Test default track height.
        let radius = Radius::circular(TRACK_HEIGHT / 2.0);
        let activated_radius = Radius::circular((TRACK_HEIGHT + 2.0) / 2.0);
        let render = slider_render(tester);
        assert!(render_paints(
            tester,
            render,
            &[
                // Inactive track.
                PaintCall::rrect_at(
                    RRect::from_ltrbr(360.4, 298.0, 776.0, 302.0, radius),
                    Some(colors.inactive_track),
                ),
                // Active track.
                PaintCall::rrect_at(
                    RRect::from_ltrbr(24.0, 297.0, 364.4, 303.0, activated_radius),
                    Some(colors.active_track),
                ),
            ],
        ));

        // Test default colors for enabled slider.
        let tracks = |inactive: Color, active: Color, secondary: Color| {
            [
                PaintCall::rrect(Some(inactive)),
                PaintCall::rrect(Some(active)),
                PaintCall::rrect(Some(secondary)),
            ]
        };
        assert!(render_paints(
            tester,
            render,
            &tracks(
                colors.inactive_track,
                colors.active_track,
                colors.secondary_active_track
            )
        ));
        assert!(render_paints(
            tester,
            render,
            &[PaintCall::shadow(Some(colors.shadow))]
        ));
        assert!(render_paints(
            tester,
            render,
            &[PaintCall::circle(Some(colors.thumb))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::circle(Some(colors.disabled_thumb))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.disabled_active_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.disabled_inactive_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(
                colors.disabled_secondary_active_track
            ))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::circle(Some(colors.active_tick_mark))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::circle(Some(colors.inactive_tick_mark))]
        ));

        // Test defaults colors for discrete slider.
        tester.pump_widget(build_app(Some(3), true));
        let render = slider_render(tester);
        assert!(render_paints(
            tester,
            render,
            &tracks(
                colors.inactive_track,
                colors.active_track,
                colors.secondary_active_track
            )
        ));
        assert!(render_paints(
            tester,
            render,
            &[
                PaintCall::circle(Some(colors.active_tick_mark)),
                PaintCall::circle(Some(colors.active_tick_mark)),
                PaintCall::circle(Some(colors.inactive_tick_mark)),
                PaintCall::circle(Some(colors.inactive_tick_mark)),
                PaintCall::shadow(Some(Colors::BLACK)),
                PaintCall::circle(Some(colors.thumb)),
            ],
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::circle(Some(colors.disabled_thumb))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.disabled_active_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.disabled_inactive_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(
                colors.disabled_secondary_active_track
            ))]
        ));

        // Test defaults colors for disabled slider.
        tester.pump_widget(build_app(None, false));
        tester.pump_and_settle();
        let render = slider_render(tester);
        assert!(render_paints(
            tester,
            render,
            &tracks(
                colors.disabled_inactive_track,
                colors.disabled_active_track,
                colors.disabled_secondary_active_track
            )
        ));
        assert!(render_paints(
            tester,
            render,
            &[
                PaintCall::shadow(Some(colors.shadow)),
                PaintCall::circle(Some(colors.disabled_thumb))
            ]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::circle(Some(colors.thumb))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.active_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.inactive_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.secondary_active_track))]
        ));

        // Test defaults colors for disabled discrete slider.
        tester.pump_widget(build_app(Some(3), false));
        let render = slider_render(tester);
        assert!(render_paints(
            tester,
            render,
            &[
                PaintCall::circle(Some(colors.disabled_active_tick_mark)),
                PaintCall::circle(Some(colors.disabled_active_tick_mark)),
                PaintCall::circle(Some(colors.disabled_inactive_tick_mark)),
                PaintCall::circle(Some(colors.disabled_inactive_tick_mark)),
                PaintCall::shadow(Some(colors.shadow)),
                PaintCall::circle(Some(colors.disabled_thumb)),
            ],
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::circle(Some(colors.thumb))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.active_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.inactive_track))]
        ));
        assert!(!render_paints(
            tester,
            render,
            &[PaintCall::rrect(Some(colors.secondary_active_track))]
        ));
    }

    /// Dart's `buildApp` of the 'Slider defaults' cases: a slider at 0.45 with a secondary
    /// track at 0.75 and a label.
    fn defaults_slider(divisions: Option<i32>, enabled: bool) -> Slider {
        let value = 0.45;
        let mut slider = Slider::new(value, enabled.then(noop).flatten())
            .secondary_track_value(0.75)
            .label(format!("{value}"));
        if let Some(divisions) = divisions {
            slider = slider.divisions(divisions);
        }
        slider
    }

    // 'Slider defaults'.
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn slider_defaults() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            let color_scheme = theme.color_scheme.clone();
            let colors = DefaultColors {
                active_track: color_scheme.primary,
                inactive_track: color_scheme.surface_container_highest(),
                secondary_active_track: color_scheme.primary.with_opacity(0.54),
                disabled_active_track: color_scheme.on_surface.with_opacity(0.38),
                disabled_inactive_track: color_scheme.on_surface.with_opacity(0.12),
                disabled_secondary_active_track: color_scheme.on_surface.with_opacity(0.12),
                shadow: color_scheme.shadow(),
                thumb: color_scheme.primary,
                disabled_thumb: Color::alpha_blend(
                    color_scheme.on_surface.with_opacity(0.38),
                    color_scheme.surface,
                ),
                active_tick_mark: color_scheme.on_primary.with_opacity(0.38),
                inactive_tick_mark: color_scheme.on_surface_variant().with_opacity(0.38),
                disabled_active_tick_mark: color_scheme.on_surface.with_opacity(0.38),
                disabled_inactive_tick_mark: color_scheme.on_surface.with_opacity(0.38),
            };
            let app_theme = default_theme(tester);
            let build_app = |divisions: Option<i32>, enabled: bool| {
                material_app(
                    app_theme.clone(),
                    waft_widgets::Directionality::new(
                        TextDirection::Ltr,
                        Material::new().child(Center::new().child(crate::theme::Theme::new(
                            theme.clone(),
                            defaults_slider(divisions, enabled),
                        ))),
                    )
                    .into_widget(),
                )
            };
            expect_slider_defaults(tester, &colors, &build_app);
        });
    }

    // 'Material 2' 'Slider defaults'.
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn material2_slider_defaults() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            let color_scheme = theme.color_scheme.clone();
            let colors = DefaultColors {
                active_track: color_scheme.primary,
                inactive_track: color_scheme.primary.with_opacity(0.24),
                secondary_active_track: color_scheme.primary.with_opacity(0.54),
                disabled_active_track: color_scheme.on_surface.with_opacity(0.32),
                disabled_inactive_track: color_scheme.on_surface.with_opacity(0.12),
                disabled_secondary_active_track: color_scheme.on_surface.with_opacity(0.12),
                shadow: color_scheme.shadow(),
                thumb: color_scheme.primary,
                disabled_thumb: Color::alpha_blend(
                    color_scheme.on_surface.with_opacity(0.38),
                    color_scheme.surface,
                ),
                active_tick_mark: color_scheme.on_primary.with_opacity(0.54),
                inactive_tick_mark: color_scheme.primary.with_opacity(0.54),
                disabled_active_tick_mark: color_scheme.on_primary.with_opacity(0.12),
                disabled_inactive_tick_mark: color_scheme.on_surface.with_opacity(0.12),
            };
            let value_indicator_color = Color::alpha_blend(
                color_scheme.on_surface.with_opacity(0.60),
                color_scheme.surface.with_opacity(0.90),
            );
            let value = Rc::new(Cell::new(0.45));
            let build_app = |divisions: Option<i32>, enabled: bool| {
                let value = Rc::clone(&value);
                let mut slider = defaults_slider(divisions, enabled);
                if enabled {
                    slider.on_changed = Some(Rc::new(move |_app: &mut App, d: f64| value.set(d)));
                }
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(Center::new().child(slider))
                        .into_widget(),
                )
            };
            expect_slider_defaults(tester, &colors, &build_app);

            // Test the default color for value indicator.
            tester.pump_widget(build_app(Some(3), true));
            let center = slider_rect(tester).center();
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, center);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();
            assert_eq!(value.get(), 2.0 / 3.0);
            let value_indicator_box = tester
                .first::<crate::slider::RenderValueIndicator>()
                .as_render_object(&tester.cell.borrow());
            assert!(render_paints(
                tester,
                value_indicator_box,
                &[
                    PaintCall::path(Some(value_indicator_color)),
                    PaintCall::Paragraph
                ],
            ));
            tester.up(&gesture);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();
        });
    }

    // 'Slider uses the right theme colors for the right components'.
    #[test]
    #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
    fn a_slider_uses_the_right_theme_colors_for_the_right_components() {
        test_widgets(|tester| {
            const CUSTOM_COLOR1: Color = Color::new(0xcafefeed);
            const CUSTOM_COLOR2: Color = Color::new(0xdeadbeef);
            const CUSTOM_COLOR3: Color = Color::new(0xdecaface);
            let theme = ThemeData::builder()
                .use_material3(false)
                .platform(TargetPlatform::Android)
                .primary_swatch(Colors::BLUE)
                .slider_theme(
                    SliderThemeData::new()
                        .disabled_thumb_color(Color::new(0xff000001))
                        .disabled_active_tick_mark_color(Color::new(0xff000002))
                        .disabled_active_track_color(Color::new(0xff000003))
                        .disabled_inactive_tick_mark_color(Color::new(0xff000004))
                        .disabled_inactive_track_color(Color::new(0xff000005))
                        .active_track_color(Color::new(0xff000006))
                        .active_tick_mark_color(Color::new(0xff000007))
                        .inactive_track_color(Color::new(0xff000008))
                        .inactive_tick_mark_color(Color::new(0xff000009))
                        .overlay_color(Color::new(0xff000010))
                        .thumb_color(Color::new(0xff000011))
                        .value_indicator_color(Color::new(0xff000012))
                        .disabled_secondary_active_track_color(Color::new(0xff000013))
                        .secondary_active_track_color(Color::new(0xff000014)),
                )
                .build(&tester.cell.borrow());
            let slider_theme = theme.slider_theme.clone();
            let value = Rc::new(Cell::new(0.45));
            #[derive(Default)]
            struct Args {
                active_color: Option<Color>,
                inactive_color: Option<Color>,
                secondary_active_color: Option<Color>,
                divisions: Option<i32>,
                disabled: bool,
            }
            let build_app = |args: Args| {
                let on_changed_value = Rc::clone(&value);
                let on_changed: Option<ValueChanged<f64>> = (!args.disabled).then(|| {
                    Rc::new(move |_app: &mut App, d: f64| on_changed_value.set(d))
                        as ValueChanged<f64>
                });
                let mut slider = Slider::new(value.get(), on_changed)
                    .secondary_track_value(0.75)
                    .label(format!("{}", value.get()));
                if let Some(divisions) = args.divisions {
                    slider = slider.divisions(divisions);
                }
                if let Some(active_color) = args.active_color {
                    slider = slider.active_color(active_color);
                }
                if let Some(inactive_color) = args.inactive_color {
                    slider = slider.inactive_color(inactive_color);
                }
                if let Some(secondary_active_color) = args.secondary_active_color {
                    slider = slider.secondary_active_color(secondary_active_color);
                }
                material_app(
                    theme.clone(),
                    waft_widgets::Directionality::new(
                        TextDirection::Ltr,
                        Material::new().child(
                            Center::new().child(crate::theme::Theme::new(theme.clone(), slider)),
                        ),
                    )
                    .into_widget(),
                )
            };
            let rrect = |color: Option<Color>| PaintCall::rrect(color);
            let circle = |color: Option<Color>| PaintCall::circle(color);
            let tracks =
                |inactive: Option<Color>, active: Option<Color>, secondary: Option<Color>| {
                    [rrect(inactive), rrect(active), rrect(secondary)]
                };
            let painted = |tester: &WidgetTester, calls: &[PaintCall]| {
                render_paints(tester, slider_render(tester), calls)
            };
            let not_painted = |tester: &WidgetTester, calls: &[PaintCall]| !painted(tester, calls);
            let no_disabled_tracks = |tester: &WidgetTester| {
                assert!(not_painted(
                    tester,
                    &[circle(slider_theme.disabled_thumb_color)]
                ));
                assert!(not_painted(
                    tester,
                    &[rrect(slider_theme.disabled_active_track_color)]
                ));
                assert!(not_painted(
                    tester,
                    &[rrect(slider_theme.disabled_inactive_track_color)]
                ));
                assert!(not_painted(
                    tester,
                    &[rrect(slider_theme.disabled_secondary_active_track_color)]
                ));
            };

            tester.pump_widget(build_app(Args::default()));

            // Check default theme for enabled widget.
            assert!(painted(
                tester,
                &tracks(
                    slider_theme.inactive_track_color,
                    slider_theme.active_track_color,
                    slider_theme.secondary_active_track_color
                )
            ));
            assert!(painted(
                tester,
                &[PaintCall::shadow(Some(Color::new(0xff000000)))]
            ));
            assert!(painted(tester, &[circle(slider_theme.thumb_color)]));
            no_disabled_tracks(tester);
            assert!(not_painted(
                tester,
                &[circle(slider_theme.active_tick_mark_color)]
            ));
            assert!(not_painted(
                tester,
                &[circle(slider_theme.inactive_tick_mark_color)]
            ));

            // Test setting only the activeColor.
            tester.pump_widget(build_app(Args {
                active_color: Some(CUSTOM_COLOR1),
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &tracks(
                    slider_theme.inactive_track_color,
                    Some(CUSTOM_COLOR1),
                    slider_theme.secondary_active_track_color
                )
            ));
            assert!(painted(tester, &[PaintCall::shadow(Some(Colors::BLACK))]));
            assert!(painted(tester, &[circle(Some(CUSTOM_COLOR1))]));
            assert!(not_painted(tester, &[circle(slider_theme.thumb_color)]));
            no_disabled_tracks(tester);

            // Test setting only the inactiveColor.
            tester.pump_widget(build_app(Args {
                inactive_color: Some(CUSTOM_COLOR1),
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &tracks(
                    Some(CUSTOM_COLOR1),
                    slider_theme.active_track_color,
                    slider_theme.secondary_active_track_color
                )
            ));
            assert!(painted(tester, &[PaintCall::shadow(Some(Colors::BLACK))]));
            assert!(painted(tester, &[circle(slider_theme.thumb_color)]));
            no_disabled_tracks(tester);

            // Test setting only the secondaryActiveColor.
            tester.pump_widget(build_app(Args {
                secondary_active_color: Some(CUSTOM_COLOR1),
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &tracks(
                    slider_theme.inactive_track_color,
                    slider_theme.active_track_color,
                    Some(CUSTOM_COLOR1)
                )
            ));
            assert!(painted(tester, &[PaintCall::shadow(Some(Colors::BLACK))]));
            assert!(painted(tester, &[circle(slider_theme.thumb_color)]));
            no_disabled_tracks(tester);

            // Test setting both activeColor, inactiveColor, and secondaryActiveColor.
            tester.pump_widget(build_app(Args {
                active_color: Some(CUSTOM_COLOR1),
                inactive_color: Some(CUSTOM_COLOR2),
                secondary_active_color: Some(CUSTOM_COLOR3),
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &tracks(
                    Some(CUSTOM_COLOR2),
                    Some(CUSTOM_COLOR1),
                    Some(CUSTOM_COLOR3)
                )
            ));
            assert!(painted(tester, &[PaintCall::shadow(Some(Colors::BLACK))]));
            assert!(painted(tester, &[circle(Some(CUSTOM_COLOR1))]));
            assert!(not_painted(tester, &[circle(slider_theme.thumb_color)]));
            no_disabled_tracks(tester);

            // Test colors for discrete slider.
            tester.pump_widget(build_app(Args {
                divisions: Some(3),
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &tracks(
                    slider_theme.inactive_track_color,
                    slider_theme.active_track_color,
                    slider_theme.secondary_active_track_color
                )
            ));
            assert!(painted(
                tester,
                &[
                    circle(slider_theme.active_tick_mark_color),
                    circle(slider_theme.active_tick_mark_color),
                    circle(slider_theme.inactive_tick_mark_color),
                    circle(slider_theme.inactive_tick_mark_color),
                    PaintCall::shadow(Some(Colors::BLACK)),
                    circle(slider_theme.thumb_color),
                ]
            ));
            no_disabled_tracks(tester);

            // Test colors for discrete slider with inactiveColor and activeColor set.
            tester.pump_widget(build_app(Args {
                active_color: Some(CUSTOM_COLOR1),
                inactive_color: Some(CUSTOM_COLOR2),
                secondary_active_color: Some(CUSTOM_COLOR3),
                divisions: Some(3),
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &tracks(
                    Some(CUSTOM_COLOR2),
                    Some(CUSTOM_COLOR1),
                    Some(CUSTOM_COLOR3)
                )
            ));
            assert!(painted(
                tester,
                &[
                    circle(Some(CUSTOM_COLOR2)),
                    circle(Some(CUSTOM_COLOR2)),
                    circle(Some(CUSTOM_COLOR1)),
                    circle(Some(CUSTOM_COLOR1)),
                    PaintCall::shadow(Some(Colors::BLACK)),
                    circle(Some(CUSTOM_COLOR1)),
                ]
            ));
            assert!(not_painted(tester, &[circle(slider_theme.thumb_color)]));
            no_disabled_tracks(tester);
            assert!(not_painted(
                tester,
                &[circle(slider_theme.active_tick_mark_color)]
            ));
            assert!(not_painted(
                tester,
                &[circle(slider_theme.inactive_tick_mark_color)]
            ));

            // Test default theme for disabled widget.
            tester.pump_widget(build_app(Args {
                disabled: true,
                ..Args::default()
            }));
            tester.pump_and_settle();
            assert!(painted(
                tester,
                &tracks(
                    slider_theme.disabled_inactive_track_color,
                    slider_theme.disabled_active_track_color,
                    slider_theme.disabled_secondary_active_track_color
                )
            ));
            assert!(painted(
                tester,
                &[
                    PaintCall::shadow(Some(Colors::BLACK)),
                    circle(slider_theme.disabled_thumb_color)
                ]
            ));
            assert!(not_painted(tester, &[circle(slider_theme.thumb_color)]));
            // These 2 colors are too close to distinguish.
            assert!(not_painted(
                tester,
                &[rrect(slider_theme.inactive_track_color)]
            ));
            assert!(not_painted(
                tester,
                &[rrect(slider_theme.secondary_active_track_color)]
            ));

            // Test default theme for disabled discrete widget.
            tester.pump_widget(build_app(Args {
                divisions: Some(3),
                disabled: true,
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &[
                    circle(slider_theme.disabled_active_tick_mark_color),
                    circle(slider_theme.disabled_active_tick_mark_color),
                    circle(slider_theme.disabled_inactive_tick_mark_color),
                    circle(slider_theme.disabled_inactive_tick_mark_color),
                    PaintCall::shadow(Some(Colors::BLACK)),
                    circle(slider_theme.disabled_thumb_color),
                ]
            ));
            assert!(not_painted(tester, &[circle(slider_theme.thumb_color)]));
            // These 2 colors are too close to distinguish.
            assert!(not_painted(
                tester,
                &[rrect(slider_theme.inactive_track_color)]
            ));
            assert!(not_painted(
                tester,
                &[rrect(slider_theme.secondary_active_track_color)]
            ));
            assert!(not_painted(
                tester,
                &[circle(slider_theme.active_tick_mark_color)]
            ));
            assert!(not_painted(
                tester,
                &[circle(slider_theme.inactive_tick_mark_color)]
            ));

            // Test setting the activeColor, inactiveColor and secondaryActiveColor for disabled
            // widget.
            tester.pump_widget(build_app(Args {
                active_color: Some(CUSTOM_COLOR1),
                inactive_color: Some(CUSTOM_COLOR2),
                secondary_active_color: Some(CUSTOM_COLOR3),
                disabled: true,
                ..Args::default()
            }));
            assert!(painted(
                tester,
                &tracks(
                    slider_theme.disabled_inactive_track_color,
                    slider_theme.disabled_active_track_color,
                    slider_theme.disabled_secondary_active_track_color
                )
            ));
            assert!(painted(
                tester,
                &[circle(slider_theme.disabled_thumb_color)]
            ));
            assert!(not_painted(tester, &[circle(slider_theme.thumb_color)]));
            // These colors are too close to distinguish.
            assert!(not_painted(
                tester,
                &[rrect(slider_theme.inactive_track_color)]
            ));
            assert!(not_painted(
                tester,
                &[rrect(slider_theme.secondary_active_track_color)]
            ));

            // Test that the default value indicator has the right colors.
            tester.pump_widget(build_app(Args {
                divisions: Some(3),
                ..Args::default()
            }));
            let center = slider_rect(tester).center();
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, center);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();
            assert_eq!(value.get(), 2.0 / 3.0);
            assert_paints(
                &tester.painted_ops_deep(overlay_render(tester)),
                &[
                    PaintCall::path(slider_theme.value_indicator_color),
                    PaintCall::Paragraph,
                ],
            );
            tester.up(&gesture);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();

            // Testing the custom colors are used for the indicator.
            tester.pump_widget(build_app(Args {
                active_color: Some(CUSTOM_COLOR1),
                inactive_color: Some(CUSTOM_COLOR2),
                divisions: Some(3),
                ..Args::default()
            }));
            let center = slider_rect(tester).center();
            let mut gesture = TestPointer::touch(2);
            tester.down(&mut gesture, center);
            // Wait for value indicator animation to finish.
            tester.pump_and_settle();
            assert_eq!(value.get(), 2.0 / 3.0);
            assert_paints(
                &tester.painted_ops_deep(overlay_render(tester)),
                &[
                    rrect(Some(Color::new(0xfffafafa))),
                    rrect(Some(CUSTOM_COLOR2)), // Inactive track
                    rrect(Some(CUSTOM_COLOR1)), // Active track
                    circle(Some(CUSTOM_COLOR1.with_opacity(0.12))), // overlay
                    circle(Some(CUSTOM_COLOR2)), // 1st tick mark
                    circle(Some(CUSTOM_COLOR2)), // 2nd tick mark
                    circle(Some(CUSTOM_COLOR2)), // 3rd tick mark
                    circle(Some(CUSTOM_COLOR1)), // 4th tick mark
                    PaintCall::shadow(Some(Colors::BLACK)),
                    circle(Some(CUSTOM_COLOR1)), // thumb
                    PaintCall::path(slider_theme.value_indicator_color), // indicator
                ],
            );
            tester.up(&gesture);
        });
    }

    // 'Slider can use theme overlay with material states'.
    #[test]
    fn a_slider_can_use_a_theme_overlay_with_material_states() {
        test_widgets(|tester| {
            let base_theme = ThemeData::builder()
                .platform(TargetPlatform::Android)
                .primary_swatch(Colors::BLUE)
                .build(&tester.cell.borrow());
            let brown = Colors::BROWN.get(&500).expect("a swatch shade");
            let slider_theme = base_theme.slider_theme.copy_with().overlay_color(
                waft_widgets::WidgetStateColor::resolve_with(move |states| {
                    if states.contains(&waft_widgets::WidgetState::Focused) {
                        return brown;
                    }

                    Colors::TRANSPARENT
                }),
            );
            let focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut())
                .into_erased::<waft_widgets::AnyFocusNode>();
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let value = Rc::new(Cell::new(0.5));
            let theme = ThemeData::builder()
                .slider_theme(slider_theme)
                .build(&tester.cell.borrow());
            let build_app = |enabled: bool| {
                let value = Rc::clone(&value);
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(Center::new().child(StatefulBuilder::new({
                            let focus_node = focus_node.clone();
                            move |_app, _context, set_state| {
                                let on_changed_value = Rc::clone(&value);
                                let on_changed: Option<ValueChanged<f64>> = enabled.then(|| {
                                    Rc::new(move |app: &mut App, new_value: f64| {
                                        let value = Rc::clone(&on_changed_value);
                                        set_state(app, &mut |_app| value.set(new_value));
                                    }) as ValueChanged<f64>
                                });
                                Slider::new(value.get(), on_changed)
                                    .autofocus(true)
                                    .focus_node(focus_node.clone())
                                    .into_widget()
                            }
                        })))
                        .into_widget(),
                )
            };
            tester.pump_widget(build_app(true));

            // Check that the overlay shows when focused.
            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert!(paints_circle(tester, slider_render(tester), brown));

            // Check that the overlay does not show when focused and disabled.
            tester.pump_widget(build_app(false));
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
            assert!(!paints_circle(tester, slider_render(tester), brown));
            focus_node.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// Dart's `RoundedRectSliderTrackShapeWithCustomAdditionalActiveTrackHeight`: a rounded
    /// rect track that paints with its own `additionalActiveTrackHeight`.
    #[derive(Debug)]
    struct RoundedRectSliderTrackShapeWithCustomAdditionalActiveTrackHeight {
        additional_active_track_height: f64,
    }

    impl SliderTrackShape for RoundedRectSliderTrackShapeWithCustomAdditionalActiveTrackHeight {
        fn get_preferred_rect(
            &self,
            app: &App,
            parent_box: waft_rendering::AnyRenderBox,
            offset: Offset,
            slider_theme: &SliderThemeData,
            is_enabled: Option<bool>,
            is_discrete: Option<bool>,
        ) -> waft_embedder::Rect {
            SliderTrackShape::get_preferred_rect(
                &RoundedRectSliderTrackShape::new(),
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
            context: &mut waft_rendering::PaintingContext,
            offset: Offset,
            parent_box: waft_rendering::AnyRenderBox,
            slider_theme: &SliderThemeData,
            enable_animation: waft_animation::AnyAnimation<f64>,
            thumb_center: Offset,
            secondary_offset: Option<Offset>,
            _is_enabled: Option<bool>,
            _is_discrete: Option<bool>,
            text_direction: TextDirection,
        ) {
            RoundedRectSliderTrackShape::new().paint(
                app,
                context,
                offset,
                parent_box,
                slider_theme,
                enable_animation,
                text_direction,
                thumb_center,
                secondary_offset,
                false,
                false,
                self.additional_active_track_height,
            );
        }

        fn is_rounded(&self) -> bool {
            true
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    // 'activeTrackRadius is taken into account when painting the border of the active track'.
    #[test]
    fn the_active_track_radius_counts_when_painting_the_active_track_border() {
        test_widgets(|tester| {
            let slider_theme = default_theme(tester)
                .slider_theme
                .copy_with()
                .track_shape(Rc::new(
                    RoundedRectSliderTrackShapeWithCustomAdditionalActiveTrackHeight {
                        additional_active_track_height: 10.0,
                    },
                ));
            let app = build_app(tester, slider_theme, 0.5, None, true);
            tester.pump_widget(app);
            tester.pump_and_settle();
            let center = slider_rect(tester).center();
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, center);
            assert!(render_paints(
                tester,
                slider_render(tester),
                &[
                    // Inactive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(398.0, 298.0, 776.0, 302.0, Radius::circular(2.0)),
                        None
                    ),
                    // Active track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(24.0, 293.0, 402.0, 307.0, Radius::circular(7.0)),
                        None
                    ),
                ]
            ));

            // Finish gesture to release resources.
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    /// The body Dart's two 'Default value indicator color' cases share: a discrete slider at
    /// 0.5 labelled '1' in a `Row`, pressed; `overlay` is what the `Overlay` paints, from the
    /// `Material`'s background through the indicator.
    fn default_value_indicator_color(
        tester: &mut WidgetTester,
        theme: fn(&WidgetTester) -> ThemeData,
        overlay: &[PaintCall],
    ) {
        let theme = theme(tester);
        tester.pump_widget(material_app(
            theme,
            waft_widgets::Directionality::new(
                TextDirection::Ltr,
                waft_widgets::MediaQuery::new(
                    waft_widgets::MediaQueryData::default()
                        .text_scaler(waft_painting::TextScaler::NO_SCALING),
                    Material::new().child(waft_widgets::Row::new().children(vec![
                        waft_widgets::Expanded::new(
                            Slider::new(0.5, noop()).label("1").divisions(3),
                        )
                        .into_widget(),
                    ])),
                )
                .into_widget(),
            )
            .into_widget(),
        ));

        let center = slider_rect(tester).center();
        let mut gesture = TestPointer::touch(1);
        tester.down(&mut gesture, center);
        // Wait for value indicator animation to finish.
        tester.pump_and_settle();
        assert_paints(&tester.painted_ops_deep(overlay_render(tester)), overlay);

        // Finish gesture to release resources.
        tester.up(&gesture);
        tester.pump_and_settle();
    }

    // 'Default value indicator color'.
    #[test]
    #[expect(deprecated, reason = "Dart's case reads the deprecated `value`")]
    fn the_default_value_indicator_color() {
        test_widgets(|tester| {
            let primary = ThemeData::builder()
                .platform(TargetPlatform::Android)
                .build(&tester.cell.borrow())
                .color_scheme
                .primary;
            default_value_indicator_color(
                tester,
                |tester| {
                    ThemeData::builder()
                        .platform(TargetPlatform::Android)
                        .build(&tester.cell.borrow())
                },
                &[
                    PaintCall::rrect(Some(Color::new(0xfffef7ff))),
                    PaintCall::rrect(Some(Color::new(0xffe6e0e9))),
                    PaintCall::rrect(Some(Color::new(0xff6750a4))),
                    PaintCall::path(Some(Color::new(primary.value()))),
                ],
            );
        });
    }

    // 'Material 2' 'Default value indicator color'.
    #[test]
    fn material2_the_default_value_indicator_color() {
        test_widgets(|tester| {
            default_value_indicator_color(
                tester,
                |tester| {
                    ThemeData::builder()
                        .use_material3(false)
                        .platform(TargetPlatform::Android)
                        .build(&tester.cell.borrow())
                },
                &[
                    PaintCall::rrect(Some(Color::new(0xfffafafa))),
                    PaintCall::rrect(Some(Color::new(0x3d2196f3))),
                    PaintCall::rrect(Some(Color::new(0xff2196f3))),
                    // Test that the value indicator text is painted with the correct color.
                    PaintCall::path(Some(Color::new(0xf55f5f5f))),
                ],
            );
        });
    }

    /// Dart's `_buildApp` with its `divisions`: a slider at `value` under `slider_theme` in a
    /// scaffold's body.
    fn build_discrete_app(
        tester: &WidgetTester,
        slider_theme: SliderThemeData,
        value: f64,
        divisions: i32,
    ) -> WidgetRef {
        let slider = Slider::new(value, noop())
            .label(format!("{value}"))
            .divisions(divisions);
        material_app(
            default_theme(tester),
            Scaffold::new()
                .body(Center::new().child(SliderTheme::new(slider_theme, slider)))
                .into_widget(),
        )
    }

    /// How many ops of `of`'s kind `render` records: Dart's `paintsExactlyCountTimes`.
    /// The render object that paints the slider's value indicator in the overlay.
    fn value_indicator_box(tester: &WidgetTester) -> waft_rendering::AnyRenderObject {
        tester
            .first::<crate::slider::RenderValueIndicator>()
            .as_render_object(&tester.cell.borrow())
    }

    fn count_ops(
        tester: &WidgetTester,
        render: waft_rendering::AnyRenderObject,
        of: fn(&waft_embedder::valo::Op) -> bool,
    ) -> usize {
        tester
            .painted_ops(render)
            .iter()
            .filter(|op| of(op))
            .count()
    }

    fn draw_path(op: &waft_embedder::valo::Op) -> bool {
        matches!(op, waft_embedder::valo::Op::DrawPath { path, .. }
            if crate::slider::tests::circle_of(path).is_none())
    }

    fn draw_circle(op: &waft_embedder::valo::Op) -> bool {
        matches!(op, waft_embedder::valo::Op::DrawPath { path, .. }
            if crate::slider::tests::circle_of(path).is_some())
    }

    fn draw_rect(op: &waft_embedder::valo::Op) -> bool {
        matches!(op, waft_embedder::valo::Op::DrawRect { .. })
    }

    /// The body of Dart's two 'skips all painting at zero scale' cases: a slider showing only
    /// its value indicator, which paints nothing on the press's first frame and one path the
    /// next.
    fn value_indicator_skips_painting_at_zero_scale(
        tester: &mut WidgetTester,
        configure: fn(SliderThemeData) -> SliderThemeData,
        check_material: bool,
    ) {
        use crate::slider_parts::SliderTickMarkShape;
        use crate::slider_value_indicator_shape::SliderComponentShape;
        // Pump a slider with just a value indicator.
        let slider_theme = configure(
            default_theme(tester)
                .slider_theme
                .copy_with()
                .track_height(0.0)
                .overlay_shape(<dyn SliderComponentShape>::no_overlay())
                .thumb_shape(<dyn SliderComponentShape>::no_thumb())
                .tick_mark_shape(<dyn SliderTickMarkShape>::no_tick_mark()),
        );
        let app = build_discrete_app(tester, slider_theme, 0.5, 4);
        tester.pump_widget(app);

        // Tap the center of the track to kick off the animation of the value indicator.
        let center = slider_rect(tester).center();
        let mut gesture = TestPointer::touch(1);
        tester.down(&mut gesture, center);

        // Nothing to paint at scale 0.
        tester.pump();
        if check_material {
            assert_eq!(count_ops(tester, slider_render(tester), draw_rect), 0);
            assert_eq!(count_ops(tester, slider_render(tester), draw_circle), 0);
        }
        assert_eq!(count_ops(tester, value_indicator_box(tester), draw_path), 0);

        // Painting a path for the value indicator.
        tester.pump_by(Duration::from_millis(16));
        assert_eq!(count_ops(tester, value_indicator_box(tester), draw_path), 1);

        tester.up(&gesture);
    }

    // 'PaddleSliderValueIndicatorShape skips all painting at zero scale'.
    #[test]
    #[expect(
        deprecated,
        reason = "Dart's case sets the deprecated `ShowValueIndicator.always`"
    )]
    fn the_paddle_value_indicator_skips_all_painting_at_zero_scale() {
        test_widgets(|tester| {
            value_indicator_skips_painting_at_zero_scale(
                tester,
                |theme| {
                    theme
                        .show_value_indicator(ShowValueIndicator::Always)
                        .value_indicator_shape(Rc::new(PaddleSliderValueIndicatorShape::new()))
                },
                true,
            );
        });
    }

    // 'Default slider value indicator shape skips all painting at zero scale'.
    #[test]
    fn the_default_value_indicator_shape_skips_all_painting_at_zero_scale() {
        test_widgets(|tester| {
            value_indicator_skips_painting_at_zero_scale(
                tester,
                |theme| theme.show_value_indicator(ShowValueIndicator::OnDrag),
                false,
            );
        });
    }

    /// Dart's `_buildApp(sliderTheme, value:)`, its rounded-rect track's painted rrects in
    /// order.
    fn rounded_rect_tracks(tester: &mut WidgetTester, value: f64, rrects: &[RRect]) -> bool {
        use crate::slider_parts::RoundedRectSliderTrackShape;
        let slider_theme =
            SliderThemeData::new().track_shape(Rc::new(RoundedRectSliderTrackShape::new()));
        let app = build_app(tester, slider_theme, value, None, true);
        tester.pump_widget(app);
        let calls: Vec<PaintCall> = rrects
            .iter()
            .map(|rrect| PaintCall::rrect_at(*rrect, None))
            .collect();
        render_paints(tester, slider_render(tester), &calls)
    }

    // 'RoundedRectSliderTrackShape' 'Only draw active track if thumb center is higher than
    // trackRect.left and track radius'.
    #[test]
    fn a_rounded_rect_track_draws_its_active_track_only_past_the_left_radius() {
        test_widgets(|tester| {
            assert!(rounded_rect_tracks(
                tester,
                0.0,
                // Inactive track.
                &[RRect::from_ltrbr(
                    22.0,
                    298.0,
                    776.0,
                    302.0,
                    Radius::circular(2.0)
                )]
            ));

            assert!(rounded_rect_tracks(
                tester,
                0.025,
                &[
                    // Inactive track.
                    RRect::from_ltrbr(40.8, 298.0, 776.0, 302.0, Radius::circular(2.0)),
                    // Active track.
                    RRect::from_ltrbr(24.0, 297.0, 44.8, 303.0, Radius::circular(3.0)),
                ]
            ));
        });
    }

    // 'RoundedRectSliderTrackShape' 'Only draw inactive track if thumb center is lower than
    // trackRect.right and track radius'.
    #[test]
    fn a_rounded_rect_track_draws_its_inactive_track_only_before_the_right_radius() {
        test_widgets(|tester| {
            assert!(rounded_rect_tracks(
                tester,
                1.0,
                // Active track.
                &[RRect::from_ltrbr(
                    24.0,
                    297.0,
                    778.0,
                    303.0,
                    Radius::circular(3.0)
                )]
            ));

            assert!(rounded_rect_tracks(
                tester,
                0.975,
                &[
                    // Inactive track.
                    RRect::from_ltrbr(755.2, 298.0, 776.0, 302.0, Radius::circular(2.0)),
                    // Active track.
                    RRect::from_ltrbr(24.0, 297.0, 759.2, 303.0, Radius::circular(3.0)),
                ]
            ));
        });
    }

    // 'RoundedRectSliderTrackShape' 'Active track is painted on top of the inactive track in
    // RTL'.
    #[test]
    fn the_active_track_is_painted_on_top_of_the_inactive_track_in_rtl() {
        test_widgets(|tester| {
            use crate::slider_parts::RoundedRectSliderTrackShape;
            // Regression test for https://github.com/flutter/flutter/issues/194278
            let theme = default_theme(tester);
            tester.pump_widget(material_app(
                theme,
                waft_widgets::Directionality::new(
                    TextDirection::Rtl,
                    Material::new().child(
                        Center::new().child(SliderTheme::new(
                            SliderThemeData::new()
                                .track_shape(Rc::new(RoundedRectSliderTrackShape::new())),
                            Slider::new(0.5, noop()),
                        )),
                    ),
                )
                .into_widget(),
            ));

            // Both track segments extend past the thumb center by half of the track
            // height, so the inactive track must be painted first for the rounded end
            // of the active track not to be covered by it.
            assert!(render_paints(
                tester,
                slider_render(tester),
                &[
                    // Inactive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(24.0, 298.0, 402.0, 302.0, Radius::circular(2.0)),
                        None
                    ),
                    // Active track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(398.0, 297.0, 776.0, 303.0, Radius::circular(3.0)),
                        None
                    ),
                ]
            ));
        });
    }

    // 'SliderThemeData.padding can override the default Slider padding'.
    #[test]
    fn slider_theme_data_padding_overrides_the_default_slider_padding() {
        test_widgets(|tester| {
            use waft_painting::{EdgeInsets, EdgeInsetsDirectional};
            let build_slider = |tester: &WidgetTester, padding: EdgeInsetsGeometry| {
                let theme = ThemeData::builder()
                    .slider_theme(SliderThemeData::new().padding(padding))
                    .build(&tester.cell.borrow());
                material_app(
                    theme,
                    Material::new()
                        .child(Center::new().child(
                            waft_widgets::IntrinsicHeight::new().child(Slider::new(0.5, noop())),
                        ))
                        .into_widget(),
                )
            };
            let slider_render_box_size = |tester: &WidgetTester| {
                tester
                    .first::<crate::slider::RenderSlider>()
                    .size(&tester.cell.borrow())
            };
            let tracks = |inactive: (f64, f64, f64, f64), active: (f64, f64, f64, f64)| {
                [
                    // Inactive track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(
                            inactive.0,
                            inactive.1,
                            inactive.2,
                            inactive.3,
                            Radius::circular(2.0),
                        ),
                        None,
                    ),
                    // Active track.
                    PaintCall::rrect_at(
                        RRect::from_ltrbr(
                            active.0,
                            active.1,
                            active.2,
                            active.3,
                            Radius::circular(3.0),
                        ),
                        None,
                    ),
                ]
            };

            // Test Slider height and tracks spacing with zero padding.
            let app = build_slider(tester, EdgeInsets::ZERO.into());
            tester.pump_widget(app);
            tester.pump_and_settle();

            // The height equals to the default thumb height.
            assert_eq!(slider_render_box_size(tester), Size::new(800.0, 20.0));
            assert!(render_paints(
                tester,
                slider_render(tester),
                &tracks((398.0, 8.0, 800.0, 12.0), (0.0, 7.0, 402.0, 13.0))
            ));

            // Test Slider height and tracks spacing with directional padding.
            const START_PADDING: f64 = 100.0;
            const END_PADDING: f64 = 20.0;
            let app = build_slider(
                tester,
                EdgeInsetsDirectional::only(START_PADDING, 0.0, END_PADDING, 0.0).into(),
            );
            tester.pump_widget(app);
            tester.pump_and_settle();

            assert_eq!(
                slider_render_box_size(tester),
                Size::new(800.0 - START_PADDING - END_PADDING, 20.0)
            );
            assert!(render_paints(
                tester,
                slider_render(tester),
                &tracks((338.0, 8.0, 680.0, 12.0), (0.0, 7.0, 342.0, 13.0))
            ));

            // Test Slider height and tracks spacing with top and bottom padding.
            const TOP_PADDING: f64 = 100.0;
            const BOTTOM_PADDING: f64 = 20.0;
            const TRACK_HEIGHT: f64 = 20.0;
            let app = build_slider(
                tester,
                EdgeInsetsDirectional::only(0.0, TOP_PADDING, 0.0, BOTTOM_PADDING).into(),
            );
            tester.pump_widget(app);
            tester.pump_and_settle();

            assert_eq!(
                crate::slider::tests::slider_size(tester),
                Size::new(800.0, TOP_PADDING + TRACK_HEIGHT + BOTTOM_PADDING)
            );
            assert_eq!(slider_render_box_size(tester), Size::new(800.0, 20.0));
            assert!(render_paints(
                tester,
                slider_render(tester),
                &tracks((398.0, 8.0, 800.0, 12.0), (0.0, 7.0, 402.0, 13.0))
            ));
        });
    }

    /// Whether `render` paints a circle centred at (`x`, `y`) in `color`: Dart's
    /// `paints..circle(x:, y:, color:)`.
    fn paints_circle_at(
        tester: &WidgetTester,
        render: waft_rendering::AnyRenderObject,
        x: f64,
        y: f64,
        color: Color,
    ) -> bool {
        tester.painted_ops(render).iter().any(|op| match op {
            waft_embedder::valo::Op::DrawPath { path, paint, .. }
                if waft_rendering::testing::colors_match(paint.color, color) =>
            {
                crate::slider::tests::circle_of(path).is_some_and(|circle| {
                    (circle.center().dx() - x).abs() < 1e-3
                        && (circle.center().dy() - y).abs() < 1e-3
                })
            }
            _ => false,
        })
    }

    // 'Slider with transparent track colors and custom track height can reach extreme ends'.
    #[test]
    fn a_slider_with_transparent_tall_tracks_reaches_its_extreme_ends() {
        test_widgets(|tester| {
            const SLIDER_PADDING: f64 = 24.0;
            let theme = ThemeData::builder()
                .slider_theme(
                    SliderThemeData::new()
                        .track_height(100.0)
                        .active_track_color(Colors::TRANSPARENT)
                        .inactive_track_color(Colors::TRANSPARENT),
                )
                .build(&tester.cell.borrow());
            let primary = theme.color_scheme.primary;
            let build_slider = |value: f64| {
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(
                            SizedBox::new()
                                .width(300.0)
                                .child(Slider::new(value, noop())),
                        )
                        .into_widget(),
                )
            };

            tester.pump_widget(build_slider(0.0));
            assert!(paints_circle_at(
                tester,
                slider_render(tester),
                SLIDER_PADDING,
                300.0,
                primary
            ));

            tester.pump_widget(build_slider(1.0));
            assert!(paints_circle_at(
                tester,
                slider_render(tester),
                800.0 - SLIDER_PADDING,
                300.0,
                primary
            ));
        });
    }
}
