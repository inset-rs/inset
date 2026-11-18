//! Flutter counterpart: material_ui's `progress_indicator_theme.dart`.

use waft_animation::AnimationController;
use waft_embedder::StrokeCap;
use waft_embedder::{Color, lerp_double};
use waft_foundation::{App, Ref};
use waft_painting::{BorderRadiusGeometry, EdgeInsetsGeometry};
use waft_rendering::BoxConstraints;
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::theme::Theme;

/// Overrides the default values of visual properties for descendant
/// `ProgressIndicator` widgets.
///
/// Descendant widgets obtain the current [`ProgressIndicatorThemeData`] object
/// with [`ProgressIndicatorTheme::of`]. Instances of [`ProgressIndicatorThemeData`]
/// can be customized with [`ProgressIndicatorThemeData::copy_with`].
///
/// Typically a [`ProgressIndicatorThemeData`] is specified as part of the overall
/// `Theme` with `ThemeData::progress_indicator_theme`.
///
/// See also:
///
///  * [`ProgressIndicatorTheme`], an `InheritedWidget` that propagates the
///    theme down its subtree.
///  * `ThemeData::progress_indicator_theme`, which describes the defaults for
///    any progress indicators as part of the application's `ThemeData`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProgressIndicatorThemeData {
    /// The color of the `ProgressIndicator`'s indicator.
    ///
    /// If null, then it will use `ColorScheme::primary` of the ambient
    /// `ThemeData::color_scheme`.
    ///
    /// See also:
    ///
    ///  * `ProgressIndicator.color`, which specifies the indicator color for a
    ///    specific progress indicator.
    ///  * `ProgressIndicator.valueColor`, which specifies the indicator color
    ///    a an animated color.
    pub color: Option<Color>,

    /// Color of the track being filled by the linear indicator.
    ///
    /// If `LinearProgressIndicator.backgroundColor` is null then the
    /// ambient [`linear_track_color`](Self::linear_track_color) will be used.
    /// If that is null, then the ambient theme's `ColorScheme::background`
    /// will be used to draw the track.
    pub linear_track_color: Option<Color>,

    /// The minimum height of the line used to draw the linear indicator.
    ///
    /// If `LinearProgressIndicator.minHeight` is null then it will use the
    /// ambient [`linear_min_height`](Self::linear_min_height). If that is null
    /// it will use 4dp.
    pub linear_min_height: Option<f64>,

    /// Color of the circular track being filled by the circular indicator.
    ///
    /// If `CircularProgressIndicator.backgroundColor` is null then the
    /// ambient [`circular_track_color`](Self::circular_track_color) will be used.
    /// If that is null, then the track will not be painted.
    pub circular_track_color: Option<Color>,

    /// Background color of that fills the circle under the refresh indicator.
    ///
    /// If `RefreshIndicator.backgroundColor` is null then the
    /// ambient [`refresh_background_color`](Self::refresh_background_color) will be used.
    /// If that is null, then the ambient theme's `ThemeData::canvas_color`
    /// will be used.
    pub refresh_background_color: Option<Color>,

    /// Overrides the border radius of the `ProgressIndicator`.
    pub border_radius: Option<BorderRadiusGeometry>,

    /// Overrides the stop indicator color of the `LinearProgressIndicator`.
    ///
    /// If `LinearProgressIndicator.year2023` is true or `ThemeData::use_material3`
    /// is false, then no stop indicator will be drawn.
    pub stop_indicator_color: Option<Color>,

    /// Overrides the stop indicator radius of the `LinearProgressIndicator`.
    ///
    /// If `LinearProgressIndicator.year2023` is true or `ThemeData::use_material3`
    /// is false, then no stop indicator will be drawn.
    pub stop_indicator_radius: Option<f64>,

    /// Overrides the stroke width of the `CircularProgressIndicator`.
    pub stroke_width: Option<f64>,

    /// Overrides the stroke align of the `CircularProgressIndicator`.
    pub stroke_align: Option<f64>,

    /// Overrides the stroke cap of the `CircularProgressIndicator`.
    pub stroke_cap: Option<StrokeCap>,

    /// Overrides the constraints of the `CircularProgressIndicator`.
    pub constraints: Option<BoxConstraints>,

    /// Overrides the active indicator and the background track.
    ///
    /// If `CircularProgressIndicator.year2023` is true or `ThemeData::use_material3`
    /// is false, then no track gap will be drawn.
    ///
    /// If `LinearProgressIndicator.year2023` is true or `ThemeData::use_material3`
    /// is false, then no track gap will be drawn.
    pub track_gap: Option<f64>,

    /// Overrides the padding of the `CircularProgressIndicator`.
    pub circular_track_padding: Option<EdgeInsetsGeometry>,

    /// Overrides the `CircularProgressIndicator.year2023` and
    /// `LinearProgressIndicator.year2023` properties.
    ///
    /// When true, the `CircularProgressIndicator` and `LinearProgressIndicator`
    /// will use the 2023 Material Design 3 appearance. Defaults to true.
    ///
    /// If this is set to false, the `CircularProgressIndicator` and
    /// `LinearProgressIndicator` will use the latest Material Design 3 appearance,
    /// which was introduced in December 2023.
    ///
    /// If `ThemeData::use_material3` is false, then this property is ignored.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 progress indicator appearance. Defaults to true. In the future, this flag will default to false. Use ProgressIndicatorThemeData to customize individual properties. This feature was deprecated after v3.27.0-0.2.pre."
    )]
    pub year2023: Option<bool>,

    /// Defines a default [`AnimationController`] for descendant
    /// `CircularProgressIndicator` and `LinearProgressIndicator` widgets.
    ///
    /// If a descendant progress indicator's `controller` property is null, this
    /// controller will be used to drive its indeterminate animation. This allows
    /// a single controller to synchronize the animations of multiple indicators.
    ///
    /// If this property is also null, the progress indicator will create and
    /// manage its own internal [`AnimationController`].
    pub controller: Option<Ref<AnimationController>>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `ProgressIndicatorThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait ProgressIndicatorThemeDataGetters for ProgressIndicatorThemeData, fields: progress_indicator_theme_data {
        color: Option<Color>,
        linear_track_color: Option<Color>,
        linear_min_height: Option<f64>,
        circular_track_color: Option<Color>,
        refresh_background_color: Option<Color>,
        border_radius: Option<BorderRadiusGeometry>,
        stop_indicator_color: Option<Color>,
        stop_indicator_radius: Option<f64>,
        stroke_width: Option<f64>,
        stroke_align: Option<f64>,
        stroke_cap: Option<StrokeCap>,
        constraints: Option<BoxConstraints>,
        track_gap: Option<f64>,
        circular_track_padding: Option<EdgeInsetsGeometry>,
        year2023: Option<bool>,
        controller: Option<Ref<AnimationController>>,
    }
}

#[allow(deprecated)]
impl ProgressIndicatorThemeData {
    /// Creates the set of properties used to configure `ProgressIndicator` widgets.
    pub fn new() -> ProgressIndicatorThemeData {
        ProgressIndicatorThemeData::default()
    }

    /// Dart `ProgressIndicatorThemeData(color:)`.
    pub fn color(mut self, color: Color) -> ProgressIndicatorThemeData {
        self.color = Some(color);
        self
    }

    /// Dart `ProgressIndicatorThemeData(linearTrackColor:)`.
    pub fn linear_track_color(mut self, linear_track_color: Color) -> ProgressIndicatorThemeData {
        self.linear_track_color = Some(linear_track_color);
        self
    }

    /// Dart `ProgressIndicatorThemeData(linearMinHeight:)`.
    pub fn linear_min_height(mut self, linear_min_height: f64) -> ProgressIndicatorThemeData {
        self.linear_min_height = Some(linear_min_height);
        self
    }

    /// Dart `ProgressIndicatorThemeData(circularTrackColor:)`.
    pub fn circular_track_color(
        mut self,
        circular_track_color: Color,
    ) -> ProgressIndicatorThemeData {
        self.circular_track_color = Some(circular_track_color);
        self
    }

    /// Dart `ProgressIndicatorThemeData(refreshBackgroundColor:)`.
    pub fn refresh_background_color(
        mut self,
        refresh_background_color: Color,
    ) -> ProgressIndicatorThemeData {
        self.refresh_background_color = Some(refresh_background_color);
        self
    }

    /// Dart `ProgressIndicatorThemeData(borderRadius:)`.
    pub fn border_radius(
        mut self,
        border_radius: impl Into<BorderRadiusGeometry>,
    ) -> ProgressIndicatorThemeData {
        self.border_radius = Some(border_radius.into());
        self
    }

    /// Dart `ProgressIndicatorThemeData(stopIndicatorColor:)`.
    pub fn stop_indicator_color(
        mut self,
        stop_indicator_color: Color,
    ) -> ProgressIndicatorThemeData {
        self.stop_indicator_color = Some(stop_indicator_color);
        self
    }

    /// Dart `ProgressIndicatorThemeData(stopIndicatorRadius:)`.
    pub fn stop_indicator_radius(
        mut self,
        stop_indicator_radius: f64,
    ) -> ProgressIndicatorThemeData {
        self.stop_indicator_radius = Some(stop_indicator_radius);
        self
    }

    /// Dart `ProgressIndicatorThemeData(strokeWidth:)`.
    pub fn stroke_width(mut self, stroke_width: f64) -> ProgressIndicatorThemeData {
        self.stroke_width = Some(stroke_width);
        self
    }

    /// Dart `ProgressIndicatorThemeData(strokeAlign:)`.
    pub fn stroke_align(mut self, stroke_align: f64) -> ProgressIndicatorThemeData {
        self.stroke_align = Some(stroke_align);
        self
    }

    /// Dart `ProgressIndicatorThemeData(strokeCap:)`.
    pub fn stroke_cap(mut self, stroke_cap: StrokeCap) -> ProgressIndicatorThemeData {
        self.stroke_cap = Some(stroke_cap);
        self
    }

    /// Dart `ProgressIndicatorThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> ProgressIndicatorThemeData {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `ProgressIndicatorThemeData(trackGap:)`.
    pub fn track_gap(mut self, track_gap: f64) -> ProgressIndicatorThemeData {
        self.track_gap = Some(track_gap);
        self
    }

    /// Dart `ProgressIndicatorThemeData(circularTrackPadding:)`.
    pub fn circular_track_padding(
        mut self,
        circular_track_padding: impl Into<EdgeInsetsGeometry>,
    ) -> ProgressIndicatorThemeData {
        self.circular_track_padding = Some(circular_track_padding.into());
        self
    }

    /// Dart `ProgressIndicatorThemeData(year2023:)`.
    #[deprecated(
        note = "Set this flag to false to opt into the 2024 progress indicator appearance. Defaults to true. In the future, this flag will default to false. Use ProgressIndicatorThemeData to customize individual properties. This feature was deprecated after v3.27.0-0.2.pre."
    )]
    pub fn year2023(mut self, year2023: bool) -> ProgressIndicatorThemeData {
        self.year2023 = Some(year2023);
        self
    }

    /// Dart `ProgressIndicatorThemeData(controller:)`.
    pub fn controller(
        mut self,
        controller: Ref<AnimationController>,
    ) -> ProgressIndicatorThemeData {
        self.controller = Some(controller);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().stroke_width(8.0)`).
    pub fn copy_with(&self) -> ProgressIndicatorThemeData {
        self.clone()
    }

    /// Linearly interpolate between two progress indicator themes.
    ///
    /// If both arguments are null, then null is returned.
    pub fn lerp(
        a: Option<&ProgressIndicatorThemeData>,
        b: Option<&ProgressIndicatorThemeData>,
        t: f64,
    ) -> Option<ProgressIndicatorThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(ProgressIndicatorThemeData {
            color: Color::lerp(a.and_then(|a| a.color), b.and_then(|b| b.color), t),
            linear_track_color: Color::lerp(
                a.and_then(|a| a.linear_track_color),
                b.and_then(|b| b.linear_track_color),
                t,
            ),
            linear_min_height: lerp_double(
                a.and_then(|a| a.linear_min_height),
                b.and_then(|b| b.linear_min_height),
                t,
            ),
            circular_track_color: Color::lerp(
                a.and_then(|a| a.circular_track_color),
                b.and_then(|b| b.circular_track_color),
                t,
            ),
            refresh_background_color: Color::lerp(
                a.and_then(|a| a.refresh_background_color),
                b.and_then(|b| b.refresh_background_color),
                t,
            ),
            border_radius: BorderRadiusGeometry::lerp(
                a.and_then(|a| a.border_radius),
                b.and_then(|b| b.border_radius),
                t,
            ),
            stop_indicator_color: Color::lerp(
                a.and_then(|a| a.stop_indicator_color),
                b.and_then(|b| b.stop_indicator_color),
                t,
            ),
            stop_indicator_radius: lerp_double(
                a.and_then(|a| a.stop_indicator_radius),
                b.and_then(|b| b.stop_indicator_radius),
                t,
            ),
            stroke_width: lerp_double(
                a.and_then(|a| a.stroke_width),
                b.and_then(|b| b.stroke_width),
                t,
            ),
            stroke_align: lerp_double(
                a.and_then(|a| a.stroke_align),
                b.and_then(|b| b.stroke_align),
                t,
            ),
            stroke_cap: pick(a, b, t, |theme| theme.stroke_cap),
            constraints: BoxConstraints::lerp(
                a.and_then(|a| a.constraints),
                b.and_then(|b| b.constraints),
                t,
            ),
            track_gap: lerp_double(a.and_then(|a| a.track_gap), b.and_then(|b| b.track_gap), t),
            circular_track_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.circular_track_padding),
                b.and_then(|b| b.circular_track_padding),
                t,
            ),
            year2023: pick(a, b, t, |theme| theme.year2023),
            controller: pick(a, b, t, |theme| theme.controller.clone()),
        })
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&ProgressIndicatorThemeData>,
    b: Option<&ProgressIndicatorThemeData>,
    t: f64,
    field: impl Fn(&ProgressIndicatorThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

/// An inherited widget that defines the configuration for
/// `ProgressIndicator`s in this widget's subtree.
///
/// Values specified here are used for `ProgressIndicator` properties that are not
/// given an explicit non-null value.
///
/// Here is an example of a progress indicator theme that applies a red indicator
/// color.
///
/// ```text
/// ProgressIndicatorTheme::new(
///     ProgressIndicatorThemeData::new().color(Colors::RED.primary()),
///     LinearProgressIndicator::new(),
/// )
/// ```
#[derive(Debug)]
pub struct ProgressIndicatorTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant `ProgressIndicator` widgets.
    pub data: ProgressIndicatorThemeData,
    pub child: WidgetRef,
}

