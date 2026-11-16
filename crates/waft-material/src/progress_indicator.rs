//! Flutter counterpart: material_ui's `progress_indicator.dart`.
//!
//! Dart's abstract `ProgressIndicator` widget is the [`ProgressIndicatorData`] bag of its
//! fields, which every indicator carries under the field `progress_indicator`, and the
//! [`ProgressIndicator`] trait that hands it out. `RefreshProgressIndicator extends
//! CircularProgressIndicator` holds the circular indicator it extends, and its state shares
//! `_CircularProgressIndicatorState`'s bodies through [`CircularProgressIndicatorStateBase`],
//! overriding `build`, `_buildAnimation` and `_buildMaterialIndicator`.
#![allow(deprecated)]

use std::any::Any;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationRef, AnyAnimation, Cubic, Curve,
    Curves, Interval, SawTooth, TweenSequence, TweenSequenceItem, ValueCurveTween, ValueTween,
};
use waft_cupertino::CupertinoActivityIndicator;
use waft_embedder::StrokeCap;
use waft_embedder::{
    Canvas, Color, FillRule, Offset, Paint, PaintStyle, PathBuilder, Rect, SemanticsRole, Size,
    Stroke, TargetPlatform, TextDirection, clamp_double,
};
use waft_foundation::{App, Handle, Ref};
use waft_painting::{BorderRadiusGeometry, EdgeInsets, EdgeInsetsGeometry, draw_arc, draw_rrect};
use waft_rendering::{BoxConstraints, CustomPainter};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    AnimatedBuilder, BuildContext, Builder, ClipRRect, ConstrainedBox, CustomPaint, Directionality,
    IntoWidget, KeyRef, Opacity, Padding, Semantics, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, SizedBox, State, StateData, StatefulWidget, Transform,
    WidgetRef,
};

use crate::generated::progress_indicator_defaults_m3::{
    CircularProgressIndicatorDefaultsM3, LinearProgressIndicatorDefaultsM3,
};
use crate::material::{Material, MaterialType};
use crate::progress_indicator_theme::{
    ProgressIndicatorTheme, ProgressIndicatorThemeData, ProgressIndicatorThemeDataGetters,
};
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;

// This value is extracted from
// https://cs.android.com/android/platform/superproject/+/master:frameworks/base/core/res/res/anim/progress_indeterminate_material.xml;drc=9cb5b4c2d93acb9d6f5e14167e265c328c487d6b
const K_INDETERMINATE_LINEAR_DURATION: u64 = 1800;
// This value is extracted from
// https://cs.android.com/android/platform/superproject/+/master:frameworks/base/core/res/res/anim/progress_indeterminate_rotation_material.xml;drc=077b44912b879174cec48a25307f1c19b96c2a78
const K_INDETERMINATE_CIRCULAR_DURATION: u64 = 1333 * 2222;

// The progress value below which the track gap is scaled proportionally to
// prevent a track gap from appearing at 0% progress.
const K_TRACK_GAP_RAMP_DOWN_THRESHOLD: f64 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActivityIndicatorType {
    Material,
    Adaptive,
}

const K_VALUE_CONTROLLER_ASSERTION: &str = "A progress indicator cannot have both a value and a controller.\n\
The \"value\" property is for a determinate indicator with a specific progress, \
while the \"controller\" is for controlling the animation of an indeterminate indicator.\n\
To resolve this, provide only one of the two properties.";

/// The fields of Dart's abstract `ProgressIndicator`, a base class for Material Design
/// progress indicators, which every indicator carries under the field `progress_indicator`.
///
/// The [`value`](Self::value) argument can either be null for an indeterminate
/// progress indicator, or a non-null value between 0.0 and 1.0 for a
/// determinate progress indicator.
///
/// See also:
///
///  * <https://material.io/components/progress-indicators>
#[derive(Clone, Debug, Default)]
pub struct ProgressIndicatorData {
    pub key: Option<KeyRef>,

    /// If non-null, the value of this progress indicator.
    ///
    /// A value of 0.0 means no progress and 1.0 means that progress is complete.
    /// The value will be clamped to be in the range 0.0-1.0.
    ///
    /// If null, this progress indicator is indeterminate, which means the
    /// indicator displays a predetermined animation that does not indicate how
    /// much actual progress is being made.
    pub value: Option<f64>,

    /// The progress indicator's background color.
    ///
    /// It is up to the subclass to implement this in whatever way makes sense
    /// for the given use case. See the subclass documentation for details.
    pub background_color: Option<Color>,

    /// The progress indicator's color.
    ///
    /// This is only used if [`value_color`](Self::value_color) is null.
    /// If [`color`](Self::color) is also null, then the ambient
    /// `ProgressIndicatorThemeData::color` will be used. If that
    /// is null then the current theme's `ColorScheme::primary` will
    /// be used by default.
    pub color: Option<Color>,

    /// The progress indicator's color as an animated value.
    ///
    /// If null, the progress indicator is rendered with [`color`](Self::color). If that is
    /// null, then it will use the ambient `ProgressIndicatorThemeData::color`. If that
    /// is also null then it defaults to the current theme's `ColorScheme::primary`.
    pub value_color: Option<AnimationRef<Option<Color>>>,

    /// The `SemanticsProperties.label` for this progress indicator.
    ///
    /// This value indicates the purpose of the progress bar, and will be
    /// read out by screen readers to indicate the purpose of this progress
    /// indicator.
    pub semantics_label: Option<String>,

    /// The `SemanticsProperties.value` for this progress indicator.
    ///
    /// This will be used in conjunction with the [`semantics_label`](Self::semantics_label)
    /// by screen reading software to identify the widget, and is primarily
    /// intended for use with determinate progress indicators to announce
    /// how far along they are.
    ///
    /// For determinate progress indicators, this will be defaulted to
    /// [`value`](Self::value) expressed as a percentage, i.e. `0.1` will
    /// become '10%'.
    pub semantics_value: Option<String>,
}

impl ProgressIndicatorData {
    fn effective_value(&self) -> Option<f64> {
        self.value.map(|value| clamp_double(value, 0.0, 1.0))
    }

    /// Dart's `_buildSemanticsWrapper`.
    fn build_semantics_wrapper<K>(
        &self,
        _context: BuildContext,
        child: impl IntoWidget<K>,
    ) -> WidgetRef {
        let mut is_progress_bar = false;
        let mut expanded_semantics_value = self.semantics_value.clone();
        if let Some(effective_value) = self.effective_value() {
            expanded_semantics_value
                .get_or_insert_with(|| format!("{}", (effective_value * 100.0).round() as i64));
            is_progress_bar = true;
        }
        let mut semantics = Semantics::new().role(if is_progress_bar {
            SemanticsRole::ProgressBar
        } else {
            SemanticsRole::LoadingSpinner
        });
        if let Some(semantics_label) = &self.semantics_label {
            semantics = semantics.label(semantics_label.clone());
        }
        if is_progress_bar {
            semantics = semantics.min_value("0").max_value("100");
        }
        if let Some(expanded_semantics_value) = expanded_semantics_value {
            semantics = semantics.value(expanded_semantics_value);
        }
        semantics.child(child).into_widget()
    }

    /// Dart's `_getValueColor`. `color` is the widget's `color` field as its constructor fills
    /// it, which a leaf derives from what its `color` setter was given.
    fn get_value_color(
        &self,
        app: &mut App,
        context: BuildContext,
        color: Option<Color>,
        default_color: Option<Color>,
    ) -> Color {
        if let Some(color) = self
            .clone()
            .value_color
            .and_then(|value_color| value_color.value(app))
            .or(color)
        {
            return color;
        }
        if let Some(color) = ProgressIndicatorTheme::of(app, context)
            .color
            .or(default_color)
        {
            return color;
        }
        Theme::of(app, context).color_scheme.primary
    }
}

/// A base class for Material Design progress indicators: Dart's abstract
/// `ProgressIndicator`, whose fields are [`ProgressIndicatorData`].
///
/// This widget cannot be instantiated directly. For a linear progress
/// indicator, see [`LinearProgressIndicator`]. For a circular progress indicator,
/// see [`CircularProgressIndicator`].
pub trait ProgressIndicator: 'static {
    /// The [`ProgressIndicatorData`] bag.
    fn progress_indicator_data(&self) -> &ProgressIndicatorData;
}

