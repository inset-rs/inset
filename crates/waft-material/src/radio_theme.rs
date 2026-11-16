//! Flutter counterpart: material_ui's `radio_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{AnyBorderSide, BorderSide};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, WidgetRef, WidgetStateBorderSide,
    WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Defines default property values for descendant `Radio` widgets.
///
/// Descendant widgets obtain the current [`RadioThemeData`] object using
/// [`RadioTheme::of`]. Instances of [`RadioThemeData`] can be customized
/// with [`RadioThemeData::copy_with`].
///
/// Typically a [`RadioThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::radio_theme`.
///
/// All [`RadioThemeData`] properties are `None` by default. When `None`, the
/// `Radio` will use the values from `ThemeData` if they exist, otherwise it
/// will provide its own defaults based on the overall `Theme`'s color_scheme.
/// See the individual `Radio` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
///  * [`RadioTheme`], which is used by descendants to obtain the
///    [`RadioThemeData`].
#[derive(Clone, Debug, Default)]
pub struct RadioThemeData {
    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If specified, overrides the default value of `Radio::mouse_cursor`. The
    /// default value is `WidgetStateMouseCursor::clickable`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// The color that fills the radio button, in all `WidgetState`s.
    ///
    /// If specified, overrides the default value of `Radio::fill_color`.
    pub fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The color for the radio's `Material`.
    ///
    /// If specified, overrides the default value of `Radio::overlay_color`.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The splash radius of the circular `Material` ink response.
    ///
    /// If specified, overrides the default value of `Radio::splash_radius`. The
    /// default value is `K_RADIAL_REACTION_RADIUS`.
    pub splash_radius: Option<f64>,

    /// Configures the minimum size of the tap target.
    ///
    /// If specified, overrides the default value of
    /// `Radio::material_tap_target_size`. The default value is the value of
    /// `ThemeData::material_tap_target_size`.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,

    /// Defines how compact the radio's layout will be.
    ///
    /// If specified, overrides the default value of `Radio::visual_density`. The
    /// default value is the value of `ThemeData::visual_density`.
    pub visual_density: Option<VisualDensity>,

    /// The color of the background of the radio button, in all `WidgetState`s.
    ///
    /// If specified, overrides the default value of `Radio::background_color`. The
    /// default value is transparent in all states.
    pub background_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The side for the circular border of the radio button, in all
    /// `WidgetState`s.
    ///
    /// If specified, overrides the default value of `Radio::side`. The default
    /// value is a border using the fill color.
    pub side: Option<AnyBorderSide>,

