//! Flutter counterpart: material_ui's `date_picker_theme.dart`, with the hand-coded Material 2
//! defaults of its `_DatePickerDefaultsM2`; the Material 3 ones are
//! `generated/date_picker_defaults_m3.rs`.

use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use waft_embedder::{Brightness, Color, Locale, lerp_double};
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, CircleBorder, OutlinedBorderRef, RoundedRectangleBorder,
    ShapeBorderRef, StadiumBorder, TextStyle,
};
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef, WidgetState,
    WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef, WidgetStates,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::generated::date_picker_defaults_m3::DatePickerDefaultsM3;
use crate::input_decorator::InputDecorationThemeData;
use crate::text_button::TextButton;
use crate::theme::Theme;
use crate::theme_data_getters::{DefaultsContext, theme_data_getters};

/// An inherited widget that defines the visual properties for [`DatePickerDialog`]s in this
/// widget's subtree.
///
/// Values specified here are used for [`DatePickerDialog`] properties that are not given an
/// explicit non-null value.
///
/// See also:
///
///  * [`DatePickerThemeData`], which describes the actual configuration of a date picker
///    theme.
///  * `ThemeData::date_picker_theme`, which describes the date picker theme of the overall
///    `ThemeData`.
#[derive(Debug)]
pub struct DatePickerTheme {
    pub key: Option<KeyRef>,
    /// Specifies the visual properties used by descendant [`DatePickerDialog`] widgets.
    pub data: DatePickerThemeData,
    pub child: WidgetRef,
}

impl DatePickerTheme {
    /// Creates a [`DatePickerTheme`] that controls visual parameters for descendant
    /// [`DatePickerDialog`]s.
    pub fn new<K>(data: DatePickerThemeData, child: impl IntoWidget<K>) -> DatePickerTheme {
        DatePickerTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `DatePickerTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> DatePickerTheme {
        self.key = Some(key);
        self
    }

    /// The [`data`](Self::data) from the closest instance of this class that encloses the
    /// given context, if any.
    ///
    /// If there is no [`DatePickerTheme`] in scope, this will return
    /// `ThemeData::date_picker_theme` from the ambient [`Theme`].
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), which returns null if it doesn't find a
    ///    [`DatePickerTheme`] ancestor.
    pub fn of(app: &mut App, context: BuildContext) -> DatePickerThemeData {
        Self::maybe_of(app, context)
            .unwrap_or_else(|| Theme::of(app, context).date_picker_theme.clone())
    }

    /// The data from the closest instance of this class that encloses the given context, if
    /// any.
    ///
    /// Use this function if you want to allow situations where no [`DatePickerTheme`] is in
    /// scope. Prefer using [`DatePickerTheme::of`] in situations where a
    /// [`DatePickerThemeData`] is expected to be non-null.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<DatePickerThemeData> {
        context
            .depend_on_inherited_widget_of_exact_type::<DatePickerTheme>(app)
            .map(|inherited_theme| inherited_theme.data.clone())
    }

    /// A [`DatePickerThemeData`] that is a default for the current `Theme`: the Material 3
    /// defaults when `ThemeData::use_material3` is true, and the Material 2 ones otherwise.
    pub fn defaults(app: &mut App, context: BuildContext) -> Rc<dyn DatePickerThemeDataGetters> {
        if Theme::of(app, context).use_material3 {
            Rc::new(DatePickerDefaultsM3::new(context))
        } else {
            Rc::new(DatePickerDefaultsM2::new(context))
        }
    }
}

impl InheritedTheme for DatePickerTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DatePickerTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for DatePickerTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DatePickerTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Defines the visual properties of the widget displayed with `showDatePicker` and the
/// optional [`DatePickerTheme`] ancestor.
///
/// Descendant widgets obtain the current [`DatePickerThemeData`] object using
/// [`DatePickerTheme::of`]. Instances of [`DatePickerThemeData`] can be customized with
/// [`DatePickerThemeData::copy_with`].
///
/// Typically a [`DatePickerThemeData`] is specified as part of the overall `Theme` with
/// `ThemeData::date_picker_theme`.
///
/// All [`DatePickerThemeData`] properties are null by default. When null, the
/// [`DatePickerDialog`] will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults based on the overall `Theme`'s `textTheme` and `colorScheme`.
///
/// See also:
///
///  * [`DatePickerDialog`], which is the dialog that uses this theme.
///  * [`DatePickerTheme`], which overrides the theme of the date picker in a subtree.
///  * `ThemeData::date_picker_theme`, which overrides the date picker theme in the app.
#[derive(Clone, Default)]
pub struct DatePickerThemeData {
    /// Dart's `DatePickerThemeData.backgroundColor`.
    pub background_color: Option<Color>,
    /// Dart's `DatePickerThemeData.elevation`.
    pub elevation: Option<f64>,
    /// Dart's `DatePickerThemeData.shadowColor`.
    pub shadow_color: Option<Color>,
    /// Dart's `DatePickerThemeData.surfaceTintColor`.
    pub surface_tint_color: Option<Color>,
    /// Dart's `DatePickerThemeData.shape`.
    pub shape: Option<ShapeBorderRef>,
    /// Dart's `DatePickerThemeData.headerBackgroundColor`.
    pub header_background_color: Option<Color>,
    /// Dart's `DatePickerThemeData.headerForegroundColor`.
    pub header_foreground_color: Option<Color>,
    /// Dart's `DatePickerThemeData.headerHeadlineStyle`.
    pub header_headline_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.headerHelpStyle`.
    pub header_help_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.weekdayStyle`.
    pub weekday_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.dayStyle`.
    pub day_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.dayForegroundColor`.
    pub day_foreground_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.dayBackgroundColor`.
    pub day_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.dayOverlayColor`.
    pub day_overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.dayShape`.
    pub day_shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
    /// Dart's `DatePickerThemeData.todayForegroundColor`.
    pub today_foreground_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.todayBackgroundColor`.
    pub today_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.todayBorder`.
    pub today_border: Option<BorderSide>,
    /// Dart's `DatePickerThemeData.yearStyle`.
    pub year_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.yearForegroundColor`.
    pub year_foreground_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.yearBackgroundColor`.
    pub year_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.yearOverlayColor`.
    pub year_overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.yearShape`.
    pub year_shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
    /// Dart's `DatePickerThemeData.rangePickerBackgroundColor`.
    pub range_picker_background_color: Option<Color>,
    /// Dart's `DatePickerThemeData.rangePickerElevation`.
    pub range_picker_elevation: Option<f64>,
    /// Dart's `DatePickerThemeData.rangePickerShadowColor`.
    pub range_picker_shadow_color: Option<Color>,
    /// Dart's `DatePickerThemeData.rangePickerSurfaceTintColor`.
    pub range_picker_surface_tint_color: Option<Color>,
    /// Dart's `DatePickerThemeData.rangePickerShape`.
    pub range_picker_shape: Option<ShapeBorderRef>,
    /// Dart's `DatePickerThemeData.rangePickerHeaderBackgroundColor`.
    pub range_picker_header_background_color: Option<Color>,
    /// Dart's `DatePickerThemeData.rangePickerHeaderForegroundColor`.
    pub range_picker_header_foreground_color: Option<Color>,
    /// Dart's `DatePickerThemeData.rangePickerHeaderHeadlineStyle`.
    pub range_picker_header_headline_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.rangePickerHeaderHelpStyle`.
    pub range_picker_header_help_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.rangeSelectionBackgroundColor`.
    pub range_selection_background_color: Option<Color>,
    /// Dart's `DatePickerThemeData.rangeSelectionOverlayColor`.
    pub range_selection_overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Dart's `DatePickerThemeData.dividerColor`.
    pub divider_color: Option<Color>,
    /// Dart's `DatePickerThemeData.inputDecorationTheme`.
    pub input_decoration_theme: Option<InputDecorationThemeData>,
    /// Dart's `DatePickerThemeData.cancelButtonStyle`.
    pub cancel_button_style: Option<ButtonStyle>,
    /// Dart's `DatePickerThemeData.confirmButtonStyle`.
    pub confirm_button_style: Option<ButtonStyle>,
    /// Dart's `DatePickerThemeData.locale`.
    pub locale: Option<Locale>,
    /// Dart's `DatePickerThemeData.toggleButtonTextStyle`.
    pub toggle_button_text_style: Option<TextStyle>,
    /// Dart's `DatePickerThemeData.subHeaderForegroundColor`.
    pub sub_header_foreground_color: Option<Color>,
}
theme_data_getters! {
    /// Dart's getters of `DatePickerThemeData`: the theme data answers with its fields, and a
    /// defaults subclass (`_DatePickerDefaultsM3(context)`, `_DatePickerDefaultsM2(context)`)
    /// computes the ones it overrides when they are read.
    pub trait DatePickerThemeDataGetters for DatePickerThemeData, fields: date_picker_theme_data {
        background_color: Option<Color>,
        elevation: Option<f64>,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        shape: Option<ShapeBorderRef>,
        header_background_color: Option<Color>,
        header_foreground_color: Option<Color>,
        header_headline_style: Option<TextStyle>,
        header_help_style: Option<TextStyle>,
        weekday_style: Option<TextStyle>,
        day_style: Option<TextStyle>,
        day_foreground_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        day_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        day_overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        day_shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
        today_foreground_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        today_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        today_border: Option<BorderSide>,
        year_style: Option<TextStyle>,
        year_foreground_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        year_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        year_overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        year_shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
        range_picker_background_color: Option<Color>,
        range_picker_elevation: Option<f64>,
        range_picker_shadow_color: Option<Color>,
        range_picker_surface_tint_color: Option<Color>,
        range_picker_shape: Option<ShapeBorderRef>,
        range_picker_header_background_color: Option<Color>,
        range_picker_header_foreground_color: Option<Color>,
        range_picker_header_headline_style: Option<TextStyle>,
        range_picker_header_help_style: Option<TextStyle>,
        range_selection_background_color: Option<Color>,
        range_selection_overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        divider_color: Option<Color>,
        input_decoration_theme: Option<InputDecorationThemeData>,
        cancel_button_style: Option<ButtonStyle>,
        confirm_button_style: Option<ButtonStyle>,
        locale: Option<Locale>,
        toggle_button_text_style: Option<TextStyle>,
        sub_header_foreground_color: Option<Color>,
    }
}

impl DatePickerThemeData {
    /// Creates a [`DatePickerThemeData`] that can be used to override default properties in a
    /// [`DatePickerTheme`] widget. Dart's optional named arguments are the fluent setters.
    pub fn new() -> DatePickerThemeData {
        DatePickerThemeData::default()
    }

    /// Dart `DatePickerThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> DatePickerThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `DatePickerThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> DatePickerThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `DatePickerThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> DatePickerThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `DatePickerThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> DatePickerThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `DatePickerThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> DatePickerThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `DatePickerThemeData(headerBackgroundColor:)`.
    pub fn header_background_color(
        mut self,
        header_background_color: Color,
    ) -> DatePickerThemeData {
        self.header_background_color = Some(header_background_color);
        self
    }

    /// Dart `DatePickerThemeData(headerForegroundColor:)`.
    pub fn header_foreground_color(
        mut self,
        header_foreground_color: Color,
    ) -> DatePickerThemeData {
        self.header_foreground_color = Some(header_foreground_color);
        self
    }

    /// Dart `DatePickerThemeData(headerHeadlineStyle:)`.
    pub fn header_headline_style(
        mut self,
        header_headline_style: TextStyle,
    ) -> DatePickerThemeData {
        self.header_headline_style = Some(header_headline_style);
        self
    }

    /// Dart `DatePickerThemeData(headerHelpStyle:)`.
    pub fn header_help_style(mut self, header_help_style: TextStyle) -> DatePickerThemeData {
        self.header_help_style = Some(header_help_style);
        self
    }

