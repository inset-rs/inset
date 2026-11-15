//! Flutter counterpart: material_ui's `snack_bar_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{AnyColor, EdgeInsets, ShapeBorder, TextStyle};
use waft_widgets::{
    BuildContext, DismissDirection, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateColor,
};

use crate::theme::Theme;

/// Defines where a `SnackBar` should appear within a `Scaffold` and how its
/// location should be adjusted when the scaffold also includes a
/// `FloatingActionButton` or a `BottomNavigationBar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnackBarBehavior {
    /// Fixes the `SnackBar` at the bottom of the `Scaffold`.
    ///
    /// The exception is that the `SnackBar` will be shown above a
    /// `BottomNavigationBar` or a `NavigationBar`. Additionally, the `SnackBar`
    /// will cause other non-fixed widgets inside `Scaffold` to be pushed above
    /// (for example, the `FloatingActionButton`).
    Fixed,

    /// This behavior will cause `SnackBar` to be shown above other widgets in the
    /// `Scaffold`. This includes being displayed above a `BottomNavigationBar` or
    /// a `NavigationBar`, and a `FloatingActionButton` when its location is on the
    /// bottom. When the floating action button location is on the top, this behavior
    /// will cause the `SnackBar` to be shown above other widgets in the `Scaffold`
    /// except the floating action button.
    ///
    /// See <https://material.io/design/components/snackbars.html> for more details.
    Floating,
}

/// Customizes default property values for `SnackBar` widgets.
///
/// Descendant widgets obtain the current [`SnackBarThemeData`] object using
/// [`SnackBarTheme::of`]. Instances of [`SnackBarThemeData`] can be
/// customized with [`SnackBarThemeData::copy_with`].
///
/// Typically a [`SnackBarThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::snack_bar_theme`. The default for `ThemeData::snack_bar_theme`
/// provides all `null` properties.
///
/// All [`SnackBarThemeData`] properties are `null` by default. When null, the
/// `SnackBar` will provide its own defaults.
///
/// Dart's constructor asserts run in the setters that can break them
/// ([`elevation`](Self::elevation), [`width`](Self::width), [`behavior`](Self::behavior),
/// [`action_overflow_threshold`](Self::action_overflow_threshold),
/// [`action_background_color`](Self::action_background_color) and
/// [`disabled_action_background_color`](Self::disabled_action_background_color)), so that
/// they hold in any order. A [`width`](Self::width) given with no
/// [`behavior`](Self::behavior) at all is not caught: a setter cannot know that no
/// [`behavior`](Self::behavior) is coming.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnackBarThemeData {
    /// Overrides the default value for `SnackBar::background_color`.
    ///
    /// If null, `SnackBar` defaults to dark grey: `Color(0xFF323232)`.
    pub background_color: Option<Color>,

    /// Overrides the default value for `SnackBarAction::text_color`.
    ///
    /// If null, `SnackBarAction` defaults to `ColorScheme::secondary` of
    /// `ThemeData::color_scheme`.
    pub action_text_color: Option<AnyColor>,

    /// Overrides the default value for `SnackBarAction::disabled_text_color`.
    ///
    /// If null, `SnackBarAction` defaults to `ColorScheme::on_surface` with its
    /// opacity set to 0.30 if the `Theme`'s brightness is `Brightness::Dark`, 0.38
    /// otherwise.
    pub disabled_action_text_color: Option<Color>,

    /// Used to configure the `DefaultTextStyle` for the `SnackBar::content` widget.
    ///
    /// If null, `SnackBar` defines its default.
    pub content_text_style: Option<TextStyle>,

    /// Overrides the default value for `SnackBar::elevation`.
    ///
    /// If null, `SnackBar` uses a default of 6.0.
    pub elevation: Option<f64>,

    /// Overrides the default value for `SnackBar::shape`.
    ///
    /// If null, `SnackBar` provides different defaults depending on the
    /// [`SnackBarBehavior`]. For [`SnackBarBehavior::Fixed`], no overriding shape is
    /// specified, so the `SnackBar` is rectangular. For
    /// [`SnackBarBehavior::Floating`], it uses a `RoundedRectangleBorder` with a
    /// circular corner radius of 4.0.
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// Overrides the default value for `SnackBar::behavior`.
    ///
    /// If null, `SnackBar` will default to [`SnackBarBehavior::Fixed`].
    pub behavior: Option<SnackBarBehavior>,

    /// Overrides the default value for `SnackBar::width`.
    ///
    /// If this property is null, then the snack bar will take up the full device
    /// width less the margin. This value is only used when [`behavior`](Self::behavior) is
    /// [`SnackBarBehavior::Floating`].
    pub width: Option<f64>,

    /// Overrides the default value for `SnackBar::margin`.
    ///
    /// This value is only used when [`behavior`](Self::behavior) is [`SnackBarBehavior::Floating`].
    pub inset_padding: Option<EdgeInsets>,

    /// Overrides the default value for `SnackBar::show_close_icon`.
    ///
    /// Whether to show an optional "Close" icon.
    pub show_close_icon: Option<bool>,

    /// Overrides the default value for `SnackBar::close_icon_color`.
    ///
    /// This value is only used if [`show_close_icon`](Self::show_close_icon) is true.
    pub close_icon_color: Option<AnyColor>,

    /// Overrides the default value for `SnackBar::action_overflow_threshold`.
    ///
    /// Must be a value between 0 and 1, if present.
    pub action_overflow_threshold: Option<f64>,

    /// Overrides default value for `SnackBarAction::background_color`.
    ///
    /// If null, `SnackBarAction` falls back to `Colors::TRANSPARENT`.
    pub action_background_color: Option<AnyColor>,

    /// Overrides default value for `SnackBarAction::disabled_background_color`.
    ///
    /// If null, `SnackBarAction` falls back to `Colors::TRANSPARENT`.
    pub disabled_action_background_color: Option<Color>,

    /// Overrides the default value for `SnackBar::dismiss_direction`.
    ///
    /// If null, `SnackBar` will default to [`DismissDirection::Down`].
    pub dismiss_direction: Option<DismissDirection>,
}