    /// The radius of the inner circle of the radio button, in all `WidgetState`s.
    ///
    /// If specified, overrides the default value of `Radio::inner_radius`. The
    /// default value is `4.5` in all states.
    pub inner_radius: Option<WidgetStatePropertyRef<Option<f64>>>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `RadioThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait RadioThemeDataGetters for RadioThemeData, fields: radio_theme_data {
        mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
        fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        splash_radius: Option<f64>,
        material_tap_target_size: Option<MaterialTapTargetSize>,
        visual_density: Option<VisualDensity>,
        background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        side: Option<AnyBorderSide>,
        inner_radius: Option<WidgetStatePropertyRef<Option<f64>>>,
    }
}

impl RadioThemeData {
    /// Creates a theme that can be used for `ThemeData::radio_theme`.
    pub fn new() -> RadioThemeData {
        RadioThemeData::default()
    }

    /// Dart `RadioThemeData(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> RadioThemeData {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `RadioThemeData(fillColor:)`.
    pub fn fill_color(
        mut self,
        fill_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> RadioThemeData {
        self.fill_color = Some(fill_color);
        self
    }

    /// Dart `RadioThemeData(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> RadioThemeData {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `RadioThemeData(splashRadius:)`.
    pub fn splash_radius(mut self, splash_radius: f64) -> RadioThemeData {
        self.splash_radius = Some(splash_radius);
        self
    }

    /// Dart `RadioThemeData(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> RadioThemeData {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `RadioThemeData(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> RadioThemeData {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `RadioThemeData(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> RadioThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `RadioThemeData(side:)`.
    pub fn side(mut self, side: impl Into<AnyBorderSide>) -> RadioThemeData {
        self.side = Some(side.into());
        self
    }

    /// Dart `RadioThemeData(innerRadius:)`.
    pub fn inner_radius(
        mut self,
        inner_radius: WidgetStatePropertyRef<Option<f64>>,
    ) -> RadioThemeData {
        self.inner_radius = Some(inner_radius);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().splash_radius(1.0)`).
    pub fn copy_with(&self) -> RadioThemeData {
        self.clone()
    }

    // Special case because BorderSide.lerp() doesn't support null arguments.
    fn lerp_sides(
        a: Option<&AnyBorderSide>,
        b: Option<&AnyBorderSide>,
        t: f64,
    ) -> Option<BorderSide> {
        if a.is_none() && b.is_none() {
            return None;
        }
        let a = a.and_then(Self::resolve_side_in_no_state);
        let b = b.and_then(Self::resolve_side_in_no_state);
        let a = a.unwrap_or_else(|| BorderSide {
            width: 0.0,
            color: b.expect("one side is given").color.with_alpha(0),
            ..BorderSide::default()
        });
        let b = b.unwrap_or(BorderSide {
            width: 0.0,
            color: a.color.with_alpha(0),
            ..BorderSide::default()
        });

        Some(BorderSide::lerp(a, b, t))
    }

    /// Dart's `side is WidgetStateBorderSide ? side.resolve(const <WidgetState>{}) : side`.
    fn resolve_side_in_no_state(side: &AnyBorderSide) -> Option<BorderSide> {
        match side.extension::<WidgetStateBorderSide>() {
            Some(side) => side.resolve(&WidgetStates::new()),
            None => Some(**side),
        }
    }

    /// Linearly interpolate between two [`RadioThemeData`]s.
    ///
    /// The argument `t` represents position on the timeline, with 0.0 meaning
    /// that the interpolation has not started, returning `a` (or something
    /// equivalent to `a`), 1.0 meaning that the interpolation has finished,
    /// returning `b` (or something equivalent to `b`), and values in between
    /// meaning that the interpolation is at the relevant point on the timeline
    /// between `a` and `b`.
    pub fn lerp(a: Option<&RadioThemeData>, b: Option<&RadioThemeData>, t: f64) -> RadioThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        RadioThemeData {
            mouse_cursor: pick(a, b, t, |theme| theme.mouse_cursor.clone()),
            fill_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.fill_color.clone()),
                b.and_then(|b| b.fill_color.clone()),
                t,
                Color::lerp,
            ),
            material_tap_target_size: pick(a, b, t, |theme| theme.material_tap_target_size),
            overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.overlay_color.clone()),
                b.and_then(|b| b.overlay_color.clone()),
                t,
                Color::lerp,
            ),
            splash_radius: lerp_double(
                a.and_then(|a| a.splash_radius),
                b.and_then(|b| b.splash_radius),
                t,
            ),
            visual_density: pick(a, b, t, |theme| theme.visual_density),
            background_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.background_color.clone()),
                b.and_then(|b| b.background_color.clone()),
                t,
                Color::lerp,
            ),
            side: Self::lerp_sides(
                a.and_then(|a| a.side.as_ref()),
                b.and_then(|b| b.side.as_ref()),
                t,
            )
            .map(AnyBorderSide::new),
            inner_radius: <dyn WidgetStateProperty<Option<f64>>>::lerp_optional(
                a.and_then(|a| a.inner_radius.clone()),
                b.and_then(|b| b.inner_radius.clone()),
                t,
                lerp_double,
            ),
        }
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&RadioThemeData>,
    b: Option<&RadioThemeData>,
    t: f64,
    field: impl Fn(&RadioThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

impl PartialEq for RadioThemeData {
    fn eq(&self, other: &RadioThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.mouse_cursor == self.mouse_cursor
            && other.fill_color == self.fill_color
            && other.overlay_color == self.overlay_color
            && other.splash_radius == self.splash_radius
            && other.material_tap_target_size == self.material_tap_target_size
            && other.visual_density == self.visual_density
            && other.background_color == self.background_color
            && other.side == self.side
            && other.inner_radius == self.inner_radius
    }
}

/// Applies a radio theme to descendant `Radio` widgets.
///
/// Descendant widgets obtain the current theme's [`RadioTheme`] object using
/// [`RadioTheme::of`]. When a widget uses [`RadioTheme::of`], it is automatically
/// rebuilt if the theme later changes.
///
/// A radio theme can be specified as part of the overall Material theme using
/// `ThemeData::radio_theme`.
///
/// See also:
///
///  * [`RadioThemeData`], which describes the actual configuration of a radio
///    theme.
#[derive(Debug)]
pub struct RadioTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant `Radio` widgets.
    pub data: RadioThemeData,
    pub child: WidgetRef,
}

impl RadioTheme {
    /// Constructs a radio theme that configures all descendant `Radio` widgets.
    pub fn new<K>(data: RadioThemeData, child: impl IntoWidget<K>) -> RadioTheme {
        RadioTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `RadioTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> RadioTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration [`data`](Self::data) from the closest [`RadioTheme`] ancestor.
    /// If there is no ancestor, it returns `ThemeData::radio_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = RadioTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> RadioThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<RadioTheme>(app) {
            Some(radio_theme) => radio_theme.data.clone(),
            None => Theme::of(app, context).radio_theme.clone(),
        }
    }
}

impl InheritedWidget for RadioTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &RadioTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::rc::Rc;

    use waft_embedder::Size;
    use waft_services::SystemMouseCursors;
    use waft_widgets::{FocusHighlightStrategy, SizedBox, WidgetState, WidgetStatePropertyAll};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::radio::Radio;
    use crate::radio::tests::{
        CircleArguments, assert_paints, center_of_radio, circle, find_radio, hover_at, no_op,
        press_at, radio_material, scaffold_app,
    };
    use crate::theme_data::ThemeData;

    fn theme_with(tester: &WidgetTester, radio_theme: RadioThemeData) -> ThemeData {
        ThemeData::builder()
            .radio_theme(radio_theme)
            .build(&tester.cell.borrow())
    }

    /// Dart's `_pointGestureToRadio`.
    fn point_gesture_to_radio(tester: &mut WidgetTester) {
        let center = center_of_radio::<i32>(tester);
        hover_at(tester, center);
    }

    fn radio_size(tester: &WidgetTester) -> Size {
        let render = tester.render_box_of(find_radio::<i32>(tester));
        render.size(&tester.cell.borrow())
    }

    #[test]
    fn radio_theme_data_copy_with_equality_basics() {
        assert_eq!(RadioThemeData::new(), RadioThemeData::new().copy_with());
    }

    #[test]
    fn radio_theme_data_lerp_special_cases() {
        assert_eq!(RadioThemeData::lerp(None, None, 0.0), RadioThemeData::new());
        let fill_color =
            <dyn WidgetStateProperty<Option<Color>>>::all(Some(Color::new(0xff000001)));
        let data = RadioThemeData::new().fill_color(Rc::clone(&fill_color));
        // Dart's `identical`: the same data comes back, its properties not lerped.
        let lerped = RadioThemeData::lerp(Some(&data), Some(&data), 0.5);
        assert!(Rc::ptr_eq(
            lerped.fill_color.as_ref().expect("a fill color"),
            &fill_color
        ));
    }

    #[test]
    fn radio_theme_data_defaults() {
        let theme_data = RadioThemeData::new();
        let all_none = |data: &RadioThemeData| {
            data.mouse_cursor.is_none()
                && data.fill_color.is_none()
                && data.overlay_color.is_none()
                && data.splash_radius.is_none()
                && data.material_tap_target_size.is_none()
                && data.visual_density.is_none()
                && data.background_color.is_none()
                && data.side.is_none()
                && data.inner_radius.is_none()
        };
        assert!(all_none(&theme_data));

        let theme = RadioTheme::new(RadioThemeData::new(), SizedBox::new());
        assert!(all_none(&theme.data));
    }

    #[test]
    fn a_radio_is_themeable() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const DEFAULT_FILL_COLOR: Color = Color::new(0xfffffff0);
            const SELECTED_FILL_COLOR: Color = Color::new(0xfffffff1);
            const FOCUS_OVERLAY_COLOR: Color = Color::new(0xfffffff2);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xfffffff3);
            const DEFAULT_BACKGROUND_COLOR: Color = Color::new(0xfffffff4);
            const SELECTED_BACKGROUND_COLOR: Color = Color::new(0xfffffff5);
            const SPLASH_RADIUS: f64 = 1.0;
            const MATERIAL_TAP_TARGET_SIZE: MaterialTapTargetSize =
                MaterialTapTargetSize::ShrinkWrap;
            const VISUAL_DENSITY: VisualDensity = VisualDensity::new(1.0, 1.0);
            const INNER_RADIUS: f64 = 5.0;
            let radio_theme = RadioThemeData::new()
                .mouse_cursor(Rc::new(WidgetStatePropertyAll::new(Some(
                    SystemMouseCursors::TEXT.into(),
                ))))
                .fill_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                    |states| {
                        if states.contains(&WidgetState::Selected) {
                            return Some(SELECTED_FILL_COLOR);
                        }
                        Some(DEFAULT_FILL_COLOR)
                    },
                ))
                .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                    |states| {
                        if states.contains(&WidgetState::Focused) {
                            return Some(FOCUS_OVERLAY_COLOR);
                        }
                        if states.contains(&WidgetState::Hovered) {
                            return Some(HOVER_OVERLAY_COLOR);
                        }
                        None
                    },
                ))
                .splash_radius(SPLASH_RADIUS)
                .material_tap_target_size(MATERIAL_TAP_TARGET_SIZE)
                .visual_density(VISUAL_DENSITY)
                .background_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                    |states| {
                        if states.contains(&WidgetState::Selected) {
                            return Some(SELECTED_BACKGROUND_COLOR);
                        }
                        Some(DEFAULT_BACKGROUND_COLOR)
                    },
                ))
                .inner_radius(Rc::new(WidgetStatePropertyAll::new(Some(INNER_RADIUS))));
            let theme = theme_with(tester, radio_theme);
            let build_radio = |selected: bool, autofocus: bool| {
                scaffold_app(
                    theme.clone(),
                    Radio::new(if selected { 1 } else { 0 })
                        .on_changed(no_op())
                        .group_value(1)
                        .autofocus(autofocus),
                )
            };

            // Radio.
            tester.pump_widget(build_radio(false, false));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(DEFAULT_BACKGROUND_COLOR), circle(DEFAULT_FILL_COLOR)],
            );
            // Size from MaterialTapTargetSize.shrinkWrap with added VisualDensity.
            assert_eq!(
                radio_size(tester),
                Size::new(40.0, 40.0) + VISUAL_DENSITY.base_size_adjustment()
            );

            // Selected radio.
            tester.pump_widget(build_radio(true, false));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[
                    circle(SELECTED_BACKGROUND_COLOR),
                    circle(SELECTED_FILL_COLOR),
                    circle(SELECTED_FILL_COLOR).radius(INNER_RADIUS),
                ],
            );

            // Radio with hover.
            tester.pump_widget(build_radio(false, false));
            point_gesture_to_radio(tester);
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(HOVER_OVERLAY_COLOR)],
            );
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Radio with focus.
            tester.pump_widget(build_radio(false, true));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(FOCUS_OVERLAY_COLOR).radius(SPLASH_RADIUS)],
            );
        });
    }

    /// The radio of the side cases: value 0 or 1 against a group value of `group_value`.
    fn side_radio(tester: &mut WidgetTester, theme: &ThemeData, radio: Radio<i32>) {
        tester.pump_widget(scaffold_app(theme.clone(), radio.on_changed(no_op())));
    }

    fn side(color: Color, width: f64) -> BorderSide {
        BorderSide {
            color,
            width,
            ..BorderSide::default()
        }
    }

    #[test]
    fn the_radio_side_is_themeable() {
        test_widgets(|tester| {
            let default_side = side(Color::new(0xfffffff0), 2.0);
            let selected_side = side(Color::new(0xfffffff1), 3.0);
            let theme = theme_with(
                tester,
                RadioThemeData::new().side(WidgetStateBorderSide::resolve_with(move |states| {
                    if states.contains(&WidgetState::Selected) {
                        return Some(selected_side);
                    }
                    Some(default_side)
                })),
            );

            // Radio.
            side_radio(tester, &theme, Radio::new(0).group_value(1));
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(default_side.color).stroke_width(default_side.width)],
            );

            // Selected radio.
            side_radio(tester, &theme, Radio::new(1).group_value(1));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(selected_side.color).stroke_width(selected_side.width)],
            );
        });
    }

    #[test]
    fn radio_properties_are_taken_over_the_theme_values() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const THEME_DEFAULT_FILL_COLOR: Color = Color::new(0xfffffff0);
            const THEME_SELECTED_FILL_COLOR: Color = Color::new(0xfffffff1);
            const THEME_FOCUS_OVERLAY_COLOR: Color = Color::new(0xfffffff2);
            const THEME_HOVER_OVERLAY_COLOR: Color = Color::new(0xfffffff3);
            const THEME_DEFAULT_BACKGROUND_COLOR: Color = Color::new(0xfffffff4);
            const THEME_SELECTED_BACKGROUND_COLOR: Color = Color::new(0xfffffff5);
            const THEME_SPLASH_RADIUS: f64 = 1.0;
            const THEME_INNER_RADIUS: f64 = 5.0;

            const DEFAULT_FILL_COLOR: Color = Color::new(0xeffffff0);
            const SELECTED_FILL_COLOR: Color = Color::new(0xeffffff1);
            const FOCUS_COLOR: Color = Color::new(0xeffffff2);
            const HOVER_COLOR: Color = Color::new(0xeffffff3);
            const DEFAULT_BACKGROUND_COLOR: Color = Color::new(0xeffffff4);
            const SELECTED_BACKGROUND_COLOR: Color = Color::new(0xeffffff5);
            const SPLASH_RADIUS: f64 = 2.0;
            const VISUAL_DENSITY: VisualDensity = VisualDensity::new(1.0, 1.0);
            const INNER_RADIUS: f64 = 6.0;

            let theme = theme_with(
                tester,
                RadioThemeData::new()
                    .mouse_cursor(Rc::new(WidgetStatePropertyAll::new(Some(
                        SystemMouseCursors::CLICK.into(),
                    ))))
                    .fill_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            if states.contains(&WidgetState::Selected) {
                                return Some(THEME_SELECTED_FILL_COLOR);
                            }
                            Some(THEME_DEFAULT_FILL_COLOR)
                        },
                    ))
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            if states.contains(&WidgetState::Focused) {
                                return Some(THEME_FOCUS_OVERLAY_COLOR);
                            }
                            if states.contains(&WidgetState::Hovered) {
                                return Some(THEME_HOVER_OVERLAY_COLOR);
                            }
                            None
                        },
                    ))
                    .splash_radius(THEME_SPLASH_RADIUS)
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .visual_density(VisualDensity::STANDARD)
                    .background_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            if states.contains(&WidgetState::Selected) {
                                return Some(THEME_SELECTED_BACKGROUND_COLOR);
                            }
                            Some(THEME_DEFAULT_BACKGROUND_COLOR)
                        },
                    ))
                    .inner_radius(Rc::new(WidgetStatePropertyAll::new(Some(
                        THEME_INNER_RADIUS,
                    )))),
            );
            let build_radio = |selected: bool, autofocus: bool| {
                scaffold_app(
                    theme.clone(),
                    Radio::new(if selected { 0 } else { 1 })
                        .on_changed(no_op())
                        .group_value(0)
                        .autofocus(autofocus)
                        .mouse_cursor(SystemMouseCursors::TEXT.into())
                        .fill_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            |states| {
                                if states.contains(&WidgetState::Selected) {
                                    return Some(SELECTED_FILL_COLOR);
                                }
                                Some(DEFAULT_FILL_COLOR)
                            },
                        ))
                        .focus_color(FOCUS_COLOR)
                        .hover_color(HOVER_COLOR)
                        .splash_radius(SPLASH_RADIUS)
                        .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                        .visual_density(VISUAL_DENSITY)
                        .background_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            |states| {
                                if states.contains(&WidgetState::Selected) {
                                    return Some(SELECTED_BACKGROUND_COLOR);
                                }
                                Some(DEFAULT_BACKGROUND_COLOR)
                            },
                        ))
                        .inner_radius(Rc::new(WidgetStatePropertyAll::new(Some(INNER_RADIUS)))),
                )
            };

            // Radio.
            tester.pump_widget(build_radio(false, false));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(DEFAULT_BACKGROUND_COLOR), circle(DEFAULT_FILL_COLOR)],
            );
            // Size from MaterialTapTargetSize.shrinkWrap with added VisualDensity.
            assert_eq!(
                radio_size(tester),
                Size::new(40.0, 40.0) + VISUAL_DENSITY.base_size_adjustment()
            );

            // Selected radio.
            tester.pump_widget(build_radio(true, false));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[
                    circle(SELECTED_BACKGROUND_COLOR),
                    circle(SELECTED_FILL_COLOR),
                    circle(SELECTED_FILL_COLOR).radius(INNER_RADIUS),
                ],
            );

            // Radio with hover.
            tester.pump_widget(build_radio(false, false));
            point_gesture_to_radio(tester);
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(HOVER_COLOR)],
            );
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Radio with focus.
            tester.pump_widget(build_radio(false, true));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(FOCUS_COLOR).radius(SPLASH_RADIUS)],
            );
        });
    }

    #[test]
    fn the_radio_side_property_is_taken_over_the_theme_values() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let theme_default_side = side(Color::new(0xfffffff0), 2.0);
            let theme_selected_side = side(Color::new(0xfffffff1), 3.0);
            let default_side = side(Color::new(0xeffffff2), 4.0);
            let selected_side = side(Color::new(0xeffffff3), 5.0);
            let theme = theme_with(
                tester,
                RadioThemeData::new().side(WidgetStateBorderSide::resolve_with(move |states| {
                    if states.contains(&WidgetState::Selected) {
                        return Some(theme_selected_side);
                    }
                    Some(theme_default_side)
                })),
            );
            let radio = |selected: bool| {
                Radio::new(if selected { 0 } else { 1 })
                    .group_value(0)
                    .side(WidgetStateBorderSide::resolve_with(move |states| {
                        if states.contains(&WidgetState::Selected) {
                            return Some(selected_side);
                        }
                        Some(default_side)
                    }))
            };

            // Radio.
            side_radio(tester, &theme, radio(false));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(default_side.color).stroke_width(default_side.width)],
            );

            // Selected radio.
            side_radio(tester, &theme, radio(true));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(selected_side.color).stroke_width(selected_side.width)],
            );
        });
    }

    #[test]
    fn the_radio_active_color_is_taken_over_the_theme() {
        test_widgets(|tester| {
            const THEME_DEFAULT_FILL_COLOR: Color = Color::new(0xfffffff0);
            const THEME_SELECTED_FILL_COLOR: Color = Color::new(0xfffffff1);
            const SELECTED_FILL_COLOR: Color = Color::new(0xfffffff1);
            let theme = theme_with(
                tester,
                RadioThemeData::new().fill_color(
                    <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                        if states.contains(&WidgetState::Selected) {
                            return Some(THEME_SELECTED_FILL_COLOR);
                        }
                        Some(THEME_DEFAULT_FILL_COLOR)
                    }),
                ),
            );
            let radio = |selected: bool| {
                Radio::new(if selected { 0 } else { 1 })
                    .group_value(0)
                    .active_color(SELECTED_FILL_COLOR)
            };

            // Radio.
            side_radio(tester, &theme, radio(false));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(THEME_DEFAULT_FILL_COLOR)],
            );

            // Selected radio.
            side_radio(tester, &theme, radio(true));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(SELECTED_FILL_COLOR)],
            );
        });
    }

    #[test]
    fn the_theme_overlay_color_resolves_in_active_and_pressed_states() {
        test_widgets(|tester| {
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const SPLASH_RADIUS: f64 = 24.0;
            let theme = theme_with(
                tester,
                RadioThemeData::new()
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            if states.contains(&WidgetState::Pressed) {
                                if states.contains(&WidgetState::Selected) {
                                    return Some(ACTIVE_PRESSED_OVERLAY_COLOR);
                                }
                                return Some(INACTIVE_PRESSED_OVERLAY_COLOR);
                            }
                            None
                        },
                    ))
                    .splash_radius(SPLASH_RADIUS),
            );
            let radio = |active: bool| Radio::new(if active { 1 } else { 0 }).group_value(1);

            side_radio(tester, &theme, radio(false));
            let center = center_of_radio::<i32>(tester);
            press_at(tester, center, 1);
            tester.pump_and_settle();
            // Inactive pressed Radio should have overlay color INACTIVE_PRESSED_OVERLAY_COLOR
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(INACTIVE_PRESSED_OVERLAY_COLOR).radius(SPLASH_RADIUS)],
            );

            side_radio(tester, &theme, radio(true));
            let center = center_of_radio::<i32>(tester);
            press_at(tester, center, 2);
            tester.pump_and_settle();
            // Active pressed Radio should have overlay color ACTIVE_PRESSED_OVERLAY_COLOR
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(ACTIVE_PRESSED_OVERLAY_COLOR).radius(SPLASH_RADIUS)],
            );
        });
    }

    #[test]
    fn a_local_radio_theme_overrides_the_global_one() {
        test_widgets(|tester| {
            const GLOBAL_THEME_FILL_COLOR: Color = Color::new(0xfffffff1);
            const LOCAL_THEME_FILL_COLOR: Color = Color::new(0xffff0000);
            let theme = theme_with(
                tester,
                RadioThemeData::new().fill_color(Rc::new(WidgetStatePropertyAll::new(Some(
                    GLOBAL_THEME_FILL_COLOR,
                )))),
            );

            tester.pump_widget(scaffold_app(
                theme,
                RadioTheme::new(
                    RadioThemeData::new().fill_color(Rc::new(WidgetStatePropertyAll::new(Some(
                        LOCAL_THEME_FILL_COLOR,
                    )))),
                    Radio::new(1).group_value(1).on_changed(no_op()),
                ),
            ));
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(LOCAL_THEME_FILL_COLOR)],
            );
        });
    }
}