/// The fluent setters of the `super.` arguments of a progress indicator's Dart constructor,
/// writing the [`ProgressIndicatorData`] at `$($bag).+`.
macro_rules! progress_indicator_setters {
    ($name:ident, $($bag:ident).+) => {
        #[doc = concat!("Dart `", stringify!($name), "(key:)`.")]
        pub fn key(mut self, key: KeyRef) -> $name {
            self.$($bag).+.key = Some(key);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(backgroundColor:)`.")]
        pub fn background_color(mut self, background_color: Color) -> $name {
            self.$($bag).+.background_color = Some(background_color);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(color:)`.")]
        pub fn color(mut self, color: Color) -> $name {
            self.$($bag).+.color = Some(color);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(valueColor:)`.")]
        pub fn value_color(mut self, value_color: AnimationRef<Option<Color>>) -> $name {
            self.$($bag).+.value_color = Some(value_color);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(semanticsLabel:)`.")]
        pub fn semantics_label(mut self, semantics_label: impl Into<String>) -> $name {
            self.$($bag).+.semantics_label = Some(semantics_label.into());
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(semanticsValue:)`.")]
        pub fn semantics_value(mut self, semantics_value: impl Into<String>) -> $name {
            self.$($bag).+.semantics_value = Some(semantics_value.into());
            self
        }
    };
}

/// Dart's `_LinearProgressIndicatorPainter`.
#[derive(Debug)]
struct LinearProgressIndicatorPainter {
    track_color: Color,
    value_color: Color,
    value: Option<f64>,
    animation_value: f64,
    text_direction: TextDirection,
    indicator_border_radius: Option<BorderRadiusGeometry>,
    stop_indicator_color: Option<Color>,
    stop_indicator_radius: Option<f64>,
    track_gap: Option<f64>,
}

impl LinearProgressIndicatorPainter {
    // The indeterminate progress animation displays two lines whose leading (head)
    // and trailing (tail) endpoints are defined by the following four curves.
    fn line1_head() -> Interval {
        Interval::new(
            0.0,
            750.0 / K_INDETERMINATE_LINEAR_DURATION as f64,
            Rc::new(Cubic::new(0.2, 0.0, 0.8, 1.0)),
        )
    }

    fn line1_tail() -> Interval {
        Interval::new(
            333.0 / K_INDETERMINATE_LINEAR_DURATION as f64,
            (333.0 + 750.0) / K_INDETERMINATE_LINEAR_DURATION as f64,
            Rc::new(Cubic::new(0.4, 0.0, 1.0, 1.0)),
        )
    }

    fn line2_head() -> Interval {
        Interval::new(
            1000.0 / K_INDETERMINATE_LINEAR_DURATION as f64,
            (1000.0 + 567.0) / K_INDETERMINATE_LINEAR_DURATION as f64,
            Rc::new(Cubic::new(0.0, 0.0, 0.65, 1.0)),
        )
    }

    fn line2_tail() -> Interval {
        Interval::new(
            1267.0 / K_INDETERMINATE_LINEAR_DURATION as f64,
            (1267.0 + 533.0) / K_INDETERMINATE_LINEAR_DURATION as f64,
            Rc::new(Cubic::new(0.10, 0.0, 0.45, 1.0)),
        )
    }

    fn draw_linear_indicator(
        &self,
        canvas: &mut Canvas,
        size: Size,
        start_fraction: f64,
        end_fraction: f64,
        color: Color,
    ) {
        if end_fraction - start_fraction <= 0.0 {
            return;
        }

        let is_ltr = self.text_direction == TextDirection::Ltr;
        let left = (if is_ltr {
            start_fraction
        } else {
            1.0 - end_fraction
        }) * size.width();
        let right = (if is_ltr {
            end_fraction
        } else {
            1.0 - start_fraction
        }) * size.width();

        let rect = Rect::from_ltrb(left, 0.0, right, size.height());
        let paint = Paint::from_color(color.into());

        match self.indicator_border_radius {
            Some(indicator_border_radius) => {
                let rrect = indicator_border_radius
                    .resolve(Some(self.text_direction))
                    .to_rrect(rect);
                draw_rrect(canvas, rrect, &paint);
            }
            None => canvas.draw_rect(rect, &paint),
        }
    }

    fn draw_stop_indicator(&self, canvas: &mut Canvas, size: Size) {
        // Limit the stop indicator to the height of the indicator.
        let max_radius = size.height() / 2.0;
        let radius = self
            .stop_indicator_radius
            .expect("drawn only with a radius")
            .min(max_radius);
        let indicator_paint = Paint::from_color(
            self.stop_indicator_color
                .expect("drawn only with a colour")
                .into(),
        );
        let position = match self.text_direction {
            TextDirection::Rtl => Offset::new(max_radius, max_radius),
            TextDirection::Ltr => Offset::new(size.width() - max_radius, max_radius),
        };
        canvas.draw_circle(position, radius as f32, &indicator_paint);
    }

    // Calculates a track gap fraction that is scaled proportionally to a given
    // value.
    // This is used for a smooth transition of the track gap's size, preventing
    // it from appearing or disappearing abruptly. The returned value increases
    // linearly from 0 to the full `trackGapFraction` as `currentValue`
    // increases from 0 to `_kTrackGapRampDownThreshold`.
    fn get_effective_track_gap_fraction(current_value: f64, track_gap_fraction: f64) -> f64 {
        track_gap_fraction * clamp_double(current_value, 0.0, K_TRACK_GAP_RAMP_DOWN_THRESHOLD)
            / K_TRACK_GAP_RAMP_DOWN_THRESHOLD
    }

    fn paint_determinate(
        &self,
        canvas: &mut Canvas,
        size: Size,
        effective_value: f64,
        track_gap_fraction: f64,
    ) {
        let track_start_fraction = if track_gap_fraction > 0.0 {
            effective_value
                + LinearProgressIndicatorPainter::get_effective_track_gap_fraction(
                    effective_value,
                    track_gap_fraction,
                )
        } else {
            0.0
        };

        // Draw the track when there is still space.
        if track_start_fraction < 1.0 {
            self.draw_linear_indicator(canvas, size, track_start_fraction, 1.0, self.track_color);
        }

        // Draw the stop indicator.
        if self
            .stop_indicator_radius
            .is_some_and(|stop_indicator_radius| stop_indicator_radius > 0.0)
        {
            self.draw_stop_indicator(canvas, size);
        }

        // Draw the active indicator.
        if effective_value > 0.0 {
            self.draw_linear_indicator(canvas, size, 0.0, effective_value, self.value_color);
        }
    }

    fn paint_indeterminate(&self, canvas: &mut Canvas, size: Size, track_gap_fraction: f64) {
        // For LTR text direction the `head` is the right endpoint and the `tail` is
        // the left endpoint.
        let first_line_head =
            LinearProgressIndicatorPainter::line1_head().transform(self.animation_value);
        let first_line_tail =
            LinearProgressIndicatorPainter::line1_tail().transform(self.animation_value);
        let second_line_head =
            LinearProgressIndicatorPainter::line2_head().transform(self.animation_value);
        let second_line_tail =
            LinearProgressIndicatorPainter::line2_tail().transform(self.animation_value);
        let gap = LinearProgressIndicatorPainter::get_effective_track_gap_fraction;

        // Draw the track before line 1. Assuming text direction is LTR, this track
        // appears on the right side of line 1.
        if first_line_head < 1.0 - track_gap_fraction {
            let track_start_fraction = if first_line_head > 0.0 {
                first_line_head + gap(first_line_head, track_gap_fraction)
            } else {
                0.0
            };
            self.draw_linear_indicator(canvas, size, track_start_fraction, 1.0, self.track_color);
        }

        // Draw the line 1.
        if first_line_head - first_line_tail > 0.0 {
            self.draw_linear_indicator(
                canvas,
                size,
                first_line_tail,
                first_line_head,
                self.value_color,
            );
        }

        // Draw the track between line 1 and line 2. Assuming text direction is
        // LTR, this track appears on the left side of line 1 and on the right side
        // of line 2.
        if first_line_tail > track_gap_fraction {
            let track_start_fraction = if second_line_head > 0.0 {
                second_line_head + gap(second_line_head, track_gap_fraction)
            } else {
                0.0
            };
            let track_end_fraction = if first_line_tail < 1.0 {
                first_line_tail - gap(1.0 - first_line_tail, track_gap_fraction)
            } else {
                1.0
            };
            self.draw_linear_indicator(
                canvas,
                size,
                track_start_fraction,
                track_end_fraction,
                self.track_color,
            );
        }

        // Draw the line 2.
        if second_line_head - second_line_tail > 0.0 {
            self.draw_linear_indicator(
                canvas,
                size,
                second_line_tail,
                second_line_head,
                self.value_color,
            );
        }

        // Draw the track after line 2. Assuming text direction is LTR, this track
        // appears on the left side of line 2.
        if second_line_tail > track_gap_fraction {
            let track_end_fraction = if second_line_tail < 1.0 {
                second_line_tail - gap(1.0 - second_line_tail, track_gap_fraction)
            } else {
                1.0
            };
            self.draw_linear_indicator(canvas, size, 0.0, track_end_fraction, self.track_color);
        }
    }
}

impl CustomPainter for LinearProgressIndicatorPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let effective_track_gap = self.track_gap.unwrap_or(0.0);
        let track_gap_fraction = effective_track_gap / size.width();
        let effective_value = self.value.map(|value| clamp_double(value, 0.0, 1.0));

        match effective_value {
            Some(effective_value) => {
                self.paint_determinate(canvas, size, effective_value, track_gap_fraction);
            }
            None => self.paint_indeterminate(canvas, size, track_gap_fraction),
        }
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<LinearProgressIndicatorPainter>()
            .expect("Dart's covariant parameter");
        old_painter.track_color != self.track_color
            || old_painter.value_color != self.value_color
            || old_painter.value != self.value
            || old_painter.animation_value != self.animation_value
            || old_painter.text_direction != self.text_direction
            || old_painter.indicator_border_radius != self.indicator_border_radius
            || old_painter.stop_indicator_color != self.stop_indicator_color
            || old_painter.stop_indicator_radius != self.stop_indicator_radius
            || old_painter.track_gap != self.track_gap
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Material Design linear progress indicator, also known as a progress bar.
///
/// A widget that shows progress along a line. There are two kinds of linear
/// progress indicators:
///
///  * _Determinate_. Determinate progress indicators have a specific value at
///    each point in time, and the value should increase monotonically from 0.0
///    to 1.0, at which time the indicator is complete. To create a determinate
///    progress indicator, use a non-null `value` between 0.0 and 1.0.
///  * _Indeterminate_. Indeterminate progress indicators do not have a specific
///    value at each point in time and instead indicate that progress is being
///    made without indicating how much progress remains. To create an
///    indeterminate progress indicator, use a null `value`.
///
/// The indicator line is displayed with `value_color`, an animated value. To
/// specify a constant color value use: `AlwaysStoppedAnimation::new(Some(color))`.
///
/// The minimum height of the indicator can be specified using [`min_height`](Self::min_height).
/// The indicator can be made taller by wrapping the widget with a `SizedBox`.
///
/// ## Animation synchronization
///
/// When multiple [`CircularProgressIndicator`]s or [`LinearProgressIndicator`]s are
/// animating on screen simultaneously (e.g., in a list of loading items), their
/// uncoordinated animations can appear visually cluttered. To address this, the
/// animation of an indicator can be driven by a custom [`AnimationController`].
///
/// This allows multiple indicators to be synchronized to a single animation
/// source. The most convenient way to achieve this for a group of indicators is
/// by providing a controller via [`ProgressIndicatorTheme`] (see
/// [`ProgressIndicatorThemeData::controller`]). All [`CircularProgressIndicator`]s
/// or [`LinearProgressIndicator`]s within that theme's subtree will then share
/// the same animation, resulting in a more coordinated and visually pleasing
/// effect.
///
/// Alternatively, a specific [`AnimationController`] can be passed directly to the
/// [`controller`](Self::controller) property of an individual indicator.
///
/// See also:
///
///  * [`CircularProgressIndicator`], which shows progress along a circular arc.
///  * `RefreshIndicator`, which automatically displays a [`CircularProgressIndicator`]
///    when the underlying vertical scrollable is overscrolled.
///  * <https://material.io/design/components/progress-indicators.html#linear-progress-indicators>
#[derive(Clone, Debug, Default)]
pub struct LinearProgressIndicator {
    /// The base class's fields.
    ///
    /// Its `background_color` is the color of the track being filled by the linear
    /// indicator. If it is null then the ambient
    /// `ProgressIndicatorThemeData::linear_track_color` will be used. If that is null,
    /// then the ambient theme's `ColorScheme::background` will be used to draw the track.
    pub progress_indicator: ProgressIndicatorData,

    /// The minimum height of the line used to draw the linear indicator.
    ///
    /// If [`min_height`](Self::min_height) is null then it will use the
    /// ambient `ProgressIndicatorThemeData::linear_min_height`. If that is null
    /// it will use 4dp.
    pub min_height: Option<f64>,

    /// The border radius of both the indicator and the track.
    ///
    /// If null, then the `ProgressIndicatorThemeData::border_radius` will be used.
    /// If that is also null, then defaults to radius of 2, which produces a
    /// rounded shape with a rounded indicator. If `ThemeData::use_material3` is false,
    /// then defaults to `BorderRadius::ZERO`, which produces a rectangular shape
    /// with a rectangular indicator.
    pub border_radius: Option<BorderRadiusGeometry>,

    /// The color of the stop indicator.
    ///
    /// If [`year2023`](Self::year2023) is true or `ThemeData::use_material3` is false, then no
    /// stop indicator will be drawn.
    ///
    /// If null, then the `ProgressIndicatorThemeData::stop_indicator_color` will be used.
    /// If that is null, then the `ColorScheme::primary` will be used.
    pub stop_indicator_color: Option<Color>,

    /// The radius of the stop indicator.
    ///
    /// If [`year2023`](Self::year2023) is true or `ThemeData::use_material3` is false, then no
    /// stop indicator will be drawn.
    ///
    /// Set [`stop_indicator_radius`](Self::stop_indicator_radius) to 0 to hide the stop
    /// indicator.
    ///
    /// If null, then the `ProgressIndicatorThemeData::stop_indicator_radius` will be used.
    /// If that is null, then defaults to 2.
    pub stop_indicator_radius: Option<f64>,

    /// The gap between the indicator and the track.
    ///
    /// If [`year2023`](Self::year2023) is true or `ThemeData::use_material3` is false, then no
    /// track gap will be drawn.
    ///
    /// Set [`track_gap`](Self::track_gap) to 0 to hide the track gap.
    ///
    /// If null, then the `ProgressIndicatorThemeData::track_gap` will be used.
    /// If that is null, then defaults to 4.
    pub track_gap: Option<f64>,

    /// When true, the [`LinearProgressIndicator`] will use the 2023 Material Design 3
    /// appearance.
    ///
    /// If null, then the `ProgressIndicatorThemeData::year2023` will be used.
    /// If that is null, then defaults to true.
    ///
    /// If this is set to false, the [`LinearProgressIndicator`] will use the
    /// latest Material Design 3 appearance, which was introduced in December 2023.
    ///
    /// If `ThemeData::use_material3` is false, then this property is ignored.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 progress indicator appearance. Defaults to true. In the future, this flag will default to false. Use ProgressIndicatorThemeData to customize individual properties. This feature was deprecated after v3.27.0-0.1.pre."
    )]
    pub year2023: Option<bool>,

    /// An optional [`AnimationController`] that controls the animation of this
    /// indeterminate progress indicator.
    ///
    /// This controller is only used when the indicator is indeterminate (i.e.,
    /// when `value` is null). If this property is non-null, `value` must be null.
    ///
    /// The controller's value is expected to be a linear progression from 0.0 to
    /// 1.0, which represents one full cycle of the indeterminate animation.
    ///
    /// If this controller is null (and `value` is also null), the widget will
    /// look for a [`ProgressIndicatorThemeData::controller`]. If that is also null,
    /// the widget will create and manage its own internal [`AnimationController`]
    /// to drive the default indeterminate animation.
    ///
    /// See also:
    ///
    ///  * [`LinearProgressIndicator::DEFAULT_ANIMATION_DURATION`], default duration
    ///    for one full cycle of the indeterminate animation.
    pub controller: Option<Ref<AnimationController>>,
}

impl LinearProgressIndicator {
    /// The default duration for one full cycle of the indeterminate animation.
    ///
    /// This duration is used when the widget creates its own [`AnimationController`]
    /// because no [`controller`](Self::controller) was provided, either directly or through a
    /// [`ProgressIndicatorTheme`].
    pub const DEFAULT_ANIMATION_DURATION: Duration =
        Duration::from_millis(K_INDETERMINATE_LINEAR_DURATION);

    /// Creates a linear progress indicator.
    ///
    /// The `value` argument can either be null for an indeterminate
    /// progress indicator, or a non-null value between 0.0 and 1.0 for a
    /// determinate progress indicator.
    pub fn new() -> LinearProgressIndicator {
        LinearProgressIndicator::default()
    }

    progress_indicator_setters!(LinearProgressIndicator, progress_indicator);

    /// Dart `LinearProgressIndicator(value:)`.
    pub fn value(mut self, value: f64) -> LinearProgressIndicator {
        debug_assert!(self.controller.is_none(), "{K_VALUE_CONTROLLER_ASSERTION}");
        self.progress_indicator.value = Some(value);
        self
    }

    /// Dart `LinearProgressIndicator(minHeight:)`.
    pub fn min_height(mut self, min_height: f64) -> LinearProgressIndicator {
        debug_assert!(min_height > 0.0);
        self.min_height = Some(min_height);
        self
    }

    /// Dart `LinearProgressIndicator(borderRadius:)`.
    pub fn border_radius(
        mut self,
        border_radius: impl Into<BorderRadiusGeometry>,
    ) -> LinearProgressIndicator {
        self.border_radius = Some(border_radius.into());
        self
    }

    /// Dart `LinearProgressIndicator(stopIndicatorColor:)`.
    pub fn stop_indicator_color(mut self, stop_indicator_color: Color) -> LinearProgressIndicator {
        self.stop_indicator_color = Some(stop_indicator_color);
        self
    }

    /// Dart `LinearProgressIndicator(stopIndicatorRadius:)`.
    pub fn stop_indicator_radius(mut self, stop_indicator_radius: f64) -> LinearProgressIndicator {
        self.stop_indicator_radius = Some(stop_indicator_radius);
        self
    }

    /// Dart `LinearProgressIndicator(trackGap:)`.
    pub fn track_gap(mut self, track_gap: f64) -> LinearProgressIndicator {
        self.track_gap = Some(track_gap);
        self
    }

    /// Dart `LinearProgressIndicator(year2023:)`.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 progress indicator appearance. Defaults to true. In the future, this flag will default to false. Use ProgressIndicatorThemeData to customize individual properties. This feature was deprecated after v3.26.0-0.1.pre."
    )]
    pub fn year2023(mut self, year2023: bool) -> LinearProgressIndicator {
        self.year2023 = Some(year2023);
        self
    }

    /// Dart `LinearProgressIndicator(controller:)`.
    pub fn controller(mut self, controller: Ref<AnimationController>) -> LinearProgressIndicator {
        debug_assert!(
            self.progress_indicator.value.is_none(),
            "{K_VALUE_CONTROLLER_ASSERTION}"
        );
        self.controller = Some(controller);
        self
    }
}

impl ProgressIndicator for LinearProgressIndicator {
    fn progress_indicator_data(&self) -> &ProgressIndicatorData {
        &self.progress_indicator
    }
}

impl StatefulWidget for LinearProgressIndicator {
    type State = LinearProgressIndicatorState;

    fn key(&self) -> Option<&KeyRef> {
        self.progress_indicator.key.as_ref()
    }

    fn create_state(&self) -> LinearProgressIndicatorState {
        LinearProgressIndicatorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            internal_controller: None,
        }
    }
}

/// Dart's `_LinearProgressIndicatorState`.
pub struct LinearProgressIndicatorState {
    state: StateData<LinearProgressIndicator>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    internal_controller: Option<Ref<AnimationController>>,
}

impl LinearProgressIndicatorState {
    fn internal_controller(self: Handle<Self>, app: &App) -> &Ref<AnimationController> {
        app.get(self)
            .internal_controller
            .as_ref()
            .expect("created in init_state")
    }

    fn controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        let context = self.context(app);
        self.widget(app)
            .controller
            .clone()
            .or_else(|| {
                context
                    .get_inherited_widget_of_exact_type::<ProgressIndicatorTheme>(app)
                    .and_then(|theme| theme.data.controller.clone())
            })
            .or_else(|| {
                context
                    .find_ancestor_widget_of_exact_type::<Theme>(app)
                    .and_then(|theme| theme.data.progress_indicator_theme.controller.clone())
            })
            .unwrap_or_else(|| self.internal_controller(app).clone())
    }

    fn update_controller_animating_status(self: Handle<Self>, app: &mut App) {
        let effective_value = self.widget(app).progress_indicator.effective_value();
        let internal_controller = self.internal_controller(app).handle();
        if effective_value.is_none() && !internal_controller.is_animating(app) {
            internal_controller.repeat(app, None, None, false, None, None);
        } else if effective_value.is_some() && internal_controller.is_animating(app) {
            internal_controller.stop(app, true);
        }
    }

    fn build_indicator(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation_value: f64,
        text_direction: TextDirection,
    ) -> WidgetRef {
        let indicator_theme = ProgressIndicatorTheme::of(app, context);
        let widget = self.widget(app);
        let year2023 = widget.year2023.or(indicator_theme.year2023).unwrap_or(true);
        let defaults: Box<dyn ProgressIndicatorThemeDataGetters> =
            if Theme::of(app, context).use_material3 {
                if year2023 {
                    Box::new(LinearProgressIndicatorDefaultsM3Year2023::new(context))
                } else {
                    Box::new(LinearProgressIndicatorDefaultsM3::new(context))
                }
            } else {
                Box::new(LinearProgressIndicatorDefaultsM2::new(context))
            };
        let widget = self.widget(app).clone();
        let track_color = widget
            .progress_indicator
            .background_color
            .or(indicator_theme.linear_track_color)
            .or_else(|| defaults.linear_track_color(app))
            .expect("the defaults give a track colour");
        let min_height = widget
            .min_height
            .or(indicator_theme.linear_min_height)
            .or_else(|| defaults.linear_min_height(app))
            .expect("the defaults give a minimum height");
        let border_radius = widget
            .border_radius
            .or(indicator_theme.border_radius)
            .or_else(|| defaults.border_radius(app));
        let stop_indicator_color = if !year2023 {
            widget
                .stop_indicator_color
                .or(indicator_theme.stop_indicator_color)
                .or_else(|| defaults.stop_indicator_color(app))
        } else {
            None
        };
        let stop_indicator_radius = if !year2023 {
            widget
                .stop_indicator_radius
                .or(indicator_theme.stop_indicator_radius)
                .or_else(|| defaults.stop_indicator_radius(app))
        } else {
            None
        };
        let track_gap = if !year2023 {
            widget
                .track_gap
                .or(indicator_theme.track_gap)
                .or_else(|| defaults.track_gap(app))
        } else {
            None
        };
        let progress_indicator = widget.progress_indicator.clone();
        let effective_value = progress_indicator.effective_value();
        let default_color = defaults.color(app);

        let mut result = ConstrainedBox::new(
            BoxConstraints::new()
                .min_width(f64::INFINITY)
                .min_height(min_height),
        )
        .child(CustomPaint::new().painter(LinearProgressIndicatorPainter {
            track_color,
            value_color: progress_indicator.get_value_color(
                app,
                context,
                progress_indicator.color,
                default_color,
            ),
            value: effective_value, // may be null
            animation_value,        // ignored if widget._effectiveValue is not null
            text_direction,
            indicator_border_radius: border_radius,
            stop_indicator_color,
            stop_indicator_radius,
            track_gap,
        }))
        .into_widget();

        // Clip is only needed with indeterminate progress indicators
        if let Some(border_radius) = border_radius
            && effective_value.is_none()
        {
            result = ClipRRect::new()
                .border_radius(border_radius)
                .child(result)
                .into_widget();
        }

        progress_indicator.build_semantics_wrapper(context, result)
    }
}

impl SingleTickerProviderStateMixin for LinearProgressIndicatorState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for LinearProgressIndicatorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for LinearProgressIndicatorState {
    type Widget = LinearProgressIndicator;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let internal_controller = AnimationController::new(
            app,
            None,
            Some(LinearProgressIndicator::DEFAULT_ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).internal_controller = Some(internal_controller.clone());
        self.update_controller_animating_status(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &LinearProgressIndicator) {
        self.update_controller_animating_status(app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let internal_controller = self.internal_controller(app).handle();
        internal_controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_direction = Directionality::of(app, context);

        if self
            .widget(app)
            .progress_indicator
            .effective_value()
            .is_some()
        {
            let animation_value = self.controller(app).value(app);
            return self.build_indicator(app, context, animation_value, text_direction);
        }

        AnimatedBuilder::new(
            Rc::new(self.controller(app).erase::<AnyAnimation<f64>>()),
            move |app, context, _child| {
                let animation_value = self.controller(app).value(app);
                self.build_indicator(app, context, animation_value, text_direction)
            },
        )
        .into_widget()
    }
}

/// A stroke `paint` of `color`, `stroke_width` wide, ending in `cap`.
fn stroke_paint(color: Color, stroke_width: f64, cap: StrokeCap) -> Paint {
    let mut paint = Paint::from_color(color.into());
    paint.style = PaintStyle::Stroke(Stroke {
        cap,
        ..Stroke::new(stroke_width as f32)
    });
    paint
}

/// Dart's `_CircularProgressIndicatorPainter`.
#[derive(Debug)]
struct CircularProgressIndicatorPainter {
    track_color: Option<Color>,
    value_color: Color,
    value: Option<f64>,
    head_value: f64,
    tail_value: f64,
    offset_value: f64,
    rotation_value: f64,
    stroke_width: f64,
    stroke_align: f64,
    arc_start: f64,
    arc_sweep: f64,
    stroke_cap: Option<StrokeCap>,
    track_gap: Option<f64>,
    year2023: bool,
}

impl CircularProgressIndicatorPainter {
    const TWO_PI: f64 = PI * 2.0;
    const EPSILON: f64 = 0.001;
    // Canvas.drawArc(r, 0, 2*PI) doesn't draw anything, so just get close.
    const SWEEP: f64 =
        CircularProgressIndicatorPainter::TWO_PI - CircularProgressIndicatorPainter::EPSILON;
    const START_ANGLE: f64 = -PI / 2.0;

    #[allow(clippy::too_many_arguments)]
    fn new(
        track_color: Option<Color>,
        value_color: Color,
        value: Option<f64>,
        head_value: f64,
        tail_value: f64,
        offset_value: f64,
        rotation_value: f64,
        stroke_width: f64,
        stroke_align: f64,
        stroke_cap: Option<StrokeCap>,
        track_gap: Option<f64>,
        year2023: bool,
    ) -> CircularProgressIndicatorPainter {
        let arc_start = match value {
            Some(_) => CircularProgressIndicatorPainter::START_ANGLE,
            None => {
                CircularProgressIndicatorPainter::START_ANGLE
                    + tail_value * 3.0 / 2.0 * PI
                    + rotation_value * PI * 2.0
                    + offset_value * 0.5 * PI
            }
        };
        let arc_sweep = match value {
            Some(value) => clamp_double(value, 0.0, 1.0) * CircularProgressIndicatorPainter::SWEEP,
            None => (head_value * 3.0 / 2.0 * PI - tail_value * 3.0 / 2.0 * PI)
                .max(CircularProgressIndicatorPainter::EPSILON),
        };
        CircularProgressIndicatorPainter {
            track_color,
            value_color,
            value,
            head_value,
            tail_value,
            offset_value,
            rotation_value,
            stroke_width,
            stroke_align,
            arc_start,
            arc_sweep,
            stroke_cap,
            track_gap,
            year2023,
        }
    }

    fn paint_track(&self, canvas: &mut Canvas, size: Size, track_color: Color, arc_rect: Rect) {
        let has_gap = self.track_gap.is_some_and(|track_gap| track_gap > 0.0);
        let background_paint = stroke_paint(
            track_color,
            self.stroke_width,
            self.stroke_cap.unwrap_or(StrokeCap::Round),
        );
        // If hasGap is true, draw the background arc with a gap.
        match self.value {
            Some(value) if has_gap && value > CircularProgressIndicatorPainter::EPSILON => {
                let arc_radius = arc_rect.shortest_side() / 2.0;
                let stroke_radius = self.stroke_width / arc_radius;
                let gap_radius = self.track_gap.expect("has_gap") / arc_radius;
                let start_gap = stroke_radius + gap_radius;
                let end_gap = if value < CircularProgressIndicatorPainter::EPSILON {
                    start_gap
                } else {
                    start_gap * 2.0
                };
                let start_sweep = (-PI / 2.0) + start_gap;
                let end_sweep = (CircularProgressIndicatorPainter::TWO_PI
                    - clamp_double(value, 0.0, 1.0) * CircularProgressIndicatorPainter::TWO_PI
                    - end_gap)
                    .max(0.0);
                // Flip the canvas for the background arc.
                canvas.save();
                canvas.scale(-1.0, 1.0);
                canvas.translate(-size.width() as f32, 0.0);
                draw_arc(
                    canvas,
                    arc_rect,
                    start_sweep,
                    end_sweep,
                    false,
                    &background_paint,
                );
                // Restore the canvas to draw the foreground arc.
                canvas.restore();
            }
            _ => draw_arc(
                canvas,
                arc_rect,
                0.0,
                CircularProgressIndicatorPainter::SWEEP,
                false,
                &background_paint,
            ),
        }
    }

    fn active_stroke_cap(&self) -> StrokeCap {
        if self.year2023 {
            if self.value.is_none() && self.stroke_cap.is_none() {
                // Indeterminate
                StrokeCap::Square
            } else {
                // Butt when determinate (value != null) && strokeCap == null;
                self.stroke_cap.unwrap_or(StrokeCap::Butt)
            }
        } else {
            self.stroke_cap.unwrap_or(StrokeCap::Round)
        }
    }
}

impl CustomPainter for CircularProgressIndicatorPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        // Use the negative operator as intended to keep the exposed constant value
        // as users are already familiar with.
        let stroke_offset = self.stroke_width / 2.0 * -self.stroke_align;
        let arc_base_offset = Offset::new(stroke_offset, stroke_offset);
        let arc_actual_size = Size::new(
            size.width() - stroke_offset * 2.0,
            size.height() - stroke_offset * 2.0,
        );
        let arc_rect = arc_base_offset & arc_actual_size;

        if let Some(track_color) = self.track_color {
            self.paint_track(canvas, size, track_color, arc_rect);
        }

        let paint = stroke_paint(
            self.value_color,
            self.stroke_width,
            self.active_stroke_cap(),
        );
        draw_arc(
            canvas,
            arc_rect,
            self.arc_start,
            self.arc_sweep,
            false,
            &paint,
        );
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<CircularProgressIndicatorPainter>()
            .expect("Dart's covariant parameter");
        old_painter.track_color != self.track_color
            || old_painter.value_color != self.value_color
            || old_painter.value != self.value
            || old_painter.head_value != self.head_value
            || old_painter.tail_value != self.tail_value
            || old_painter.offset_value != self.offset_value
            || old_painter.rotation_value != self.rotation_value
            || old_painter.stroke_width != self.stroke_width
            || old_painter.stroke_align != self.stroke_align
            || old_painter.stroke_cap != self.stroke_cap
            || old_painter.track_gap != self.track_gap
            || old_painter.year2023 != self.year2023
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Material Design circular progress indicator, which spins to indicate that
/// the application is busy.
///
/// A widget that shows progress along a circle. There are two kinds of circular
/// progress indicators:
///
///  * _Determinate_. Determinate progress indicators have a specific value at
///    each point in time, and the value should increase monotonically from 0.0
///    to 1.0, at which time the indicator is complete. To create a determinate
///    progress indicator, use a non-null `value` between 0.0 and 1.0.
///  * _Indeterminate_. Indeterminate progress indicators do not have a specific
///    value at each point in time and instead indicate that progress is being
///    made without indicating how much progress remains. To create an
///    indeterminate progress indicator, use a null `value`.
///
/// The indicator arc is displayed with `value_color`, an animated value. To
/// specify a constant color use: `AlwaysStoppedAnimation::new(Some(color))`.
///
/// For synchronizing the animations of several indicators, see
/// [`LinearProgressIndicator`]'s "Animation synchronization".
///
/// See also:
///
///  * [`LinearProgressIndicator`], which displays progress along a line.
///  * `RefreshIndicator`, which automatically displays a [`CircularProgressIndicator`]
///    when the underlying vertical scrollable is overscrolled.
///  * <https://material.io/design/components/progress-indicators.html#circular-progress-indicators>
#[derive(Clone, Debug)]
pub struct CircularProgressIndicator {
    /// The base class's fields.
    ///
    /// Its `background_color` is the color of the circular track being filled by the
    /// circular indicator. If it is null then the ambient
    /// `ProgressIndicatorThemeData::circular_track_color` will be used. If that is null,
    /// then the track will not be painted.
    pub progress_indicator: ProgressIndicatorData,

    indicator_type: ActivityIndicatorType,

    /// The width of the line used to draw the circle.
    pub stroke_width: Option<f64>,

    /// The relative position of the stroke on a [`CircularProgressIndicator`].
    ///
    /// Values typically range from -1.0 ([`STROKE_ALIGN_INSIDE`](Self::STROKE_ALIGN_INSIDE),
    /// inside stroke) to 1.0 ([`STROKE_ALIGN_OUTSIDE`](Self::STROKE_ALIGN_OUTSIDE), outside
    /// stroke), without any bound constraints (e.g., a value of -2.0 is not typical, but
    /// allowed). A value of 0 ([`STROKE_ALIGN_CENTER`](Self::STROKE_ALIGN_CENTER)) will center
    /// the border on the edge of the widget.
    ///
    /// If [`year2023`](Self::year2023) is true, then the default value is
    /// [`STROKE_ALIGN_CENTER`](Self::STROKE_ALIGN_CENTER). Otherwise, the default value is
    /// [`STROKE_ALIGN_INSIDE`](Self::STROKE_ALIGN_INSIDE).
    pub stroke_align: Option<f64>,

    /// The progress indicator's line ending.
    ///
    /// This determines the shape of the stroke ends of the progress indicator.
    /// By default, [`stroke_cap`](Self::stroke_cap) is null.
    /// When `value` is null (indeterminate), the stroke ends are set to
    /// [`StrokeCap::Square`]. When `value` is not null, the stroke
    /// ends are set to [`StrokeCap::Butt`].
    ///
    /// Setting [`stroke_cap`](Self::stroke_cap) to [`StrokeCap::Round`] will result in a rounded end.
    /// Setting [`stroke_cap`](Self::stroke_cap) to [`StrokeCap::Butt`] with `value` == null will
    /// result in a slightly different indeterminate animation; the indicator completely
    /// disappears and reappears on its minimum value.
    /// Setting [`stroke_cap`](Self::stroke_cap) to [`StrokeCap::Square`] with `value` != null will
    /// result in a different display of `value`. The indicator will start
    /// drawing from slightly less than the start, and end slightly after
    /// the end. This will produce an alternative result, as the
    /// default behavior, for example, that a `value` of 0.5 starts at 90 degrees
    /// and ends at 270 degrees. With [`StrokeCap::Square`], it could start 85
    /// degrees and end at 275 degrees.
    pub stroke_cap: Option<StrokeCap>,

    /// Defines minimum and maximum sizes for a [`CircularProgressIndicator`].
    ///
    /// If null, then the [`ProgressIndicatorThemeData::constraints`] will be used.
    /// Otherwise, defaults to a minimum width and height of 36 pixels.
    pub constraints: Option<BoxConstraints>,

    /// The gap between the active indicator and the background track.
    ///
    /// If [`year2023`](Self::year2023) is true or `ThemeData::use_material3` is false, then no
    /// track gap will be drawn.
    ///
    /// Set [`track_gap`](Self::track_gap) to 0 to hide the track gap.
    ///
    /// If null, then the [`ProgressIndicatorThemeData::track_gap`] will be used.
    /// If that is null, then defaults to 4.
    pub track_gap: Option<f64>,

    /// When true, the [`CircularProgressIndicator`] will use the 2023 Material Design 3
    /// appearance.
    ///
    /// If null, then the `ProgressIndicatorThemeData::year2023` will be used.
    /// If that is null, then defaults to true.
    ///
    /// If this is set to false, the [`CircularProgressIndicator`] will use the
    /// latest Material Design 3 appearance, which was introduced in December 2023.
    ///
    /// If `ThemeData::use_material3` is false, then this property is ignored.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 progress indicator appearance. Defaults to true. In the future, this flag will default to false. Use ProgressIndicatorThemeData to customize individual properties. This feature was deprecated after v3.27.0-0.2.pre."
    )]
    pub year2023: Option<bool>,

    /// The padding around the indicator track.
    ///
    /// If null, then the [`ProgressIndicatorThemeData::circular_track_padding`] will be
    /// used. If that is null and [`year2023`](Self::year2023) is false, then defaults to
    /// `EdgeInsets::all(4.0)` padding. Otherwise, defaults to zero padding.
    pub padding: Option<EdgeInsetsGeometry>,

    /// An optional [`AnimationController`] that controls the animation of this
    /// indeterminate progress indicator; see [`LinearProgressIndicator::controller`].
    ///
    /// See also:
    ///
    ///  * [`CircularProgressIndicator::DEFAULT_ANIMATION_DURATION`], default duration
    ///    for one full cycle of the indeterminate animation.
    pub controller: Option<Ref<AnimationController>>,
}

impl CircularProgressIndicator {
    /// The indicator stroke is drawn fully inside of the indicator path.
    ///
    /// This is a constant for use with [`stroke_align`](Self::stroke_align).
    pub const STROKE_ALIGN_INSIDE: f64 = -1.0;

    /// The indicator stroke is drawn on the center of the indicator path,
    /// with half of the [`stroke_width`](Self::stroke_width) on the inside, and the other half
    /// on the outside of the path.
    ///
    /// This is a constant for use with [`stroke_align`](Self::stroke_align).
    ///
    /// This is the default value for [`stroke_align`](Self::stroke_align).
    pub const STROKE_ALIGN_CENTER: f64 = 0.0;

    /// The indicator stroke is drawn on the outside of the indicator path.
    ///
    /// This is a constant for use with [`stroke_align`](Self::stroke_align).
    pub const STROKE_ALIGN_OUTSIDE: f64 = 1.0;

    /// The default duration for one full cycle of the indeterminate animation.
    ///
    /// During this period, the indicator completes several full rotations.
    ///
    /// This duration is used when the widget creates its own [`AnimationController`]
    /// because no [`controller`](Self::controller) was provided, either directly or through a
    /// [`ProgressIndicatorTheme`].
    pub const DEFAULT_ANIMATION_DURATION: Duration =
        Duration::from_millis(K_INDETERMINATE_CIRCULAR_DURATION);

    /// Creates a circular progress indicator.
    ///
    /// The `value` argument can either be null for an indeterminate
    /// progress indicator, or a non-null value between 0.0 and 1.0 for a
    /// determinate progress indicator.
    pub fn new() -> CircularProgressIndicator {
        CircularProgressIndicator::with_type(ActivityIndicatorType::Material)
    }

    /// Creates an adaptive progress indicator that is a
    /// `CupertinoActivityIndicator` on [`TargetPlatform::IOS`] &
    /// [`TargetPlatform::MacOS`] and a [`CircularProgressIndicator`] in material
    /// theme/non-Apple platforms.
    ///
    /// The `value_color`, [`stroke_width`](Self::stroke_width),
    /// [`stroke_align`](Self::stroke_align), [`stroke_cap`](Self::stroke_cap),
    /// `semantics_label`, `semantics_value`, [`track_gap`](Self::track_gap),
    /// [`year2023`](Self::year2023) will be ignored on iOS & macOS.
    ///
    /// Dart's `CircularProgressIndicator.adaptive` has no `color` parameter: its `color` is
    /// null, so a `color` set on it is ignored ([`effective_color`](Self::effective_color)).
    pub fn adaptive() -> CircularProgressIndicator {
        CircularProgressIndicator::with_type(ActivityIndicatorType::Adaptive)
    }

    fn with_type(indicator_type: ActivityIndicatorType) -> CircularProgressIndicator {
        CircularProgressIndicator {
            progress_indicator: ProgressIndicatorData::default(),
            indicator_type,
            stroke_width: None,
            stroke_align: None,
            stroke_cap: None,
            constraints: None,
            track_gap: None,
            year2023: None,
            padding: None,
            controller: None,
        }
    }

    progress_indicator_setters!(CircularProgressIndicator, progress_indicator);

    /// Dart `CircularProgressIndicator(value:)`.
    pub fn value(mut self, value: f64) -> CircularProgressIndicator {
        debug_assert!(self.controller.is_none(), "{K_VALUE_CONTROLLER_ASSERTION}");
        self.progress_indicator.value = Some(value);
        self
    }

    /// Dart `CircularProgressIndicator(strokeWidth:)`.
    pub fn stroke_width(mut self, stroke_width: f64) -> CircularProgressIndicator {
        self.stroke_width = Some(stroke_width);
        self
    }

    /// Dart `CircularProgressIndicator(strokeAlign:)`.
    pub fn stroke_align(mut self, stroke_align: f64) -> CircularProgressIndicator {
        self.stroke_align = Some(stroke_align);
        self
    }

    /// Dart `CircularProgressIndicator(strokeCap:)`.
    pub fn stroke_cap(mut self, stroke_cap: StrokeCap) -> CircularProgressIndicator {
        self.stroke_cap = Some(stroke_cap);
        self
    }

    /// Dart `CircularProgressIndicator(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> CircularProgressIndicator {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `CircularProgressIndicator(trackGap:)`.
    pub fn track_gap(mut self, track_gap: f64) -> CircularProgressIndicator {
        self.track_gap = Some(track_gap);
        self
    }

    /// Dart `CircularProgressIndicator(year2023:)`.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 progress indicator appearance. Defaults to true. In the future, this flag will default to false. Use ProgressIndicatorThemeData to customize individual properties. This feature was deprecated after v3.27.0-0.1.pre."
    )]
    pub fn year2023(mut self, year2023: bool) -> CircularProgressIndicator {
        self.year2023 = Some(year2023);
        self
    }

    /// Dart `CircularProgressIndicator(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> CircularProgressIndicator {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `CircularProgressIndicator(controller:)`.
    pub fn controller(mut self, controller: Ref<AnimationController>) -> CircularProgressIndicator {
        debug_assert!(
            self.progress_indicator.value.is_none(),
            "{K_VALUE_CONTROLLER_ASSERTION}"
        );
        self.controller = Some(controller);
        self
    }

    /// Dart's `color` field: what the `color` setter was given, and null for an indicator
    /// made with [`CircularProgressIndicator::adaptive`], whose constructor has no `color`.
    pub fn effective_color(&self) -> Option<Color> {
        match self.indicator_type {
            ActivityIndicatorType::Material => self.progress_indicator.color,
            ActivityIndicatorType::Adaptive => None,
        }
    }
}

impl Default for CircularProgressIndicator {
    fn default() -> CircularProgressIndicator {
        CircularProgressIndicator::new()
    }
}

impl ProgressIndicator for CircularProgressIndicator {
    fn progress_indicator_data(&self) -> &ProgressIndicatorData {
        &self.progress_indicator
    }
}

impl StatefulWidget for CircularProgressIndicator {
    type State = CircularProgressIndicatorState;

    fn key(&self) -> Option<&KeyRef> {
        self.progress_indicator.key.as_ref()
    }

    fn create_state(&self) -> CircularProgressIndicatorState {
        CircularProgressIndicatorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            circular_state: CircularProgressIndicatorStateData::default(),
        }
    }
}

const PATH_COUNT: i64 = (K_INDETERMINATE_CIRCULAR_DURATION / 1333) as i64;
const ROTATION_COUNT: i64 = (K_INDETERMINATE_CIRCULAR_DURATION / 2222) as i64;

/// Dart's `static final Animatable<double> _strokeHeadTween`.
fn stroke_head_tween() -> impl Animatable<f64> {
    ValueCurveTween::new(Rc::new(Interval::new(0.0, 0.5, Curves::fast_out_slow_in())))
        .chain(ValueCurveTween::new(Rc::new(SawTooth::new(PATH_COUNT))))
}

/// Dart's `static final Animatable<double> _strokeTailTween`.
fn stroke_tail_tween() -> impl Animatable<f64> {
    ValueCurveTween::new(Rc::new(Interval::new(0.5, 1.0, Curves::fast_out_slow_in())))
        .chain(ValueCurveTween::new(Rc::new(SawTooth::new(PATH_COUNT))))
}

/// Dart's `static final Animatable<double> _offsetTween`.
fn offset_tween() -> impl Animatable<f64> {
    ValueCurveTween::new(Rc::new(SawTooth::new(PATH_COUNT)))
}

/// Dart's `static final Animatable<double> _rotationTween`.
fn rotation_tween() -> impl Animatable<f64> {
    ValueCurveTween::new(Rc::new(SawTooth::new(ROTATION_COUNT)))
}

/// The fields of Dart's `_CircularProgressIndicatorState` that
/// `_RefreshProgressIndicatorState` inherits, under the field `circular_state`.
#[derive(Debug, Default)]
struct CircularProgressIndicatorStateData {
    internal_controller: Option<Ref<AnimationController>>,
}

/// Dart's `_CircularProgressIndicatorState` as a base class: its bodies, and the override
/// points `build`, `_buildAnimation` and `_buildMaterialIndicator` that
/// `_RefreshProgressIndicatorState` overrides.
trait CircularProgressIndicatorStateBase:
    State + SingleTickerProviderStateMixin + TickerProviderObject
{
    fn circular_state_data(self: Handle<Self>, app: &App) -> &CircularProgressIndicatorStateData;

    fn circular_state_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut CircularProgressIndicatorStateData;

    /// Dart's `widget`, as the [`CircularProgressIndicator`] it is or extends.
    fn circular_widget(self: Handle<Self>, app: &App) -> &CircularProgressIndicator;

    fn init_state(self: Handle<Self>, app: &mut App) {
        let internal_controller = AnimationController::new(
            app,
            None,
            Some(CircularProgressIndicator::DEFAULT_ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        self.circular_state_data_mut(app).internal_controller = Some(internal_controller.clone());
        self.update_controller_animating_status(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App) {
        self.update_controller_animating_status(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let internal_controller = self.internal_controller(app).handle();
        internal_controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn internal_controller(self: Handle<Self>, app: &App) -> &Ref<AnimationController> {
        self.circular_state_data(app)
            .internal_controller
            .as_ref()
            .expect("created in init_state")
    }

    fn controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        let context = self.context(app);
        self.circular_widget(app)
            .controller
            .clone()
            .or_else(|| {
                context
                    .get_inherited_widget_of_exact_type::<ProgressIndicatorTheme>(app)
                    .and_then(|theme| theme.data.controller.clone())
            })
            .or_else(|| {
                context
                    .find_ancestor_widget_of_exact_type::<Theme>(app)
                    .and_then(|theme| theme.data.progress_indicator_theme.controller.clone())
            })
            .unwrap_or_else(|| self.internal_controller(app).clone())
    }

    fn update_controller_animating_status(self: Handle<Self>, app: &mut App) {
        let effective_value = self
            .circular_widget(app)
            .progress_indicator
            .effective_value();
        let internal_controller = self.internal_controller(app).handle();
        if effective_value.is_none() && !internal_controller.is_animating(app) {
            internal_controller.repeat(app, None, None, false, None, None);
        } else if effective_value.is_some() && internal_controller.is_animating(app) {
            internal_controller.stop(app, true);
        }
    }

    fn build_cupertino_indicator(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let progress_indicator = &self.circular_widget(app).progress_indicator;
        let tick_color = progress_indicator.background_color;
        let value = progress_indicator.effective_value();
        let indicator = match value {
            None => CupertinoActivityIndicator::new(),
            Some(value) => CupertinoActivityIndicator::partially_revealed().progress(value),
        };
        let indicator = match progress_indicator.key.clone() {
            Some(key) => indicator.key(key),
            None => indicator,
        };
        match tick_color {
            Some(tick_color) => indicator.color(tick_color),
            None => indicator,
        }
        .into_widget()
    }

    fn build_material_indicator(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        head_value: f64,
        tail_value: f64,
        offset_value: f64,
        rotation_value: f64,
    ) -> WidgetRef {
        let indicator_theme = ProgressIndicatorTheme::of(app, context);
        let year2023 = self
            .circular_widget(app)
            .year2023
            .or(indicator_theme.year2023)
            .unwrap_or(true);
        let indeterminate = self
            .circular_widget(app)
            .progress_indicator
            .effective_value()
            .is_none();
        let defaults: Box<dyn ProgressIndicatorThemeDataGetters> =
            if Theme::of(app, context).use_material3 {
                if year2023 {
                    Box::new(CircularProgressIndicatorDefaultsM3Year2023::new(
                        context,
                        indeterminate,
                    ))
                } else {
                    Box::new(CircularProgressIndicatorDefaultsM3::new(
                        context,
                        indeterminate,
                    ))
                }
            } else {
                Box::new(CircularProgressIndicatorDefaultsM2::new(
                    context,
                    indeterminate,
                ))
            };
        let widget = self.circular_widget(app).clone();
        let track_color = widget
            .progress_indicator
            .background_color
            .or(indicator_theme.circular_track_color)
            .or_else(|| defaults.circular_track_color(app));
        let stroke_width = widget
            .stroke_width
            .or(indicator_theme.stroke_width)
            .or_else(|| defaults.stroke_width(app))
            .expect("the defaults give a stroke width");
        let stroke_align = widget
            .stroke_align
            .or(indicator_theme.stroke_align)
            .or_else(|| defaults.stroke_align(app))
            .expect("the defaults give a stroke alignment");
        let stroke_cap = widget.stroke_cap.or(indicator_theme.stroke_cap);
        let constraints = widget
            .constraints
            .or(indicator_theme.constraints)
            .or_else(|| defaults.constraints(app))
            .expect("the defaults give constraints");
        let track_gap = if year2023 {
            None
        } else {
            widget
                .track_gap
                .or(indicator_theme.track_gap)
                .or_else(|| defaults.track_gap(app))
        };
        let effective_padding = widget
            .padding
            .or(indicator_theme.circular_track_padding)
            .or_else(|| defaults.circular_track_padding(app));
        let progress_indicator = widget.progress_indicator.clone();
        let color = widget.effective_color();
        let default_color = defaults.color(app);

        let result = ConstrainedBox::new(constraints).child(CustomPaint::new().painter(
            CircularProgressIndicatorPainter::new(
                track_color,
                progress_indicator.get_value_color(app, context, color, default_color),
                progress_indicator.effective_value(), // may be null
                head_value, // remaining arguments are ignored if widget._effectiveValue is not null
                tail_value,
                offset_value,
                rotation_value,
                stroke_width,
                stroke_align,
                stroke_cap,
                track_gap,
                year2023,
            ),
        ));

        let result = match effective_padding {
            Some(effective_padding) => Padding::new(effective_padding).child(result).into_widget(),
            None => result.into_widget(),
        };

        progress_indicator.build_semantics_wrapper(context, result)
    }

    fn build_animation(self: Handle<Self>, app: &mut App) -> WidgetRef {
        AnimatedBuilder::new(
            Rc::new(self.controller(app).erase::<AnyAnimation<f64>>()),
            move |app, context, _child| {
                let value = self.controller(app).value(app);
                self.build_material_indicator(
                    app,
                    context,
                    stroke_head_tween().transform(app, value),
                    stroke_tail_tween().transform(app, value),
                    offset_tween().transform(app, value),
                    rotation_tween().transform(app, value),
                )
            },
        )
        .into_widget()
    }

    fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Builder::new(move |app, context| {
            let widget = self.circular_widget(app);
            let determinate = widget.progress_indicator.effective_value().is_some();
            match widget.indicator_type {
                ActivityIndicatorType::Material => {
                    if determinate {
                        return self.build_material_indicator(app, context, 0.0, 0.0, 0.0, 0.0);
                    }
                    self.build_animation(app)
                }
                ActivityIndicatorType::Adaptive => {
                    let theme = Theme::of(app, context);
                    match theme.platform {
                        TargetPlatform::IOS | TargetPlatform::MacOS => {
                            self.build_cupertino_indicator(app)
                        }
                        TargetPlatform::Android
                        | TargetPlatform::Fuchsia
                        | TargetPlatform::Linux
                        | TargetPlatform::Windows => {
                            if determinate {
                                return self
                                    .build_material_indicator(app, context, 0.0, 0.0, 0.0, 0.0);
                            }
                            self.build_animation(app)
                        }
                    }
                }
            }
        })
        .into_widget()
    }
}

/// Dart's `_CircularProgressIndicatorState`.
pub struct CircularProgressIndicatorState {
    state: StateData<CircularProgressIndicator>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    circular_state: CircularProgressIndicatorStateData,
}

impl CircularProgressIndicatorStateBase for CircularProgressIndicatorState {
    fn circular_state_data(self: Handle<Self>, app: &App) -> &CircularProgressIndicatorStateData {
        &app.get(self).circular_state
    }

    fn circular_state_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut CircularProgressIndicatorStateData {
        &mut app.get_mut(self).circular_state
    }

    fn circular_widget(self: Handle<Self>, app: &App) -> &CircularProgressIndicator {
        self.widget(app)
    }
}

impl SingleTickerProviderStateMixin for CircularProgressIndicatorState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for CircularProgressIndicatorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CircularProgressIndicatorState {
    type Widget = CircularProgressIndicator;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        CircularProgressIndicatorStateBase::init_state(self, app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        _old_widget: &CircularProgressIndicator,
    ) {
        CircularProgressIndicatorStateBase::did_update_widget(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        CircularProgressIndicatorStateBase::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        CircularProgressIndicatorStateBase::build(self, app, context)
    }
}

/// Dart's `_RefreshProgressIndicatorPainter`, which extends the circular painter it holds.
#[derive(Debug)]
struct RefreshProgressIndicatorPainter {
    circular: CircularProgressIndicatorPainter,
    arrowhead_scale: f64,
}

impl RefreshProgressIndicatorPainter {
    fn paint_arrowhead(&self, canvas: &mut Canvas, size: Size) {
        let circular = &self.circular;
        // ux, uy: a unit vector whose direction parallels the base of the arrowhead.
        // (So ux, -uy points in the direction the arrowhead points.)
        let arc_end = circular.arc_start + circular.arc_sweep;
        let ux = arc_end.cos();
        let uy = arc_end.sin();

        debug_assert_eq!(size.width(), size.height());
        let radius = size.width() / 2.0;
        let arrowhead_point_x =
            radius + ux * radius + -uy * circular.stroke_width * 2.0 * self.arrowhead_scale;
        let arrowhead_point_y =
            radius + uy * radius + ux * circular.stroke_width * 2.0 * self.arrowhead_scale;
        let arrowhead_radius = circular.stroke_width * 2.0 * self.arrowhead_scale;
        let inner_radius = radius - arrowhead_radius;
        let outer_radius = radius + arrowhead_radius;

        let mut path = PathBuilder::new();
        path.move_to((
            (radius + ux * inner_radius) as f32,
            (radius + uy * inner_radius) as f32,
        ))
        .line_to((
            (radius + ux * outer_radius) as f32,
            (radius + uy * outer_radius) as f32,
        ))
        .line_to((arrowhead_point_x as f32, arrowhead_point_y as f32))
        .close();

        let paint = Paint::from_color(circular.value_color.into());
        canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
    }
}

impl CustomPainter for RefreshProgressIndicatorPainter {
    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        self.circular.paint(app, canvas, size);
        if self.arrowhead_scale > 0.0 {
            self.paint_arrowhead(canvas, size);
        }
    }

    fn should_repaint(&self, app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<RefreshProgressIndicatorPainter>()
            .expect("Dart's covariant parameter");
        self.circular.should_repaint(app, &old_painter.circular)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An indicator for the progress of refreshing the contents of a widget.
///
/// Typically used for swipe-to-refresh interactions. See `RefreshIndicator` for
/// a complete implementation of swipe-to-refresh driven by a `Scrollable`
/// widget.
///
/// The indicator arc is displayed with `value_color`, an animated value. To
/// specify a constant color use: `AlwaysStoppedAnimation::new(Some(color))`.
///
/// See also:
///
///  * `RefreshIndicator`, which automatically displays a [`CircularProgressIndicator`]
///    when the underlying vertical scrollable is overscrolled.
#[derive(Debug)]
pub struct RefreshProgressIndicator {
    /// The [`CircularProgressIndicator`] this extends.
    ///
    /// Its `progress_indicator.background_color` is the background color of that fills the
    /// circle under the refresh indicator. If `RefreshIndicator.backgroundColor` is null then
    /// the ambient [`ProgressIndicatorThemeData::refresh_background_color`] will be used.
    /// If that is null, then the ambient theme's `ThemeData::canvas_color` will be used.
    pub circular_progress_indicator: CircularProgressIndicator,

    /// The z-coordinate at which to place this material relative to its parent.
    pub elevation: f64,

    /// The amount of space by which to inset the whole indicator.
    /// It accommodates the [`elevation`](Self::elevation) of the indicator.
    pub indicator_margin: EdgeInsetsGeometry,

    /// The amount of space by which to inset the inner refresh indicator.
    pub indicator_padding: EdgeInsetsGeometry,
}

impl RefreshProgressIndicator {
    /// Default stroke width.
    pub const DEFAULT_STROKE_WIDTH: f64 = 2.5;

    /// Creates a refresh progress indicator.
    ///
    /// Rather than creating a refresh progress indicator directly, consider using
    /// a `RefreshIndicator` together with a `Scrollable` widget.
    ///
    /// The `value` argument can either be null for an indeterminate
    /// progress indicator, or a non-null value between 0.0 and 1.0 for a
    /// determinate progress indicator.
    pub fn new() -> RefreshProgressIndicator {
        RefreshProgressIndicator {
            // Different default than CircularProgressIndicator.
            circular_progress_indicator: CircularProgressIndicator::new()
                .stroke_width(RefreshProgressIndicator::DEFAULT_STROKE_WIDTH),
            elevation: 2.0,
            indicator_margin: EdgeInsets::all(4.0).into(),
            indicator_padding: EdgeInsets::all(12.0).into(),
        }
    }

    progress_indicator_setters!(
        RefreshProgressIndicator,
        circular_progress_indicator.progress_indicator
    );

    /// Dart `RefreshProgressIndicator(value:)`.
    pub fn value(mut self, value: f64) -> RefreshProgressIndicator {
        self.circular_progress_indicator = self.circular_progress_indicator.value(value);
        self
    }

    /// Dart `RefreshProgressIndicator(strokeWidth:)`.
    pub fn stroke_width(mut self, stroke_width: f64) -> RefreshProgressIndicator {
        self.circular_progress_indicator =
            self.circular_progress_indicator.stroke_width(stroke_width);
        self
    }

    /// Dart `RefreshProgressIndicator(strokeAlign:)`.
    pub fn stroke_align(mut self, stroke_align: f64) -> RefreshProgressIndicator {
        self.circular_progress_indicator =
            self.circular_progress_indicator.stroke_align(stroke_align);
        self
    }

    /// Dart `RefreshProgressIndicator(strokeCap:)`.
    pub fn stroke_cap(mut self, stroke_cap: StrokeCap) -> RefreshProgressIndicator {
        self.circular_progress_indicator = self.circular_progress_indicator.stroke_cap(stroke_cap);
        self
    }

    /// Dart `RefreshProgressIndicator(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> RefreshProgressIndicator {
        self.elevation = elevation;
        self
    }

    /// Dart `RefreshProgressIndicator(indicatorMargin:)`.
    pub fn indicator_margin(
        mut self,
        indicator_margin: impl Into<EdgeInsetsGeometry>,
    ) -> RefreshProgressIndicator {
        self.indicator_margin = indicator_margin.into();
        self
    }

    /// Dart `RefreshProgressIndicator(indicatorPadding:)`.
    pub fn indicator_padding(
        mut self,
        indicator_padding: impl Into<EdgeInsetsGeometry>,
    ) -> RefreshProgressIndicator {
        self.indicator_padding = indicator_padding.into();
        self
    }
}

impl Default for RefreshProgressIndicator {
    fn default() -> RefreshProgressIndicator {
        RefreshProgressIndicator::new()
    }
}

impl ProgressIndicator for RefreshProgressIndicator {
    fn progress_indicator_data(&self) -> &ProgressIndicatorData {
        &self.circular_progress_indicator.progress_indicator
    }
}

impl StatefulWidget for RefreshProgressIndicator {
    type State = RefreshProgressIndicatorState;

    fn key(&self) -> Option<&KeyRef> {
        self.circular_progress_indicator
            .progress_indicator
            .key
            .as_ref()
    }

    fn create_state(&self) -> RefreshProgressIndicatorState {
        RefreshProgressIndicatorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            circular_state: CircularProgressIndicatorStateData::default(),
            convert_tween: ValueCurveTween::new(Rc::new(Interval::new(
                0.1,
                RefreshProgressIndicatorState::STROKE_HEAD_INTERVAL,
                Curves::linear(),
            ))),
            additional_rotation_tween: TweenSequence::new(vec![
                // Makes arrow to expand a little bit earlier, to match the Android look.
                TweenSequenceItem::new(
                    Rc::new(ValueTween::new(-0.1, -0.2)),
                    RefreshProgressIndicatorState::STROKE_HEAD_INTERVAL,
                ),
                // Additional rotation after the arrow expanded
                TweenSequenceItem::new(
                    Rc::new(ValueTween::new(-0.2, 1.35)),
                    1.0 - RefreshProgressIndicatorState::STROKE_HEAD_INTERVAL,
                ),
            ]),
            last_value: None,
        }
    }
}

/// Dart's `_RefreshProgressIndicatorState`, which extends `_CircularProgressIndicatorState`
/// through [`CircularProgressIndicatorStateBase`].
pub struct RefreshProgressIndicatorState {
    state: StateData<RefreshProgressIndicator>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    circular_state: CircularProgressIndicatorStateData,
    convert_tween: ValueCurveTween,
    additional_rotation_tween: TweenSequence<f64>,
    // Last value received from the widget before null.
    last_value: Option<f64>,
}

impl RefreshProgressIndicatorState {
    const INDICATOR_SIZE: f64 = 41.0;

    /// Interval for arrow head to fully grow.
    const STROKE_HEAD_INTERVAL: f64 = 0.33;
}

impl CircularProgressIndicatorStateBase for RefreshProgressIndicatorState {
    fn circular_state_data(self: Handle<Self>, app: &App) -> &CircularProgressIndicatorStateData {
        &app.get(self).circular_state
    }

    fn circular_state_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut CircularProgressIndicatorStateData {
        &mut app.get_mut(self).circular_state
    }

    fn circular_widget(self: Handle<Self>, app: &App) -> &CircularProgressIndicator {
        &self.widget(app).circular_progress_indicator
    }

    // Always show the indeterminate version of the circular progress indicator.
    //
    // When value is non-null the sweep of the progress indicator arrow's arc
    // varies from 0 to about 300 degrees.
    //
    // When value is null the arrow animation starting from wherever we left it.
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let value = self
            .circular_widget(app)
            .progress_indicator
            .effective_value();
        if let Some(value) = value {
            app.get_mut(self).last_value = Some(value);
            let converted = app.get(self).convert_tween.transform(app, value);
            self.controller(app).set_value(
                app,
                converted * (1333.0 / 2.0 / K_INDETERMINATE_CIRCULAR_DURATION as f64),
            );
        }
        CircularProgressIndicatorStateBase::build_animation(self, app)
    }

    fn build_animation(self: Handle<Self>, app: &mut App) -> WidgetRef {
        AnimatedBuilder::new(
            Rc::new(self.controller(app).erase::<AnyAnimation<f64>>()),
            move |app, context, _child| {
                let value = self.controller(app).value(app);
                self.build_material_indicator(
                    app,
                    context,
                    // Lengthen the arc a little
                    1.05 * stroke_head_tween().transform(app, value),
                    stroke_tail_tween().transform(app, value),
                    offset_tween().transform(app, value),
                    rotation_tween().transform(app, value),
                )
            },
        )
        .into_widget()
    }

    fn build_material_indicator(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        head_value: f64,
        tail_value: f64,
        offset_value: f64,
        rotation_value: f64,
    ) -> WidgetRef {
        let value = self
            .circular_widget(app)
            .progress_indicator
            .effective_value();
        let arrowhead_scale = match value {
            None => 0.0,
            Some(value) => Interval::new(
                0.1,
                RefreshProgressIndicatorState::STROKE_HEAD_INTERVAL,
                Curves::linear(),
            )
            .transform(value),
        };
        let last_value = app.get(self).last_value;
        let rotation = if value.is_none() && last_value.is_none() {
            0.0
        } else {
            PI * app.get(self).additional_rotation_tween.transform(
                app,
                value.unwrap_or_else(|| last_value.expect("checked above")),
            )
        };

        let widget = self.circular_widget(app);
        let color = widget.effective_color();
        let progress_indicator = widget.progress_indicator.clone();
        let value_color = progress_indicator.get_value_color(app, context, color, None);
        let opacity = value_color.opacity();
        let value_color = value_color.with_opacity(1.0);

        let defaults: Box<dyn ProgressIndicatorThemeDataGetters> =
            if Theme::of(app, context).use_material3 {
                Box::new(CircularProgressIndicatorDefaultsM3Year2023::new(
                    context,
                    value.is_none(),
                ))
            } else {
                Box::new(CircularProgressIndicatorDefaultsM2::new(
                    context,
                    value.is_none(),
                ))
            };
        let indicator_theme = ProgressIndicatorTheme::of(app, context);
        let background_color = match progress_indicator
            .background_color
            .or(indicator_theme.refresh_background_color)
        {
            Some(background_color) => background_color,
            None => Theme::of(app, context).canvas_color,
        };
        let circular = self.circular_widget(app).clone();
        let stroke_width = circular
            .stroke_width
            .or(indicator_theme.stroke_width)
            .or_else(|| defaults.stroke_width(app))
            .expect("the defaults give a stroke width");
        let stroke_align = circular
            .stroke_align
            .or(indicator_theme.stroke_align)
            .or_else(|| defaults.stroke_align(app))
            .expect("the defaults give a stroke alignment");
        let stroke_cap = circular.stroke_cap.or(indicator_theme.stroke_cap);
        let widget = self.widget(app);

        progress_indicator.build_semantics_wrapper(
            context,
            Padding::new(widget.indicator_margin).child(
                SizedBox::from_size(Some(Size::square(
                    RefreshProgressIndicatorState::INDICATOR_SIZE,
                )))
                .child(
                    Material::new()
                        .r#type(MaterialType::Circle)
                        .color(background_color)
                        .elevation(widget.elevation)
                        .child(Padding::new(widget.indicator_padding).child(
                            Opacity::new(opacity).child(Transform::rotate(rotation).child(
                                CustomPaint::new().painter(RefreshProgressIndicatorPainter {
                                    circular: CircularProgressIndicatorPainter::new(
                                        None,
                                        value_color,
                                        // Draw the indeterminate progress indicator.
                                        None,
                                        head_value,
                                        tail_value,
                                        offset_value,
                                        rotation_value,
                                        stroke_width,
                                        stroke_align,
                                        stroke_cap,
                                        None,
                                        true,
                                    ),
                                    arrowhead_scale,
                                }),
                            )),
                        )),
                ),
            ),
        )
    }
}

impl SingleTickerProviderStateMixin for RefreshProgressIndicatorState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for RefreshProgressIndicatorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for RefreshProgressIndicatorState {
    type Widget = RefreshProgressIndicator;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        CircularProgressIndicatorStateBase::init_state(self, app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        _old_widget: &RefreshProgressIndicator,
    ) {
        CircularProgressIndicatorStateBase::did_update_widget(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        CircularProgressIndicatorStateBase::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        CircularProgressIndicatorStateBase::build(self, app, context)
    }
}

// Hand coded defaults based on Material Design 2.
/// Dart's `_CircularProgressIndicatorDefaultsM2`.
struct CircularProgressIndicatorDefaultsM2 {
    /// Dart's `super(..)` arguments.
    progress_indicator_theme_data: ProgressIndicatorThemeData,
    context: DefaultsContext,
    /// Dart's `indeterminate`.
    #[expect(
        dead_code,
        reason = "Dart's constructor argument, which no getter of this class reads"
    )]
    indeterminate: bool,
}

impl CircularProgressIndicatorDefaultsM2 {
    fn new(context: BuildContext, indeterminate: bool) -> CircularProgressIndicatorDefaultsM2 {
        CircularProgressIndicatorDefaultsM2 {
            progress_indicator_theme_data: ProgressIndicatorThemeData::new(),
            context: DefaultsContext::new(context),
            indeterminate,
        }
    }
}

impl ProgressIndicatorThemeDataGetters for CircularProgressIndicatorDefaultsM2 {
    fn progress_indicator_theme_data(&self) -> &ProgressIndicatorThemeData {
        &self.progress_indicator_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn stroke_width(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }

    fn stroke_align(&self, _app: &mut App) -> Option<f64> {
        Some(CircularProgressIndicator::STROKE_ALIGN_CENTER)
    }

    fn constraints(&self, _app: &mut App) -> Option<BoxConstraints> {
        Some(BoxConstraints::new().min_width(36.0).min_height(36.0))
    }
}

/// Dart's `_LinearProgressIndicatorDefaultsM2`.
struct LinearProgressIndicatorDefaultsM2 {
    /// Dart's `super(..)` arguments.
    progress_indicator_theme_data: ProgressIndicatorThemeData,
    context: DefaultsContext,
}

impl LinearProgressIndicatorDefaultsM2 {
    fn new(context: BuildContext) -> LinearProgressIndicatorDefaultsM2 {
        LinearProgressIndicatorDefaultsM2 {
            progress_indicator_theme_data: ProgressIndicatorThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl ProgressIndicatorThemeDataGetters for LinearProgressIndicatorDefaultsM2 {
    fn progress_indicator_theme_data(&self) -> &ProgressIndicatorThemeData {
        &self.progress_indicator_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn linear_track_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).background())
    }

    fn linear_min_height(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }
}

/// Dart's `_CircularProgressIndicatorDefaultsM3Year2023`.
struct CircularProgressIndicatorDefaultsM3Year2023 {
    /// Dart's `super(..)` arguments.
    progress_indicator_theme_data: ProgressIndicatorThemeData,
    context: DefaultsContext,
    /// Dart's `indeterminate`.
    #[expect(
        dead_code,
        reason = "Dart's constructor argument, which no getter of this class reads"
    )]
    indeterminate: bool,
}

impl CircularProgressIndicatorDefaultsM3Year2023 {
    fn new(
        context: BuildContext,
        indeterminate: bool,
    ) -> CircularProgressIndicatorDefaultsM3Year2023 {
        CircularProgressIndicatorDefaultsM3Year2023 {
            progress_indicator_theme_data: ProgressIndicatorThemeData::new(),
            context: DefaultsContext::new(context),
            indeterminate,
        }
    }
}

impl ProgressIndicatorThemeDataGetters for CircularProgressIndicatorDefaultsM3Year2023 {
    fn progress_indicator_theme_data(&self) -> &ProgressIndicatorThemeData {
        &self.progress_indicator_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn stroke_width(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }

    fn stroke_align(&self, _app: &mut App) -> Option<f64> {
        Some(CircularProgressIndicator::STROKE_ALIGN_CENTER)
    }

    fn constraints(&self, _app: &mut App) -> Option<BoxConstraints> {
        Some(BoxConstraints::new().min_width(36.0).min_height(36.0))
    }
}

/// Dart's `_LinearProgressIndicatorDefaultsM3Year2023`.
struct LinearProgressIndicatorDefaultsM3Year2023 {
    /// Dart's `super(..)` arguments.
    progress_indicator_theme_data: ProgressIndicatorThemeData,
    context: DefaultsContext,
}

impl LinearProgressIndicatorDefaultsM3Year2023 {
    fn new(context: BuildContext) -> LinearProgressIndicatorDefaultsM3Year2023 {
        LinearProgressIndicatorDefaultsM3Year2023 {
            progress_indicator_theme_data: ProgressIndicatorThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl ProgressIndicatorThemeDataGetters for LinearProgressIndicatorDefaultsM3Year2023 {
    fn progress_indicator_theme_data(&self) -> &ProgressIndicatorThemeData {
        &self.progress_indicator_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn linear_track_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).secondary_container())
    }

    fn linear_min_height(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/progress_indicator_test.dart`.

    use waft_animation::AlwaysStoppedAnimation;
    use waft_embedder::valo::{self, Op};
    use waft_embedder::{RRect, Radius};
    use waft_painting::BorderRadius;
    use waft_rendering::{
        AnyRenderObject, RenderBox, RenderCustomPaint, RenderHandle, RenderObject,
    };
    use waft_scheduler::SchedulerBinding;
    use waft_widgets::{AnyElement, Center, InheritedTheme, TickerMode, downcast_widget};

    use waft_widgets::testing::{
        TargetPlatformVariant, WidgetTester, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::app::MaterialApp;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::scaffold::Scaffold;
    use crate::theme_data::ThemeData;
    use waft_widgets::testing::TestVSync;

    /// `tester.pumpWidget(build(app))`.
    pub(crate) fn pump_with(tester: &mut WidgetTester, build: impl FnOnce(&mut App) -> WidgetRef) {
        let root = build(&mut tester.cell.borrow_mut());
        tester.pump_widget(root);
    }

    /// Dart's `ThemeData()`.
    pub(crate) fn theme(app: &App) -> ThemeData {
        ThemeData::new(app)
    }

    /// Dart's `theme.copyWith(progressIndicatorTheme: data)`.
    pub(crate) fn theme_with(app: &App, data: ProgressIndicatorThemeData) -> ThemeData {
        ThemeData::builder()
            .progress_indicator_theme(data)
            .build(app)
    }

    pub(crate) fn themed<K>(theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        Theme::new(theme, child).into_widget()
    }

    pub(crate) fn directional<K>(direction: TextDirection, child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(direction, child).into_widget()
    }

    /// `Center(child: SizedBox(width: width, child: child))`.
    pub(crate) fn centered_width<K>(width: f64, child: impl IntoWidget<K>) -> WidgetRef {
        Center::new()
            .child(SizedBox::new().width(width).child(child))
            .into_widget()
    }

    /// `MaterialApp(theme: theme, home: Scaffold(body: body))`.
    pub(crate) fn scaffold_app<K>(theme: Option<ThemeData>, body: impl IntoWidget<K>) -> WidgetRef {
        let app = MaterialApp::new().home(Scaffold::new().body(body));
        match theme {
            Some(theme) => app.theme(theme),
            None => app,
        }
        .into_widget()
    }

    fn transient_callback_count(tester: &WidgetTester) -> usize {
        SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut())
    }

    /// The first render object of type `T` at or under `node` that `accept`s.
    fn find_render<T: RenderObject>(
        app: &App,
        node: AnyRenderObject,
        accept: &dyn Fn(RenderHandle<T>) -> bool,
    ) -> Option<RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app)
            && accept(found)
        {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find_render(app, child, accept);
            }
        });
        found
    }

    /// The first element whose widget is a `W`: Dart's `find.byType(W)`.
    pub(crate) fn first<W: 'static>(tester: &WidgetTester) -> AnyElement {
        *tester
            .find_by_type::<W>()
            .first()
            .expect("a widget of that type")
    }

    /// The painter of the first `CustomPaint` under the first `W`, and the size it paints.
    pub(crate) fn painter_under<W: 'static>(
        tester: &WidgetTester,
    ) -> (Rc<dyn CustomPainter>, Size) {
        let element = first::<W>(tester);
        let app = tester.cell.borrow();
        let root = element
            .find_render_object(&app)
            .expect("the widget has a render object");
        let custom = find_render(&app, root, &|custom: RenderHandle<RenderCustomPaint>| {
            custom.painter(&app).is_some()
        })
        .expect("a CustomPaint with a painter under it");
        (custom.painter(&app).expect("a painter"), custom.size(&app))
    }

    /// What the painter under the first `W` records: the calls Dart's `paints` matcher reads
    /// on `find.byType(W)`.
    pub(crate) fn painted_by<W: 'static>(tester: &WidgetTester) -> Vec<Op> {
        let (painter, size) = painter_under::<W>(tester);
        let mut canvas = Canvas::new();
        painter.paint(&mut tester.cell.borrow_mut(), &mut canvas, size);
        canvas.build().ops().to_vec()
    }

    /// The arc the circular indicator under the first `W` draws its value with: Dart's
    /// `paints..arc(startAngle:, sweepAngle:)`.
    pub(crate) fn active_arc_of<W: 'static>(tester: &WidgetTester) -> (f64, f64) {
        let (painter, _) = painter_under::<W>(tester);
        let painter = painter.as_any();
        let circular = painter
            .downcast_ref::<CircularProgressIndicatorPainter>()
            .or_else(|| {
                painter
                    .downcast_ref::<RefreshProgressIndicatorPainter>()
                    .map(|refresh| &refresh.circular)
            })
            .expect("a circular indicator's painter");
        (circular.arc_start, circular.arc_sweep)
    }

    /// Dart's `paintsExactlyCountTimes(#drawArc, n)`: the stroked paths recorded.
    pub(crate) fn arc_count(ops: &[Op]) -> usize {
        ops.iter()
            .filter(|op| {
                matches!(op, Op::DrawPath { paint, .. } if matches!(paint.style, PaintStyle::Stroke(_)))
            })
            .count()
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    enum Kind {
        #[default]
        Rect,
        RRect,
        Circle,
        Arc,
    }

    /// One call of Dart's `paints` matcher; the fields it leaves null match anything.
    #[derive(Clone, Copy, Debug, Default)]
    pub(crate) struct Expect {
        kind: Kind,
        rect: Option<Rect>,
        rrect: Option<RRect>,
        center: Option<Offset>,
        radius: Option<f64>,
        color: Option<Color>,
        stroke_width: Option<f64>,
        stroke_cap: Option<StrokeCap>,
    }

    /// `..rect(rect:)`.
    pub(crate) fn rect(rect: Rect) -> Expect {
        Expect {
            kind: Kind::Rect,
            rect: Some(rect),
            ..Expect::default()
        }
    }

    /// `..rrect(rrect:)`.
    pub(crate) fn rrect(rrect: RRect) -> Expect {
        Expect {
            kind: Kind::RRect,
            rrect: Some(rrect),
            ..Expect::default()
        }
    }

    /// `..circle()`.
    pub(crate) fn circle() -> Expect {
        Expect {
            kind: Kind::Circle,
            ..Expect::default()
        }
    }

    /// `..arc()`.
    pub(crate) fn arc() -> Expect {
        Expect {
            kind: Kind::Arc,
            ..Expect::default()
        }
    }

    impl Expect {
        pub(crate) fn color(mut self, color: Color) -> Expect {
            self.color = Some(color);
            self
        }

        /// `..circle(x:, y:)`.
        pub(crate) fn at(mut self, x: f64, y: f64) -> Expect {
            self.center = Some(Offset::new(x, y));
            self
        }

        /// `..circle(radius:)`.
        pub(crate) fn radius(mut self, radius: f64) -> Expect {
            self.radius = Some(radius);
            self
        }

        /// `..arc(rect:)`.
        pub(crate) fn in_rect(mut self, rect: Rect) -> Expect {
            self.rect = Some(rect);
            self
        }

        pub(crate) fn stroke_width(mut self, stroke_width: f64) -> Expect {
            self.stroke_width = Some(stroke_width);
            self
        }

        pub(crate) fn stroke_cap(mut self, stroke_cap: StrokeCap) -> Expect {
            self.stroke_cap = Some(stroke_cap);
            self
        }

        fn matches(&self, op: &Op) -> bool {
            let paint = match op {
                Op::DrawRect { paint, .. } | Op::DrawPath { paint, .. } => paint,
                _ => return false,
            };
            if self
                .color
                .is_some_and(|color| paint.color != valo::Color::from(color))
            {
                return false;
            }
            match self.kind {
                Kind::Rect => matches!(
                    op,
                    Op::DrawRect { rect, .. }
                        if Some(*rect) == self.rect.map(valo::Rect::from)
                ),
                Kind::RRect => same_geometry(op, &rrect_op(self.rrect.expect("an rrect"))),
                Kind::Circle => self.matches_circle(op),
                Kind::Arc => self.matches_arc(op),
            }
        }

        fn matches_circle(&self, op: &Op) -> bool {
            let Op::DrawPath { path, .. } = op else {
                return false;
            };
            let Some(oval) = path.as_oval() else {
                return false;
            };
            let radius = f64::from(oval.width) / 2.0;
            let center = Offset::new(
                f64::from(oval.x) + radius,
                f64::from(oval.y) + f64::from(oval.height) / 2.0,
            );
            self.center.is_none_or(|expected| {
                close(center.dx(), expected.dx()) && close(center.dy(), expected.dy())
            }) && self.radius.is_none_or(|expected| close(radius, expected))
        }

        fn matches_arc(&self, op: &Op) -> bool {
            let Op::DrawPath { path, paint, .. } = op else {
                return false;
            };
            let PaintStyle::Stroke(stroke) = &paint.style else {
                return false;
            };
            self.stroke_width
                .is_none_or(|width| stroke.width == width as f32)
                && self.stroke_cap.is_none_or(|cap| stroke.cap == cap)
                && self.rect.is_none_or(|rect| on_oval_of(path, rect))
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-4
    }

    /// The op `draw_rrect` records for `rrect`.
    fn rrect_op(rrect: RRect) -> Op {
        let mut canvas = Canvas::new();
        draw_rrect(&mut canvas, rrect, &Paint::default());
        canvas.build().ops()[0].clone()
    }

    fn same_geometry(a: &Op, b: &Op) -> bool {
        match (a, b) {
            (Op::DrawRect { rect: a, .. }, Op::DrawRect { rect: b, .. }) => a == b,
            (Op::DrawPath { path: a, .. }, Op::DrawPath { path: b, .. }) => **a == **b,
            _ => false,
        }
    }

    /// Whether `path` runs along the oval inscribed in `rect`, sampled at every tenth of
    /// its length: an arc that `drawArc(rect, …)` drew.
    fn on_oval_of(path: &valo::Path, rect: Rect) -> bool {
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

    /// Dart's `paints..a()..b()`: the calls in order, others between them skipped.
    pub(crate) fn paints(ops: &[Op], expected: &[Expect]) -> bool {
        let mut ops = ops.iter();
        expected
            .iter()
            .all(|expect| ops.any(|op| expect.matches(op)))
    }

    /// Every `D` under the first `W`: Dart's `find.descendant(of: find.byType(W), matching:
    /// find.byType(D))`.
    pub(crate) fn descendants<W: 'static, D: 'static>(tester: &WidgetTester) -> Vec<AnyElement> {
        let of = first::<W>(tester);
        tester
            .find_by_type::<D>()
            .into_iter()
            .filter(|element| tester.ancestors(*element).contains(&of))
            .collect()
    }

    pub(crate) fn size_of(tester: &WidgetTester, element: AnyElement) -> Size {
        tester.global_rect(tester.render_box_of(element)).size()
    }

    pub(crate) fn controller_at(tester: &WidgetTester, value: f64) -> Ref<AnimationController> {
        AnimationController::new(
            &mut tester.cell.borrow_mut(),
            Some(value),
            None,
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        )
    }

    fn always(app: &mut App, color: Option<Color>) -> AnimationRef<Option<Color>> {
        app.create(AlwaysStoppedAnimation::new(color))
            .into_erased::<AnyAnimation<Option<Color>>>()
    }

    #[test]
    fn linear_progress_indicator_custom_min_height() {
        test_widgets(|tester| {
            pump_with(tester, |app| {
                themed(
                    theme(app),
                    directional(
                        TextDirection::Ltr,
                        centered_width(
                            200.0,
                            LinearProgressIndicator::new().value(0.25).min_height(2.0),
                        ),
                    ),
                )
            });
            let expected = [
                rect(Rect::from_ltrb(0.0, 0.0, 200.0, 2.0)),
                rect(Rect::from_ltrb(0.0, 0.0, 50.0, 2.0)),
            ];
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &expected
            ));

            // Same test, but using the theme
            pump_with(tester, |app| {
                themed(
                    theme_with(
                        app,
                        ProgressIndicatorThemeData::new().linear_min_height(2.0),
                    ),
                    directional(
                        TextDirection::Ltr,
                        centered_width(200.0, LinearProgressIndicator::new().value(0.25)),
                    ),
                )
            });
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &expected
            ));
        });
    }

    fn pump_linear(
        tester: &mut WidgetTester,
        direction: TextDirection,
        indicator: LinearProgressIndicator,
    ) {
        pump_with(tester, |app| {
            themed(
                theme(app),
                directional(direction, centered_width(200.0, indicator)),
            )
        });
    }

    #[test]
    fn linear_progress_indicator_paint_ltr() {
        test_widgets(|tester| {
            pump_linear(
                tester,
                TextDirection::Ltr,
                LinearProgressIndicator::new().value(0.25),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    rect(Rect::from_ltrb(0.0, 0.0, 200.0, 4.0)),
                    rect(Rect::from_ltrb(0.0, 0.0, 50.0, 4.0)),
                ]
            ));
            assert_eq!(transient_callback_count(tester), 0);
        });
    }

    #[test]
    fn linear_progress_indicator_paint_rtl() {
        test_widgets(|tester| {
            pump_linear(
                tester,
                TextDirection::Rtl,
                LinearProgressIndicator::new().value(0.25),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    rect(Rect::from_ltrb(0.0, 0.0, 200.0, 4.0)),
                    rect(Rect::from_ltrb(150.0, 0.0, 200.0, 4.0)),
                ]
            ));
            assert_eq!(transient_callback_count(tester), 0);
        });
    }

    fn line1_head_at_300ms() -> f64 {
        Interval::new(0.0, 750.0 / 1800.0, Rc::new(Cubic::new(0.2, 0.0, 0.8, 1.0)))
            .transform(300.0 / 1800.0)
    }

    #[test]
    fn linear_progress_indicator_indeterminate_ltr() {
        test_widgets(|tester| {
            pump_linear(tester, TextDirection::Ltr, LinearProgressIndicator::new());
            tester.pump_by(Duration::from_millis(300));
            let animation_value = line1_head_at_300ms();
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    // Track
                    rect(Rect::from_ltrb(animation_value * 200.0, 0.0, 200.0, 4.0)),
                    // Active indicator
                    rect(Rect::from_ltrb(0.0, 0.0, animation_value * 200.0, 4.0)),
                ]
            ));
            assert_eq!(transient_callback_count(tester), 1);
        });
    }

    #[test]
    fn linear_progress_indicator_indeterminate_rtl() {
        test_widgets(|tester| {
            pump_linear(tester, TextDirection::Rtl, LinearProgressIndicator::new());
            tester.pump_by(Duration::from_millis(300));
            let animation_value = line1_head_at_300ms();
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    rect(Rect::from_ltrb(
                        0.0,
                        0.0,
                        200.0 - animation_value * 200.0,
                        4.0
                    )),
                    rect(Rect::from_ltrb(
                        200.0 - animation_value * 200.0,
                        0.0,
                        200.0,
                        4.0
                    )),
                ]
            ));
            assert_eq!(transient_callback_count(tester), 1);
        });
    }

    #[test]
    fn linear_progress_indicator_with_colors() {
        test_widgets(|tester| {
            let default_theme = theme(&tester.cell.borrow());
            let track = Rect::from_ltrb(0.0, 0.0, 200.0, 4.0);
            let active = Rect::from_ltrb(0.0, 0.0, 50.0, 4.0);
            pump_with(tester, |_app| {
                MaterialApp::new()
                    .home(centered_width(
                        200.0,
                        LinearProgressIndicator::new().value(0.25),
                    ))
                    .into_widget()
            });

            // Defaults.
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    rect(track).color(default_theme.color_scheme.secondary_container()),
                    rect(active).color(default_theme.color_scheme.primary),
                ]
            ));

            // With valueColor & color provided
            pump_with(tester, |app| {
                let value_color = always(app, Some(Colors::WHITE));
                themed(
                    theme(app),
                    directional(
                        TextDirection::Ltr,
                        centered_width(
                            200.0,
                            LinearProgressIndicator::new()
                                .value(0.25)
                                .background_color(Colors::BLACK)
                                .color(Colors::BLUE.primary())
                                .value_color(value_color.clone()),
                        ),
                    ),
                )
            });

            // Should use valueColor
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[rect(track), rect(active).color(Colors::WHITE)]
            ));

            // With just color provided
            pump_with(tester, |app| {
                themed(
                    theme(app),
                    directional(
                        TextDirection::Ltr,
                        centered_width(
                            200.0,
                            LinearProgressIndicator::new()
                                .value(0.25)
                                .background_color(Colors::BLACK)
                                .color(Colors::WHITE12),
                        ),
                    ),
                )
            });

            // Should use color
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[rect(track), rect(active).color(Colors::WHITE12)]
            ));

            // With no color provided
            let primary_color = Color::new(0xff008800);
            pump_with(tester, |app| {
                let color_scheme = ColorScheme::from_swatch()
                    .build()
                    .copy_with()
                    .with_primary(primary_color);
                themed(
                    theme(app).copy_with().color_scheme(color_scheme).build(),
                    directional(
                        TextDirection::Ltr,
                        centered_width(
                            200.0,
                            LinearProgressIndicator::new()
                                .value(0.25)
                                .background_color(Colors::BLACK),
                        ),
                    ),
                )
            });

            // Should use the theme's primary color
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[rect(track), rect(active).color(primary_color)]
            ));

            // With ProgressIndicatorTheme colors
            let indicator_color = Color::new(0xff0000ff);
            pump_with(tester, |app| {
                themed(
                    theme_with(
                        app,
                        ProgressIndicatorThemeData::new()
                            .color(indicator_color)
                            .linear_track_color(Colors::BLACK),
                    ),
                    directional(
                        TextDirection::Ltr,
                        centered_width(200.0, LinearProgressIndicator::new().value(0.25)),
                    ),
                )
            });

            // Should use the progress indicator theme colors
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[rect(track), rect(active).color(indicator_color)]
            ));
        });
    }

    #[test]
    fn linear_progress_indicator_with_animation_with_null_colors() {
        test_widgets(|tester| {
            pump_with(tester, |app| {
                let value_color = always(app, None);
                themed(
                    theme(app),
                    directional(
                        TextDirection::Ltr,
                        centered_width(
                            200.0,
                            LinearProgressIndicator::new()
                                .value(0.25)
                                .value_color(value_color.clone())
                                .background_color(Colors::BLACK),
                        ),
                    ),
                )
            });
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    rect(Rect::from_ltrb(0.0, 0.0, 200.0, 4.0)),
                    rect(Rect::from_ltrb(0.0, 0.0, 50.0, 4.0)),
                ]
            ));
        });
    }

    #[test]
    fn circular_progress_indicator_stroke_width() {
        test_widgets(|tester| {
            pump_with(tester, |app| {
                themed(theme(app), CircularProgressIndicator::new())
            });
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[arc().stroke_width(4.0)]
            ));

            pump_with(tester, |app| {
                themed(
                    theme(app),
                    CircularProgressIndicator::new().stroke_width(16.0),
                )
            });
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[arc().stroke_width(16.0)]
            ));
        });
    }

    #[test]
    fn circular_progress_indicator_adaptive_stroke_width() {
        test_widgets(|tester| {
            tester.pump_widget(CircularProgressIndicator::adaptive().into_widget());

            // The default strokeWidth is 4.0
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[arc().stroke_width(4.0)]
            ));

            let theme_data =
                |app: &App| theme_with(app, ProgressIndicatorThemeData::new().stroke_width(10.0));
            pump_with(tester, |app| {
                themed(theme_data(app), CircularProgressIndicator::adaptive())
            });

            // Get the theme's strokeWidth.
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[arc().stroke_width(10.0)]
            ));

            pump_with(tester, |app| {
                themed(
                    theme_data(app),
                    CircularProgressIndicator::adaptive().stroke_width(16.0),
                )
            });

            // The strokeWidth parameter should override the theme's strokeWidth.
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[arc().stroke_width(16.0)]
            ));
        });
    }

    #[test]
    fn circular_progress_indicator_stroke_align() {
        test_widgets(|tester| {
            let cases = [
                (None, Offset::ZERO & Size::new(800.0, 600.0)),
                (
                    Some(CircularProgressIndicator::STROKE_ALIGN_INSIDE),
                    Offset::new(2.0, 2.0) & Size::new(796.0, 596.0),
                ),
                (
                    Some(CircularProgressIndicator::STROKE_ALIGN_OUTSIDE),
                    Offset::new(-2.0, -2.0) & Size::new(804.0, 604.0),
                ),
                // Unbounded alignment.
                (Some(2.0), Offset::new(-4.0, -4.0) & Size::new(808.0, 608.0)),
            ];
            for (stroke_align, expected) in cases {
                pump_with(tester, |app| {
                    let indicator = CircularProgressIndicator::new();
                    let indicator = match stroke_align {
                        Some(stroke_align) => indicator.stroke_align(stroke_align),
                        None => indicator,
                    };
                    themed(theme(app), indicator)
                });
                assert!(paints(
                    &painted_by::<CircularProgressIndicator>(tester),
                    &[arc().in_rect(expected)]
                ));
            }
        });
    }

    #[test]
    fn circular_progress_indicator_with_stroke_cap() {
        test_widgets(|tester| {
            tester.pump_widget(CircularProgressIndicator::new().into_widget());
            assert!(
                paints(
                    &painted_by::<CircularProgressIndicator>(tester),
                    &[arc().stroke_cap(StrokeCap::Square)]
                ),
                "Default indeterminate strokeCap is StrokeCap.square."
            );

            tester.pump_widget(directional(
                TextDirection::Ltr,
                CircularProgressIndicator::new().value(0.5),
            ));
            assert!(
                paints(
                    &painted_by::<CircularProgressIndicator>(tester),
                    &[arc().stroke_cap(StrokeCap::Butt)]
                ),
                "Default determinate strokeCap is StrokeCap.butt."
            );

            tester.pump_widget(
                CircularProgressIndicator::new()
                    .stroke_cap(StrokeCap::Butt)
                    .into_widget(),
            );
            assert!(
                paints(
                    &painted_by::<CircularProgressIndicator>(tester),
                    &[arc().stroke_cap(StrokeCap::Butt)]
                ),
                "strokeCap can be set to StrokeCap.butt, and will not be overridden."
            );

            tester.pump_widget(
                CircularProgressIndicator::new()
                    .stroke_cap(StrokeCap::Round)
                    .into_widget(),
            );
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[arc().stroke_cap(StrokeCap::Round)]
            ));
        });
    }

    #[test]
    fn linear_progress_indicator_with_indicator_border_radius() {
        test_widgets(|tester| {
            pump_with(tester, |app| {
                themed(
                    theme(app),
                    directional(
                        TextDirection::Ltr,
                        Center::new().child(
                            SizedBox::new().width(100.0).height(4.0).child(
                                LinearProgressIndicator::new()
                                    .value(0.25)
                                    .border_radius(BorderRadius::all(Radius::circular(10.0))),
                            ),
                        ),
                    ),
                )
            });
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    rrect(RRect::from_ltrbr(
                        0.0,
                        0.0,
                        100.0,
                        4.0,
                        Radius::circular(10.0)
                    )),
                    rrect(RRect::from_rect_and_radius(
                        Rect::from_ltrb(0.0, 0.0, 25.0, 4.0),
                        Radius::circular(10.0),
                    )),
                ]
            ));
            assert_eq!(transient_callback_count(tester), 0);
        });
    }

    #[test]
    fn linear_progress_indicator_reflects_controller_value() {
        test_widgets(|tester| {
            let controller = controller_at(tester, 0.5);
            pump_with(tester, |_app| {
                MaterialApp::new()
                    .home(Material::new().child(centered_width(
                        200.0,
                        LinearProgressIndicator::new().controller(controller.clone()),
                    )))
                    .into_widget()
            });
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[rect(Rect::from_ltrb(127.79541015625, 0.0, 200.0, 4.0))]
            ));
            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    #[test]
    fn circular_progress_indicator_paint_colors() {
        test_widgets(|tester| {
            let green = Color::new(0xFF00FF00);
            let blue = Color::new(0xFF0000FF);
            let red = Color::new(0xFFFF0000);

            // With valueColor & color provided
            pump_with(tester, |app| {
                let value_color = always(app, Some(blue));
                themed(
                    theme(app),
                    CircularProgressIndicator::new()
                        .color(red)
                        .value_color(value_color.clone()),
                )
            });
            let ops = painted_by::<CircularProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(blue)]));

            // With just color provided
            pump_with(tester, |app| {
                themed(theme(app), CircularProgressIndicator::new().color(red))
            });
            let ops = painted_by::<CircularProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(red)]));

            // With no color provided
            pump_with(tester, |app| {
                let color_scheme = ColorScheme::from_swatch()
                    .build()
                    .copy_with()
                    .with_primary(green);
                themed(
                    theme(app).copy_with().color_scheme(color_scheme).build(),
                    CircularProgressIndicator::new(),
                )
            });
            let ops = painted_by::<CircularProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(green)]));

            // With background
            pump_with(tester, |app| {
                themed(
                    theme(app),
                    CircularProgressIndicator::new()
                        .background_color(green)
                        .color(blue),
                )
            });
            let ops = painted_by::<CircularProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 2);
            assert!(paints(&ops, &[arc().color(green), arc().color(blue)]));

            // With ProgressIndicatorTheme
            pump_with(tester, |app| {
                themed(
                    theme_with(
                        app,
                        ProgressIndicatorThemeData::new()
                            .color(green)
                            .circular_track_color(blue),
                    ),
                    CircularProgressIndicator::new(),
                )
            });
            let ops = painted_by::<CircularProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 2);
            assert!(paints(&ops, &[arc().color(blue), arc().color(green)]));
        });
    }

    /// The `Material` the first `RefreshProgressIndicator` builds.
    fn refresh_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let material = descendants::<RefreshProgressIndicator, Material>(tester)[0];
        tester.widget(material, read)
    }

    #[test]
    fn refresh_progress_indicator_paint_colors() {
        test_widgets(|tester| {
            let green = Color::new(0xFF00FF00);
            let blue = Color::new(0xFF0000FF);
            let red = Color::new(0xFFFF0000);

            // With valueColor & color provided
            pump_with(tester, |app| {
                let value_color = always(app, Some(blue));
                themed(
                    theme(app),
                    RefreshProgressIndicator::new()
                        .color(red)
                        .value_color(value_color.clone()),
                )
            });
            let ops = painted_by::<RefreshProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(blue)]));

            // With just color provided
            pump_with(tester, |app| {
                themed(theme(app), RefreshProgressIndicator::new().color(red))
            });
            let ops = painted_by::<RefreshProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(red)]));

            // With no color provided
            pump_with(tester, |app| {
                let color_scheme = ColorScheme::from_swatch()
                    .build()
                    .copy_with()
                    .with_primary(green);
                themed(
                    theme(app).copy_with().color_scheme(color_scheme).build(),
                    RefreshProgressIndicator::new(),
                )
            });
            let ops = painted_by::<RefreshProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(green)]));

            // With background
            pump_with(tester, |app| {
                themed(
                    theme(app),
                    RefreshProgressIndicator::new()
                        .color(blue)
                        .background_color(green),
                )
            });
            let ops = painted_by::<RefreshProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(blue)]));
            refresh_material(tester, |material| {
                assert_eq!(material.r#type, MaterialType::Circle);
                assert_eq!(material.color, Some(green));
            });

            // With ProgressIndicatorTheme
            pump_with(tester, |app| {
                themed(
                    theme_with(
                        app,
                        ProgressIndicatorThemeData::new()
                            .color(green)
                            .refresh_background_color(blue),
                    ),
                    RefreshProgressIndicator::new(),
                )
            });
            let ops = painted_by::<RefreshProgressIndicator>(tester);
            assert_eq!(arc_count(&ops), 1);
            assert!(paints(&ops, &[arc().color(green)]));
            refresh_material(tester, |material| {
                assert_eq!(material.r#type, MaterialType::Circle);
                assert_eq!(material.color, Some(blue));
            });
        });
    }

    #[test]
    fn refresh_progress_indicator_with_a_round_indicator() {
        test_widgets(|tester| {
            tester.pump_widget(RefreshProgressIndicator::new().into_widget());
            assert!(
                paints(
                    &painted_by::<RefreshProgressIndicator>(tester),
                    &[arc().stroke_cap(StrokeCap::Square)]
                ),
                "Default indeterminate strokeCap is StrokeCap.square"
            );

            pump_with(tester, |app| {
                themed(
                    theme(app),
                    directional(
                        TextDirection::Ltr,
                        centered_width(
                            200.0,
                            RefreshProgressIndicator::new().stroke_cap(StrokeCap::Round),
                        ),
                    ),
                )
            });
            assert!(paints(
                &painted_by::<RefreshProgressIndicator>(tester),
                &[arc().stroke_cap(StrokeCap::Round)]
            ));
        });
    }

    #[test]
    fn indeterminate_refresh_progress_indicator_keeps_spinning_until_end_of_time_approximate() {
        // Regression test for https://github.com/flutter/flutter/issues/13782
        test_widgets(|tester| {
            pump_with(tester, |app| {
                themed(
                    theme(app),
                    directional(
                        TextDirection::Ltr,
                        centered_width(200.0, RefreshProgressIndicator::new()),
                    ),
                )
            });
            assert!(transient_callback_count(tester) > 0);

            tester.pump_by(Duration::from_secs(5));
            assert!(transient_callback_count(tester) > 0);

            tester.pump_by(Duration::from_millis(1));
            assert!(transient_callback_count(tester) > 0);

            tester.pump_by(Duration::from_secs(9999 * 24 * 60 * 60));
            assert!(transient_callback_count(tester) > 0);
        });
    }

    #[test]
    fn determinate_circular_progress_indicator_stops_the_animator() {
        test_widgets(|tester| {
            let pump_value = |tester: &mut WidgetTester, progress_value: Option<f64>| {
                pump_with(tester, |app| {
                    let indicator = CircularProgressIndicator::new();
                    let indicator = match progress_value {
                        Some(value) => indicator.value(value),
                        None => indicator,
                    };
                    themed(
                        theme(app),
                        directional(TextDirection::Ltr, Center::new().child(indicator)),
                    )
                });
            };
            pump_value(tester, None);
            assert!(transient_callback_count(tester) > 0);

            pump_value(tester, Some(1.0));
            tester.pump_by(Duration::from_millis(1));
            assert_eq!(transient_callback_count(tester), 0);

            pump_value(tester, None);
            tester.pump_by(Duration::from_millis(1));
            assert!(transient_callback_count(tester) > 0);
        });
    }

    #[test]
    fn an_indeterminate_indicator_stops_ticking_while_its_ticker_mode_is_off() {
        test_widgets(|tester| {
            let pump_enabled = |tester: &mut WidgetTester, enabled: bool| {
                pump_with(tester, |app| {
                    themed(
                        theme(app),
                        TickerMode::new(
                            enabled,
                            Center::new().child(CircularProgressIndicator::new()),
                        ),
                    )
                });
            };
            pump_enabled(tester, false);
            assert_eq!(transient_callback_count(tester), 0);
            let before = active_arc_of::<CircularProgressIndicator>(tester);
            tester.pump_by(Duration::from_millis(500));
            assert_eq!(active_arc_of::<CircularProgressIndicator>(tester), before);

            pump_enabled(tester, true);
            assert_eq!(transient_callback_count(tester), 1);
        });
    }

    #[test]
    fn linear_progress_indicator_with_height_12_0() {
        test_widgets(|tester| {
            for (height, painted) in [(12.0, 12.0), (3.0, 3.0), (4.0, 4.0)] {
                pump_with(tester, |app| {
                    themed(
                        theme(app),
                        directional(
                            TextDirection::Ltr,
                            Center::new().child(
                                SizedBox::new()
                                    .width(100.0)
                                    .height(height)
                                    .child(LinearProgressIndicator::new().value(0.25)),
                            ),
                        ),
                    )
                });
                assert!(paints(
                    &painted_by::<LinearProgressIndicator>(tester),
                    &[
                        rect(Rect::from_ltrb(0.0, 0.0, 100.0, painted)),
                        rect(Rect::from_ltrb(0.0, 0.0, 25.0, painted)),
                    ]
                ));
                assert_eq!(transient_callback_count(tester), 0);
            }
        });
    }

    fn pump_adaptive(tester: &mut WidgetTester, indicator: CircularProgressIndicator) {
        pump_with(tester, |app| {
            scaffold_app(Some(theme(app)), Material::new().child(indicator))
        });
    }

    const APPLE: [TargetPlatform; 2] = [TargetPlatform::IOS, TargetPlatform::MacOS];

    #[test]
    fn adaptive_circular_progress_indicator_displays_cupertino_activity_indicator_in_ios() {
        test_widgets(|_| {
            for platform in APPLE {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    pump_adaptive(tester, CircularProgressIndicator::adaptive());
                    assert_eq!(tester.find_by_type::<CupertinoActivityIndicator>().len(), 1);
                });
            }
        });
    }

    #[test]
    fn adaptive_circular_progress_indicator_displays_cupertino_activity_indicator_in_ios_macos() {
        test_widgets(|_| {
            for platform in APPLE {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    pump_adaptive(tester, CircularProgressIndicator::adaptive().value(0.5));
                    let indicators = tester.find_by_type::<CupertinoActivityIndicator>();
                    assert_eq!(indicators.len(), 1);
                    let actual_progress = tester
                        .widget(indicators[0], |indicator: &CupertinoActivityIndicator| {
                            indicator.progress
                        });
                    assert_eq!(actual_progress, 0.5);
                });
            }
        });
    }

    #[test]
    fn adaptive_circular_progress_indicator_can_use_background_color_to_change_tick_color_for_ios()
    {
        test_widgets(|_| {
            for platform in APPLE {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    let color = Color::new(0xFF5D3FD3);
                    pump_adaptive(
                        tester,
                        CircularProgressIndicator::adaptive().background_color(color),
                    );
                    assert!(paints(
                        &painted_by::<CupertinoActivityIndicator>(tester),
                        &[
                            rrect(RRect::from_ltrbxy(-1.0, -10.0 / 3.0, 1.0, -10.0, 1.0, 1.0))
                                // The value of 47 comes from the alpha that is applied to the first
                                // tick.
                                .color(color.with_alpha(47))
                        ]
                    ));
                });
            }
        });
    }

    #[test]
    fn adaptive_circular_progress_indicator_does_not_display_cupertino_activity_indicator_in_non_ios()
     {
        test_widgets(|_| {
            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::Windows,
                TargetPlatform::Linux,
            ] {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    pump_adaptive(tester, CircularProgressIndicator::adaptive());
                    assert!(
                        tester
                            .find_by_type::<CupertinoActivityIndicator>()
                            .is_empty()
                    );
                });
            }
        });
    }

    #[test]
    fn progress_indicator_theme_wrap_always_creates_a_new_progress_indicator_theme() {
        test_widgets(|tester| {
            let theme_data = ProgressIndicatorThemeData::new()
                .color(Color::new(0xFFFF0000))
                .linear_track_color(Color::new(0xFF00FF00));
            let builder_context = Rc::new(std::cell::Cell::new(None));
            pump_with(tester, |app| {
                let builder_context = Rc::clone(&builder_context);
                MaterialApp::new()
                    .home(themed(
                        theme(app),
                        ProgressIndicatorTheme::new(
                            theme_data.clone(),
                            Builder::new(move |_app, context| {
                                builder_context.set(Some(context));
                                LinearProgressIndicator::new().value(0.5).into_widget()
                            }),
                        ),
                    ))
                    .into_widget()
            });
            let progress_theme = first::<ProgressIndicatorTheme>(tester);
            let wrapped_theme = {
                let mut app = tester.cell.borrow_mut();
                let widget = progress_theme.widget(&app).clone();
                let progress_theme =
                    downcast_widget::<ProgressIndicatorTheme>(&*widget).expect("found by type");
                progress_theme.wrap(
                    &mut app,
                    builder_context.get().expect("the builder ran"),
                    SizedBox::new().into_widget(),
                )
            };

            // Make sure the returned widget is a new ProgressIndicatorTheme instance
            // with the same theme data as the original.
            let app = tester.cell.borrow();
            assert!(!Rc::ptr_eq(&wrapped_theme, progress_theme.widget(&app)));
            let wrapped = downcast_widget::<ProgressIndicatorTheme>(&*wrapped_theme)
                .expect("a ProgressIndicatorTheme");
            assert_eq!(wrapped.data, theme_data);
        });
    }

    #[test]
    fn material3_default_size_of_circular_progress_indicator() {
        test_widgets(|tester| {
            pump_with(tester, |_app| {
                scaffold_app(
                    None,
                    Material::new().child(CircularProgressIndicator::new()),
                )
            });
            let indicator = first::<CircularProgressIndicator>(tester);
            assert_eq!(size_of(tester, indicator), Size::new(36.0, 36.0));
        });
    }

    #[test]
    fn material3_default_size_of_circular_progress_indicator_when_year2023_is_false() {
        test_widgets(|tester| {
            pump_with(tester, |_app| {
                scaffold_app(
                    None,
                    Material::new().child(CircularProgressIndicator::new().year2023(false)),
                )
            });
            let indicator = first::<CircularProgressIndicator>(tester);
            assert_eq!(size_of(tester, indicator), Size::new(48.0, 48.0));
        });
    }

    #[test]
    fn refresh_progress_indicator_using_fields_correctly() {
        test_widgets(|tester| {
            let fields = |tester: &WidgetTester| {
                let material = descendants::<RefreshProgressIndicator, Material>(tester)[0];
                let elevation = tester.widget(material, |material: &Material| material.elevation);
                let padding = descendants::<RefreshProgressIndicator, Padding>(tester)[0];
                let padding = tester.widget(padding, |padding: &Padding| padding.padding);
                let inner_padding = *descendants::<Material, Padding>(tester)
                    .last()
                    .expect("a Padding in the Material");
                let inner_padding =
                    tester.widget(inner_padding, |padding: &Padding| padding.padding);
                (elevation, padding, inner_padding)
            };

            // With default values.
            pump_with(tester, |app| {
                themed(theme(app), RefreshProgressIndicator::new())
            });
            assert_eq!(
                fields(tester),
                (
                    2.0,
                    EdgeInsets::all(4.0).into(),
                    EdgeInsets::all(12.0).into()
                )
            );

            // With values provided.
            let test_elevation = 1.0;
            let test_indicator_margin: EdgeInsetsGeometry = EdgeInsets::all(6.0).into();
            let test_indicator_padding: EdgeInsetsGeometry = EdgeInsets::all(10.0).into();
            pump_with(tester, |app| {
                themed(
                    theme(app),
                    RefreshProgressIndicator::new()
                        .elevation(test_elevation)
                        .indicator_margin(test_indicator_margin)
                        .indicator_padding(test_indicator_padding),
                )
            });
            assert_eq!(
                fields(tester),
                (
                    test_elevation,
                    test_indicator_margin,
                    test_indicator_padding
                )
            );
        });
    }

    fn pump_linear_2024(
        tester: &mut WidgetTester,
        direction: TextDirection,
        indicator: LinearProgressIndicator,
    ) {
        tester.pump_widget(directional(
            direction,
            centered_width(200.0, indicator.year2023(false)),
        ));
    }

    #[test]
    fn linear_progress_indicator_default_stop_indicator_when_year2023_is_false() {
        test_widgets(|tester| {
            let primary = theme(&tester.cell.borrow()).color_scheme.primary;
            pump_linear_2024(
                tester,
                TextDirection::Ltr,
                LinearProgressIndicator::new().value(0.5),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[circle().at(198.0, 2.0).radius(2.0).color(primary)]
            ));

            pump_linear_2024(
                tester,
                TextDirection::Rtl,
                LinearProgressIndicator::new().value(0.5),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[circle().at(2.0, 2.0).radius(2.0).color(primary)]
            ));
        });
    }

    #[test]
    fn indeterminate_linear_progress_indicator_does_not_paint_stop_indicator() {
        test_widgets(|tester| {
            let primary = theme(&tester.cell.borrow()).color_scheme.primary;
            let stop_indicator = [circle().at(198.0, 2.0).radius(2.0).color(primary)];

            // Determinate LinearProgressIndicator paints stop indicator.
            pump_linear_2024(
                tester,
                TextDirection::Ltr,
                LinearProgressIndicator::new().value(0.5),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &stop_indicator
            ));

            // Indeterminate LinearProgressIndicator does not paint stop indicator.
            pump_linear_2024(tester, TextDirection::Ltr, LinearProgressIndicator::new());
            assert!(!paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &stop_indicator
            ));
        });
    }

    #[test]
    fn can_customise_linear_progress_indicator_stop_indicator_when_year2023_is_false() {
        test_widgets(|tester| {
            let stop_indicator_color = Color::new(0xFF00FF00);
            let stop_indicator_radius = 5.0;
            let indicator = || LinearProgressIndicator::new().min_height(20.0).value(0.5);

            // Test customized stop indicator.
            pump_linear_2024(
                tester,
                TextDirection::Ltr,
                indicator()
                    .stop_indicator_color(stop_indicator_color)
                    .stop_indicator_radius(stop_indicator_radius),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[circle()
                    .at(190.0, 10.0)
                    .radius(stop_indicator_radius)
                    .color(stop_indicator_color)]
            ));

            // Remove stop indicator.
            pump_linear_2024(
                tester,
                TextDirection::Ltr,
                indicator().stop_indicator_radius(0.0),
            );
            assert!(!paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[circle().color(stop_indicator_color)]
            ));

            // Test stop indicator with transparent color.
            pump_linear_2024(
                tester,
                TextDirection::Ltr,
                indicator().stop_indicator_color(Color::new(0x00000000)),
            );
            // Valo drops a draw whose paint changes nothing, so the transparent circle Dart
            // records is read from the painter the indicator built.
            let (painter, _) = painter_under::<LinearProgressIndicator>(tester);
            let painter = painter
                .as_any()
                .downcast_ref::<LinearProgressIndicatorPainter>()
                .expect("a linear indicator's painter");
            assert_eq!(painter.stop_indicator_color, Some(Color::new(0x00000000)));
            assert!(
                painter
                    .stop_indicator_radius
                    .is_some_and(|stop_indicator_radius| stop_indicator_radius > 0.0)
            );
            assert!(!paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[circle()]
            ));
        });
    }

    #[test]
    fn stop_indicator_size_cannot_be_larger_than_the_progress_indicator() {
        test_widgets(|tester| {
            let primary = theme(&tester.cell.borrow()).color_scheme.primary;
            // Test stop indicator radius equals to minHeight, then larger than minHeight: the
            // radius is clamped to minHeight.
            for stop_indicator_radius in [10.0, 30.0] {
                pump_linear_2024(
                    tester,
                    TextDirection::Ltr,
                    LinearProgressIndicator::new()
                        .stop_indicator_radius(stop_indicator_radius)
                        .min_height(20.0)
                        .value(0.5),
                );
                assert!(paints(
                    &painted_by::<LinearProgressIndicator>(tester),
                    &[circle().at(190.0, 10.0).radius(10.0).color(primary)]
                ));
            }
        });
    }

    #[test]
    fn linear_progress_indicator_default_track_gap_when_year2023_is_false() {
        test_widgets(|tester| {
            let colors = theme(&tester.cell.borrow()).color_scheme;
            let default_track_gap = 4.0;
            let radius = Radius::circular(2.0);

            // Test default track gap in LTR.
            pump_linear_2024(
                tester,
                TextDirection::Ltr,
                LinearProgressIndicator::new().value(0.5),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    // Track.
                    rrect(RRect::from_ltrbr(
                        100.0 + default_track_gap,
                        0.0,
                        200.0,
                        4.0,
                        radius
                    ))
                    .color(colors.secondary_container()),
                    // Active track.
                    rrect(RRect::from_ltrbr(0.0, 0.0, 100.0, 4.0, radius)).color(colors.primary),
                ]
            ));

            // Test default track gap in RTL.
            pump_linear_2024(
                tester,
                TextDirection::Rtl,
                LinearProgressIndicator::new().value(0.5),
            );
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    // Track.
                    rrect(RRect::from_ltrbr(
                        0.0,
                        0.0,
                        100.0 - default_track_gap,
                        4.0,
                        radius
                    ))
                    .color(colors.secondary_container()),
                    // Active track.
                    rrect(RRect::from_ltrbr(100.0, 0.0, 200.0, 4.0, radius)).color(colors.primary),
                ]
            ));
        });
    }

    #[test]
    fn can_customise_linear_progress_indicator_track_gap_when_year2023_is_false() {
        test_widgets(|tester| {
            let colors = theme(&tester.cell.borrow()).color_scheme;
            let radius = Radius::circular(2.0);
            for (track_gap, track_left) in [(12.0, 100.0 + 12.0), (0.0, 0.0)] {
                pump_linear_2024(
                    tester,
                    TextDirection::Ltr,
                    LinearProgressIndicator::new()
                        .track_gap(track_gap)
                        .value(0.5),
                );
                assert!(paints(
                    &painted_by::<LinearProgressIndicator>(tester),
                    &[
                        // Track.
                        rrect(RRect::from_ltrbr(track_left, 0.0, 200.0, 4.0, radius))
                            .color(colors.secondary_container()),
                        // Stop indicator.
                        circle(),
                        // Active indicator.
                        rrect(RRect::from_ltrbr(0.0, 0.0, 100.0, 4.0, radius))
                            .color(colors.primary),
                    ]
                ));
            }
        });
    }

    /// `tester.getSize(find.byType(CircularProgressIndicator))` and that of the
    /// `ConstrainedBox` under it.
    pub(crate) fn circular_sizes(tester: &WidgetTester) -> (Size, Size) {
        let indicator = first::<CircularProgressIndicator>(tester);
        let constrained_box = descendants::<CircularProgressIndicator, ConstrainedBox>(tester)[0];
        (size_of(tester, indicator), size_of(tester, constrained_box))
    }

    pub(crate) fn padded(size: Size, padding: EdgeInsetsGeometry) -> Size {
        Size::new(
            size.width() + padding.horizontal(),
            size.height() + padding.vertical(),
        )
    }

    #[test]
    fn default_determinate_circular_progress_indicator_when_year2023_is_false() {
        test_widgets(|tester| {
            let padding: EdgeInsetsGeometry = EdgeInsets::all(4.0).into();
            pump_with(tester, |app| {
                crate::test_support::material_app(
                    theme(app),
                    Center::new()
                        .child(CircularProgressIndicator::new().year2023(false).value(0.5))
                        .into_widget(),
                )
            });
            let colors = theme(&tester.cell.borrow()).color_scheme;
            let (indicator, indicator_box) = circular_sizes(tester);
            assert_eq!(indicator, padded(indicator_box, padding));
            let arc_rect = Rect::from_ltrb(2.0, 2.0, 38.0, 38.0);
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[
                    // Track.
                    arc()
                        .in_rect(arc_rect)
                        .color(colors.secondary_container())
                        .stroke_width(4.0)
                        .stroke_cap(StrokeCap::Round),
                    // Active indicator.
                    arc()
                        .in_rect(arc_rect)
                        .color(colors.primary)
                        .stroke_width(4.0)
                        .stroke_cap(StrokeCap::Round),
                ]
            ));
        });
    }

    #[test]
    fn default_indeterminate_circular_progress_indicator_when_year2023_is_false() {
        test_widgets(|tester| {
            let padding: EdgeInsetsGeometry = EdgeInsets::all(4.0).into();
            pump_with(tester, |app| {
                crate::test_support::material_app(
                    theme(app),
                    Center::new()
                        .child(CircularProgressIndicator::new().year2023(false))
                        .into_widget(),
                )
            });

            // Advance the animation.
            tester.pump_by(Duration::from_millis(200));

            let colors = theme(&tester.cell.borrow()).color_scheme;
            let (indicator, indicator_box) = circular_sizes(tester);
            assert_eq!(indicator, padded(indicator_box, padding));
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[
                    // Active indicator.
                    arc()
                        .in_rect(Rect::from_ltrb(2.0, 2.0, 38.0, 38.0))
                        .color(colors.primary)
                        .stroke_width(4.0)
                        .stroke_cap(StrokeCap::Round),
                ]
            ));
        });
    }

    #[test]
    fn can_override_circular_progress_indicator_stroke_cap_when_year2023_is_false() {
        test_widgets(|tester| {
            pump_with(tester, |_app| {
                MaterialApp::new()
                    .home(
                        Center::new().child(
                            CircularProgressIndicator::new()
                                .year2023(false)
                                .stroke_cap(StrokeCap::Square)
                                .value(0.5),
                        ),
                    )
                    .into_widget()
            });
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[
                    // Track.
                    arc().stroke_cap(StrokeCap::Square),
                    // Active indicator.
                    arc().stroke_cap(StrokeCap::Square),
                ]
            ));
        });
    }

    #[test]
    fn circular_progress_indicator_constraints_can_override_default_size() {
        test_widgets(|tester| {
            let size = Size::new(64.0, 64.0);
            pump_with(tester, |_app| {
                MaterialApp::new()
                    .home(
                        Center::new().child(
                            CircularProgressIndicator::new()
                                .constraints(
                                    BoxConstraints::new()
                                        .min_width(size.width())
                                        .min_height(size.height()),
                                )
                                .value(0.5),
                        ),
                    )
                    .into_widget()
            });
            let indicator = first::<CircularProgressIndicator>(tester);
            assert_eq!(size_of(tester, indicator), size);
        });
    }

    #[test]
    fn circular_progress_indicator_padding_can_be_customized() {
        test_widgets(|tester| {
            let padding: EdgeInsetsGeometry = EdgeInsets::all(12.0).into();
            pump_with(tester, |_app| {
                MaterialApp::new()
                    .home(
                        Center::new().child(
                            CircularProgressIndicator::new()
                                .padding(padding)
                                .year2023(false)
                                .value(0.5),
                        ),
                    )
                    .into_widget()
            });
            let (indicator, indicator_box) = circular_sizes(tester);
            assert_eq!(indicator, padded(indicator_box, padding));
        });
    }

    #[test]
    fn circular_progress_indicator_reflects_controller_value() {
        test_widgets(|tester| {
            let controller = controller_at(tester, 0.5);
            pump_with(tester, |_app| {
                MaterialApp::new()
                    .home(Material::new().child(Center::new().child(
                        SizedBox::square(Some(200.0)).child(AnimatedBuilder::new(
                            Rc::new(controller.erase::<AnyAnimation<f64>>()),
                            {
                                let controller = controller.clone();
                                move |_app, _context, _child| {
                                    CircularProgressIndicator::new()
                                        .controller(controller.clone())
                                        .into_widget()
                                }
                            },
                        )),
                    )))
                    .into_widget()
            });
            assert_eq!(
                active_arc_of::<CircularProgressIndicator>(tester),
                (std::f64::consts::FRAC_PI_2, 0.001)
            );
            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    #[test]
    fn indicators_do_not_crash_at_zero_area() {
        fn pump_shrunk(tester: &mut WidgetTester, indicator: WidgetRef) {
            pump_with(tester, |_app| {
                MaterialApp::new()
                    .home(Center::new().child(SizedBox::shrink().child(indicator)))
                    .into_widget()
            });
        }
        test_widgets(|tester| {
            pump_shrunk(tester, RefreshProgressIndicator::new().into_widget());
            let indicator = first::<RefreshProgressIndicator>(tester);
            assert_eq!(size_of(tester, indicator), Size::ZERO);

            pump_shrunk(tester, CircularProgressIndicator::new().into_widget());
            let indicator = first::<CircularProgressIndicator>(tester);
            assert_eq!(size_of(tester, indicator), Size::ZERO);

            pump_shrunk(tester, LinearProgressIndicator::new().into_widget());
            let indicator = first::<LinearProgressIndicator>(tester);
            assert_eq!(size_of(tester, indicator), Size::ZERO);
        });
    }
}