impl ProgressIndicatorTheme {
    /// Creates a theme that controls the configurations for `ProgressIndicator`
    /// widgets.
    pub fn new<K>(
        data: ProgressIndicatorThemeData,
        child: impl IntoWidget<K>,
    ) -> ProgressIndicatorTheme {
        ProgressIndicatorTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ProgressIndicatorTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ProgressIndicatorTheme {
        self.key = Some(key);
        self
    }

    /// Returns the [`data`](Self::data) from the closest [`ProgressIndicatorTheme`]
    /// ancestor. If there is no ancestor, it returns `ThemeData::progress_indicator_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ProgressIndicatorTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ProgressIndicatorThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<ProgressIndicatorTheme>(app) {
            Some(progress_indicator_theme) => progress_indicator_theme.data.clone(),
            None => Theme::of(app, context).progress_indicator_theme.clone(),
        }
    }
}

impl InheritedTheme for ProgressIndicatorTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ProgressIndicatorTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ProgressIndicatorTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ProgressIndicatorTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/progress_indicator_theme_test.dart`.
    #![allow(deprecated)]

    use std::time::Duration;

    use waft_embedder::{RRect, Radius, Rect, Size};
    use waft_painting::{BorderRadius, BorderSide, EdgeInsets};
    use waft_widgets::{Center, SizedBox};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::material::Material;
    use crate::progress_indicator::tests::{
        active_arc_of, arc, centered_width, circle, circular_sizes, controller_at, padded,
        painted_by, paints, pump_with, rect, rrect, scaffold_app, theme_with, themed,
    };
    use crate::progress_indicator::{CircularProgressIndicator, LinearProgressIndicator};
    use crate::theme_data::ThemeData;

    #[test]
    fn progress_indicator_theme_data_copy_with_equality_basics() {
        assert_eq!(
            ProgressIndicatorThemeData::new(),
            ProgressIndicatorThemeData::new().copy_with()
        );
    }

    #[test]
    fn progress_indicator_theme_data_lerp_special_cases() {
        assert_eq!(ProgressIndicatorThemeData::lerp(None, None, 0.0), None);
        let data = ProgressIndicatorThemeData::new();
        // Dart's `identical`: the value comes back as it was.
        assert_eq!(
            ProgressIndicatorThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn can_theme_linear_progress_indicator_using_progress_indicator_theme() {
        test_widgets(|tester| {
            let color = Color::new(0xFF00FF00);
            let linear_track_color = Color::new(0xFFFF0000);
            let linear_min_height = 25.0;
            let border_radius = 8.0;
            pump_with(tester, |app| {
                let theme = theme_with(
                    app,
                    ProgressIndicatorThemeData::new()
                        .color(color)
                        .linear_track_color(linear_track_color)
                        .linear_min_height(linear_min_height)
                        .border_radius(BorderRadius::all(Radius::circular(border_radius))),
                );
                scaffold_app(
                    Some(theme),
                    centered_width(200.0, LinearProgressIndicator::new().value(0.5)),
                )
            });
            let radius = Radius::circular(border_radius);
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    // Track.
                    rrect(RRect::from_ltrbr(
                        0.0,
                        0.0,
                        200.0,
                        linear_min_height,
                        radius
                    ))
                    .color(linear_track_color),
                    // Active indicator.
                    rrect(RRect::from_ltrbr(
                        0.0,
                        0.0,
                        100.0,
                        linear_min_height,
                        radius
                    ))
                    .color(color),
                ]
            ));
        });
    }

    #[test]
    fn can_theme_linear_progress_indicator_when_year2023_to_false() {
        test_widgets(|tester| {
            let color = Color::new(0xFF00FF00);
            let linear_track_color = Color::new(0xFFFF0000);
            let linear_min_height = 25.0;
            let border_radius = 8.0;
            let stop_indicator_color = Color::new(0xFF0000FF);
            let stop_indicator_radius = 10.0;
            let track_gap = 16.0;
            pump_with(tester, |app| {
                let theme = theme_with(
                    app,
                    ProgressIndicatorThemeData::new()
                        .color(color)
                        .linear_track_color(linear_track_color)
                        .linear_min_height(linear_min_height)
                        .border_radius(BorderRadius::all(Radius::circular(border_radius)))
                        .stop_indicator_color(stop_indicator_color)
                        .stop_indicator_radius(stop_indicator_radius)
                        .track_gap(track_gap),
                );
                scaffold_app(
                    Some(theme),
                    centered_width(
                        200.0,
                        LinearProgressIndicator::new().year2023(false).value(0.5),
                    ),
                )
            });
            let radius = Radius::circular(border_radius);
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    // Track.
                    rrect(RRect::from_ltrbr(
                        100.0 + track_gap,
                        0.0,
                        200.0,
                        linear_min_height,
                        radius
                    ))
                    .color(linear_track_color),
                    // Stop indicator.
                    circle()
                        .at(187.5, 12.5)
                        .radius(stop_indicator_radius)
                        .color(stop_indicator_color),
                    // Active indicator.
                    rrect(RRect::from_ltrbr(
                        0.0,
                        0.0,
                        100.0,
                        linear_min_height,
                        radius
                    ))
                    .color(color),
                ]
            ));
        });
    }

    #[test]
    fn local_progress_indicator_theme_takes_precedence_over_inherited_progress_indicator_theme() {
        test_widgets(|tester| {
            let color = Color::new(0xFFFF00FF);
            let linear_track_color = Color::new(0xFF00FFFF);
            let linear_min_height = 20.0;
            let border_radius = 6.0;
            let stop_indicator_color = Color::new(0xFFFFFF00);
            let stop_indicator_radius = 8.0;
            let track_gap = 12.0;
            pump_with(tester, |app| {
                let theme = theme_with(
                    app,
                    ProgressIndicatorThemeData::new()
                        .color(Color::new(0xFF00FF00))
                        .linear_track_color(Color::new(0xFFFF0000))
                        .linear_min_height(25.0)
                        .border_radius(BorderRadius::all(Radius::circular(8.0)))
                        .stop_indicator_color(Color::new(0xFF0000FF))
                        .stop_indicator_radius(10.0)
                        .track_gap(16.0),
                );
                scaffold_app(
                    Some(theme),
                    Center::new().child(ProgressIndicatorTheme::new(
                        ProgressIndicatorThemeData::new()
                            .color(color)
                            .linear_track_color(linear_track_color)
                            .linear_min_height(linear_min_height)
                            .border_radius(BorderRadius::all(Radius::circular(border_radius)))
                            .stop_indicator_color(stop_indicator_color)
                            .stop_indicator_radius(stop_indicator_radius)
                            .track_gap(track_gap),
                        SizedBox::new()
                            .width(200.0)
                            .child(LinearProgressIndicator::new().value(0.5)),
                    )),
                )
            });
            let radius = Radius::circular(border_radius);
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    // Track.
                    rrect(RRect::from_ltrbr(
                        0.0,
                        0.0,
                        200.0,
                        linear_min_height,
                        radius
                    ))
                    .color(linear_track_color),
                    // Active indicator.
                    rrect(RRect::from_ltrbr(
                        0.0,
                        0.0,
                        100.0,
                        linear_min_height,
                        radius
                    ))
                    .color(color),
                ]
            ));
        });
    }

    #[test]
    fn can_theme_circular_progress_indicator_using_progress_indicator_theme() {
        test_widgets(|tester| {
            let color = Color::new(0xFFFF0000);
            let circular_track_color = Color::new(0xFF0000FF);
            let stroke_width = 8.0;
            let stroke_align = BorderSide::STROKE_ALIGN_OUTSIDE;
            let stroke_cap = StrokeCap::Butt;
            let constraints = BoxConstraints::tight_for(Some(80.0), Some(80.0));
            let padding: EdgeInsetsGeometry = EdgeInsets::all(14.0).into();
            pump_with(tester, |app| {
                let theme = theme_with(
                    app,
                    ProgressIndicatorThemeData::new()
                        .color(color)
                        .circular_track_color(circular_track_color)
                        .stroke_width(stroke_width)
                        .stroke_align(stroke_align)
                        .stroke_cap(stroke_cap)
                        .constraints(constraints)
                        .circular_track_padding(padding),
                );
                scaffold_app(
                    Some(theme),
                    Center::new().child(CircularProgressIndicator::new().value(0.5)),
                )
            });

            let (indicator, _) = circular_sizes(tester);
            assert_eq!(
                indicator,
                Size::new(
                    constraints.max_width + padding.horizontal(),
                    constraints.max_height + padding.vertical(),
                )
            );
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[
                    // Track.
                    arc()
                        .color(circular_track_color)
                        .stroke_width(stroke_width)
                        .stroke_cap(stroke_cap),
                    // Active indicator.
                    arc()
                        .color(color)
                        .stroke_width(stroke_width)
                        .stroke_cap(stroke_cap),
                ]
            ));
        });
    }

    #[test]
    fn can_theme_circular_progress_indicator_when_year2023_to_false() {
        test_widgets(|tester| {
            let color = Color::new(0xFFFF0000);
            let circular_track_color = Color::new(0xFF0000FF);
            let stroke_width = 8.0;
            let stroke_align = BorderSide::STROKE_ALIGN_OUTSIDE;
            let stroke_cap = StrokeCap::Butt;
            let constraints = BoxConstraints::tight_for(Some(80.0), Some(80.0));
            let track_gap = 12.0;
            let padding: EdgeInsetsGeometry = EdgeInsets::all(18.0).into();
            pump_with(tester, |app| {
                let theme = theme_with(
                    app,
                    ProgressIndicatorThemeData::new()
                        .color(color)
                        .circular_track_color(circular_track_color)
                        .stroke_width(stroke_width)
                        .stroke_align(stroke_align)
                        .stroke_cap(stroke_cap)
                        .constraints(constraints)
                        .track_gap(track_gap)
                        .circular_track_padding(padding),
                );
                scaffold_app(
                    Some(theme),
                    Center::new()
                        .child(CircularProgressIndicator::new().year2023(false).value(0.5)),
                )
            });

            let (indicator, indicator_box) = circular_sizes(tester);
            assert_eq!(indicator_box, constraints.biggest());
            assert_eq!(indicator, padded(indicator_box, padding));
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[
                    // Track.
                    arc()
                        .color(circular_track_color)
                        .stroke_width(stroke_width)
                        .stroke_cap(stroke_cap),
                    // Active indicator.
                    arc()
                        .color(color)
                        .stroke_width(stroke_width)
                        .stroke_cap(stroke_cap),
                ]
            ));
        });
    }

    #[test]
    fn circular_progress_indicator_year2023_set_to_false_and_provided_circular_track_color_does_not_throw_exception()
     {
        test_widgets(|tester| {
            pump_with(tester, |app| {
                themed(
                    theme_with(
                        app,
                        ProgressIndicatorThemeData::new()
                            .circular_track_color(Color::new(0xFF0000FF))
                            .year2023(false),
                    ),
                    Center::new().child(CircularProgressIndicator::new()),
                )
            });
        });
    }

    fn year2023_false_theme(app: &App) -> ThemeData {
        theme_with(app, ProgressIndicatorThemeData::new().year2023(false))
    }

    #[test]
    fn opt_into_2024_circular_progress_indicator_appearance_with_progress_indicator_theme_data_year2023()
     {
        test_widgets(|tester| {
            let padding: EdgeInsetsGeometry = EdgeInsets::all(4.0).into();
            pump_with(tester, |app| {
                scaffold_app(
                    Some(year2023_false_theme(app)),
                    Center::new().child(CircularProgressIndicator::new().value(0.5)),
                )
            });
            let colors = year2023_false_theme(&tester.cell.borrow()).color_scheme;

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
                        .color(colors.primary())
                        .stroke_width(4.0)
                        .stroke_cap(StrokeCap::Round),
                ]
            ));
        });
    }

    #[test]
    fn circular_progress_indicator_year2023_overrides_progress_indicator_theme_data_year2023() {
        test_widgets(|tester| {
            pump_with(tester, |app| {
                scaffold_app(
                    Some(year2023_false_theme(app)),
                    Center::new().child(CircularProgressIndicator::new().year2023(true).value(0.5)),
                )
            });
            let colors = year2023_false_theme(&tester.cell.borrow()).color_scheme;

            let (indicator, indicator_box) = circular_sizes(tester);
            assert_eq!(indicator, indicator_box);
            assert!(paints(
                &painted_by::<CircularProgressIndicator>(tester),
                &[
                    // Active indicator.
                    arc()
                        .in_rect(Rect::from_ltrb(-0.0, -0.0, 36.0, 36.0))
                        .color(colors.primary())
                        .stroke_width(4.0),
                ]
            ));
        });
    }

    #[test]
    fn opt_into_2024_linear_progress_indicator_appearance_with_progress_indicator_theme_data_year2023()
     {
        test_widgets(|tester| {
            let default_track_gap = 4.0;
            pump_with(tester, |app| {
                scaffold_app(
                    Some(year2023_false_theme(app)),
                    SizedBox::new()
                        .width(200.0)
                        .child(LinearProgressIndicator::new().value(0.5)),
                )
            });
            let colors = year2023_false_theme(&tester.cell.borrow()).color_scheme;
            let radius = Radius::circular(2.0);
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
                    // Stop indicator.
                    circle().at(198.0, 2.0).radius(2.0).color(colors.primary()),
                    // Active track.
                    rrect(RRect::from_ltrbr(0.0, 0.0, 100.0, 4.0, radius)).color(colors.primary()),
                ]
            ));
        });
    }

    #[test]
    fn linear_progress_indicator_year2023_overrides_progress_indicator_theme_data_year2023() {
        test_widgets(|tester| {
            pump_with(tester, |app| {
                scaffold_app(
                    Some(year2023_false_theme(app)),
                    SizedBox::new()
                        .width(200.0)
                        .child(LinearProgressIndicator::new().year2023(true).value(0.5)),
                )
            });
            let colors = year2023_false_theme(&tester.cell.borrow()).color_scheme;
            assert!(paints(
                &painted_by::<LinearProgressIndicator>(tester),
                &[
                    // Track.
                    rect(Rect::from_ltrb(0.0, 0.0, 200.0, 4.0)).color(colors.secondary_container()),
                    // Active track.
                    rect(Rect::from_ltrb(0.0, 0.0, 100.0, 4.0)).color(colors.primary()),
                ]
            ));
        });
    }

    /// The controllers of the theme-controller cases: the widget's, the
    /// `ProgressIndicatorTheme`'s and the `ThemeData`'s.
    #[derive(Clone, Default)]
    struct Controllers {
        widget: Option<Ref<AnimationController>>,
        indicator_theme: Option<Ref<AnimationController>>,
        global_theme: Option<Ref<AnimationController>>,
    }

    /// The theme-controller cases' `buildApp`, around `indicator`.
    fn build_app(
        app: &App,
        global_theme: ProgressIndicatorThemeData,
        controllers: Controllers,
        indicator: WidgetRef,
    ) -> WidgetRef {
        let with = |data: ProgressIndicatorThemeData, controller| match controller {
            Some(controller) => data.controller(controller),
            None => data,
        };
        crate::app::MaterialApp::new()
            .home(Material::new().child(Center::new().child(themed(
                theme_with(app, with(global_theme, controllers.global_theme.clone())),
                ProgressIndicatorTheme::new(
                    with(
                        ProgressIndicatorThemeData::new(),
                        controllers.indicator_theme.clone(),
                    ),
                    SizedBox::new().width(200.0).child(indicator),
                ),
            ))))
            .into_widget()
    }

    #[test]
    fn linear_progress_indicator_reflects_the_value_of_the_theme_controller() {
        test_widgets(|tester| {
            let pump = |tester: &mut WidgetTester, controllers: Controllers| {
                pump_with(tester, |app| {
                    let indicator = match controllers.widget.clone() {
                        Some(controller) => LinearProgressIndicator::new().controller(controller),
                        None => LinearProgressIndicator::new(),
                    };
                    build_app(
                        app,
                        ProgressIndicatorThemeData::new(),
                        controllers.clone(),
                        indicator.into_widget(),
                    )
                });
            };
            let expect_progress_at = |tester: &WidgetTester, left: f64, right: f64| {
                let mut expected = Vec::new();
                if right < 200.0 {
                    // Right track
                    expected.push(rect(Rect::from_ltrb(right, 0.0, 200.0, 4.0)));
                }
                expected.push(rect(Rect::from_ltrb(left, 0.0, right, 4.0)));
                if left > 0.0 {
                    // Left track
                    expected.push(rect(Rect::from_ltrb(0.0, 0.0, left, 4.0)));
                }
                assert!(
                    paints(&painted_by::<LinearProgressIndicator>(tester), &expected),
                    "progress from {left} to {right}"
                );
            };

            let mut controllers = Controllers::default();
            pump(tester, controllers.clone());
            tester.pump_by(Duration::from_millis(500));
            expect_progress_at(tester, 16.028758883476257, 141.07513427734375);

            controllers.global_theme = Some(controller_at(tester, 0.1));
            pump(tester, controllers.clone());
            expect_progress_at(tester, 0.0, 37.14974820613861);

            controllers.indicator_theme = Some(controller_at(tester, 0.5));
            pump(tester, controllers.clone());
            expect_progress_at(tester, 127.79541015625, 200.0);

            controllers.widget = Some(controller_at(tester, 0.8));
            pump(tester, controllers.clone());
            expect_progress_at(tester, 98.24226796627045, 181.18448555469513);
        });
    }

    #[test]
    fn circular_progress_indicator_reflects_the_value_of_the_theme_controller() {
        test_widgets(|tester| {
            let pump = |tester: &mut WidgetTester, controllers: Controllers| {
                pump_with(tester, |app| {
                    let indicator = match controllers.widget.clone() {
                        Some(controller) => CircularProgressIndicator::new().controller(controller),
                        None => CircularProgressIndicator::new(),
                    };
                    build_app(
                        app,
                        ProgressIndicatorThemeData::new()
                            .color(Colors::BLACK)
                            .linear_track_color(Colors::GREEN.primary()),
                        controllers.clone(),
                        indicator.into_widget(),
                    )
                });
            };
            let expect_progress_at = |tester: &WidgetTester, start: f64, sweep: f64| {
                assert_eq!(
                    active_arc_of::<CircularProgressIndicator>(tester),
                    (start, sweep)
                );
            };

            let mut controllers = Controllers::default();
            pump(tester, controllers.clone());
            tester.pump_by(Duration::from_millis(500));
            expect_progress_at(tester, 0.43225767465697107, 4.52182126629162);

            controllers.global_theme = Some(controller_at(tester, 0.1));
            pump(tester, controllers.clone());
            expect_progress_at(tester, 0.628318530718057, 2.8904563625380906);

            controllers.indicator_theme = Some(controller_at(tester, 0.5));
            pump(tester, controllers.clone());
            expect_progress_at(tester, std::f64::consts::FRAC_PI_2, 0.001);

            controllers.widget = Some(controller_at(tester, 0.8));
            pump(tester, controllers.clone());
            expect_progress_at(tester, 2.520489337828999, 4.076855234710353);
        });
    }
}