impl SnackBarThemeData {
    /// Creates a theme that can be used for `ThemeData::snack_bar_theme`; Dart's optional
    /// named arguments are the fluent setters.
    ///
    /// The [`elevation`](Self::elevation) must be null or non-negative.
    pub fn new() -> SnackBarThemeData {
        SnackBarThemeData::default()
    }

    /// Dart `SnackBarThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> SnackBarThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `SnackBarThemeData(actionTextColor:)`.
    pub fn action_text_color(
        mut self,
        action_text_color: impl Into<AnyColor>,
    ) -> SnackBarThemeData {
        self.action_text_color = Some(action_text_color.into());
        self
    }

    /// Dart `SnackBarThemeData(disabledActionTextColor:)`.
    pub fn disabled_action_text_color(
        mut self,
        disabled_action_text_color: Color,
    ) -> SnackBarThemeData {
        self.disabled_action_text_color = Some(disabled_action_text_color);
        self
    }

    /// Dart `SnackBarThemeData(contentTextStyle:)`.
    pub fn content_text_style(mut self, content_text_style: TextStyle) -> SnackBarThemeData {
        self.content_text_style = Some(content_text_style);
        self
    }

    /// Dart `SnackBarThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> SnackBarThemeData {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `SnackBarThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> SnackBarThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `SnackBarThemeData(behavior:)`.
    pub fn behavior(mut self, behavior: SnackBarBehavior) -> SnackBarThemeData {
        debug_assert!(
            self.width.is_none() || behavior == SnackBarBehavior::Floating,
            "Width can only be set if behaviour is SnackBarBehavior.floating"
        );
        self.behavior = Some(behavior);
        self
    }