    /// Dart `DatePickerThemeData(weekdayStyle:)`.
    pub fn weekday_style(mut self, weekday_style: TextStyle) -> DatePickerThemeData {
        self.weekday_style = Some(weekday_style);
        self
    }

    /// Dart `DatePickerThemeData(dayStyle:)`.
    pub fn day_style(mut self, day_style: TextStyle) -> DatePickerThemeData {
        self.day_style = Some(day_style);
        self
    }

    /// Dart `DatePickerThemeData(dayForegroundColor:)`.
    pub fn day_foreground_color(
        mut self,
        day_foreground_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.day_foreground_color = Some(day_foreground_color);
        self
    }

    /// Dart `DatePickerThemeData(dayBackgroundColor:)`.
    pub fn day_background_color(
        mut self,
        day_background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.day_background_color = Some(day_background_color);
        self
    }

    /// Dart `DatePickerThemeData(dayOverlayColor:)`.
    pub fn day_overlay_color(
        mut self,
        day_overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.day_overlay_color = Some(day_overlay_color);
        self
    }

    /// Dart `DatePickerThemeData(dayShape:)`.
    pub fn day_shape(
        mut self,
        day_shape: WidgetStatePropertyRef<Option<OutlinedBorderRef>>,
    ) -> DatePickerThemeData {
        self.day_shape = Some(day_shape);
        self
    }

    /// Dart `DatePickerThemeData(todayForegroundColor:)`.
    pub fn today_foreground_color(
        mut self,
        today_foreground_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.today_foreground_color = Some(today_foreground_color);
        self
    }

    /// Dart `DatePickerThemeData(todayBackgroundColor:)`.
    pub fn today_background_color(
        mut self,
        today_background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.today_background_color = Some(today_background_color);
        self
    }

    /// Dart `DatePickerThemeData(todayBorder:)`.
    pub fn today_border(mut self, today_border: BorderSide) -> DatePickerThemeData {
        self.today_border = Some(today_border);
        self
    }

    /// Dart `DatePickerThemeData(yearStyle:)`.
    pub fn year_style(mut self, year_style: TextStyle) -> DatePickerThemeData {
        self.year_style = Some(year_style);
        self
    }

    /// Dart `DatePickerThemeData(yearForegroundColor:)`.
    pub fn year_foreground_color(
        mut self,
        year_foreground_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.year_foreground_color = Some(year_foreground_color);
        self
    }

    /// Dart `DatePickerThemeData(yearBackgroundColor:)`.
    pub fn year_background_color(
        mut self,
        year_background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.year_background_color = Some(year_background_color);
        self
    }

    /// Dart `DatePickerThemeData(yearOverlayColor:)`.
    pub fn year_overlay_color(
        mut self,
        year_overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.year_overlay_color = Some(year_overlay_color);
        self
    }

