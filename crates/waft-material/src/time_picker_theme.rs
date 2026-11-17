//! Flutter counterpart: material_ui's `time_picker_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{
    AnyColor, AnyTextStyle, BorderSide, EdgeInsetsGeometry, OutlinedBorderRef, ShapeBorderRef,
    TextStyle,
};
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef, WidgetState,
    WidgetStateColor, WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::input_decorator::InputDecorationThemeData;
use crate::theme::Theme;

/// Defines the visual properties of the widget displayed with `showTimePicker`.
///
/// Descendant widgets obtain the current [`TimePickerThemeData`] object using
/// [`TimePickerTheme::of`]. Instances of [`TimePickerThemeData`] can be customized with
/// [`TimePickerThemeData::copy_with`].
///
/// Typically a [`TimePickerThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::time_picker_theme`.
///
/// All [`TimePickerThemeData`] properties are `None` by default. When null, the time picker
/// provides its own defaults based on the overall `Theme`'s `ThemeData::color_scheme`,
/// `ThemeData::text_theme`, and `ThemeData::icon_theme`.
///
/// See also:
///
///  * [`TimePickerTheme`], which describes the actual configuration of a time picker theme.
///  * `ThemeData`, which describes the overall theme information for the application.
#[derive(Clone, Debug, Default)]
pub struct TimePickerThemeData {
    /// The background color of a time picker.
    ///
    /// If this is null, the time picker defaults to the overall theme's
    /// `ColorScheme::surface_container_high`.
    pub background_color: Option<AnyColor>,

    /// Style of the cancel button of a time picker.
    ///
    /// If this is null, the time picker defaults to a `TextButton` styled with the theme's
    /// primary color.
    pub cancel_button_style: Option<ButtonStyle>,

    /// Style of the confirm (OK) button of a time picker.
    ///
    /// If this is null, the time picker defaults to a `TextButton` styled with the theme's
    /// primary color.
    pub confirm_button_style: Option<ButtonStyle>,

    /// The color and weight of the day period's outline.
    ///
    /// If this is null, the time picker defaults to
    /// `BorderSide(color: Color(0xFF79747E))`.
    pub day_period_border_side: Option<BorderSide>,

    /// Dart's private `_dayPeriodColor`: the color as it was given, before
    /// [`day_period_color_getter`](Self::day_period_color_getter) wraps a plain color.
    ///
    /// Keep the raw value here: the getter's wrapped color never equals another wrapped color.
    pub day_period_color: Option<AnyColor>,

    /// The shape of the day period that the time picker uses.
    ///
    /// If this is null, the time picker defaults to a `RoundedRectangleBorder` with a
    /// circular corner radius of 4.0.
    pub day_period_shape: Option<OutlinedBorderRef>,

    /// The color of the day period text that represents AM/PM.
    ///
    /// If this is null, the time picker defaults to `ColorScheme::on_surface_variant` when the
    /// day period is not selected, and `ColorScheme::on_tertiary_container` when it is.
    pub day_period_text_color: Option<AnyColor>,

    /// Used to configure the `TextStyle`s for the day period control.
    ///
    /// If this is null, the time picker defaults to the overall theme's
    /// `TextTheme::title_medium`.
    pub day_period_text_style: Option<AnyTextStyle>,

    /// The background color of the time picker dial when the entry mode is
    /// `TimePickerEntryMode::dial` or `TimePickerEntryMode::dialOnly`.
    ///
    /// If this is null and `ThemeData::use_material3` is true, the time picker dial background
    /// color defaults to `ColorScheme::surface_container_highest`.
    pub dial_background_color: Option<AnyColor>,

    /// The color of the time picker dial's hand when the entry mode is
    /// `TimePickerEntryMode::dial` or `TimePickerEntryMode::dialOnly`.
    ///
    /// If this is null and `ThemeData::use_material3` is true, the time picker dial hand color
    /// defaults to `ColorScheme::primary`.
    pub dial_hand_color: Option<AnyColor>,

    /// The color of the dial text that represents specific hours and minutes.
    ///
    /// If this is null, the time picker defaults to `ColorScheme::on_surface`.
    pub dial_text_color: Option<AnyColor>,

    /// The `TextStyle` of the dial text that represents specific hours and minutes.
    ///
    /// If this is null, the time picker defaults to the overall theme's
    /// `TextTheme::body_large`.
    pub dial_text_style: Option<AnyTextStyle>,

    /// The Material elevation for the time picker dialog.
    pub elevation: Option<f64>,

    /// The color of the entry mode `IconButton`.
    ///
    /// If this is null, the time picker defaults to `ColorScheme::on_surface_variant`.
    pub entry_mode_icon_color: Option<AnyColor>,

    /// Used to configure the `TextStyle` for the helper text in the header.
    ///
    /// If this is null, the time picker defaults to the overall theme's
    /// `TextTheme::label_medium`.
    pub help_text_style: Option<AnyTextStyle>,

    /// The background color of the hour and minute controls.
    ///
    /// If this is null, the time picker defaults to `ColorScheme::secondary_container`.
    pub hour_minute_color: Option<AnyColor>,

    /// The shape of the hour and minute controls that the time picker uses.
    ///
    /// If this is null, the time picker defaults to
    /// `RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(8.0)))`.
    pub hour_minute_shape: Option<ShapeBorderRef>,

    /// The color of the header text that represents hours and minutes.
    ///
    /// If this is null, the time picker defaults to `ColorScheme::on_secondary_container`.
    pub hour_minute_text_color: Option<AnyColor>,

    /// Used to configure the text style for the hour and minute controls.
    ///
    /// If this is null, the time picker defaults to the overall theme's
    /// `TextTheme::display_medium`.
    pub hour_minute_text_style: Option<AnyTextStyle>,

    /// The input decoration theme for the `TextField`s in the time picker.
    ///
    /// If this is null, the time picker provides its own defaults based on the overall
    /// theme's `ColorScheme`.
    pub input_decoration_theme: Option<InputDecorationThemeData>,

    /// The padding around the time picker dialog when the entry mode is
    /// `TimePickerEntryMode::dial` or `TimePickerEntryMode::dialOnly`.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The shape of the dialog that the time picker is presented in.
    ///
    /// If this is null, the time picker defaults to
    /// `RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(28.0)))`.
    pub shape: Option<ShapeBorderRef>,