    /// Dart `SnackBarThemeData(width:)`.
    pub fn width(mut self, width: f64) -> SnackBarThemeData {
        debug_assert!(
            self.behavior
                .is_none_or(|behavior| behavior == SnackBarBehavior::Floating),
            "Width can only be set if behaviour is SnackBarBehavior.floating"
        );
        self.width = Some(width);
        self
    }

    /// Dart `SnackBarThemeData(insetPadding:)`.
    pub fn inset_padding(mut self, inset_padding: EdgeInsets) -> SnackBarThemeData {
        self.inset_padding = Some(inset_padding);
        self
    }

    /// Dart `SnackBarThemeData(showCloseIcon:)`.
    pub fn show_close_icon(mut self, show_close_icon: bool) -> SnackBarThemeData {
        self.show_close_icon = Some(show_close_icon);
        self
    }

    /// Dart `SnackBarThemeData(closeIconColor:)`.
    pub fn close_icon_color(mut self, close_icon_color: impl Into<AnyColor>) -> SnackBarThemeData {
        self.close_icon_color = Some(close_icon_color.into());
        self
    }

    /// Dart `SnackBarThemeData(actionOverflowThreshold:)`.
    pub fn action_overflow_threshold(
        mut self,
        action_overflow_threshold: f64,
    ) -> SnackBarThemeData {
        debug_assert!(
            (0.0..=1.0).contains(&action_overflow_threshold),
            "Action overflow threshold must be between 0 and 1 inclusive"
        );
        self.action_overflow_threshold = Some(action_overflow_threshold);
        self
    }

    /// Dart `SnackBarThemeData(actionBackgroundColor:)`.
    pub fn action_background_color(
        mut self,
        action_background_color: impl Into<AnyColor>,
    ) -> SnackBarThemeData {
        let action_background_color = action_background_color.into();
        debug_assert!(
            !is_widget_state_color(&action_background_color)
                || self.disabled_action_background_color.is_none(),
            "disabledBackgroundColor must not be provided when background color is a WidgetStateColor"
        );
        self.action_background_color = Some(action_background_color);
        self
    }

    /// Dart `SnackBarThemeData(disabledActionBackgroundColor:)`.
    pub fn disabled_action_background_color(
        mut self,
        disabled_action_background_color: Color,
    ) -> SnackBarThemeData {
        debug_assert!(
            !self
                .action_background_color
                .as_ref()
                .is_some_and(is_widget_state_color),
            "disabledBackgroundColor must not be provided when background color is a WidgetStateColor"
        );
        self.disabled_action_background_color = Some(disabled_action_background_color);
        self
    }