    /// Dart `DatePickerThemeData(yearShape:)`.
    pub fn year_shape(
        mut self,
        year_shape: WidgetStatePropertyRef<Option<OutlinedBorderRef>>,
    ) -> DatePickerThemeData {
        self.year_shape = Some(year_shape);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerBackgroundColor:)`.
    pub fn range_picker_background_color(
        mut self,
        range_picker_background_color: Color,
    ) -> DatePickerThemeData {
        self.range_picker_background_color = Some(range_picker_background_color);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerElevation:)`.
    pub fn range_picker_elevation(mut self, range_picker_elevation: f64) -> DatePickerThemeData {
        self.range_picker_elevation = Some(range_picker_elevation);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerShadowColor:)`.
    pub fn range_picker_shadow_color(
        mut self,
        range_picker_shadow_color: Color,
    ) -> DatePickerThemeData {
        self.range_picker_shadow_color = Some(range_picker_shadow_color);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerSurfaceTintColor:)`.
    pub fn range_picker_surface_tint_color(
        mut self,
        range_picker_surface_tint_color: Color,
    ) -> DatePickerThemeData {
        self.range_picker_surface_tint_color = Some(range_picker_surface_tint_color);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerShape:)`.
    pub fn range_picker_shape(
        mut self,
        range_picker_shape: impl Into<ShapeBorderRef>,
    ) -> DatePickerThemeData {
        self.range_picker_shape = Some(range_picker_shape.into());
        self
    }

    /// Dart `DatePickerThemeData(rangePickerHeaderBackgroundColor:)`.
    pub fn range_picker_header_background_color(
        mut self,
        range_picker_header_background_color: Color,
    ) -> DatePickerThemeData {
        self.range_picker_header_background_color = Some(range_picker_header_background_color);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerHeaderForegroundColor:)`.
    pub fn range_picker_header_foreground_color(
        mut self,
        range_picker_header_foreground_color: Color,
    ) -> DatePickerThemeData {
        self.range_picker_header_foreground_color = Some(range_picker_header_foreground_color);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerHeaderHeadlineStyle:)`.
    pub fn range_picker_header_headline_style(
        mut self,
        range_picker_header_headline_style: TextStyle,
    ) -> DatePickerThemeData {
        self.range_picker_header_headline_style = Some(range_picker_header_headline_style);
        self
    }

    /// Dart `DatePickerThemeData(rangePickerHeaderHelpStyle:)`.
    pub fn range_picker_header_help_style(
        mut self,
        range_picker_header_help_style: TextStyle,
    ) -> DatePickerThemeData {
        self.range_picker_header_help_style = Some(range_picker_header_help_style);
        self
    }

    /// Dart `DatePickerThemeData(rangeSelectionBackgroundColor:)`.
    pub fn range_selection_background_color(
        mut self,
        range_selection_background_color: Color,
    ) -> DatePickerThemeData {
        self.range_selection_background_color = Some(range_selection_background_color);
        self
    }

    /// Dart `DatePickerThemeData(rangeSelectionOverlayColor:)`.
    pub fn range_selection_overlay_color(
        mut self,
        range_selection_overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DatePickerThemeData {
        self.range_selection_overlay_color = Some(range_selection_overlay_color);
        self
    }

    /// Dart `DatePickerThemeData(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> DatePickerThemeData {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `DatePickerThemeData(inputDecorationTheme:)`.
    pub fn input_decoration_theme(
        mut self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> DatePickerThemeData {
        self.input_decoration_theme = Some(input_decoration_theme.into());
        self
    }

    /// Dart `DatePickerThemeData(cancelButtonStyle:)`.
    pub fn cancel_button_style(mut self, cancel_button_style: ButtonStyle) -> DatePickerThemeData {
        self.cancel_button_style = Some(cancel_button_style);
        self
    }

    /// Dart `DatePickerThemeData(confirmButtonStyle:)`.
    pub fn confirm_button_style(
        mut self,
        confirm_button_style: ButtonStyle,
    ) -> DatePickerThemeData {
        self.confirm_button_style = Some(confirm_button_style);
        self
    }

    /// Dart `DatePickerThemeData(locale:)`.
    pub fn locale(mut self, locale: Locale) -> DatePickerThemeData {
        self.locale = Some(locale);
        self
    }

    /// Dart `DatePickerThemeData(toggleButtonTextStyle:)`.
    pub fn toggle_button_text_style(
        mut self,
        toggle_button_text_style: TextStyle,
    ) -> DatePickerThemeData {
        self.toggle_button_text_style = Some(toggle_button_text_style);
        self
    }

    /// Dart `DatePickerThemeData(subHeaderForegroundColor:)`.
    pub fn sub_header_foreground_color(
        mut self,
        sub_header_foreground_color: Color,
    ) -> DatePickerThemeData {
        self.sub_header_foreground_color = Some(sub_header_foreground_color);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(4.0)`).
    pub fn copy_with(&self) -> DatePickerThemeData {
        self.clone()
    }

    /// Linearly interpolates between two date picker themes.
    pub fn lerp(
        a: Option<&DatePickerThemeData>,
        b: Option<&DatePickerThemeData>,
        t: f64,
    ) -> DatePickerThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        DatePickerThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            header_background_color: Color::lerp(
                a.and_then(|a| a.header_background_color),
                b.and_then(|b| b.header_background_color),
                t,
            ),
            header_foreground_color: Color::lerp(
                a.and_then(|a| a.header_foreground_color),
                b.and_then(|b| b.header_foreground_color),
                t,
            ),
            header_headline_style: TextStyle::lerp(
                a.and_then(|a| a.header_headline_style.as_ref()),
                b.and_then(|b| b.header_headline_style.as_ref()),
                t,
            ),
            header_help_style: TextStyle::lerp(
                a.and_then(|a| a.header_help_style.as_ref()),
                b.and_then(|b| b.header_help_style.as_ref()),
                t,
            ),
            weekday_style: TextStyle::lerp(
                a.and_then(|a| a.weekday_style.as_ref()),
                b.and_then(|b| b.weekday_style.as_ref()),
                t,
            ),
            day_style: TextStyle::lerp(
                a.and_then(|a| a.day_style.as_ref()),
                b.and_then(|b| b.day_style.as_ref()),
                t,
            ),
            day_foreground_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.day_foreground_color.clone()),
                b.and_then(|b| b.day_foreground_color.clone()),
                t,
                Color::lerp,
            ),
            day_background_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.day_background_color.clone()),
                b.and_then(|b| b.day_background_color.clone()),
                t,
                Color::lerp,
            ),
            day_overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.day_overlay_color.clone()),
                b.and_then(|b| b.day_overlay_color.clone()),
                t,
                Color::lerp,
            ),
            day_shape: <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::lerp_optional(
                a.and_then(|a| a.day_shape.clone()),
                b.and_then(|b| b.day_shape.clone()),
                t,
                |a, b, t| OutlinedBorderRef::lerp(a.as_ref(), b.as_ref(), t),
            ),
            today_foreground_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.today_foreground_color.clone()),
                b.and_then(|b| b.today_foreground_color.clone()),
                t,
                Color::lerp,
            ),
            today_background_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.today_background_color.clone()),
                b.and_then(|b| b.today_background_color.clone()),
                t,
                Color::lerp,
            ),
            today_border: Self::lerp_border_side(
                a.and_then(|a| a.today_border),
                b.and_then(|b| b.today_border),
                t,
            ),
            year_style: TextStyle::lerp(
                a.and_then(|a| a.year_style.as_ref()),
                b.and_then(|b| b.year_style.as_ref()),
                t,
            ),
            year_foreground_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.year_foreground_color.clone()),
                b.and_then(|b| b.year_foreground_color.clone()),
                t,
                Color::lerp,
            ),
            year_background_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.year_background_color.clone()),
                b.and_then(|b| b.year_background_color.clone()),
                t,
                Color::lerp,
            ),
            year_overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.year_overlay_color.clone()),
                b.and_then(|b| b.year_overlay_color.clone()),
                t,
                Color::lerp,
            ),
            year_shape: <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::lerp_optional(
                a.and_then(|a| a.year_shape.clone()),
                b.and_then(|b| b.year_shape.clone()),
                t,
                |a, b, t| OutlinedBorderRef::lerp(a.as_ref(), b.as_ref(), t),
            ),
            range_picker_background_color: Color::lerp(
                a.and_then(|a| a.range_picker_background_color),
                b.and_then(|b| b.range_picker_background_color),
                t,
            ),
            range_picker_elevation: lerp_double(
                a.and_then(|a| a.range_picker_elevation),
                b.and_then(|b| b.range_picker_elevation),
                t,
            ),
            range_picker_shadow_color: Color::lerp(
                a.and_then(|a| a.range_picker_shadow_color),
                b.and_then(|b| b.range_picker_shadow_color),
                t,
            ),
            range_picker_surface_tint_color: Color::lerp(
                a.and_then(|a| a.range_picker_surface_tint_color),
                b.and_then(|b| b.range_picker_surface_tint_color),
                t,
            ),
            range_picker_shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.range_picker_shape.as_ref()),
                b.and_then(|b| b.range_picker_shape.as_ref()),
                t,
            ),
            range_picker_header_background_color: Color::lerp(
                a.and_then(|a| a.range_picker_header_background_color),
                b.and_then(|b| b.range_picker_header_background_color),
                t,
            ),
            range_picker_header_foreground_color: Color::lerp(
                a.and_then(|a| a.range_picker_header_foreground_color),
                b.and_then(|b| b.range_picker_header_foreground_color),
                t,
            ),
            range_picker_header_headline_style: TextStyle::lerp(
                a.and_then(|a| a.range_picker_header_headline_style.as_ref()),
                b.and_then(|b| b.range_picker_header_headline_style.as_ref()),
                t,
            ),
            range_picker_header_help_style: TextStyle::lerp(
                a.and_then(|a| a.range_picker_header_help_style.as_ref()),
                b.and_then(|b| b.range_picker_header_help_style.as_ref()),
                t,
            ),
            range_selection_background_color: Color::lerp(
                a.and_then(|a| a.range_selection_background_color),
                b.and_then(|b| b.range_selection_background_color),
                t,
            ),
            range_selection_overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.range_selection_overlay_color.clone()),
                b.and_then(|b| b.range_selection_overlay_color.clone()),
                t,
                Color::lerp,
            ),
            divider_color: Color::lerp(
                a.and_then(|a| a.divider_color),
                b.and_then(|b| b.divider_color),
                t,
            ),
            input_decoration_theme: if t < 0.5 {
                a.and_then(|a| a.input_decoration_theme.clone())
            } else {
                b.and_then(|b| b.input_decoration_theme.clone())
            },
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
            locale: if t < 0.5 {
                a.and_then(|a| a.locale.clone())
            } else {
                b.and_then(|b| b.locale.clone())
            },
            toggle_button_text_style: TextStyle::lerp(
                a.and_then(|a| a.toggle_button_text_style.as_ref()),
                b.and_then(|b| b.toggle_button_text_style.as_ref()),
                t,
            ),
            sub_header_foreground_color: Color::lerp(
                a.and_then(|a| a.sub_header_foreground_color),
                b.and_then(|b| b.sub_header_foreground_color),
                t,
            ),
        }
    }

    fn lerp_border_side(
        a: Option<BorderSide>,
        b: Option<BorderSide>,
        t: f64,
    ) -> Option<BorderSide> {
        if a == b {
            return a;
        }
        match a {
            None => {
                let b = b.expect("b! in Dart");
                Some(BorderSide::lerp(
                    BorderSide {
                        width: 0.0,
                        color: b.color.with_alpha(0),
                        ..BorderSide::default()
                    },
                    b,
                    t,
                ))
            }
            Some(a) => Some(BorderSide::lerp(
                a,
                BorderSide {
                    width: 0.0,
                    color: a.color.with_alpha(0),
                    ..BorderSide::default()
                },
                t,
            )),
        }
    }
}

impl PartialEq for DatePickerThemeData {
    fn eq(&self, other: &DatePickerThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        true && other.background_color == self.background_color
            && other.elevation == self.elevation
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.shape == self.shape
            && other.header_background_color == self.header_background_color
            && other.header_foreground_color == self.header_foreground_color
            && other.header_headline_style == self.header_headline_style
            && other.header_help_style == self.header_help_style
            && other.weekday_style == self.weekday_style
            && other.day_style == self.day_style
            && other.day_foreground_color == self.day_foreground_color
            && other.day_background_color == self.day_background_color
            && other.day_overlay_color == self.day_overlay_color
            && other.day_shape == self.day_shape
            && other.today_foreground_color == self.today_foreground_color
            && other.today_background_color == self.today_background_color
            && other.today_border == self.today_border
            && other.year_style == self.year_style
            && other.year_foreground_color == self.year_foreground_color
            && other.year_background_color == self.year_background_color
            && other.year_overlay_color == self.year_overlay_color
            && other.year_shape == self.year_shape
            && other.range_picker_background_color == self.range_picker_background_color
            && other.range_picker_elevation == self.range_picker_elevation
            && other.range_picker_shadow_color == self.range_picker_shadow_color
            && other.range_picker_surface_tint_color == self.range_picker_surface_tint_color
            && other.range_picker_shape == self.range_picker_shape
            && other.range_picker_header_background_color
                == self.range_picker_header_background_color
            && other.range_picker_header_foreground_color
                == self.range_picker_header_foreground_color
            && other.range_picker_header_headline_style == self.range_picker_header_headline_style
            && other.range_picker_header_help_style == self.range_picker_header_help_style
            && other.range_selection_background_color == self.range_selection_background_color
            && other.range_selection_overlay_color == self.range_selection_overlay_color
            && other.divider_color == self.divider_color
            && other.input_decoration_theme == self.input_decoration_theme
            && other.cancel_button_style == self.cancel_button_style
            && other.confirm_button_style == self.confirm_button_style
            && other.locale == self.locale
            && other.toggle_button_text_style == self.toggle_button_text_style
            && other.sub_header_foreground_color == self.sub_header_foreground_color
    }
}

impl Debug for DatePickerThemeData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("DatePickerThemeData");
        if let Some(background_color) = &self.background_color {
            properties.field("backgroundColor", background_color);
        }
        if let Some(elevation) = &self.elevation {
            properties.field("elevation", elevation);
        }
        if let Some(shadow_color) = &self.shadow_color {
            properties.field("shadowColor", shadow_color);
        }
        if let Some(surface_tint_color) = &self.surface_tint_color {
            properties.field("surfaceTintColor", surface_tint_color);
        }
        if let Some(shape) = &self.shape {
            properties.field("shape", shape);
        }
        if let Some(header_background_color) = &self.header_background_color {
            properties.field("headerBackgroundColor", header_background_color);
        }
        if let Some(header_foreground_color) = &self.header_foreground_color {
            properties.field("headerForegroundColor", header_foreground_color);
        }
        if let Some(header_headline_style) = &self.header_headline_style {
            properties.field("headerHeadlineStyle", header_headline_style);
        }
        if let Some(header_help_style) = &self.header_help_style {
            properties.field("headerHelpStyle", header_help_style);
        }
        if let Some(weekday_style) = &self.weekday_style {
            properties.field("weekdayStyle", weekday_style);
        }
        if let Some(day_style) = &self.day_style {
            properties.field("dayStyle", day_style);
        }
        if let Some(day_foreground_color) = &self.day_foreground_color {
            properties.field("dayForegroundColor", day_foreground_color);
        }
        if let Some(day_background_color) = &self.day_background_color {
            properties.field("dayBackgroundColor", day_background_color);
        }
        if let Some(day_overlay_color) = &self.day_overlay_color {
            properties.field("dayOverlayColor", day_overlay_color);
        }
        if let Some(day_shape) = &self.day_shape {
            properties.field("dayShape", day_shape);
        }
        if let Some(today_foreground_color) = &self.today_foreground_color {
            properties.field("todayForegroundColor", today_foreground_color);
        }
        if let Some(today_background_color) = &self.today_background_color {
            properties.field("todayBackgroundColor", today_background_color);
        }
        if let Some(today_border) = &self.today_border {
            properties.field("todayBorder", today_border);
        }
        if let Some(year_style) = &self.year_style {
            properties.field("yearStyle", year_style);
        }
        if let Some(year_foreground_color) = &self.year_foreground_color {
            properties.field("yearForegroundColor", year_foreground_color);
        }
        if let Some(year_background_color) = &self.year_background_color {
            properties.field("yearBackgroundColor", year_background_color);
        }
        if let Some(year_overlay_color) = &self.year_overlay_color {
            properties.field("yearOverlayColor", year_overlay_color);
        }
        if let Some(year_shape) = &self.year_shape {
            properties.field("yearShape", year_shape);
        }
        if let Some(range_picker_background_color) = &self.range_picker_background_color {
            properties.field("rangePickerBackgroundColor", range_picker_background_color);
        }
        if let Some(range_picker_elevation) = &self.range_picker_elevation {
            properties.field("rangePickerElevation", range_picker_elevation);
        }
        if let Some(range_picker_shadow_color) = &self.range_picker_shadow_color {
            properties.field("rangePickerShadowColor", range_picker_shadow_color);
        }
        if let Some(range_picker_surface_tint_color) = &self.range_picker_surface_tint_color {
            properties.field(
                "rangePickerSurfaceTintColor",
                range_picker_surface_tint_color,
            );
        }
        if let Some(range_picker_shape) = &self.range_picker_shape {
            properties.field("rangePickerShape", range_picker_shape);
        }
        if let Some(range_picker_header_background_color) =
            &self.range_picker_header_background_color
        {
            properties.field(
                "rangePickerHeaderBackgroundColor",
                range_picker_header_background_color,
            );
        }
        if let Some(range_picker_header_foreground_color) =
            &self.range_picker_header_foreground_color
        {
            properties.field(
                "rangePickerHeaderForegroundColor",
                range_picker_header_foreground_color,
            );
        }
        if let Some(range_picker_header_headline_style) = &self.range_picker_header_headline_style {
            properties.field(
                "rangePickerHeaderHeadlineStyle",
                range_picker_header_headline_style,
            );
        }
        if let Some(range_picker_header_help_style) = &self.range_picker_header_help_style {
            properties.field("rangePickerHeaderHelpStyle", range_picker_header_help_style);
        }
        if let Some(range_selection_background_color) = &self.range_selection_background_color {
            properties.field(
                "rangeSelectionBackgroundColor",
                range_selection_background_color,
            );
        }
        if let Some(range_selection_overlay_color) = &self.range_selection_overlay_color {
            properties.field("rangeSelectionOverlayColor", range_selection_overlay_color);
        }
        if let Some(divider_color) = &self.divider_color {
            properties.field("dividerColor", divider_color);
        }
        if self.input_decoration_theme.is_some() {
            properties.field("inputDecorationTheme", &"InputDecorationThemeData");
        }
        if let Some(cancel_button_style) = &self.cancel_button_style {
            properties.field("cancelButtonStyle", cancel_button_style);
        }
        if let Some(confirm_button_style) = &self.confirm_button_style {
            properties.field("confirmButtonStyle", confirm_button_style);
        }
        if let Some(locale) = &self.locale {
            properties.field("locale", locale);
        }
        if let Some(toggle_button_text_style) = &self.toggle_button_text_style {
            properties.field("toggleButtonTextStyle", toggle_button_text_style);
        }
        if let Some(sub_header_foreground_color) = &self.sub_header_foreground_color {
            properties.field("subHeaderForegroundColor", sub_header_foreground_color);
        }
        properties.finish()
    }
}

// Hand coded defaults based on Material Design 2.

/// Dart's `_DatePickerDefaultsM2`.
struct DatePickerDefaultsM2 {
    /// Dart's `super(..)` arguments.
    date_picker_theme_data: DatePickerThemeData,
    context: DefaultsContext,
}

impl DatePickerDefaultsM2 {
    fn new(context: BuildContext) -> DatePickerDefaultsM2 {
        DatePickerDefaultsM2 {
            date_picker_theme_data: DatePickerThemeData::new()
                .elevation(24.0)
                .shape(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(4.0),
                ))
                .day_shape(Rc::new(WidgetStatePropertyAll::new(Some(
                    OutlinedBorderRef::new(CircleBorder::default()),
                ))))
                .year_shape(Rc::new(WidgetStatePropertyAll::new(Some(
                    OutlinedBorderRef::new(StadiumBorder::default()),
                ))))
                .range_picker_elevation(0.0)
                .range_picker_shape(RoundedRectangleBorder::default()),
            context: DefaultsContext::new(context),
        }
    }
}

impl DatePickerDefaultsM2 {
    /// Dart's `_isDark`.
    fn is_dark(&self, app: &mut App) -> bool {
        self.context.colors(app).brightness == Brightness::Dark
    }
}

#[allow(deprecated)]
impl DatePickerThemeDataGetters for DatePickerDefaultsM2 {
    fn date_picker_theme_data(&self) -> &DatePickerThemeData {
        &self.date_picker_theme_data
    }

    fn header_background_color(&self, app: &mut App) -> Option<Color> {
        let colors = self.context.colors(app);
        Some(if self.context.colors(app).brightness == Brightness::Dark {
            colors.surface
        } else {
            colors.primary
        })
    }

    fn sub_header_foreground_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_surface.with_opacity(0.60))
    }

    fn toggle_button_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let color = self.sub_header_foreground_color(app);
        self.context
            .text_theme(app)
            .title_small
            .as_ref()
            .map(|style| style.apply().color(color.expect("a color")).into_style())
    }

    fn cancel_button_style(&self, _app: &mut App) -> Option<ButtonStyle> {
        Some(TextButton::style_from().build())
    }

    fn confirm_button_style(&self, _app: &mut App) -> Option<ButtonStyle> {
        Some(TextButton::style_from().build())
    }

    fn header_foreground_color(&self, app: &mut App) -> Option<Color> {
        let is_dark = self.is_dark(app);
        let colors = self.context.colors(app);
        Some(if is_dark {
            colors.on_surface
        } else {
            colors.on_primary
        })
    }

    fn header_headline_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).headline_small.clone()
    }

    fn header_help_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).label_small.clone()
    }

    fn weekday_style(&self, app: &mut App) -> Option<TextStyle> {
        let color = self.context.colors(app).on_surface.with_opacity(0.60);
        self.context
            .text_theme(app)
            .body_small
            .as_ref()
            .map(|style| style.apply().color(color).into_style())
    }

    fn day_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_small.clone()
    }

    fn day_foreground_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, base) = (colors.on_primary, colors.on_surface);
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(on_primary)
                } else if states.contains(&WidgetState::Disabled) {
                    Some(base.with_opacity(0.38))
                } else {
                    Some(base)
                }
            },
        ))
    }

    fn day_background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let primary = self.context.colors(app).primary;
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(primary)
                } else {
                    None
                }
            },
        ))
    }

    fn day_overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, on_surface_variant) = (colors.on_primary, colors.on_surface_variant());
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_primary.with_opacity(0.38));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_primary.with_opacity(0.12));
                    }
                } else {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_surface_variant.with_opacity(0.12));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_surface_variant.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_surface_variant.with_opacity(0.12));
                    }
                }
                None
            },
        ))
    }

    fn today_foreground_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, on_surface, base) = (colors.on_primary, colors.on_surface, colors.primary);
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    Some(on_primary)
                } else if states.contains(&WidgetState::Disabled) {
                    Some(on_surface.with_opacity(0.38))
                } else {
                    Some(base)
                }
            },
        ))
    }

    fn today_background_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        self.day_background_color(app)
    }

    fn today_border(&self, app: &mut App) -> Option<BorderSide> {
        Some(BorderSide {
            color: self.context.colors(app).primary,
            ..BorderSide::default()
        })
    }

    fn year_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_large.clone()
    }

    fn range_picker_background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface)
    }

    fn range_picker_shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn range_picker_surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn range_picker_header_background_color(&self, app: &mut App) -> Option<Color> {
        let is_dark = self.is_dark(app);
        let colors = self.context.colors(app);
        Some(if is_dark {
            colors.surface
        } else {
            colors.primary
        })
    }

    fn range_picker_header_foreground_color(&self, app: &mut App) -> Option<Color> {
        let is_dark = self.is_dark(app);
        let colors = self.context.colors(app);
        Some(if is_dark {
            colors.on_surface
        } else {
            colors.on_primary
        })
    }

    fn range_picker_header_headline_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).headline_small.clone()
    }

    fn range_picker_header_help_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).label_small.clone()
    }

    fn range_selection_background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary.with_opacity(0.12))
    }

    fn range_selection_overlay_color(
        &self,
        app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app);
        let (on_primary, on_surface_variant) = (colors.on_primary, colors.on_surface_variant());
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_primary.with_opacity(0.38));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_primary.with_opacity(0.12));
                    }
                } else {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_surface_variant.with_opacity(0.12));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_surface_variant.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_surface_variant.with_opacity(0.12));
                    }
                }
                None
            },
        ))
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_embedder::TargetPlatform;
    use waft_rendering::RenderBox as _;
    use waft_widgets::{Builder, Container};

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::test_support::{PaintInspection, material_app};
    use crate::text_theme::TextTheme;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3};

    fn states(states: impl IntoIterator<Item = WidgetState>) -> WidgetStates {
        states.into_iter().collect()
    }

    fn resolve_color(
        property: &Option<WidgetStatePropertyRef<Option<Color>>>,
        states: &WidgetStates,
    ) -> Option<Option<Color>> {
        property.as_ref().map(|property| property.resolve(states))
    }

    /// What the tests of `DatePickerTheme.defaults` read: the defaults with every getter
    /// answered, and the theme they came from.
    struct Captured {
        defaults: DatePickerThemeData,
        color_scheme: ColorScheme,
        text_theme: TextTheme,
    }

    fn capture_defaults(tester: &mut WidgetTester, theme: ThemeData) -> Captured {
        let captured: Rc<RefCell<Option<Captured>>> = Rc::default();
        let root = {
            let captured = Rc::clone(&captured);
            material_app(
                theme,
                Builder::new(move |app, context| {
                    let defaults = DatePickerTheme::defaults(app, context).copy_with(app);
                    let theme = Theme::of(app, context);
                    *captured.borrow_mut() = Some(Captured {
                        defaults,
                        color_scheme: theme.color_scheme.clone(),
                        text_theme: theme.text_theme.clone(),
                    });
                    Container::new().into_widget()
                })
                .into_widget(),
            )
        };
        tester.pump_widget(root);
        captured.take().expect("the builder ran")
    }

    #[test]
    fn date_picker_theme_data_copy_with_equals_and_hash_code_basics() {
        assert_eq!(
            DatePickerThemeData::new(),
            DatePickerThemeData::new().copy_with()
        );
    }

    #[test]
    fn date_picker_theme_data_lerp_special_cases() {
        let data = DatePickerThemeData::new();
        assert_eq!(
            DatePickerThemeData::lerp(Some(&data), Some(&data), 0.5),
            data
        );
    }

    #[test]
    fn date_picker_theme_data_defaults() {
        let theme = DatePickerThemeData::new();
        assert_eq!(theme.background_color, None);
        assert_eq!(theme.elevation, None);
        assert_eq!(theme.shadow_color, None);
        assert_eq!(theme.surface_tint_color, None);
        assert!(theme.shape.is_none());
        assert_eq!(theme.header_background_color, None);
        assert_eq!(theme.header_foreground_color, None);
        assert_eq!(theme.header_headline_style, None);
        assert_eq!(theme.header_help_style, None);
        assert_eq!(theme.weekday_style, None);
        assert_eq!(theme.day_style, None);
        assert!(theme.day_foreground_color.is_none());
        assert!(theme.day_background_color.is_none());
        assert!(theme.day_overlay_color.is_none());
        assert!(theme.day_shape.is_none());
        assert!(theme.today_foreground_color.is_none());
        assert!(theme.today_background_color.is_none());
        assert_eq!(theme.today_border, None);
        assert_eq!(theme.year_style, None);
        assert!(theme.year_foreground_color.is_none());
        assert!(theme.year_background_color.is_none());
        assert!(theme.year_overlay_color.is_none());
        assert!(theme.year_shape.is_none());
        assert_eq!(theme.range_picker_background_color, None);
        assert_eq!(theme.range_picker_elevation, None);
        assert_eq!(theme.range_picker_shadow_color, None);
        assert_eq!(theme.range_picker_surface_tint_color, None);
        assert!(theme.range_picker_shape.is_none());
        assert_eq!(theme.range_picker_header_background_color, None);
        assert_eq!(theme.range_picker_header_foreground_color, None);
        assert_eq!(theme.range_picker_header_headline_style, None);
        assert_eq!(theme.range_picker_header_help_style, None);
        assert_eq!(theme.range_selection_background_color, None);
        assert!(theme.range_selection_overlay_color.is_none());
        assert_eq!(theme.divider_color, None);
        assert!(theme.input_decoration_theme.is_none());
        assert!(theme.cancel_button_style.is_none());
        assert!(theme.confirm_button_style.is_none());
        assert_eq!(theme.locale, None);
        assert_eq!(theme.sub_header_foreground_color, None);
        assert_eq!(theme.toggle_button_text_style, None);
    }

    #[test]
    fn date_picker_theme_defaults_m3_defaults() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let Captured {
                defaults: m3,
                color_scheme,
                text_theme,
            } = capture_defaults(tester, theme);
            let none = WidgetStates::new();
            let selected = states([WidgetState::Selected]);

            assert_eq!(
                m3.background_color,
                Some(color_scheme.surface_container_high())
            );
            assert_eq!(m3.elevation, Some(6.0));
            assert_eq!(m3.shadow_color, Some(Color::new(0x00000000)));
            assert_eq!(m3.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(
                m3.shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(28.0),
                )))
            );
            assert_eq!(m3.header_background_color, Some(Color::new(0x00000000)));
            assert_eq!(
                m3.header_foreground_color,
                Some(color_scheme.on_surface_variant())
            );
            assert_eq!(m3.header_headline_style, text_theme.headline_large);
            assert_eq!(m3.header_help_style, text_theme.label_large);
            assert_eq!(
                m3.weekday_style,
                text_theme
                    .body_large
                    .as_ref()
                    .map(|style| style.apply().color(color_scheme.on_surface).into_style())
            );
            assert_eq!(m3.day_style, text_theme.body_large);
            assert_eq!(
                resolve_color(&m3.day_foreground_color, &none),
                Some(Some(color_scheme.on_surface))
            );
            assert_eq!(
                resolve_color(&m3.day_foreground_color, &selected),
                Some(Some(color_scheme.on_primary))
            );
            assert_eq!(
                resolve_color(&m3.day_foreground_color, &states([WidgetState::Disabled])),
                Some(Some(color_scheme.on_surface.with_opacity(0.38)))
            );
            assert_eq!(resolve_color(&m3.day_background_color, &none), Some(None));
            assert_eq!(
                resolve_color(&m3.day_background_color, &selected),
                Some(Some(color_scheme.primary))
            );
            assert_eq!(resolve_color(&m3.day_overlay_color, &none), Some(None));
            let overlay = |states: WidgetStates| resolve_color(&m3.day_overlay_color, &states);
            assert_eq!(
                overlay(states([WidgetState::Selected, WidgetState::Hovered])),
                Some(Some(color_scheme.on_primary.with_opacity(0.08)))
            );
            assert_eq!(
                overlay(states([WidgetState::Selected, WidgetState::Focused])),
                Some(Some(color_scheme.on_primary.with_opacity(0.1)))
            );
            assert_eq!(
                overlay(states([WidgetState::Hovered])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.08)))
            );
            assert_eq!(
                overlay(states([WidgetState::Focused])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.1)))
            );
            assert_eq!(
                overlay(states([WidgetState::Pressed])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.1)))
            );
            assert_eq!(
                overlay(states([
                    WidgetState::Selected,
                    WidgetState::Hovered,
                    WidgetState::Focused
                ])),
                Some(Some(color_scheme.on_primary.with_opacity(0.08)))
            );
            assert_eq!(
                overlay(states([
                    WidgetState::Selected,
                    WidgetState::Hovered,
                    WidgetState::Pressed
                ])),
                Some(Some(color_scheme.on_primary.with_opacity(0.1)))
            );
            assert_eq!(
                overlay(states([WidgetState::Hovered, WidgetState::Focused])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.08)))
            );
            assert_eq!(
                overlay(states([WidgetState::Hovered, WidgetState::Pressed])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.1)))
            );
            assert_eq!(
                m3.day_shape.as_ref().map(|shape| shape.resolve(&none)),
                Some(Some(OutlinedBorderRef::new(CircleBorder::default())))
            );
            assert_eq!(
                resolve_color(&m3.today_foreground_color, &none),
                Some(Some(color_scheme.primary))
            );
            assert_eq!(
                resolve_color(&m3.today_foreground_color, &states([WidgetState::Disabled])),
                Some(Some(color_scheme.primary.with_opacity(0.38)))
            );
            assert_eq!(
                m3.today_border,
                Some(BorderSide {
                    color: color_scheme.primary,
                    ..BorderSide::default()
                })
            );
            assert_eq!(m3.year_style, text_theme.body_large);
            assert_eq!(
                resolve_color(&m3.year_foreground_color, &none),
                Some(Some(color_scheme.on_surface_variant()))
            );
            assert_eq!(
                resolve_color(&m3.year_foreground_color, &selected),
                Some(Some(color_scheme.on_primary))
            );
            assert_eq!(
                resolve_color(&m3.year_foreground_color, &states([WidgetState::Disabled])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.38)))
            );
            assert_eq!(resolve_color(&m3.year_background_color, &none), Some(None));
            assert_eq!(
                resolve_color(&m3.year_background_color, &selected),
                Some(Some(color_scheme.primary))
            );
            assert_eq!(resolve_color(&m3.year_overlay_color, &none), Some(None));
            let year_overlay =
                |states: WidgetStates| resolve_color(&m3.year_overlay_color, &states);
            assert_eq!(
                year_overlay(states([WidgetState::Selected, WidgetState::Hovered])),
                Some(Some(color_scheme.on_primary.with_opacity(0.08)))
            );
            assert_eq!(
                year_overlay(states([WidgetState::Selected, WidgetState::Focused])),
                Some(Some(color_scheme.on_primary.with_opacity(0.1)))
            );
            assert_eq!(
                year_overlay(states([WidgetState::Hovered])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.08)))
            );
            assert_eq!(
                year_overlay(states([WidgetState::Focused])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.1)))
            );
            assert_eq!(
                year_overlay(states([WidgetState::Pressed])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.1)))
            );
            assert_eq!(m3.range_picker_elevation, Some(0.0));
            assert_eq!(
                m3.range_picker_shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::default()))
            );
            assert_eq!(m3.range_picker_shadow_color, Some(Colors::TRANSPARENT));
            assert_eq!(
                m3.range_picker_surface_tint_color,
                Some(Colors::TRANSPARENT)
            );
            assert_eq!(
                resolve_color(&m3.range_selection_overlay_color, &none),
                Some(None)
            );
            assert_eq!(
                m3.range_picker_header_background_color,
                Some(Colors::TRANSPARENT)
            );
            assert_eq!(
                m3.range_picker_header_foreground_color,
                Some(color_scheme.on_surface_variant())
            );
            assert_eq!(
                m3.range_picker_header_headline_style,
                text_theme.title_large
            );
            assert_eq!(m3.range_picker_header_help_style, text_theme.title_small);
            assert_eq!(m3.divider_color, None);
            assert!(m3.input_decoration_theme.is_none());
            let text_button = format!("{:?}", TextButton::style_from().build());
            assert_eq!(
                format!("{:?}", m3.cancel_button_style.expect("a style")),
                text_button
            );
            assert_eq!(
                format!("{:?}", m3.confirm_button_style.expect("a style")),
                text_button
            );
            assert_eq!(m3.locale, None);
            assert_eq!(
                m3.sub_header_foreground_color,
                Some(color_scheme.on_surface.with_opacity(0.60))
            );
            assert_eq!(
                m3.toggle_button_text_style,
                text_theme.title_small.as_ref().map(|style| style
                    .apply()
                    .color(color_scheme.on_surface.with_opacity(0.60))
                    .into_style())
            );
        });
    }

    #[test]
    fn date_picker_theme_defaults_m2_defaults() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());
            let Captured {
                defaults: m2,
                color_scheme,
                text_theme,
            } = capture_defaults(tester, theme);
            let none = WidgetStates::new();
            let selected = states([WidgetState::Selected]);

            assert_eq!(m2.elevation, Some(24.0));
            assert_eq!(
                m2.shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(4.0),
                )))
            );
            assert_eq!(m2.header_background_color, Some(color_scheme.primary));
            assert_eq!(m2.header_foreground_color, Some(color_scheme.on_primary));
            assert_eq!(m2.header_headline_style, text_theme.headline_small);
            assert_eq!(m2.header_help_style, text_theme.label_small);
            assert_eq!(
                m2.weekday_style,
                text_theme.body_small.as_ref().map(|style| style
                    .apply()
                    .color(color_scheme.on_surface.with_opacity(0.60))
                    .into_style())
            );
            assert_eq!(m2.day_style, text_theme.body_small);
            assert_eq!(
                resolve_color(&m2.day_foreground_color, &none),
                Some(Some(color_scheme.on_surface))
            );
            assert_eq!(
                resolve_color(&m2.day_foreground_color, &selected),
                Some(Some(color_scheme.on_primary))
            );
            assert_eq!(
                resolve_color(&m2.day_foreground_color, &states([WidgetState::Disabled])),
                Some(Some(color_scheme.on_surface.with_opacity(0.38)))
            );
            assert_eq!(resolve_color(&m2.day_background_color, &none), Some(None));
            assert_eq!(
                resolve_color(&m2.day_background_color, &selected),
                Some(Some(color_scheme.primary))
            );
            assert_eq!(resolve_color(&m2.day_overlay_color, &none), Some(None));
            let overlay = |states: WidgetStates| resolve_color(&m2.day_overlay_color, &states);
            assert_eq!(
                overlay(states([WidgetState::Selected, WidgetState::Hovered])),
                Some(Some(color_scheme.on_primary.with_opacity(0.08)))
            );
            assert_eq!(
                overlay(states([WidgetState::Selected, WidgetState::Focused])),
                Some(Some(color_scheme.on_primary.with_opacity(0.12)))
            );
            assert_eq!(
                overlay(states([WidgetState::Selected, WidgetState::Pressed])),
                Some(Some(color_scheme.on_primary.with_opacity(0.38)))
            );
            assert_eq!(
                overlay(states([
                    WidgetState::Selected,
                    WidgetState::Hovered,
                    WidgetState::Focused
                ])),
                Some(Some(color_scheme.on_primary.with_opacity(0.08)))
            );
            assert_eq!(
                overlay(states([
                    WidgetState::Selected,
                    WidgetState::Hovered,
                    WidgetState::Pressed
                ])),
                Some(Some(color_scheme.on_primary.with_opacity(0.38)))
            );
            assert_eq!(
                overlay(states([WidgetState::Hovered])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.08)))
            );
            assert_eq!(
                overlay(states([WidgetState::Focused])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.12)))
            );
            assert_eq!(
                overlay(states([WidgetState::Pressed])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.12)))
            );
            assert_eq!(
                m2.day_shape.as_ref().map(|shape| shape.resolve(&none)),
                Some(Some(OutlinedBorderRef::new(CircleBorder::default())))
            );
            assert_eq!(
                resolve_color(&m2.today_foreground_color, &none),
                Some(Some(color_scheme.primary))
            );
            assert_eq!(
                resolve_color(&m2.today_foreground_color, &states([WidgetState::Disabled])),
                Some(Some(color_scheme.on_surface.with_opacity(0.38)))
            );
            assert_eq!(
                m2.today_border,
                Some(BorderSide {
                    color: color_scheme.primary,
                    ..BorderSide::default()
                })
            );
            assert_eq!(m2.year_style, text_theme.body_large);
            assert_eq!(m2.range_picker_background_color, Some(color_scheme.surface));
            assert_eq!(m2.range_picker_elevation, Some(0.0));
            assert_eq!(
                m2.range_picker_shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::default()))
            );
            assert_eq!(m2.range_picker_shadow_color, Some(Colors::TRANSPARENT));
            assert_eq!(
                m2.range_picker_surface_tint_color,
                Some(Colors::TRANSPARENT)
            );
            assert_eq!(
                resolve_color(&m2.range_selection_overlay_color, &none),
                Some(None)
            );
            let range_overlay =
                |states: WidgetStates| resolve_color(&m2.range_selection_overlay_color, &states);
            assert_eq!(
                range_overlay(states([WidgetState::Selected, WidgetState::Hovered])),
                Some(Some(color_scheme.on_primary.with_opacity(0.08)))
            );
            assert_eq!(
                range_overlay(states([WidgetState::Selected, WidgetState::Focused])),
                Some(Some(color_scheme.on_primary.with_opacity(0.12)))
            );
            assert_eq!(
                range_overlay(states([WidgetState::Selected, WidgetState::Pressed])),
                Some(Some(color_scheme.on_primary.with_opacity(0.38)))
            );
            assert_eq!(
                range_overlay(states([WidgetState::Hovered])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.08)))
            );
            assert_eq!(
                range_overlay(states([WidgetState::Focused])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.12)))
            );
            assert_eq!(
                range_overlay(states([WidgetState::Pressed])),
                Some(Some(color_scheme.on_surface_variant().with_opacity(0.12)))
            );
            assert_eq!(
                m2.range_picker_header_background_color,
                Some(color_scheme.primary)
            );
            assert_eq!(
                m2.range_picker_header_foreground_color,
                Some(color_scheme.on_primary)
            );
            assert_eq!(
                m2.range_picker_header_headline_style,
                text_theme.headline_small
            );
            assert_eq!(m2.range_picker_header_help_style, text_theme.label_small);
            assert_eq!(m2.divider_color, None);
            assert!(m2.input_decoration_theme.is_none());
            let text_button = format!("{:?}", TextButton::style_from().build());
            assert_eq!(
                format!("{:?}", m2.cancel_button_style.expect("a style")),
                text_button
            );
            assert_eq!(
                format!("{:?}", m2.confirm_button_style.expect("a style")),
                text_button
            );
            assert_eq!(m2.locale, None);
            assert_eq!(
                m2.year_shape.as_ref().map(|shape| shape.resolve(&none)),
                Some(Some(OutlinedBorderRef::new(StadiumBorder::default())))
            );
            assert_eq!(
                m2.sub_header_foreground_color,
                Some(color_scheme.on_surface.with_opacity(0.60))
            );
            assert_eq!(
                m2.toggle_button_text_style,
                text_theme.title_small.as_ref().map(|style| style
                    .apply()
                    .color(color_scheme.on_surface.with_opacity(0.60))
                    .into_style())
            );
        });
    }

    use waft_foundation::DateTime;
    use waft_painting::{BorderSide, CircleBorder};
    use waft_widgets::{MediaQuery, SizedBox};

    use crate::calendar_date_picker::YearPicker;
    use crate::chip::tests::size_of;
    use crate::scaffold::Scaffold;

    fn year_container(tester: &WidgetTester, year: &str) -> waft_widgets::AnyElement {
        let text = *tester
            .find_text(year)
            .first()
            .unwrap_or_else(|| panic!("a Text showing {year:?}"));
        tester.ancestor_of_type::<Container>(text)
    }

    /// Dart's `findTextDecoration(tester, date)`.
    fn year_decoration(tester: &WidgetTester, year: &str) -> waft_painting::ShapeDecoration {
        let container = year_container(tester, year);
        tester.widget(container, |container: &Container| {
            container
                .decoration
                .as_deref()
                .and_then(|decoration| {
                    decoration
                        .as_any()
                        .downcast_ref::<waft_painting::ShapeDecoration>()
                })
                .expect("a ShapeDecoration")
                .clone()
        })
    }

    fn year_picker_with_scale(text_scale_factor: f64) -> WidgetRef {
        MediaQuery::with_clamped_text_scaling(
            None,
            text_scale_factor,
            text_scale_factor,
            Scaffold::new().body(
                YearPicker::new(
                    DateTime::utc(2021, 1, 1),
                    DateTime::utc(2030, 1, 1),
                    Some(DateTime::utc(2025, 1, 1)),
                    Rc::new(|_app, _date| {}),
                )
                .current_date(DateTime::utc(2025, 1, 1)),
            ),
        )
    }

    #[test]
    fn year_picker_maintains_default_year_shape_at_text_scale_factor_1_1_5_2() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            // Dart reads the year's decoration paint, an rrect of
            // `RRect.fromLTRBR(0.5, 0.5, w - 0.5, h - 0.5, Radius.circular((h - 1) / 2))` in
            // the color 0xFF6750A4: the container's size is what the rrect's bounds give, and
            // the decoration is the stadium of the selected year in the primary color.
            for (text_scale_factor, width, height) in
                [(1.0, 72.0, 36.0), (1.5, 108.0, 52.0), (2.0, 144.0, 52.0)]
            {
                tester.pump_widget(material_app(
                    theme.clone(),
                    year_picker_with_scale(text_scale_factor),
                ));
                let size = size_of(tester, year_container(tester, "2025"));
                assert_eq!((size.width(), size.height()), (width, height));
                let decoration = year_decoration(tester, "2025");
                assert_eq!(
                    decoration.color.map(|color| color.color()),
                    Some(Color::new(0xFF6750A4))
                );
            }
        });
    }

    #[test]
    fn year_picker_applies_shape_from_date_picker_theme_data_year_shape_correctly() {
        test_widgets(|tester| {
            let year_shape = CircleBorder::default();
            let today_border = BorderSide {
                width: 3.0,
                color: Color::new(0x00000000),
                ..BorderSide::default()
            };
            let today_foreground_color = Color::new(0xfffffff8);
            let theme = {
                let app = tester.cell.borrow();
                ThemeData::builder()
                    .date_picker_theme(
                        DatePickerThemeData::new()
                            .today_border(today_border)
                            .today_foreground_color(<dyn WidgetStateProperty<Option<Color>>>::all(
                                Some(today_foreground_color),
                            ))
                            .year_shape(<dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::all(
                                Some(OutlinedBorderRef::new(year_shape)),
                            )),
                    )
                    .build(&app)
            };
            tester.pump_widget(material_app(
                theme,
                waft_material_material_center(
                    YearPicker::new(
                        DateTime::utc(2021, 1, 1),
                        DateTime::utc(2030, 1, 1),
                        Some(DateTime::utc(2025, 1, 1)),
                        Rc::new(|_app, _date| {}),
                    )
                    .current_date(DateTime::utc(2025, 1, 1)),
                ),
            ));

            let circle = |tester: &WidgetTester, year: &str| -> BorderSide {
                let decoration = year_decoration(tester, year);
                let shape = decoration.shape.clone();
                shape
                    .as_any()
                    .downcast_ref::<CircleBorder>()
                    .expect("a CircleBorder")
                    .side
            };
            let side_2022 = circle(tester, "2022");
            assert_eq!(side_2022.width, 0.0);
            assert_eq!(side_2022.color, year_shape.side.color);

            let side_2025 = circle(tester, "2025");
            assert_eq!(side_2025.width, today_border.width);
            assert_eq!(side_2025.color, today_foreground_color);
            let _ = SizedBox::shrink();
        });
    }

    fn waft_material_material_center(child: YearPicker) -> WidgetRef {
        crate::material::Material::new()
            .child(waft_widgets::Directionality::new(
                waft_painting::TextDirection::Ltr,
                waft_widgets::Center::new().child(child),
            ))
            .into_widget()
    }

    // ---- the dialogs under the theme (date_picker_theme_test.dart) ----

    use waft_embedder::Size;
    use waft_painting::{RoundedRectangleBorder, TextDirection};
    use waft_widgets::{AnyElement, Center, Directionality, Text};

    use crate::button_style::ButtonStyle;
    use crate::chip::tests::center_of;
    use crate::date::DateTimeRange;
    use crate::date_picker::{DatePickerDialog, DateRangePickerDialog};
    use crate::dialog::Dialog;
    use crate::divider::{Divider, VerticalDivider};
    use crate::icons::Icons;
    use crate::ink_decoration::Ink;
    use crate::input_border::UnderlineInputBorder;
    use crate::material::Material;
    use crate::test_support::{PaintCall, paints_calls};

    /// What Dart's `paints` reads from the first `_RenderInkFeatures`: it paints its subtree.
    fn ink_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
        let ink_features = tester.first::<crate::material::RenderInkFeatures>();
        tester.painted_ops_deep(ink_features.as_object())
    }

    /// Dart's `paints` on a `Material`'s render object, which does not record the fill that the
    /// `Material` itself paints as a physical model (the test canvas paints a pushed layer's
    /// children inline and skips the layer's own shape): the calls are matched from any draw on,
    /// so the fills drawn before them do not stand in the way.
    fn paints_after_material_fills(ops: &[waft_embedder::valo::Op], calls: &[PaintCall]) -> bool {
        (0..ops.len()).any(|start| paints_calls(&ops[start..], calls))
    }

    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let at = center_of(tester, text_of(tester, text));
        tester.tap(at, 1);
    }

    fn date_picker_theme() -> DatePickerThemeData {
        let all_color =
            |color: u32| <dyn WidgetStateProperty<Option<Color>>>::all(Some(Color::new(color)));
        let all_shape = || {
            <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::all(Some(OutlinedBorderRef::new(
                RoundedRectangleBorder::default(),
            )))
        };
        let foreground_style = |color: u32| ButtonStyle {
            foreground_color: Some(all_color(color)),
            ..ButtonStyle::default()
        };
        DatePickerThemeData::new()
            .background_color(Color::new(0xfffffff0))
            .elevation(6.0)
            .shadow_color(Color::new(0xfffffff1))
            .surface_tint_color(Color::new(0xfffffff2))
            .shape(ShapeBorderRef::new(RoundedRectangleBorder::default()))
            .header_background_color(Color::new(0xfffffff3))
            .header_foreground_color(Color::new(0xfffffff4))
            .header_headline_style(TextStyle::new().font_size(10.0))
            .header_help_style(TextStyle::new().font_size(11.0))
            .weekday_style(TextStyle::new().font_size(12.0))
            .day_style(TextStyle::new().font_size(13.0))
            .day_foreground_color(all_color(0xfffffff5))
            .day_background_color(all_color(0xfffffff6))
            .day_overlay_color(all_color(0xfffffff7))
            .day_shape(all_shape())
            .today_foreground_color(all_color(0xfffffff8))
            .today_background_color(all_color(0xfffffff9))
            .today_border(BorderSide {
                width: 3.0,
                color: Color::new(0x00000000),
                ..BorderSide::default()
            })
            .year_style(TextStyle::new().font_size(13.0))
            .year_foreground_color(all_color(0xfffffffa))
            .year_background_color(all_color(0xfffffffb))
            .year_overlay_color(all_color(0xfffffffc))
            .year_shape(all_shape())
            .range_picker_background_color(Color::new(0xfffffffd))
            .range_picker_elevation(7.0)
            .range_picker_shadow_color(Color::new(0xfffffffe))
            .range_picker_surface_tint_color(Color::new(0xffffffff))
            .range_picker_shape(ShapeBorderRef::new(RoundedRectangleBorder::default()))
            .range_picker_header_background_color(Color::new(0xffffff0f))
            .range_picker_header_foreground_color(Color::new(0xffffff1f))
            .range_picker_header_headline_style(TextStyle::new().font_size(14.0))
            .range_picker_header_help_style(TextStyle::new().font_size(15.0))
            .range_selection_background_color(Color::new(0xffffff2f))
            .range_selection_overlay_color(all_color(0xffffff3f))
            .divider_color(Color::new(0xffffff4f))
            .input_decoration_theme(
                crate::input_decorator::InputDecorationThemeData::new()
                    .fill_color(Color::new(0xffffff5f))
                    .border(UnderlineInputBorder::default()),
            )
            .cancel_button_style(foreground_style(0xffffff6f))
            .confirm_button_style(foreground_style(0xffffff7f))
            .locale(waft_embedder::Locale::new("en"))
            .sub_header_foreground_color(Color::new(0xffffff8f))
            .toggle_button_text_style(TextStyle::new().font_size(13.0))
    }

    const WIDE_WINDOW_SIZE: Size = Size::new(1920.0, 1080.0);
    const NARROW_WINDOW_SIZE: Size = Size::new(1070.0, 1770.0);

    /// A tester whose view is `size` physical pixels at a device pixel ratio of 1: Dart's
    /// `tester.view.physicalSize = size; tester.view.devicePixelRatio = 1.0`.
    fn tester_of_size(size: Size) -> WidgetTester {
        WidgetTester::with_view(
            waft_test::TestPlatform::new().on(TargetPlatform::Android),
            waft_test::TestView::with_pixel_ratio(size.width(), size.height(), 1.0),
        )
    }

    /// Dart's `gesture.removePointer()`: `TestGesture` dispatches a `PointerRemovedEvent` that
    /// carries the pointer's id, so a recognizer tracking the pointer sees it (and gives up a
    /// press that is still down); the engine's converter would leave the id at zero.
    fn remove_pointer(tester: &mut WidgetTester, pointer: &TestPointer) {
        tester.update(|app| {
            waft_gestures::GestureBinding::instance(app).handle_pointer_event(
                app,
                waft_gestures::PointerEvent::Removed(waft_gestures::PointerRemovedEvent {
                    kind: waft_embedder::PointerDeviceKind::Mouse,
                    device: pointer.id,
                    pointer: pointer.id,
                    position: pointer.position,
                    ..waft_gestures::PointerRemovedEvent::default()
                }),
            );
            app.drain_microtasks();
        });
        tester.cell.checkpoint();
    }

    fn themed(theme: DatePickerThemeData, use_material3: bool, tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .date_picker_theme(theme)
            .use_material3(use_material3)
            .build(&tester.cell.borrow())
    }

    /// The `MaterialApp` home Dart's tests give: a `Directionality`, `Material` and `Center`
    /// around the dialog.
    fn centered_home<K>(dialog: impl IntoWidget<K>) -> WidgetRef {
        Material::new()
            .child(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(dialog),
            ))
            .into_widget()
    }

    fn calendar_dialog() -> DatePickerDialog {
        DatePickerDialog::new(DateTime::utc(2022, 1, 1), DateTime::utc(2024, 12, 31))
            .initial_date(DateTime::utc(2023, 1, 25))
            .current_date(DateTime::utc(2023, 1, 24))
    }

    fn range_dialog() -> DateRangePickerDialog {
        DateRangePickerDialog::new(DateTime::utc(2023, 1, 1), DateTime::utc(2023, 1, 31))
            .initial_date_range(DateTimeRange::new(
                DateTime::utc(2023, 1, 17),
                DateTime::utc(2023, 1, 20),
            ))
            .with_current_date(DateTime::utc(2023, 1, 23))
    }

    fn text_of(tester: &WidgetTester, text: &str) -> AnyElement {
        *tester
            .find_text(text)
            .first()
            .unwrap_or_else(|| panic!("a Text showing {text:?}"))
    }

    fn text_style(tester: &WidgetTester, text: &str) -> TextStyle {
        tester
            .widget(text_of(tester, text), |text: &Text| text.style.clone())
            .unwrap_or_else(|| panic!("the Text {text:?} has a style"))
    }

    fn style_color(style: &TextStyle) -> Option<Color> {
        style.color.as_ref().map(|color| color.color())
    }

    /// Dart's `findDialogMaterial`.
    fn find_dialog_material(tester: &WidgetTester) -> AnyElement {
        let dialog = tester.find_by_type::<Dialog>()[0];
        tester.descendants_of_type::<Material>(dialog)[0]
    }

    struct MaterialRead {
        color: Option<Color>,
        elevation: f64,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        shape: Option<ShapeBorderRef>,
    }

    fn read_material(tester: &WidgetTester, material: AnyElement) -> MaterialRead {
        tester.widget(material, |material: &Material| MaterialRead {
            color: material.color,
            elevation: material.elevation,
            shadow_color: material.shadow_color,
            surface_tint_color: material.surface_tint_color,
            shape: material.shape.clone(),
        })
    }

    /// Dart's `findHeaderMaterial`: the nearest `Material` above the text.
    fn find_header_material(tester: &WidgetTester, text: &str) -> AnyElement {
        tester.ancestor_of_type::<Material>(text_of(tester, text))
    }

    fn shape_decoration_of(
        decoration: Option<&dyn waft_painting::Decoration>,
    ) -> waft_painting::ShapeDecoration {
        decoration
            .and_then(|decoration| {
                decoration
                    .as_any()
                    .downcast_ref::<waft_painting::ShapeDecoration>()
            })
            .expect("a ShapeDecoration")
            .clone()
    }

    /// Dart's `findDayDecoration`.
    fn find_day_decoration(tester: &WidgetTester, day: &str) -> waft_painting::ShapeDecoration {
        let ink = tester.ancestor_of_type::<Ink>(text_of(tester, day));
        tester.widget(ink, |ink: &Ink| {
            shape_decoration_of(ink.decoration.as_deref())
        })
    }

    fn resolved(property: &Option<WidgetStatePropertyRef<Option<Color>>>) -> Option<Color> {
        property
            .as_ref()
            .and_then(|property| property.resolve(&WidgetStates::new()))
    }

    fn action_button_style(tester: &WidgetTester, text: &str) -> Option<ButtonStyle> {
        let button =
            tester.ancestor_of_type::<crate::text_button::TextButton>(text_of(tester, text));
        tester.widget(button, |button: &crate::text_button::TextButton| {
            crate::button_style_button::ButtonStyleButton::button_style_button_data(button)
                .style
                .clone()
        })
    }

    #[test]
    fn date_picker_dialog_uses_theme_data_date_picker_theme_calendar_mode() {
        test_widgets(|tester| {
            let theme_data = date_picker_theme();
            let theme = themed(theme_data.clone(), true, tester);
            tester.pump_widget(material_app(theme, centered_home(calendar_dialog())));

            let material = read_material(tester, find_dialog_material(tester));
            assert_eq!(material.color, theme_data.background_color);
            assert_eq!(Some(material.elevation), theme_data.elevation);
            assert_eq!(material.shadow_color, theme_data.shadow_color);
            assert_eq!(material.surface_tint_color, theme_data.surface_tint_color);
            assert_eq!(material.shape, theme_data.shape);

            let select_date = text_style(tester, "Select date");
            let header_material =
                read_material(tester, find_header_material(tester, "Select date"));
            assert_eq!(
                style_color(&select_date),
                theme_data.header_foreground_color
            );
            assert_eq!(
                select_date.font_size,
                theme_data
                    .header_help_style
                    .as_ref()
                    .and_then(|style| style.font_size)
            );
            assert_eq!(header_material.color, theme_data.header_background_color);

            let weekday = text_style(tester, "W");
            assert_eq!(
                style_color(&weekday),
                theme_data.weekday_style.as_ref().and_then(style_color)
            );
            assert_eq!(
                weekday.font_size,
                theme_data
                    .weekday_style
                    .as_ref()
                    .and_then(|style| style.font_size)
            );

            let selected_date = text_style(tester, "Wed, Jan 25");
            assert_eq!(
                style_color(&selected_date),
                theme_data.header_foreground_color
            );
            assert_eq!(
                selected_date.font_size,
                theme_data
                    .header_headline_style
                    .as_ref()
                    .and_then(|style| style.font_size)
            );

            let day_font_size = theme_data
                .day_style
                .as_ref()
                .and_then(|style| style.font_size);
            let day31 = text_style(tester, "31");
            let day31_decoration = find_day_decoration(tester, "31");
            assert_eq!(
                style_color(&day31),
                resolved(&theme_data.day_foreground_color)
            );
            assert_eq!(day31.font_size, day_font_size);
            assert_eq!(
                day31_decoration.color.map(|color| color.color()),
                resolved(&theme_data.day_background_color)
            );
            let day_shape = theme_data
                .day_shape
                .as_ref()
                .and_then(|shape| shape.resolve(&WidgetStates::new()))
                .expect("a day shape");
            assert_eq!(
                day31_decoration.shape,
                ShapeBorderRef::from(day_shape.clone())
            );

            // DatePickerDialog.currentDate.
            let day24 = text_style(tester, "24");
            let day24_decoration = find_day_decoration(tester, "24");
            assert_eq!(day24.font_size, day_font_size);
            assert_eq!(
                style_color(&day24),
                resolved(&theme_data.today_foreground_color)
            );
            assert_eq!(
                day24_decoration.color.map(|color| color.color()),
                resolved(&theme_data.today_background_color)
            );
            let today_border = theme_data.today_border.expect("a today border");
            let today_side = BorderSide {
                color: resolved(&theme_data.today_foreground_color).expect("a color"),
                ..today_border
            };
            let expected_today_shape = day_shape
                .as_any()
                .downcast_ref::<RoundedRectangleBorder>()
                .expect("a RoundedRectangleBorder")
                .copy_with(Some(today_side), None);
            assert_eq!(
                day24_decoration.shape,
                ShapeBorderRef::new(expected_today_shape)
            );
            assert_eq!(
                day24_decoration
                    .shape
                    .as_outlined_border()
                    .expect("an OutlinedBorder")
                    .side()
                    .width,
                today_border.width
            );

            // The toggle mode button style.
            let january_2023 = text_style(tester, "January 2023");
            assert_eq!(
                january_2023.font_size,
                theme_data
                    .toggle_button_text_style
                    .as_ref()
                    .and_then(|style| style.font_size)
            );
            assert_eq!(
                style_color(&january_2023),
                theme_data.sub_header_foreground_color
            );
            let arrow = tester.find_by_icon(&Icons::arrow_drop_down());
            assert_eq!(
                tester
                    .widget(arrow, |icon: &waft_widgets::Icon| icon.color.clone())
                    .map(|color| color.color()),
                theme_data.sub_header_foreground_color
            );

            // The day overlay color.
            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::ZERO);
            let at = center_of(tester, text_of(tester, "25"));
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();
            assert!(paints_calls(
                &ink_ops(tester),
                &[PaintCall::circle(resolved(&theme_data.day_overlay_color))]
            ));

            // Show the year selector.
            tap_text(tester, "January 2023");
            tester.pump_and_settle();

            let year_font_size = theme_data
                .year_style
                .as_ref()
                .and_then(|style| style.font_size);
            let year_2022 = text_style(tester, "2022");
            let year_2022_decoration = year_decoration(tester, "2022");
            assert_eq!(year_2022.font_size, year_font_size);
            assert_eq!(
                style_color(&year_2022),
                resolved(&theme_data.year_foreground_color)
            );
            assert_eq!(
                year_2022_decoration.color.map(|color| color.color()),
                resolved(&theme_data.year_background_color)
            );
            let year_shape = theme_data
                .year_shape
                .as_ref()
                .and_then(|shape| shape.resolve(&WidgetStates::new()))
                .expect("a year shape");
            assert_eq!(year_2022_decoration.shape, ShapeBorderRef::from(year_shape));

            // DatePickerDialog.currentDate.
            let year_2023 = text_style(tester, "2023");
            let year_2023_decoration = year_decoration(tester, "2023");
            assert_eq!(year_2023.font_size, year_font_size);
            assert_eq!(
                style_color(&year_2023),
                resolved(&theme_data.today_foreground_color)
            );
            assert_eq!(
                year_2023_decoration.color.map(|color| color.color()),
                resolved(&theme_data.today_background_color)
            );
            let year_2023_side = year_2023_decoration
                .shape
                .as_outlined_border()
                .expect("an OutlinedBorder")
                .side();
            assert_eq!(
                Some(year_2023_side.width),
                theme_data.today_border.map(|side| side.width)
            );
            assert_eq!(
                Some(year_2023_side.color),
                resolved(&theme_data.today_foreground_color)
            );

            // The year overlay color.
            let at = center_of(tester, text_of(tester, "2024"));
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();
            assert!(paints_after_material_fills(
                &ink_ops(tester),
                &[PaintCall::rect(resolved(&theme_data.year_overlay_color))]
            ));

            assert_eq!(
                action_button_style(tester, "Cancel"),
                theme_data.cancel_button_style
            );
            assert_eq!(
                action_button_style(tester, "OK"),
                theme_data.confirm_button_style
            );
        });
    }

    #[test]
    fn date_picker_dialog_uses_theme_data_date_picker_theme_input_mode() {
        test_widgets(|tester| {
            let theme_data = date_picker_theme();
            let theme = themed(theme_data.clone(), true, tester);
            tester.pump_widget(material_app(
                theme,
                centered_home(
                    calendar_dialog().initial_entry_mode(crate::date::DatePickerEntryMode::Input),
                ),
            ));

            let material = read_material(tester, find_dialog_material(tester));
            assert_eq!(material.color, theme_data.background_color);
            assert_eq!(Some(material.elevation), theme_data.elevation);
            assert_eq!(material.shadow_color, theme_data.shadow_color);
            assert_eq!(material.surface_tint_color, theme_data.surface_tint_color);
            assert_eq!(material.shape, theme_data.shape);

            let select_date = text_style(tester, "Select date");
            let header_material =
                read_material(tester, find_header_material(tester, "Select date"));
            assert_eq!(
                style_color(&select_date),
                theme_data.header_foreground_color
            );
            assert_eq!(
                select_date.font_size,
                theme_data
                    .header_help_style
                    .as_ref()
                    .and_then(|style| style.font_size)
            );
            assert_eq!(header_material.color, theme_data.header_background_color);

            let decoration = input_decoration(tester);
            assert_eq!(
                decoration.fill_color.map(|color| color.color()),
                theme_data
                    .input_decoration_theme
                    .as_ref()
                    .and_then(|theme| theme.fill_color.clone())
                    .map(|color| color.color())
            );

            assert_eq!(
                action_button_style(tester, "Cancel"),
                theme_data.cancel_button_style
            );
            assert_eq!(
                action_button_style(tester, "OK"),
                theme_data.confirm_button_style
            );
        });
    }

    fn input_decoration(tester: &WidgetTester) -> crate::input_decorator::InputDecoration {
        let text_field = tester.find_by_type::<crate::text_field::TextField>()[0];
        tester.widget(text_field, |text_field: &crate::text_field::TextField| {
            text_field.decoration.clone().expect("a decoration")
        })
    }

    /// The shared assertions of the two `DateRangePickerDialog` theme cases, which differ in
    /// the Material version and the help text's case.
    fn check_range_dialog_theme(
        tester: &mut WidgetTester,
        use_material3: bool,
        select_range: &str,
    ) {
        let theme_data = date_picker_theme();
        let theme = themed(theme_data.clone(), use_material3, tester);
        tester.pump_widget(material_app(theme, centered_home(range_dialog())));

        let material = read_material(tester, find_dialog_material(tester));
        assert_eq!(material.color, theme_data.background_color);
        let scaffold = tester.find_by_type::<crate::scaffold::Scaffold>()[0];
        assert_eq!(
            tester.widget(scaffold, |scaffold: &crate::scaffold::Scaffold| scaffold
                .background_color),
            theme_data.range_picker_background_color
        );
        assert_eq!(Some(material.elevation), theme_data.range_picker_elevation);
        assert_eq!(material.shadow_color, theme_data.range_picker_shadow_color);
        assert_eq!(
            material.surface_tint_color,
            theme_data.range_picker_surface_tint_color
        );
        assert_eq!(material.shape, theme_data.range_picker_shape);

        let app_bar = tester.find_by_type::<crate::app_bar::AppBar>()[0];
        assert_eq!(
            tester.widget(app_bar, |app_bar: &crate::app_bar::AppBar| app_bar
                .background_color
                .as_ref()
                .map(|color| color.color())),
            theme_data.range_picker_header_background_color
        );

        let select_range = text_style(tester, select_range);
        assert_eq!(
            style_color(&select_range),
            theme_data.range_picker_header_foreground_color
        );
        assert_eq!(
            select_range.font_size,
            theme_data
                .range_picker_header_help_style
                .as_ref()
                .and_then(|style| style.font_size)
        );

        let selected_date = text_style(tester, "Jan 17");
        assert_eq!(
            style_color(&selected_date),
            theme_data.range_picker_header_foreground_color
        );
        assert_eq!(
            selected_date.font_size,
            theme_data
                .range_picker_header_headline_style
                .as_ref()
                .and_then(|style| style.font_size)
        );

        // The day overlay color.
        let mut mouse = TestPointer::mouse(1, 0);
        tester.add_pointer(&mut mouse, waft_embedder::Offset::ZERO);
        let at = center_of(tester, text_of(tester, "16"));
        tester.move_to(&mut mouse, at, false);
        tester.pump_and_settle();
        assert!(paints_calls(
            &ink_ops(tester),
            &[PaintCall::circle(resolved(&theme_data.day_overlay_color))]
        ));

        // The range selection overlay color.
        let at = center_of(tester, text_of(tester, "18"));
        tester.move_to(&mut mouse, at, false);
        tester.pump_and_settle();
        assert!(paints_calls(
            &ink_ops(tester),
            &[PaintCall::circle(resolved(
                &theme_data.range_selection_overlay_color
            ))]
        ));
    }

    #[test]
    fn date_range_picker_dialog_uses_theme_data_date_picker_theme() {
        test_widgets(|tester| check_range_dialog_theme(tester, true, "Select range"));
    }

    #[test]
    fn material2_date_range_picker_dialog_uses_theme_data_date_picker_theme() {
        test_widgets(|tester| check_range_dialog_theme(tester, false, "SELECT RANGE"));
    }

    #[test]
    fn dividers_use_date_picker_theme_data_divider_color() {
        test_widgets(|_| {
            let theme_data = date_picker_theme();
            let show_picker = |size: Size| {
                let mut tester = tester_of_size(size);
                let theme = themed(theme_data.clone(), true, &tester);
                tester.pump_widget(material_app(theme, centered_home(calendar_dialog())));
                tester
            };

            let tester = show_picker(WIDE_WINDOW_SIZE);
            let vertical = tester.find_by_type::<VerticalDivider>()[0];
            assert_eq!(
                tester.widget(vertical, |divider: &VerticalDivider| divider.color),
                theme_data.divider_color
            );

            // The portrait layout.
            let tester = show_picker(NARROW_WINDOW_SIZE);
            let horizontal = tester.find_by_type::<Divider>()[0];
            assert_eq!(
                tester.widget(horizontal, |divider: &Divider| divider.color),
                theme_data.divider_color
            );
        });
    }

    #[test]
    fn date_picker_uses_theme_data_input_decoration_theme_properties_which_are_null_in_date_picker_theme_data_input_decoration_theme()
     {
        test_widgets(|tester| {
            let build_app =
                |tester: &WidgetTester,
                 input_decoration_theme: crate::input_decorator::InputDecorationThemeData,
                 date_picker_theme: Option<DatePickerThemeData>| {
                    let mut builder =
                        ThemeData::builder().input_decoration_theme(input_decoration_theme);
                    if let Some(date_picker_theme) = date_picker_theme {
                        builder = builder.date_picker_theme(date_picker_theme);
                    }
                    let theme = builder.build(&tester.cell.borrow());
                    material_app(
                        theme,
                        centered_home(
                            calendar_dialog()
                                .initial_entry_mode(crate::date::DatePickerEntryMode::Input),
                        ),
                    )
                };

            // With DatePickerThemeData.inputDecorationTheme.
            let root = build_app(
                tester,
                crate::input_decorator::InputDecorationThemeData::new().filled(true),
                Some(date_picker_theme()),
            );
            tester.pump_widget(root);
            let decoration = input_decoration(tester);
            let theme_data = date_picker_theme();
            let input_theme = theme_data.input_decoration_theme.expect("an input theme");
            assert_eq!(
                decoration.fill_color.map(|color| color.color()),
                input_theme.fill_color.map(|color| color.color())
            );
            assert_eq!(decoration.border, input_theme.border);

            // With ThemeData.inputDecorationTheme.
            let root = build_app(
                tester,
                crate::input_decorator::InputDecorationThemeData::new()
                    .filled(true)
                    .fill_color(Color::new(0xFF00FF00))
                    .border(crate::input_border::OutlineInputBorder::default()),
                None,
            );
            tester.pump_widget(root);
            tester.pump_and_settle();

            let decoration = input_decoration(tester);
            assert_eq!(
                decoration.fill_color.map(|color| color.color()),
                Some(Color::new(0xFF00FF00))
            );
            assert_eq!(
                decoration.border,
                Some(crate::input_border::OutlineInputBorder::default().into())
            );
        });
    }

    fn overlay_by_state(
        hovered: u32,
        focused: Option<u32>,
        pressed: u32,
    ) -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Hovered) {
                Some(Color::new(hovered))
            } else if let Some(focused) = focused
                && states.contains(&WidgetState::Focused)
            {
                Some(Color::new(focused))
            } else if states.contains(&WidgetState::Pressed) {
                Some(Color::new(pressed))
            } else {
                Some(Colors::TRANSPARENT)
            }
        })
    }

    fn tab(tester: &mut WidgetTester) {
        tester.send_key_event(
            waft_services::PhysicalKeyboardKey::TAB,
            waft_services::LogicalKeyboardKey::TAB,
        );
    }

    #[test]
    fn date_picker_dialog_resolves_date_picker_theme_day_overlay_color_states() {
        test_widgets(|tester| {
            let day_overlay_color = overlay_by_state(0xff00ff00, Some(0xffff00ff), 0xffffff00);
            let hovered = Color::new(0xff00ff00);
            let pressed = Color::new(0xffffff00);
            let focused = Color::new(0xffff00ff);
            let theme = themed(
                DatePickerThemeData::new().day_overlay_color(day_overlay_color),
                true,
                tester,
            );
            tester.pump_widget(material_app(
                theme,
                centered_home(waft_widgets::Focus::new(calendar_dialog())),
            ));

            // The hover overlay color.
            let mut mouse = TestPointer::mouse(1, 1);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::ZERO);
            let at = center_of(tester, text_of(tester, "20"));
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();
            assert!(paints_calls(
                &tester.nearest_material_ops(text_of(tester, "17")),
                &[
                    PaintCall::circle(None), // Today decoration.
                    PaintCall::circle(None), // Selected day decoration.
                    PaintCall::circle(Some(hovered)),
                ]
            ));

            // The pressed overlay color.
            tester.down(&mut mouse, at);
            tester.pump_and_settle();
            assert!(paints_calls(
                &tester.nearest_material_ops(text_of(tester, "17")),
                &[
                    PaintCall::circle(None), // Today decoration.
                    PaintCall::circle(None), // Selected day decoration.
                    PaintCall::circle(Some(hovered)),
                    PaintCall::circle(Some(pressed)),
                ]
            ));

            remove_pointer(tester, &mouse);
            tester.pump_and_settle();

            // Focus day selection.
            for _ in 0..5 {
                tab(tester);
                tester.pump_and_settle();
            }

            // The focused overlay color.
            assert!(paints_calls(
                &tester.nearest_material_ops(text_of(tester, "17")),
                &[
                    PaintCall::circle(None), // Today decoration.
                    PaintCall::circle(None), // Selected day decoration.
                    PaintCall::circle(Some(focused)),
                ]
            ));
        });
    }

    #[test]
    fn date_picker_dialog_resolves_date_picker_theme_year_overlay_color_states() {
        test_widgets(|tester| {
            let year_overlay_color = overlay_by_state(0xff00ff00, Some(0xffff00ff), 0xffffff00);
            let hovered = Color::new(0xff00ff00);
            let pressed = Color::new(0xffffff00);
            let focused = Color::new(0xffff00ff);
            let theme = themed(
                DatePickerThemeData::new().year_overlay_color(year_overlay_color),
                true,
                tester,
            );
            tester.pump_widget(material_app(
                theme,
                centered_home(waft_widgets::Focus::new(
                    calendar_dialog().initial_calendar_mode(crate::date::DatePickerMode::Year),
                )),
            ));

            // The hover overlay color.
            let mut mouse = TestPointer::mouse(1, 1);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::ZERO);
            let at = center_of(tester, text_of(tester, "2022"));
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();
            assert!(paints_after_material_fills(
                &ink_ops(tester),
                &[PaintCall::rect(Some(hovered))]
            ));

            // The pressed overlay color.
            tester.down(&mut mouse, at);
            tester.pump_and_settle();
            assert!(paints_after_material_fills(
                &ink_ops(tester),
                &[
                    PaintCall::rect(Some(hovered)),
                    PaintCall::rect(Some(pressed))
                ]
            ));

            remove_pointer(tester, &mouse);
            tester.pump_and_settle();

            // Focus year selection.
            for _ in 0..3 {
                tab(tester);
                tester.pump_and_settle();
            }

            // The focused overlay color.
            assert!(paints_after_material_fills(
                &ink_ops(tester),
                &[PaintCall::rect(Some(focused))]
            ));
        });
    }

    #[test]
    fn date_range_picker_dialog_resolves_date_picker_theme_range_selection_overlay_color_states() {
        test_widgets(|tester| {
            let range_selection_overlay_color = overlay_by_state(0xff00ff00, None, 0xffffff00);
            let hovered = Color::new(0xff00ff00);
            let pressed = Color::new(0xffffff00);
            let theme = themed(
                DatePickerThemeData::new()
                    .range_selection_overlay_color(range_selection_overlay_color),
                true,
                tester,
            );
            tester.pump_widget(material_app(theme, centered_home(range_dialog())));

            // The hover overlay color.
            let mut mouse = TestPointer::mouse(1, 1);
            tester.add_pointer(&mut mouse, waft_embedder::Offset::ZERO);
            let at = center_of(tester, text_of(tester, "18"));
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();
            assert!(paints_calls(
                &ink_ops(tester),
                &[PaintCall::circle(Some(hovered))]
            ));

            // The pressed overlay color.
            tester.down(&mut mouse, at);
            tester.pump_and_settle();
            assert!(paints_calls(
                &ink_ops(tester),
                &[
                    PaintCall::circle(Some(hovered)),
                    PaintCall::circle(Some(pressed))
                ]
            ));
        });
    }

    #[test]
    fn toggle_button_uses_date_picker_theme_toggle_button_text_style_color_when_it_is_defined() {
        test_widgets(|tester| {
            let toggle_button_text_color = Color::new(0xff00ff00);
            let sub_header_foreground_color = Color::new(0xffff0000);
            let theme = themed(
                DatePickerThemeData::new()
                    .toggle_button_text_style(TextStyle::new().color(toggle_button_text_color))
                    .sub_header_foreground_color(sub_header_foreground_color),
                true,
                tester,
            );
            tester.pump_widget(material_app(theme, calendar_dialog().into_widget()));

            assert_eq!(
                style_color(&text_style(tester, "January 2023")),
                Some(toggle_button_text_color)
            );
        });
    }

    #[test]
    fn toggle_button_uses_date_picker_theme_sub_header_foreground_color_when_date_picker_theme_toggle_button_text_style_color_is_not_defined()
     {
        test_widgets(|tester| {
            let sub_header_foreground_color = Color::new(0xffff0000);
            let theme = themed(
                DatePickerThemeData::new()
                    .toggle_button_text_style(TextStyle::new())
                    .sub_header_foreground_color(sub_header_foreground_color),
                true,
                tester,
            );
            tester.pump_widget(material_app(theme, calendar_dialog().into_widget()));

            assert_eq!(
                style_color(&text_style(tester, "January 2023")),
                Some(sub_header_foreground_color)
            );
        });
    }
}