    /// The color of the time selector separator between the hour and minute controls.
    ///
    /// If this is null, the time picker defaults to `ColorScheme::on_surface`.
    pub time_selector_separator_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The text style of the time selector separator between the hour and minute controls.
    ///
    /// If this is null, the time picker defaults to the overall theme's
    /// `TextTheme::display_large`, with the color of
    /// [`time_selector_separator_color`](Self::time_selector_separator_color).
    pub time_selector_separator_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `TimePickerThemeData`, as the lazy-defaults trait: the theme data
    /// answers with its fields, and the time picker's defaults (`_TimePickerDefaultsM3`) answer
    /// with computed values.
    ///
    /// `day_period_color` answers the raw private `_dayPeriodColor`; the wrapped getter of
    /// Dart is [`TimePickerThemeData::day_period_color_getter`].
    pub trait TimePickerThemeDataGetters for TimePickerThemeData, fields: time_picker_theme_data {
        background_color: Option<AnyColor>,
        cancel_button_style: Option<ButtonStyle>,
        confirm_button_style: Option<ButtonStyle>,
        day_period_border_side: Option<BorderSide>,
        day_period_color: Option<AnyColor>,
        day_period_shape: Option<OutlinedBorderRef>,
        day_period_text_color: Option<AnyColor>,
        day_period_text_style: Option<AnyTextStyle>,
        dial_background_color: Option<AnyColor>,
        dial_hand_color: Option<AnyColor>,
        dial_text_color: Option<AnyColor>,
        dial_text_style: Option<AnyTextStyle>,
        elevation: Option<f64>,
        entry_mode_icon_color: Option<AnyColor>,
        help_text_style: Option<AnyTextStyle>,
        hour_minute_color: Option<AnyColor>,
        hour_minute_shape: Option<ShapeBorderRef>,
        hour_minute_text_color: Option<AnyColor>,
        hour_minute_text_style: Option<AnyTextStyle>,
        input_decoration_theme: Option<InputDecorationThemeData>,
        padding: Option<EdgeInsetsGeometry>,
        shape: Option<ShapeBorderRef>,
        time_selector_separator_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        time_selector_separator_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
    }
}

impl TimePickerThemeData {
    /// Creates a theme that can be used for `ThemeData::time_picker_theme`.
    ///
    /// Dart's named arguments are the fluent setters.
    pub fn new() -> TimePickerThemeData {
        TimePickerThemeData::default()
    }