    /// Dart `SnackBarThemeData(dismissDirection:)`.
    pub fn dismiss_direction(mut self, dismiss_direction: DismissDirection) -> SnackBarThemeData {
        self.dismiss_direction = Some(dismiss_direction);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(2.0)`).
    pub fn copy_with(&self) -> SnackBarThemeData {
        self.clone()
    }

    /// Linearly interpolate between two SnackBar themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&SnackBarThemeData>,
        b: Option<&SnackBarThemeData>,
        t: f64,
    ) -> SnackBarThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        let lerped = SnackBarThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            action_text_color: AnyColor::lerp(
                a.and_then(|a| a.action_text_color.as_ref()),
                b.and_then(|b| b.action_text_color.as_ref()),
                t,
            )
            .map(AnyColor::from),
            disabled_action_text_color: Color::lerp(
                a.and_then(|a| a.disabled_action_text_color),
                b.and_then(|b| b.disabled_action_text_color),
                t,
            ),
            content_text_style: TextStyle::lerp(
                a.and_then(|a| a.content_text_style.as_ref()),
                b.and_then(|b| b.content_text_style.as_ref()),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.shape.as_deref()),
                b.and_then(|b| b.shape.as_deref()),
                t,
            ),
            behavior: if t < 0.5 {
                a.and_then(|a| a.behavior)
            } else {
                b.and_then(|b| b.behavior)
            },
            width: lerp_double(a.and_then(|a| a.width), b.and_then(|b| b.width), t),
            inset_padding: EdgeInsets::lerp(
                a.and_then(|a| a.inset_padding),
                b.and_then(|b| b.inset_padding),
                t,
            ),
            show_close_icon: if t < 0.5 {
                a.and_then(|a| a.show_close_icon)
            } else {
                b.and_then(|b| b.show_close_icon)
            },
            close_icon_color: AnyColor::lerp(
                a.and_then(|a| a.close_icon_color.as_ref()),
                b.and_then(|b| b.close_icon_color.as_ref()),
                t,
            )
            .map(AnyColor::from),
            action_overflow_threshold: lerp_double(
                a.and_then(|a| a.action_overflow_threshold),
                b.and_then(|b| b.action_overflow_threshold),
                t,
            ),
            action_background_color: AnyColor::lerp(
                a.and_then(|a| a.action_background_color.as_ref()),
                b.and_then(|b| b.action_background_color.as_ref()),
                t,
            )
            .map(AnyColor::from),
            disabled_action_background_color: Color::lerp(
                a.and_then(|a| a.disabled_action_background_color),
                b.and_then(|b| b.disabled_action_background_color),
                t,
            ),
            dismiss_direction: if t < 0.5 {
                a.and_then(|a| a.dismiss_direction)
            } else {
                b.and_then(|b| b.dismiss_direction)
            },
        };
        lerped.debug_assert_valid();
        lerped
    }
}

impl SnackBarThemeData {
    /// Dart's constructor asserts, for a value assembled field by field ([`lerp`](Self::lerp)).
    fn debug_assert_valid(&self) {
        debug_assert!(self.elevation.is_none_or(|elevation| elevation >= 0.0));
        debug_assert!(
            self.width.is_none() || self.behavior == Some(SnackBarBehavior::Floating),
            "Width can only be set if behaviour is SnackBarBehavior.floating"
        );
        debug_assert!(
            self.action_overflow_threshold
                .is_none_or(|threshold| (0.0..=1.0).contains(&threshold)),
            "Action overflow threshold must be between 0 and 1 inclusive"
        );
        debug_assert!(
            !self
                .action_background_color
                .as_ref()
                .is_some_and(is_widget_state_color)
                || self.disabled_action_background_color.is_none(),
            "disabledBackgroundColor must not be provided when background color is a WidgetStateColor"
        );
    }
}

/// Dart's `color is WidgetStateColor`.
fn is_widget_state_color(color: &AnyColor) -> bool {
    color.extension::<WidgetStateColor>().is_some()
}

/// An inherited widget that defines the configuration for
/// `SnackBar`s in this widget's subtree.
///
/// Values specified here are used for `SnackBar` properties that are not
/// given an explicit non-null value.
#[derive(Debug)]
pub struct SnackBarTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant `SnackBar` widgets.
    pub data: SnackBarThemeData,
    pub child: WidgetRef,
}

impl SnackBarTheme {
    /// Creates a snackbar theme that controls the configurations for
    /// `SnackBar`s in its widget subtree.
    pub fn new<K>(data: SnackBarThemeData, child: impl IntoWidget<K>) -> SnackBarTheme {
        SnackBarTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `SnackBarTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> SnackBarTheme {
        self.key = Some(key);
        self
    }

    /// The closest instance of this class's [`data`](Self::data) value that encloses the given
    /// context.
    ///
    /// If there is no ancestor, it returns `ThemeData::snack_bar_theme`. Applications
    /// can assume that the returned value will not be null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = SnackBarTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> SnackBarThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<SnackBarTheme>(app) {
            Some(snack_bar_theme) => snack_bar_theme.data.clone(),
            None => Theme::of(app, context).snack_bar_theme,
        }
    }
}

impl InheritedTheme for SnackBarTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        SnackBarTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for SnackBarTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &SnackBarTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
    use waft_widgets::{SizedBox, WidgetState};

    use super::*;
    use crate::colors::Colors;
    use crate::drawer_theme::tests::theme_seen_through_capture;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(radius))
    }

    fn widget_state_color() -> AnyColor {
        WidgetStateColor::resolve_with(|states| {
            if states.contains(&WidgetState::Disabled) {
                Colors::BLUE.primary()
            } else {
                Colors::PURPLE.primary()
            }
        })
        .into_any()
    }

    #[test]
    fn snack_bar_theme_data_copy_with_equality_basics() {
        assert_eq!(
            SnackBarThemeData::new(),
            SnackBarThemeData::new().copy_with()
        );

        let theme = SnackBarThemeData::new()
            .elevation(4.0)
            .shape(rounded(8.0))
            .behavior(SnackBarBehavior::Floating)
            .width(300.0);
        assert_eq!(theme.copy_with(), theme);
        assert_eq!(
            theme
                .copy_with()
                .dismiss_direction(DismissDirection::Up)
                .elevation,
            Some(4.0)
        );
        assert_eq!(
            theme
                .copy_with()
                .dismiss_direction(DismissDirection::Up)
                .dismiss_direction,
            Some(DismissDirection::Up)
        );
        assert_ne!(theme.copy_with().elevation(5.0), theme);
        assert_ne!(theme.copy_with().shape(rounded(9.0)), theme);
    }

    #[test]
    fn snack_bar_theme_data_lerp_special_cases() {
        assert_eq!(
            SnackBarThemeData::lerp(None, None, 0.0),
            SnackBarThemeData::new()
        );
        let data = SnackBarThemeData::new();
        assert_eq!(SnackBarThemeData::lerp(Some(&data), Some(&data), 0.5), data);
    }

    #[test]
    fn snack_bar_theme_data_lerp_keeps_show_close_icon() {
        let a = SnackBarThemeData::new().show_close_icon(true);
        let b = SnackBarThemeData::new().show_close_icon(false);

        assert_eq!(
            SnackBarThemeData::lerp(Some(&a), Some(&b), 0.25).show_close_icon,
            Some(true)
        );
        assert_eq!(
            SnackBarThemeData::lerp(Some(&a), Some(&b), 0.75).show_close_icon,
            Some(false)
        );
        assert_eq!(
            SnackBarThemeData::lerp(Some(&a), None, 0.25).show_close_icon,
            Some(true)
        );
        assert_eq!(
            SnackBarThemeData::lerp(None, Some(&a), 0.75).show_close_icon,
            Some(true)
        );
    }

    #[test]
    fn snack_bar_theme_data_null_fields_by_default() {
        let snack_bar_theme = SnackBarThemeData::new();
        assert_eq!(snack_bar_theme.background_color, None);
        assert_eq!(snack_bar_theme.action_text_color, None);
        assert_eq!(snack_bar_theme.disabled_action_text_color, None);
        assert_eq!(snack_bar_theme.content_text_style, None);
        assert_eq!(snack_bar_theme.elevation, None);
        assert!(snack_bar_theme.shape.is_none());
        assert_eq!(snack_bar_theme.behavior, None);
        assert_eq!(snack_bar_theme.width, None);
        assert_eq!(snack_bar_theme.inset_padding, None);
        assert_eq!(snack_bar_theme.show_close_icon, None);
        assert_eq!(snack_bar_theme.close_icon_color, None);
        assert_eq!(snack_bar_theme.action_overflow_threshold, None);
        assert_eq!(snack_bar_theme.action_background_color, None);
        assert_eq!(snack_bar_theme.disabled_action_background_color, None);
        assert_eq!(snack_bar_theme.dismiss_direction, None);
    }

    #[test]
    fn snack_bar_theme_data_lerps_each_field() {
        let a = SnackBarThemeData::new()
            .background_color(Colors::BLACK)
            .action_text_color(Colors::BLACK)
            .elevation(2.0)
            .shape(rounded(4.0))
            .behavior(SnackBarBehavior::Floating)
            .width(100.0)
            .inset_padding(EdgeInsets::all(10.0))
            .action_overflow_threshold(0.2)
            .dismiss_direction(DismissDirection::Down);
        let b = SnackBarThemeData::new()
            .background_color(Colors::WHITE)
            .action_text_color(Colors::WHITE)
            .close_icon_color(Colors::RED.primary())
            .elevation(6.0)
            .shape(rounded(8.0))
            .behavior(SnackBarBehavior::Floating)
            .width(300.0)
            .inset_padding(EdgeInsets::all(30.0))
            .action_overflow_threshold(0.6)
            .dismiss_direction(DismissDirection::Up);

        let halfway = SnackBarThemeData::lerp(Some(&a), Some(&b), 0.5);

        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.action_text_color.map(|color| color.color()),
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            halfway.close_icon_color.map(|color| color.color()),
            Color::lerp(None, Some(Colors::RED.primary()), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(
            halfway.shape.as_deref(),
            Some(&rounded(6.0) as &dyn ShapeBorder)
        );
        assert_eq!(halfway.behavior, Some(SnackBarBehavior::Floating));
        assert_eq!(halfway.width, Some(200.0));
        assert_eq!(halfway.inset_padding, Some(EdgeInsets::all(20.0)));
        assert_eq!(halfway.action_overflow_threshold, Some(0.4));
        assert_eq!(halfway.dismiss_direction, Some(DismissDirection::Up));
        assert_eq!(
            SnackBarThemeData::lerp(Some(&a), Some(&b), 0.25).dismiss_direction,
            Some(DismissDirection::Down)
        );
    }

    #[test]
    #[should_panic(expected = "Width can only be set if behaviour is SnackBarBehavior.floating")]
    fn snack_bar_theme_data_asserts_if_width_is_provided_with_fixed_behavior() {
        SnackBarThemeData::new()
            .behavior(SnackBarBehavior::Fixed)
            .width(300.0);
    }

    #[test]
    #[should_panic(expected = "Width can only be set if behaviour is SnackBarBehavior.floating")]
    fn snack_bar_theme_data_asserts_if_fixed_behavior_is_provided_with_width() {
        SnackBarThemeData::new()
            .width(300.0)
            .behavior(SnackBarBehavior::Fixed);
    }

    #[test]
    fn snack_bar_theme_data_accepts_width_with_floating_behavior_in_either_order() {
        let width_first = SnackBarThemeData::new()
            .width(300.0)
            .behavior(SnackBarBehavior::Floating);
        let behavior_first = SnackBarThemeData::new()
            .behavior(SnackBarBehavior::Floating)
            .width(300.0);

        assert_eq!(width_first, behavior_first);
    }

    #[test]
    fn snack_bar_theme_data_asserts_for_action_overflow_threshold_outside_of_0_to_1() {
        for overflow_threshold in [-1.0, -0.0001, 1.000001, 5.0] {
            let result = std::panic::catch_unwind(|| {
                SnackBarThemeData::new().action_overflow_threshold(overflow_threshold)
            });
            assert!(result.is_err(), "{overflow_threshold} was accepted");
        }
        for overflow_threshold in [0.0, 0.5, 1.0] {
            assert_eq!(
                SnackBarThemeData::new()
                    .action_overflow_threshold(overflow_threshold)
                    .action_overflow_threshold,
                Some(overflow_threshold)
            );
        }
    }

    #[test]
    #[should_panic]
    fn snack_bar_theme_data_asserts_for_a_negative_elevation() {
        SnackBarThemeData::new().elevation(-1.0);
    }

    #[test]
    #[should_panic(
        expected = "disabledBackgroundColor must not be provided when background color is a WidgetStateColor"
    )]
    fn snack_bar_theme_data_asserts_when_action_background_color_is_a_widget_state_color_and_disabled_action_background_color_is_also_provided()
     {
        SnackBarThemeData::new()
            .action_background_color(widget_state_color())
            .disabled_action_background_color(Colors::AMBER.primary());
    }

    #[test]
    #[should_panic(
        expected = "disabledBackgroundColor must not be provided when background color is a WidgetStateColor"
    )]
    fn snack_bar_theme_data_asserts_when_disabled_action_background_color_is_provided_before_a_widget_state_color()
     {
        SnackBarThemeData::new()
            .disabled_action_background_color(Colors::AMBER.primary())
            .action_background_color(widget_state_color());
    }

    #[test]
    fn snack_bar_theme_data_keeps_a_widget_state_color_as_the_action_background_color() {
        let theme = SnackBarThemeData::new().action_background_color(widget_state_color());

        let color = theme.action_background_color.expect("a color");

        assert!(color.extension::<WidgetStateColor>().is_some());
        assert_eq!(color.color(), Colors::PURPLE.primary());
    }

    #[test]
    fn snack_bar_theme_data_accepts_a_plain_action_background_color_with_a_disabled_one() {
        let theme = SnackBarThemeData::new()
            .action_background_color(Colors::PURPLE.primary())
            .disabled_action_background_color(Colors::AMBER.primary());

        assert_eq!(
            theme.disabled_action_background_color,
            Some(Colors::AMBER.primary())
        );
    }

    #[test]
    fn of_returns_the_theme_data_snack_bar_theme_without_an_ancestor() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let expected = SnackBarThemeData::new()
                .elevation(3.0)
                .background_color(Colors::RED.primary());
            let theme_data_snack_bar = expected.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .snack_bar_theme(theme_data_snack_bar.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                SnackBarTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_snack_bar_theme_over_theme_data() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let widget_snack_bar = SnackBarThemeData::new().elevation(7.0);
            let expected = widget_snack_bar.clone();

            let seen = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .snack_bar_theme(SnackBarThemeData::new().elevation(3.0))
                        .build(app);
                    Theme::new(data, SnackBarTheme::new(widget_snack_bar.clone(), child))
                        .into_widget()
                },
                SnackBarTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn a_captured_snack_bar_theme_is_kept_under_another_snack_bar_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let captured = SnackBarThemeData::new().elevation(1.0);
            let shadowing = SnackBarThemeData::new().elevation(2.0);

            let seen = theme_seen_through_capture(
                &cell,
                |probe| SnackBarTheme::new(captured.clone(), probe).into_widget(),
                |wrapped| SnackBarTheme::new(shadowing.clone(), wrapped).into_widget(),
                SnackBarTheme::of,
            );

            assert_eq!(seen.elevation, Some(1.0));
        });
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: SnackBarThemeData| SnackBarTheme::new(data, SizedBox::shrink());
        let old = theme(SnackBarThemeData::new().elevation(1.0));

        assert!(!theme(SnackBarThemeData::new().elevation(1.0)).update_should_notify(&old));
        assert!(theme(SnackBarThemeData::new().elevation(2.0)).update_should_notify(&old));
    }

    #[test]
    fn theme_data_holds_the_snack_bar_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.snack_bar_theme, SnackBarThemeData::new());

            let raised = ThemeData::builder()
                .snack_bar_theme(SnackBarThemeData::new().elevation(8.0))
                .build(&app);
            assert_eq!(raised.snack_bar_theme.elevation, Some(8.0));
            assert_ne!(theme, raised);
            assert_eq!(
                theme
                    .copy_with()
                    .snack_bar_theme(raised.snack_bar_theme.clone())
                    .build(),
                raised
            );

            let flat = ThemeData::builder()
                .snack_bar_theme(SnackBarThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &raised, 0.5)
                    .snack_bar_theme
                    .elevation,
                Some(4.0)
            );
        });
    }
}