    /// Dart's `dayPeriodColor` getter.
    ///
    /// The color of the day period's background: a [`WidgetStateColor`] that is the given color
    /// when the day period is selected and transparent otherwise, unless the given color is
    /// already a state color, which is returned as it is.
    pub fn day_period_color_getter(&self) -> Option<AnyColor> {
        let day_period_color = self.day_period_color.as_ref()?;
        if day_period_color.has_extension() {
            return Some(day_period_color.clone());
        }
        let selected_color = day_period_color.color();
        Some(AnyColor::from(WidgetStateColor::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Selected) {
                    return selected_color;
                }
                // The unselected day period should match the overall picker dialog color.
                // Making it transparent enables that without being redundant and allows
                // the optional elevation overlay for dark mode to be visible.
                Colors::TRANSPARENT
            },
        )))
    }

    /// Dart `TimePickerThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: impl Into<AnyColor>) -> Self {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `TimePickerThemeData(cancelButtonStyle:)`.
    pub fn cancel_button_style(mut self, cancel_button_style: ButtonStyle) -> Self {
        self.cancel_button_style = Some(cancel_button_style);
        self
    }

    /// Dart `TimePickerThemeData(confirmButtonStyle:)`.
    pub fn confirm_button_style(mut self, confirm_button_style: ButtonStyle) -> Self {
        self.confirm_button_style = Some(confirm_button_style);
        self
    }

    /// Dart `TimePickerThemeData(dayPeriodBorderSide:)`.
    pub fn day_period_border_side(mut self, day_period_border_side: BorderSide) -> Self {
        self.day_period_border_side = Some(day_period_border_side);
        self
    }

    /// Dart `TimePickerThemeData(dayPeriodColor:)`.
    pub fn day_period_color(mut self, day_period_color: impl Into<AnyColor>) -> Self {
        self.day_period_color = Some(day_period_color.into());
        self
    }

    /// Dart `TimePickerThemeData(dayPeriodShape:)`.
    pub fn day_period_shape(mut self, day_period_shape: impl Into<OutlinedBorderRef>) -> Self {
        self.day_period_shape = Some(day_period_shape.into());
        self
    }

    /// Dart `TimePickerThemeData(dayPeriodTextColor:)`.
    pub fn day_period_text_color(mut self, day_period_text_color: impl Into<AnyColor>) -> Self {
        self.day_period_text_color = Some(day_period_text_color.into());
        self
    }

    /// Dart `TimePickerThemeData(dayPeriodTextStyle:)`.
    pub fn day_period_text_style(mut self, day_period_text_style: impl Into<AnyTextStyle>) -> Self {
        self.day_period_text_style = Some(day_period_text_style.into());
        self
    }

    /// Dart `TimePickerThemeData(dialBackgroundColor:)`.
    pub fn dial_background_color(mut self, dial_background_color: impl Into<AnyColor>) -> Self {
        self.dial_background_color = Some(dial_background_color.into());
        self
    }

    /// Dart `TimePickerThemeData(dialHandColor:)`.
    pub fn dial_hand_color(mut self, dial_hand_color: impl Into<AnyColor>) -> Self {
        self.dial_hand_color = Some(dial_hand_color.into());
        self
    }

    /// Dart `TimePickerThemeData(dialTextColor:)`.
    pub fn dial_text_color(mut self, dial_text_color: impl Into<AnyColor>) -> Self {
        self.dial_text_color = Some(dial_text_color.into());
        self
    }

    /// Dart `TimePickerThemeData(dialTextStyle:)`.
    pub fn dial_text_style(mut self, dial_text_style: impl Into<AnyTextStyle>) -> Self {
        self.dial_text_style = Some(dial_text_style.into());
        self
    }

    /// Dart `TimePickerThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `TimePickerThemeData(entryModeIconColor:)`.
    pub fn entry_mode_icon_color(mut self, entry_mode_icon_color: impl Into<AnyColor>) -> Self {
        self.entry_mode_icon_color = Some(entry_mode_icon_color.into());
        self
    }

    /// Dart `TimePickerThemeData(helpTextStyle:)`.
    pub fn help_text_style(mut self, help_text_style: impl Into<AnyTextStyle>) -> Self {
        self.help_text_style = Some(help_text_style.into());
        self
    }

    /// Dart `TimePickerThemeData(hourMinuteColor:)`.
    pub fn hour_minute_color(mut self, hour_minute_color: impl Into<AnyColor>) -> Self {
        self.hour_minute_color = Some(hour_minute_color.into());
        self
    }

    /// Dart `TimePickerThemeData(hourMinuteShape:)`.
    pub fn hour_minute_shape(mut self, hour_minute_shape: impl Into<ShapeBorderRef>) -> Self {
        self.hour_minute_shape = Some(hour_minute_shape.into());
        self
    }

    /// Dart `TimePickerThemeData(hourMinuteTextColor:)`.
    pub fn hour_minute_text_color(mut self, hour_minute_text_color: impl Into<AnyColor>) -> Self {
        self.hour_minute_text_color = Some(hour_minute_text_color.into());
        self
    }

    /// Dart `TimePickerThemeData(hourMinuteTextStyle:)`.
    pub fn hour_minute_text_style(
        mut self,
        hour_minute_text_style: impl Into<AnyTextStyle>,
    ) -> Self {
        self.hour_minute_text_style = Some(hour_minute_text_style.into());
        self
    }

    /// Dart `TimePickerThemeData(inputDecorationTheme:)`.
    pub fn input_decoration_theme(
        mut self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> Self {
        self.input_decoration_theme = Some(input_decoration_theme.into());
        self
    }

    /// Dart `TimePickerThemeData(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `TimePickerThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> Self {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `TimePickerThemeData(timeSelectorSeparatorColor:)`.
    pub fn time_selector_separator_color(
        mut self,
        time_selector_separator_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> Self {
        self.time_selector_separator_color = Some(time_selector_separator_color);
        self
    }

    /// Dart `TimePickerThemeData(timeSelectorSeparatorTextStyle:)`.
    pub fn time_selector_separator_text_style(
        mut self,
        time_selector_separator_text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> Self {
        self.time_selector_separator_text_style = Some(time_selector_separator_text_style);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(4.0)`).
    pub fn copy_with(&self) -> TimePickerThemeData {
        self.clone()
    }

    /// Linearly interpolate between two time picker themes.
    pub fn lerp(
        a: Option<&TimePickerThemeData>,
        b: Option<&TimePickerThemeData>,
        t: f64,
    ) -> TimePickerThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        // Workaround since BorderSide's lerp does not allow for null arguments.
        let day_period_border_side = match (
            a.and_then(|a| a.day_period_border_side),
            b.and_then(|b| b.day_period_border_side),
        ) {
            (None, None) => None,
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (Some(a), Some(b)) => Some(BorderSide::lerp(a, b, t)),
        };
        let lerp_color = |a: Option<&AnyColor>, b: Option<&AnyColor>| {
            Color::lerp(a.map(AnyColor::color), b.map(AnyColor::color), t).map(AnyColor::from)
        };
        let lerp_text_style = |a: Option<&AnyTextStyle>, b: Option<&AnyTextStyle>| {
            TextStyle::lerp(a.map(AnyTextStyle::value), b.map(AnyTextStyle::value), t)
                .map(AnyTextStyle::new)
        };
        TimePickerThemeData {
            background_color: lerp_color(
                a.and_then(|a| a.background_color.as_ref()),
                b.and_then(|b| b.background_color.as_ref()),
            ),
            cancel_button_style: ButtonStyle::lerp(
                a.and_then(|a| a.cancel_button_style.as_ref()),
                b.and_then(|b| b.cancel_button_style.as_ref()),
                t,
            ),
            confirm_button_style: ButtonStyle::lerp(
                a.and_then(|a| a.confirm_button_style.as_ref()),
                b.and_then(|b| b.confirm_button_style.as_ref()),
                t,
            ),
            day_period_border_side,
            day_period_color: lerp_color(
                a.and_then(|a| a.day_period_color_getter()).as_ref(),
                b.and_then(|b| b.day_period_color_getter()).as_ref(),
            ),
            day_period_shape: OutlinedBorderRef::lerp(
                a.and_then(|a| a.day_period_shape.as_ref()),
                b.and_then(|b| b.day_period_shape.as_ref()),
                t,
            ),
            day_period_text_color: lerp_color(
                a.and_then(|a| a.day_period_text_color.as_ref()),
                b.and_then(|b| b.day_period_text_color.as_ref()),
            ),
            day_period_text_style: lerp_text_style(
                a.and_then(|a| a.day_period_text_style.as_ref()),
                b.and_then(|b| b.day_period_text_style.as_ref()),
            ),
            dial_background_color: lerp_color(
                a.and_then(|a| a.dial_background_color.as_ref()),
                b.and_then(|b| b.dial_background_color.as_ref()),
            ),
            dial_hand_color: lerp_color(
                a.and_then(|a| a.dial_hand_color.as_ref()),
                b.and_then(|b| b.dial_hand_color.as_ref()),
            ),
            dial_text_color: lerp_color(
                a.and_then(|a| a.dial_text_color.as_ref()),
                b.and_then(|b| b.dial_text_color.as_ref()),
            ),
            dial_text_style: lerp_text_style(
                a.and_then(|a| a.dial_text_style.as_ref()),
                b.and_then(|b| b.dial_text_style.as_ref()),
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            entry_mode_icon_color: lerp_color(
                a.and_then(|a| a.entry_mode_icon_color.as_ref()),
                b.and_then(|b| b.entry_mode_icon_color.as_ref()),
            ),
            help_text_style: lerp_text_style(
                a.and_then(|a| a.help_text_style.as_ref()),
                b.and_then(|b| b.help_text_style.as_ref()),
            ),
            hour_minute_color: lerp_color(
                a.and_then(|a| a.hour_minute_color.as_ref()),
                b.and_then(|b| b.hour_minute_color.as_ref()),
            ),
            hour_minute_shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.hour_minute_shape.as_ref()),
                b.and_then(|b| b.hour_minute_shape.as_ref()),
                t,
            ),
            hour_minute_text_color: lerp_color(
                a.and_then(|a| a.hour_minute_text_color.as_ref()),
                b.and_then(|b| b.hour_minute_text_color.as_ref()),
            ),
            hour_minute_text_style: lerp_text_style(
                a.and_then(|a| a.hour_minute_text_style.as_ref()),
                b.and_then(|b| b.hour_minute_text_style.as_ref()),
            ),
            input_decoration_theme: if t < 0.5 {
                a.and_then(|a| a.input_decoration_theme.clone())
            } else {
                b.and_then(|b| b.input_decoration_theme.clone())
            },
            padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.padding),
                b.and_then(|b| b.padding),
                t,
            ),
            shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            time_selector_separator_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.time_selector_separator_color.clone()),
                b.and_then(|b| b.time_selector_separator_color.clone()),
                t,
                Color::lerp,
            ),
            time_selector_separator_text_style:
                <dyn WidgetStateProperty<Option<TextStyle>>>::lerp_optional(
                    a.and_then(|a| a.time_selector_separator_text_style.clone()),
                    b.and_then(|b| b.time_selector_separator_text_style.clone()),
                    t,
                    |a, b, t| TextStyle::lerp(a.as_ref(), b.as_ref(), t),
                ),
        }
    }
}

impl PartialEq for TimePickerThemeData {
    fn eq(&self, other: &TimePickerThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.background_color == self.background_color
            && other.cancel_button_style == self.cancel_button_style
            && other.confirm_button_style == self.confirm_button_style
            && other.day_period_border_side == self.day_period_border_side
            && other.day_period_color_getter() == self.day_period_color_getter()
            && other.day_period_shape == self.day_period_shape
            && other.day_period_text_color == self.day_period_text_color
            && other.day_period_text_style == self.day_period_text_style
            && other.dial_background_color == self.dial_background_color
            && other.dial_hand_color == self.dial_hand_color
            && other.dial_text_color == self.dial_text_color
            && other.dial_text_style == self.dial_text_style
            && other.elevation == self.elevation
            && other.entry_mode_icon_color == self.entry_mode_icon_color
            && other.help_text_style == self.help_text_style
            && other.hour_minute_color == self.hour_minute_color
            && other.hour_minute_shape == self.hour_minute_shape
            && other.hour_minute_text_color == self.hour_minute_text_color
            && other.hour_minute_text_style == self.hour_minute_text_style
            && other.input_decoration_theme == self.input_decoration_theme
            && other.padding == self.padding
            && other.shape == self.shape
            && other.time_selector_separator_color == self.time_selector_separator_color
            && other.time_selector_separator_text_style == self.time_selector_separator_text_style
    }
}

/// An inherited widget that defines the configuration for time pickers displayed using
/// `showTimePicker` in this widget's subtree.
///
/// Values specified here are used for time picker properties that are not given an explicit
/// non-null value.
#[derive(Debug)]
pub struct TimePickerTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant time picker widgets.
    pub data: TimePickerThemeData,
    pub child: WidgetRef,
}

impl TimePickerTheme {
    /// Creates a time picker theme that controls the configurations for descendant time
    /// pickers.
    pub fn new<K>(data: TimePickerThemeData, child: impl IntoWidget<K>) -> TimePickerTheme {
        TimePickerTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `TimePickerTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> TimePickerTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`TimePickerThemeData`] from the closest ancestor [`TimePickerTheme`].
    ///
    /// If there is no enclosing [`TimePickerTheme`] widget, then
    /// `ThemeData::time_picker_theme` is used.
    pub fn of(app: &mut App, context: BuildContext) -> TimePickerThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<TimePickerTheme>(app) {
            Some(time_picker_theme) => time_picker_theme.data.clone(),
            None => Theme::of(app, context).time_picker_theme.clone(),
        }
    }
}

impl InheritedTheme for TimePickerTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        TimePickerTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for TimePickerTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &TimePickerTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use std::rc::Rc;

    use waft_embedder::{Radius, Size};
    use waft_foundation::Listener;
    use waft_painting::{
        BorderRadius, BorderRadiusGeometry, FontStyle, RoundedRectangleBorder, ShapeBorderRef,
    };
    use waft_widgets::{
        AnyElement, Builder, Center, IntoWidget, Text, WidgetStateColor, WidgetStatePropertyAll,
        WidgetStates,
    };

    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::dialog::Dialog;
    use crate::elevated_button::ElevatedButton;
    use crate::icon_button::IconButton;
    use crate::input_border::OutlineInputBorder;
    use crate::input_decorator::InputDecorationThemeData;
    use crate::material::Material;
    use crate::test_support::{PaintInspection, material_app};

    use crate::text_button::TextButton;
    use crate::text_field::TextField;
    use crate::text_theme::TextTheme;
    use crate::theme_data::ThemeData;
    use crate::time::TimeOfDay;
    use crate::time_picker::tests::{dial_label_styles, dial_paint};
    use crate::time_picker::{ShowTimePicker, TimePickerEntryMode};
    use crate::tooltip::tests::{m2, m3};
    use crate::typography::Typography;
    use waft_rendering::testing::{PaintCall, assert_paints};

    fn resolve(color: &AnyColor, states: &WidgetStates) -> Color {
        <dyn WidgetStateProperty<Color>>::resolve_as(color, states).color()
    }

    #[test]
    fn time_picker_theme_data_copy_with_equals_and_hash_code_basics() {
        assert_eq!(
            TimePickerThemeData::new(),
            TimePickerThemeData::new().copy_with()
        );
    }

    #[test]
    fn time_picker_theme_data_lerp_special_cases() {
        let data = TimePickerThemeData::new();
        assert_eq!(
            TimePickerThemeData::lerp(Some(&data), Some(&data), 0.5),
            data
        );
    }

    #[test]
    fn time_picker_theme_data_has_null_fields_by_default() {
        let time_picker_theme = TimePickerThemeData::new();
        assert_eq!(time_picker_theme.background_color, None);
        assert!(time_picker_theme.cancel_button_style.is_none());
        assert!(time_picker_theme.confirm_button_style.is_none());
        assert_eq!(time_picker_theme.day_period_border_side, None);
        assert_eq!(time_picker_theme.day_period_color_getter(), None);
        assert!(time_picker_theme.day_period_shape.is_none());
        assert_eq!(time_picker_theme.day_period_text_color, None);
        assert_eq!(time_picker_theme.day_period_text_style, None);
        assert_eq!(time_picker_theme.dial_background_color, None);
        assert_eq!(time_picker_theme.dial_hand_color, None);
        assert_eq!(time_picker_theme.dial_text_color, None);
        assert_eq!(time_picker_theme.dial_text_style, None);
        assert_eq!(time_picker_theme.elevation, None);
        assert_eq!(time_picker_theme.entry_mode_icon_color, None);
        assert_eq!(time_picker_theme.help_text_style, None);
        assert_eq!(time_picker_theme.hour_minute_color, None);
        assert!(time_picker_theme.hour_minute_shape.is_none());
        assert_eq!(time_picker_theme.hour_minute_text_color, None);
        assert_eq!(time_picker_theme.hour_minute_text_style, None);
        assert!(time_picker_theme.input_decoration_theme.is_none());
        assert_eq!(time_picker_theme.padding, None);
        assert!(time_picker_theme.shape.is_none());
        assert!(time_picker_theme.time_selector_separator_color.is_none());
        assert!(
            time_picker_theme
                .time_selector_separator_text_style
                .is_none()
        );
    }

    #[test]
    fn day_period_color_getter_is_transparent_unless_selected_for_a_plain_color() {
        let red = Color::new(0xffff0000);
        let theme = TimePickerThemeData::new().day_period_color(red);
        let wrapped = theme.day_period_color_getter().expect("a color");
        let selected: WidgetStates = [WidgetState::Selected].into_iter().collect();
        assert_eq!(
            resolve(&wrapped, &selected),
            red,
            "a selected day period shows the given color"
        );
        assert_eq!(
            resolve(&wrapped, &WidgetStates::new()),
            Colors::TRANSPARENT,
            "an unselected day period matches the dialog"
        );
    }

    #[test]
    fn day_period_color_getter_keeps_a_widget_state_color_as_it_is() {
        let state_color = WidgetStateColor::resolve_with(|_| Color::new(0xff00ff00));
        let theme = TimePickerThemeData::new().day_period_color(state_color);
        let wrapped = theme.day_period_color_getter().expect("a color");
        assert_eq!(
            resolve(&wrapped, &WidgetStates::new()),
            Color::new(0xff00ff00)
        );
        assert_eq!(theme, theme.copy_with());
    }

    fn selected_color() -> Color {
        Colors::GREEN.shade100()
    }

    fn unselected_color() -> Color {
        Colors::GREEN.shade200()
    }

    /// Dart's `Color.withOpacity`.
    fn opacity(color: Color, opacity: f64) -> Color {
        color.with_alpha((255.0 * opacity).round() as i32)
    }

    fn state_color() -> WidgetStateColor {
        WidgetStateColor::resolve_with(|states| {
            if states.contains(&WidgetState::Selected) {
                selected_color()
            } else {
                unselected_color()
            }
        })
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(radius))
    }

    /// Dart's `_timePickerTheme`.
    fn time_picker_theme(include_input_decoration: bool) -> TimePickerThemeData {
        let color = |color: Color| BorderSide {
            color,
            ..BorderSide::default()
        };
        let outline = |side_color: Color| OutlineInputBorder::new().border_side(color(side_color));
        let mut theme = TimePickerThemeData::new()
            .background_color(Colors::ORANGE.primary())
            .cancel_button_style(
                TextButton::style_from()
                    .foreground_color(Colors::RED.primary())
                    .build(),
            )
            .confirm_button_style(
                TextButton::style_from()
                    .foreground_color(Colors::GREEN.primary())
                    .build(),
            )
            .hour_minute_text_color(state_color())
            .hour_minute_color(state_color())
            .day_period_text_color(state_color())
            .day_period_color(state_color())
            .dial_hand_color(Colors::BROWN.primary())
            .dial_background_color(Colors::PINK_ACCENT.primary())
            .dial_text_color(state_color())
            .entry_mode_icon_color(Colors::RED.primary())
            .hour_minute_text_style(TextStyle::new().font_size(8.0))
            .day_period_text_style(TextStyle::new().font_size(8.0))
            .help_text_style(TextStyle::new().font_size(8.0))
            .shape(rounded(16.0))
            .hour_minute_shape(rounded(16.0))
            .day_period_shape(rounded(16.0))
            .day_period_border_side(color(Colors::BLUE_ACCENT.primary()));
        if include_input_decoration {
            theme = theme.input_decoration_theme(
                InputDecorationThemeData::new()
                    .filled(true)
                    .fill_color(Colors::PURPLE.primary())
                    .enabled_border(outline(Colors::BLUE.primary()))
                    .error_border(outline(Colors::GREEN.primary()))
                    .focused_border(outline(Colors::YELLOW.primary()))
                    .focused_error_border(outline(Colors::RED.primary()))
                    .hint_style(TextStyle::new().font_size(8.0)),
            );
        }
        theme
    }

    /// Dart's `_TimePickerLauncher`: a button that shows the time picker at 7:15, tapped.
    fn launch(tester: &mut WidgetTester, theme: ThemeData, entry_mode: TimePickerEntryMode) {
        let launcher = Material::new()
            .child(Center::new().child(Builder::new(move |_app, context| {
                ElevatedButton::new(
                    Some(Listener::new(move |app| {
                        drop({
                            let mut show = ShowTimePicker::new(TimeOfDay::new(7, 15));
                            show = show.initial_entry_mode(entry_mode);
                            show.show(app, context)
                        });
                    })),
                    Some(Text::new("X")),
                )
                .into_widget()
            })))
            .into_widget();
        tester.pump_widget(material_app(theme, launcher));
        let x = *find::text("X")
            .evaluate(tester)
            .first()
            .expect("the X button");
        let center = tester.get_center(x);
        tester.tap_at(center, 1);
        tester.pump_and_settle_by(Duration::from_secs(1));
    }

    fn text_element(tester: &WidgetTester, text: &str) -> AnyElement {
        tester.element(find::text(text).first())
    }

    /// Dart's `_dialogMaterial`.
    fn dialog_material(tester: &WidgetTester) -> (Option<Color>, Option<ShapeBorderRef>) {
        let material = tester.element(
            find::descendant(find::by_type::<Dialog>().at(0), find::by_type::<Material>()).first(),
        );
        tester.widget(material, |material: &Material| {
            (material.color, material.shape.clone())
        })
    }

    /// Dart's `_textMaterial`: the first `Material` above the text.
    fn text_material(tester: &WidgetTester, text: &str) -> (Option<Color>, Option<ShapeBorderRef>) {
        let material =
            tester.element(find::ancestor(find::text(text), find::by_type::<Material>()).first());
        tester.widget(material, |material: &Material| {
            (material.color, material.shape.clone())
        })
    }

    fn paragraph_style(tester: &WidgetTester, text: &str) -> TextStyle {
        crate::bottom_navigation_bar::tests::paragraph_style(tester, text)
    }

    /// Dart's `_textField(tester, text).decoration` and `.style`.
    fn text_field(tester: &WidgetTester, text: &str) -> TextField {
        let field =
            tester.element(find::ancestor(find::text(text), find::by_type::<TextField>()).first());
        tester.widget(field, |field: &TextField| field.clone())
    }

    fn entry_mode_icon_color(tester: &WidgetTester) -> Option<Color> {
        let button = tester.element(
            find::descendant(
                find::by_type::<Dialog>().at(0),
                find::by_type::<IconButton>(),
            )
            .first(),
        );
        tester.widget(button, |button: &IconButton| button.color)
    }

    /// Dart's `_actionButtonStyle`.
    fn action_button_style(tester: &WidgetTester, text: &str) -> ButtonStyle {
        let button =
            tester.element(find::ancestor(find::text(text), find::by_type::<TextButton>()).first());
        tester.widget(button, |button: &TextButton| {
            crate::button_style_button::ButtonStyleButton::button_style_button_data(button)
                .style
                .clone()
                .expect("a styled button")
        })
    }

    /// What the dial's `CustomPaint` paints: Dart's `paints` on its render object.
    fn dial_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
        tester.painted_ops_deep(tester.render_object(dial_paint(tester)))
    }

    /// The style a text gets from a typography: Dart's
    /// `typography.englishLike.x!.merge(typography.black.x)`.
    fn base_style(
        typography: &Typography,
        white: bool,
        field: fn(&TextTheme) -> &Option<TextStyle>,
    ) -> TextStyle {
        let tint = if white {
            &typography.white
        } else {
            &typography.black
        };
        field(&typography.english_like)
            .as_ref()
            .expect("an english-like style")
            .merge(field(tint).as_ref())
    }

    fn color_of(style: &TextStyle) -> Option<Color> {
        style.color.as_ref().map(|color| color.color())
    }

    fn rounded_with_side(side_color: Color, top_left: bool, radius: f64) -> Option<ShapeBorderRef> {
        let radius = Radius::circular(radius);
        let none = Radius::ZERO;
        let border_radius = if top_left {
            BorderRadius::only(radius, none, radius, none)
        } else {
            BorderRadius::only(none, radius, none, radius)
        };
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide {
                color: side_color,
                ..BorderSide::default()
            },
            BorderRadiusGeometry::from(border_radius),
        )))
    }

    #[test]
    fn material2_passing_no_time_picker_theme_data_uses_defaults() {
        test_widgets(|tester| {
            let default_theme = m2(&tester.cell.borrow());
            launch(tester, default_theme.clone(), TimePickerEntryMode::Dial);
            let scheme = &default_theme.color_scheme;
            let typography = Typography::material2014().build();

            let (color, shape) = dialog_material(tester);
            assert_eq!(color, Some(scheme.surface));
            assert_eq!(shape, Some(ShapeBorderRef::new(rounded(4.0))));

            assert_paints(
                &dial_ops(tester),
                &[
                    PaintCall::circle(Some(opacity(scheme.on_surface, 0.08))),
                    PaintCall::circle(Some(scheme.primary)),
                ],
            );

            let display_medium =
                |white| base_style(&typography, white, |theme| &theme.display_medium);
            let title_medium = base_style(&typography, false, |theme| &theme.title_medium);
            assert_eq!(
                paragraph_style(tester, "7"),
                display_medium(false).copy_with().color(scheme.primary)
            );
            assert_eq!(
                paragraph_style(tester, "15"),
                display_medium(false).copy_with().color(scheme.on_surface)
            );
            assert_eq!(
                paragraph_style(tester, "AM"),
                title_medium.copy_with().color(scheme.primary)
            );
            assert_eq!(
                paragraph_style(tester, "PM"),
                title_medium
                    .copy_with()
                    .color(opacity(scheme.on_surface, 0.6))
            );
            assert_eq!(
                paragraph_style(tester, "SELECT TIME"),
                base_style(&typography, false, |theme| &theme.label_small)
            );

            let (primary, selected) = dial_label_styles(tester);
            assert_eq!(
                primary,
                base_style(&typography, false, |theme| &theme.body_large)
                    .copy_with()
                    .color(scheme.on_surface)
            );
            assert_eq!(
                selected,
                base_style(&typography, true, |theme| &theme.body_large)
                    .copy_with()
                    .color(scheme.on_primary)
            );

            let (color, shape) = text_material(tester, "7");
            assert_eq!(color, Some(opacity(scheme.primary, 0.12)));
            assert_eq!(shape, Some(ShapeBorderRef::new(rounded(4.0))));
            let (color, shape) = text_material(tester, "15");
            assert_eq!(color, Some(opacity(scheme.on_surface, 0.12)));
            assert_eq!(shape, Some(ShapeBorderRef::new(rounded(4.0))));

            let (am_color, am_shape) = text_material(tester, "AM");
            assert_eq!(am_color, Some(opacity(scheme.primary, 0.12)));
            let (pm_color, pm_shape) = text_material(tester, "PM");
            assert_eq!(pm_color, Some(Colors::TRANSPARENT));

            let border_color = Color::alpha_blend(opacity(scheme.on_surface, 0.38), scheme.surface);
            assert_eq!(am_shape, rounded_with_side(border_color, true, 4.0));
            assert_eq!(pm_shape, rounded_with_side(border_color, false, 4.0));

            assert_eq!(
                entry_mode_icon_color(tester),
                Some(opacity(scheme.on_surface, 0.6))
            );

            let default_style = TextButton::style_from().build();
            assert_eq!(action_button_style(tester, "CANCEL"), default_style);
            assert_eq!(action_button_style(tester, "OK"), default_style);
        });
    }

    /// The style an M3 text gets: the typography's, with the colors the dialog gives it.
    fn m3_style(
        typography: &Typography,
        field: fn(&TextTheme) -> &Option<TextStyle>,
        color: Color,
        decoration_color: Color,
    ) -> TextStyle {
        base_style(typography, false, field)
            .copy_with()
            .color(color)
            .decoration_color(decoration_color)
    }

    #[test]
    fn material3_passing_no_time_picker_theme_data_uses_defaults() {
        test_widgets(|tester| {
            let default_theme = m3(&tester.cell.borrow());
            launch(tester, default_theme.clone(), TimePickerEntryMode::Dial);
            let scheme = &default_theme.color_scheme;
            let typography = Typography::material2021().build();
            let decoration = scheme.on_surface;

            let (color, shape) = dialog_material(tester);
            assert_eq!(color, Some(scheme.surface_container_high()));
            assert_eq!(shape, Some(ShapeBorderRef::new(rounded(28.0))));

            assert_paints(
                &dial_ops(tester),
                &[
                    PaintCall::circle(Some(scheme.surface_container_highest())),
                    PaintCall::circle(Some(scheme.primary)),
                ],
            );

            assert_eq!(
                paragraph_style(tester, "7"),
                m3_style(
                    &typography,
                    |theme| &theme.display_large,
                    scheme.on_primary_container(),
                    decoration
                )
            );
            assert_eq!(
                paragraph_style(tester, "15"),
                m3_style(
                    &typography,
                    |theme| &theme.display_large,
                    scheme.on_surface,
                    decoration
                )
            );
            assert_eq!(
                paragraph_style(tester, "AM"),
                m3_style(
                    &typography,
                    |theme| &theme.title_medium,
                    scheme.on_tertiary_container(),
                    decoration
                )
            );
            assert_eq!(
                paragraph_style(tester, "PM"),
                m3_style(
                    &typography,
                    |theme| &theme.title_medium,
                    scheme.on_surface_variant(),
                    decoration
                )
            );
            assert_eq!(
                paragraph_style(tester, "Select time"),
                m3_style(
                    &typography,
                    |theme| &theme.body_medium,
                    scheme.on_surface,
                    decoration
                )
            );

            let (primary, selected) = dial_label_styles(tester);
            assert_eq!(
                primary,
                m3_style(
                    &typography,
                    |theme| &theme.body_large,
                    scheme.on_surface,
                    decoration
                )
            );
            assert_eq!(
                selected,
                m3_style(
                    &typography,
                    |theme| &theme.body_large,
                    scheme.on_primary,
                    decoration
                )
            );

            let (color, shape) = text_material(tester, "7");
            assert_eq!(color, Some(scheme.primary_container()));
            assert_eq!(shape, Some(ShapeBorderRef::new(rounded(8.0))));
            let (color, shape) = text_material(tester, "15");
            assert_eq!(color, Some(scheme.surface_container_highest()));
            assert_eq!(shape, Some(ShapeBorderRef::new(rounded(8.0))));

            let (am_color, am_shape) = text_material(tester, "AM");
            assert_eq!(am_color, Some(scheme.tertiary_container()));
            let (pm_color, pm_shape) = text_material(tester, "PM");
            assert_eq!(pm_color, Some(Colors::TRANSPARENT));
            assert_eq!(am_shape, rounded_with_side(scheme.outline(), true, 8.0));
            assert_eq!(pm_shape, rounded_with_side(scheme.outline(), false, 8.0));

            assert_eq!(entry_mode_icon_color(tester), None);

            let default_style = TextButton::style_from().build();
            assert_eq!(action_button_style(tester, "Cancel"), default_style);
            assert_eq!(action_button_style(tester, "OK"), default_style);
        });
    }

    fn outline_border(side_color: Color, width: f64, radius: Option<f64>) -> OutlineInputBorder {
        let border = OutlineInputBorder::new().border_side(BorderSide {
            color: side_color,
            width,
            ..BorderSide::default()
        });
        match radius {
            Some(radius) => border.border_radius(BorderRadius::circular(radius)),
            None => border,
        }
    }

    fn assert_borders(
        decoration: &crate::input_decorator::InputDecoration,
        scheme: &crate::color_scheme::ColorScheme,
        radius: Option<f64>,
    ) {
        assert_eq!(
            decoration.enabled_border,
            Some(outline_border(Colors::TRANSPARENT, 1.0, radius).into())
        );
        assert_eq!(
            decoration.error_border,
            Some(outline_border(scheme.error, 2.0, radius).into())
        );
        assert_eq!(
            decoration.focused_border,
            Some(outline_border(scheme.primary, 2.0, radius).into())
        );
        assert_eq!(
            decoration.focused_error_border,
            Some(outline_border(scheme.error, 2.0, radius).into())
        );
    }

    fn resolved(color: &Option<AnyColor>) -> Option<Color> {
        color
            .as_ref()
            .map(|color| resolve(color, &WidgetStates::new()))
    }

    #[test]
    fn material2_passing_no_time_picker_theme_data_uses_defaults_input_mode() {
        test_widgets(|tester| {
            let default_theme = m2(&tester.cell.borrow());
            launch(tester, default_theme.clone(), TimePickerEntryMode::Input);
            let scheme = &default_theme.color_scheme;
            let typography = Typography::material2014().build();

            let decoration = text_field(tester, "7")
                .decoration
                .expect("a decorated field");
            assert_eq!(decoration.filled, Some(true));
            assert_eq!(
                resolved(&decoration.fill_color),
                Some(opacity(scheme.on_surface, 0.12))
            );
            assert_borders(&decoration, scheme, None);
            let hint_style = typography
                .english_like
                .display_medium
                .as_ref()
                .expect("a display medium")
                .merge(Some(
                    &default_theme
                        .text_theme
                        .display_medium
                        .as_ref()
                        .expect("a display medium")
                        .copy_with()
                        .color(opacity(scheme.on_surface, 0.36)),
                ));
            assert_eq!(
                decoration
                    .hint_style
                    .as_ref()
                    .map(|style| style.value().clone()),
                Some(hint_style)
            );

            let default_style = TextButton::style_from().build();
            assert_eq!(action_button_style(tester, "CANCEL"), default_style);
            assert_eq!(action_button_style(tester, "OK"), default_style);
        });
    }

    #[test]
    fn material3_passing_no_time_picker_theme_data_uses_defaults_input_mode() {
        test_widgets(|tester| {
            let default_theme = m3(&tester.cell.borrow());
            launch(tester, default_theme.clone(), TimePickerEntryMode::Input);
            let scheme = &default_theme.color_scheme;
            let typography = Typography::material2021().build();

            let expected = m3_style(
                &typography,
                |theme| &theme.display_medium,
                scheme.on_surface,
                scheme.on_surface,
            );
            let field = text_field(tester, "7");
            assert_eq!(
                field.style.as_ref().map(|style| style.value().clone()),
                Some(expected.clone())
            );
            assert_eq!(
                text_field(tester, "15")
                    .style
                    .as_ref()
                    .map(|style| style.value().clone()),
                Some(expected)
            );

            let decoration = field.decoration.expect("a decorated field");
            assert_eq!(decoration.filled, Some(true));
            assert_eq!(
                resolved(&decoration.fill_color),
                Some(scheme.surface_container_highest())
            );
            assert_borders(&decoration, scheme, Some(8.0));
            assert_eq!(
                decoration
                    .hint_style
                    .as_ref()
                    .map(|style| style.value().clone()),
                Some(TextStyle::new().color(opacity(scheme.on_surface, 0.36)))
            );

            let default_style = TextButton::style_from().build();
            assert_eq!(action_button_style(tester, "Cancel"), default_style);
            assert_eq!(action_button_style(tester, "OK"), default_style);
        });
    }

    fn themed(theme_data: ThemeData, time_picker_theme: &TimePickerThemeData) -> ThemeData {
        theme_data
            .copy_with()
            .time_picker_theme(time_picker_theme.clone())
            .build()
    }

    fn resolved_color(color: &Option<AnyColor>) -> Option<Color> {
        color.as_ref().map(|color| color.color())
    }

    fn style_value(style: &Option<AnyTextStyle>) -> TextStyle {
        style.as_ref().expect("a text style").value().clone()
    }

    /// What "Time picker uses values from TimePickerThemeData" asserts of both designs: the
    /// dialog, the dial, the hour and minute and day period materials, the entry mode icon and
    /// the action buttons.
    fn assert_theme_values(
        tester: &WidgetTester,
        theme: &TimePickerThemeData,
        entry_mode_icon_color_expected: Option<Color>,
        cancel: &str,
    ) {
        let (color, shape) = dialog_material(tester);
        assert_eq!(color, resolved_color(&theme.background_color));
        assert_eq!(shape, theme.shape.clone());

        assert_paints(
            &dial_ops(tester),
            &[
                PaintCall::circle(Some(
                    resolved_color(&theme.dial_background_color).expect("a dial color"),
                )),
                PaintCall::circle(Some(
                    resolved_color(&theme.dial_hand_color).expect("a hand color"),
                )),
            ],
        );

        let (color, shape) = text_material(tester, "7");
        assert_eq!(color, Some(selected_color()));
        assert_eq!(shape, theme.hour_minute_shape.clone());
        let (color, shape) = text_material(tester, "15");
        assert_eq!(color, Some(unselected_color()));
        assert_eq!(shape, theme.hour_minute_shape.clone());

        let (am_color, am_shape) = text_material(tester, "AM");
        assert_eq!(am_color, Some(selected_color()));
        let (pm_color, pm_shape) = text_material(tester, "PM");
        assert_eq!(pm_color, Some(unselected_color()));
        let side = theme.day_period_border_side.expect("a border side").color;
        assert_eq!(am_shape, rounded_with_side(side, true, 16.0));
        assert_eq!(pm_shape, rounded_with_side(side, false, 16.0));

        assert_eq!(
            entry_mode_icon_color(tester),
            entry_mode_icon_color_expected
        );
        assert_eq!(
            action_button_style(tester, cancel),
            theme.cancel_button_style.clone().expect("a cancel style")
        );
        assert_eq!(
            action_button_style(tester, "OK"),
            theme.confirm_button_style.clone().expect("a confirm style")
        );
    }

    #[test]
    fn material2_time_picker_uses_values_from_time_picker_theme_data() {
        test_widgets(|tester| {
            let theme = time_picker_theme(false);
            let theme_data = themed(m2(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Dial);
            let typography = Typography::material2014().build();

            assert_theme_values(
                tester,
                &theme,
                resolved_color(&theme.entry_mode_icon_color),
                "CANCEL",
            );

            let with_theme = |field: fn(&TextTheme) -> &Option<TextStyle>,
                              themed_style: &Option<AnyTextStyle>,
                              color: Color| {
                base_style(&typography, false, field)
                    .merge(Some(&style_value(themed_style)))
                    .copy_with()
                    .color(color)
            };
            assert_eq!(
                paragraph_style(tester, "7"),
                with_theme(
                    |theme| &theme.body_medium,
                    &theme.hour_minute_text_style,
                    selected_color()
                )
            );
            assert_eq!(
                paragraph_style(tester, "15"),
                with_theme(
                    |theme| &theme.body_medium,
                    &theme.hour_minute_text_style,
                    unselected_color()
                )
            );
            assert_eq!(
                paragraph_style(tester, "AM"),
                with_theme(
                    |theme| &theme.title_medium,
                    &theme.day_period_text_style,
                    selected_color()
                )
            );
            assert_eq!(
                paragraph_style(tester, "PM"),
                with_theme(
                    |theme| &theme.title_medium,
                    &theme.day_period_text_style,
                    unselected_color()
                )
            );
            assert_eq!(
                paragraph_style(tester, "SELECT TIME"),
                base_style(&typography, false, |theme| &theme.body_medium)
                    .merge(Some(&style_value(&theme.help_text_style)))
            );

            let (primary, selected) = dial_label_styles(tester);
            assert_eq!(
                primary,
                base_style(&typography, false, |theme| &theme.body_large)
                    .copy_with()
                    .color(unselected_color())
            );
            assert_eq!(
                selected,
                base_style(&typography, true, |theme| &theme.body_large)
                    .copy_with()
                    .color(selected_color())
            );
        });
    }

    #[test]
    fn material3_time_picker_uses_values_from_time_picker_theme_data() {
        test_widgets(|tester| {
            let theme = time_picker_theme(false);
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data.clone(), TimePickerEntryMode::Dial);
            let typography = Typography::material2021().build();
            let decoration_color = Color::new(0xff1d1b20);
            let on_surface = theme_data.color_scheme.on_surface;

            assert_theme_values(tester, &theme, None, "Cancel");

            let hour_minute = |color: Color| {
                base_style(&typography, false, |theme| &theme.body_medium)
                    .merge(Some(&style_value(&theme.hour_minute_text_style)))
                    .copy_with()
                    .color(color)
                    .decoration_color(decoration_color)
            };
            assert_eq!(paragraph_style(tester, "7"), hour_minute(selected_color()));
            assert_eq!(
                paragraph_style(tester, "15"),
                hour_minute(unselected_color())
            );
            assert_eq!(paragraph_style(tester, "AM"), hour_minute(selected_color()));
            assert_eq!(
                paragraph_style(tester, "PM"),
                hour_minute(unselected_color())
            );
            assert_eq!(
                paragraph_style(tester, "Select time"),
                base_style(&typography, false, |theme| &theme.body_medium)
                    .merge(Some(&style_value(&theme.help_text_style)))
                    .copy_with()
                    .color(on_surface)
                    .decoration_color(on_surface)
            );

            let (primary, selected) = dial_label_styles(tester);
            assert_eq!(
                primary,
                base_style(&typography, false, |theme| &theme.body_large)
                    .copy_with()
                    .color(unselected_color())
                    .decoration_color(on_surface)
            );
            assert_eq!(
                selected,
                base_style(&typography, false, |theme| &theme.body_large)
                    .copy_with()
                    .color(selected_color())
                    .decoration_color(on_surface)
            );
        });
    }

    #[test]
    fn time_picker_uses_values_from_time_picker_theme_data_when_input_decoration_theme_is_provided_input_mode()
     {
        test_widgets(|tester| {
            let theme = time_picker_theme(true);
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            let decoration = text_field(tester, "7")
                .decoration
                .expect("a decorated field");
            let input = theme
                .input_decoration_theme
                .expect("an input decoration theme");
            assert_eq!(decoration.filled, Some(input.filled));
            assert_eq!(decoration.fill_color, input.fill_color);
            assert_eq!(decoration.enabled_border, input.enabled_border);
            assert_eq!(decoration.error_border, input.error_border);
            assert_eq!(decoration.focused_border, input.focused_border);
            assert_eq!(decoration.focused_error_border, input.focused_error_border);
            assert_eq!(decoration.hint_style, input.hint_style);
        });
    }

    #[test]
    fn time_picker_uses_values_from_time_picker_theme_data_when_input_decoration_theme_is_not_provided_input_mode()
     {
        test_widgets(|tester| {
            let theme = time_picker_theme(false);
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            let decoration = text_field(tester, "7")
                .decoration
                .expect("a decorated field");
            let fill = decoration.fill_color.expect("a fill color");
            let hour_minute = theme.hour_minute_color.expect("an hour minute color");
            // Dart compares the `value`s of the two colors; the picker resolves the theme's
            // color with the focus states, so the unfocused color is compared.
            let states = WidgetStates::new();
            assert_eq!(resolve(&fill, &states), resolve(&hour_minute, &states));
        });
    }

    #[test]
    fn time_picker_day_period_color_does_the_right_thing_with_non_widget_state_color() {
        test_widgets(|tester| {
            let theme = time_picker_theme(false).day_period_color(Colors::RED.primary());
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            assert_eq!(text_material(tester, "AM").0, Some(Colors::RED.primary()));
            assert_eq!(text_material(tester, "PM").0, Some(Colors::TRANSPARENT));
        });
    }

    #[test]
    fn time_picker_day_period_color_does_the_right_thing_with_widget_state_color() {
        test_widgets(|tester| {
            let test_color = WidgetStateColor::resolve_with(|states| {
                if states.contains(&WidgetState::Selected) {
                    Colors::GREEN.primary()
                } else {
                    Colors::BLUE.primary()
                }
            });
            let theme = time_picker_theme(false).day_period_color(test_color);
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            assert_eq!(text_material(tester, "AM").0, Some(Colors::GREEN.primary()));
            assert_eq!(text_material(tester, "PM").0, Some(Colors::BLUE.primary()));
        });
    }

    #[test]
    fn time_selector_separator_color_uses_the_time_selector_separator_color_value() {
        test_widgets(|tester| {
            let theme = time_picker_theme(false).time_selector_separator_color(Rc::new(
                WidgetStatePropertyAll::new(Some(Color::new(0xff00ff00))),
            ));
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            assert_eq!(
                color_of(&paragraph_style(tester, ":")),
                Some(Color::new(0xff00ff00))
            );
        });
    }

    #[test]
    fn time_selector_separator_text_style_uses_the_time_selector_separator_text_style_value() {
        test_widgets(|tester| {
            let theme = time_picker_theme(false).time_selector_separator_text_style(Rc::new(
                WidgetStatePropertyAll::new(Some(
                    TextStyle::new()
                        .font_size(35.0)
                        .font_style(FontStyle::Italic),
                )),
            ));
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            let style = paragraph_style(tester, ":");
            assert_eq!(style.font_size, Some(35.0));
            assert_eq!(style.font_style, Some(FontStyle::Italic));
        });
    }

    fn border_painter_size(tester: &WidgetTester) -> Size {
        tester.get_size(
            find::descendant(
                find::by_type::<crate::input_decorator::InputDecorator>().first(),
                find::by_type::<waft_widgets::CustomPaint>(),
            )
            .first(),
        )
    }

    fn enter_invalid_hour_and_tap_ok(tester: &mut WidgetTester) {
        let editable = tester.element(find::by_type::<waft_widgets::EditableText>().at(0));
        let state = editable
            .state_handle::<waft_widgets::EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state");
        state.request_keyboard(&mut tester.cell.borrow_mut());
        tester.pump();
        waft_services::TextInputClient::update_editing_value(
            state,
            &mut tester.cell.borrow_mut(),
            waft_embedder::TextEditingValue::new().text("AB").selection(
                waft_embedder::TextSelection::collapsed(2, waft_embedder::TextAffinity::Downstream),
            ),
        );
        tester.cell.checkpoint();
        let ok = text_element(tester, "OK");
        let center = tester.get_center(ok);
        tester.tap_at(center, 1);
    }

    #[test]
    fn time_picker_hour_minute_does_not_resize_on_error() {
        test_widgets(|tester| {
            let theme = time_picker_theme(true);
            let theme_data = themed(m3(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            assert_eq!(border_painter_size(tester), Size::new(96.0, 72.0));

            enter_invalid_hour_and_tap_ok(tester);

            assert_eq!(border_painter_size(tester), Size::new(96.0, 72.0));
        });
    }

    #[test]
    fn material2_time_picker_hour_minute_does_not_resize_on_error() {
        test_widgets(|tester| {
            let theme = time_picker_theme(true);
            let theme_data = themed(m2(&tester.cell.borrow()), &theme);
            launch(tester, theme_data, TimePickerEntryMode::Input);

            assert_eq!(border_painter_size(tester), Size::new(96.0, 70.0));

            enter_invalid_hour_and_tap_ok(tester);

            assert_eq!(border_painter_size(tester), Size::new(96.0, 70.0));
        });
    }
}
