//! Flutter counterpart: material_ui's `time_picker.dart`. Its part
//! `generated/time_picker_defaults_m3.g.dart` is `generated/time_picker_defaults_m3.rs`.

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::fmt;
use std::rc::Rc;

use waft_animation::{
    AnimationBehavior, AnimationController, AnimationRef, CurveTween, Drive, Tween,
};
use waft_embedder::{
    Brightness, Canvas, Clip, ClipOp, Color, Locale, Offset, Paint, Path, PathBuilder, Size,
    TargetPlatform, TextBaseline, TextEditingValue,
};
use waft_foundation::{App, Handle, ListenableObject, Listener, Ref, Timer, VoidCallback};
use waft_painting::{
    Alignment, AlignmentDirectional, AlignmentGeometry, AnyColor, AnyTextStyle, Axis, BorderRadius,
    BorderRadiusGeometry, BorderSide, EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry,
    OutlinedBorderRef, PaintingBinding, RoundedRectangleBorder, ShapeBorderRef, TextAlign,
    TextDirection, TextOverflow, TextPainter, TextScaler, TextSpan, TextStyle, VerticalDirection,
    draw_line, force_to_point,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData,
    ChildLayoutHelper, ChildLayouter, CrossAxisAlignment, CustomPainter, MainAxisAlignment,
    MainAxisSize, PaintingContext, RenderBox, RenderBoxBase, RenderBoxData, RenderHandle,
    RenderObject, RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin,
    RenderShiftedBox,
};
use waft_services::{
    HapticFeedback, LengthLimitingTextInputFormatter, TextInputAction, TextInputFormatterRef,
    TextInputType,
};
use waft_widgets::{
    Align, AspectRatio, AutovalidateMode, BuildContext, Builder, Center, Column, ConstrainedBox,
    CustomPaint, Directionality, ExcludeSemantics, Expanded, Flexible, FocusNode, FocusNodeLeaf,
    FocusScope, Form, FormState, GestureDetector, GlobalKey, Icon, InheritedModel,
    InheritedModelKind, InheritedWidget, IntoWidget, LayoutBuilder, Localizations, MediaQuery,
    Navigator, Orientation, OverflowBar, OverflowBarAlignment, Padding, RenderObjectWidget,
    RestorableBool, RestorableBoolN, RestorableEnum, RestorableEnumN, RestorableListenable,
    RestorableProperty, RestorableTextEditingController, RestorableValue, RestorationMixin,
    RestorationMixinData, Row, Semantics, SingleChildRenderObjectWidget, SingleChildScrollView,
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, SizedBox, Stack, State,
    StateData, StatefulWidget, StatelessWidget, Text, TextEditingControllerLeaf,
    UnfocusDisposition, UnmanagedRestorationScope, WidgetRef, WidgetState, WidgetStateColor,
    WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
#[allow(deprecated)]
use crate::curves::standard_easing;
use crate::dialog::Dialog;
use crate::generated::time_picker_defaults_m3::TimePickerDefaultsM3;
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::ink_well::InkWell;
use crate::input_border::OutlineInputBorder;
use crate::input_decorator::{InputDecoration, InputDecorationThemeData};
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::text_button::TextButton;
use crate::text_form_field::TextFormField;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, ThemeData};
use crate::theme_data_getters::DefaultsContext;
use crate::time::{
    DayPeriod, HourFormat, RestorableTimeOfDay, TimeOfDay, TimeOfDayFormat, hour_format,
};
use crate::time_picker_theme::{TimePickerTheme, TimePickerThemeData};

const K_DIALOG_SIZE_ANIMATION_DURATION: std::time::Duration = std::time::Duration::from_millis(200);
const K_DIAL_ANIMATE_DURATION: std::time::Duration = std::time::Duration::from_millis(200);
const K_TWO_PI: f64 = 2.0 * std::f64::consts::PI;
const K_VIBRATE_COMMIT_DELAY: std::time::Duration = std::time::Duration::from_millis(100);

const K_TIME_PICKER_HEADER_LANDSCAPE_WIDTH: f64 = 216.0;
const K_TIME_PICKER_INNER_DIAL_OFFSET: f64 = 28.0;
const K_TIME_PICKER_DIAL_MIN_RADIUS: f64 = 50.0;
const K_TIME_PICKER_DIAL_PADDING: f64 = 28.0;

/// Dart's `TimePickerEntryMode`: the ways a time picker dialog can be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimePickerEntryMode {
    /// User picks time from a clock dial. Can switch to [`Input`](Self::Input) by pressing a
    /// button in the dialog.
    Dial,

    /// User can input the time by typing it into text fields. Can switch to
    /// [`Dial`](Self::Dial) by pressing a button in the dialog.
    Input,

    /// User can only pick time from a clock dial. Not user-switchable.
    DialOnly,

    /// User can only input the time by typing it into text fields. Not user-switchable.
    InputOnly,
}

/// Whether the dial-mode time picker is currently selecting the hour or the minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HourMinuteMode {
    Hour,
    Minute,
}

/// Dart's `_HourDialType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HourDialType {
    TwentyFourHour,
    TwentyFourHourDoubleRing,
    TwelveHour,
}

/// Aspects of [`TimePickerModel`] that can be depended upon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum TimePickerAspect {
    Use24HourFormat,
    UseMaterial3,
    EntryMode,
    HourMinuteMode,
    OnHourMinuteModeChanged,
    OnHourDoubleTapped,
    OnMinuteDoubleTapped,
    HourDialType,
    SelectedTime,
    OnSelectedTimeChanged,
    Orientation,
    Theme,
    DefaultTheme,
}

type HourMinuteModeChanged = Rc<dyn Fn(&mut App, HourMinuteMode)>;
type SelectedTimeChanged = Rc<dyn Fn(&mut App, TimeOfDay)>;
type StringChanged = Rc<dyn Fn(&mut App, &str)>;

/// Dart's `_TimePickerModel`.
struct TimePickerModel {
    entry_mode: TimePickerEntryMode,
    hour_minute_mode: HourMinuteMode,
    on_hour_minute_mode_changed: HourMinuteModeChanged,
    on_hour_double_tapped: Listener,
    on_minute_double_tapped: Listener,
    selected_time: TimeOfDay,
    on_selected_time_changed: SelectedTimeChanged,
    use_24_hour_format: bool,
    use_material3: bool,
    hour_dial_type: HourDialType,
    orientation: Orientation,
    theme: TimePickerThemeData,
    default_theme: Rc<dyn TimePickerDefaults>,
    child: WidgetRef,
}

impl fmt::Debug for TimePickerModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimePickerModel")
            .field("entry_mode", &self.entry_mode)
            .field("hour_minute_mode", &self.hour_minute_mode)
            .field("selected_time", &self.selected_time)
            .finish_non_exhaustive()
    }
}

impl TimePickerModel {
    /// The erased widget: an [`InheritedModel`] has two `IntoWidget` impls, so the kind is
    /// named here.
    fn into_widget(self) -> WidgetRef {
        IntoWidget::<InheritedModelKind>::into_widget(self)
    }

    fn of<R>(
        app: &mut App,
        context: BuildContext,
        aspect: TimePickerAspect,
        read: impl FnOnce(&TimePickerModel) -> R,
    ) -> R {
        read(
            TimePickerModel::inherit_from(app, context, Some(aspect))
                .expect("a time picker model above"),
        )
    }

    fn entry_mode_of(app: &mut App, context: BuildContext) -> TimePickerEntryMode {
        Self::of(app, context, TimePickerAspect::EntryMode, |model| {
            model.entry_mode
        })
    }

    fn hour_minute_mode_of(app: &mut App, context: BuildContext) -> HourMinuteMode {
        Self::of(app, context, TimePickerAspect::HourMinuteMode, |model| {
            model.hour_minute_mode
        })
    }

    fn selected_time_of(app: &mut App, context: BuildContext) -> TimeOfDay {
        Self::of(app, context, TimePickerAspect::SelectedTime, |model| {
            model.selected_time
        })
    }

    fn use_24_hour_format_of(app: &mut App, context: BuildContext) -> bool {
        Self::of(app, context, TimePickerAspect::Use24HourFormat, |model| {
            model.use_24_hour_format
        })
    }

    fn use_material3_of(app: &mut App, context: BuildContext) -> bool {
        Self::of(app, context, TimePickerAspect::UseMaterial3, |model| {
            model.use_material3
        })
    }

    fn hour_dial_type_of(app: &mut App, context: BuildContext) -> HourDialType {
        Self::of(app, context, TimePickerAspect::HourDialType, |model| {
            model.hour_dial_type
        })
    }

    fn orientation_of(app: &mut App, context: BuildContext) -> Orientation {
        Self::of(app, context, TimePickerAspect::Orientation, |model| {
            model.orientation
        })
    }

    fn theme_of(app: &mut App, context: BuildContext) -> TimePickerThemeData {
        Self::of(app, context, TimePickerAspect::Theme, |model| {
            model.theme.clone()
        })
    }

    fn default_theme_of(app: &mut App, context: BuildContext) -> Rc<dyn TimePickerDefaults> {
        Self::of(app, context, TimePickerAspect::DefaultTheme, |model| {
            Rc::clone(&model.default_theme)
        })
    }

    fn on_hour_double_tapped_of(app: &mut App, context: BuildContext) -> Listener {
        Self::of(
            app,
            context,
            TimePickerAspect::OnHourDoubleTapped,
            |model| model.on_hour_double_tapped.clone(),
        )
    }

    fn on_minute_double_tapped_of(app: &mut App, context: BuildContext) -> Listener {
        Self::of(
            app,
            context,
            TimePickerAspect::OnMinuteDoubleTapped,
            |model| model.on_minute_double_tapped.clone(),
        )
    }

    fn set_selected_time(app: &mut App, context: BuildContext, value: TimeOfDay) {
        let callback = Self::of(
            app,
            context,
            TimePickerAspect::OnSelectedTimeChanged,
            |model| Rc::clone(&model.on_selected_time_changed),
        );
        callback(app, value);
    }

    fn set_hour_minute_mode(app: &mut App, context: BuildContext, value: HourMinuteMode) {
        let callback = Self::of(
            app,
            context,
            TimePickerAspect::OnHourMinuteModeChanged,
            |model| Rc::clone(&model.on_hour_minute_mode_changed),
        );
        callback(app, value);
    }
}

impl InheritedWidget for TimePickerModel {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &TimePickerModel) -> bool {
        self.use_24_hour_format != old_widget.use_24_hour_format
            || self.use_material3 != old_widget.use_material3
            || self.entry_mode != old_widget.entry_mode
            || self.hour_minute_mode != old_widget.hour_minute_mode
            || !Rc::ptr_eq(
                &self.on_hour_minute_mode_changed,
                &old_widget.on_hour_minute_mode_changed,
            )
            || self.on_hour_double_tapped != old_widget.on_hour_double_tapped
            || self.on_minute_double_tapped != old_widget.on_minute_double_tapped
            || self.hour_dial_type != old_widget.hour_dial_type
            || self.selected_time != old_widget.selected_time
            || !Rc::ptr_eq(
                &self.on_selected_time_changed,
                &old_widget.on_selected_time_changed,
            )
            || self.orientation != old_widget.orientation
            || self.theme != old_widget.theme
            || !Rc::ptr_eq(&self.default_theme, &old_widget.default_theme)
    }
}

impl InheritedModel for TimePickerModel {
    type Aspect = TimePickerAspect;

    fn update_should_notify_dependent(
        &self,
        old_widget: &TimePickerModel,
        dependencies: &HashSet<TimePickerAspect>,
    ) -> bool {
        let changed =
            |aspect: TimePickerAspect, changed: bool| changed && dependencies.contains(&aspect);
        changed(
            TimePickerAspect::Use24HourFormat,
            self.use_24_hour_format != old_widget.use_24_hour_format,
        ) || changed(
            TimePickerAspect::UseMaterial3,
            self.use_material3 != old_widget.use_material3,
        ) || changed(
            TimePickerAspect::EntryMode,
            self.entry_mode != old_widget.entry_mode,
        ) || changed(
            TimePickerAspect::HourMinuteMode,
            self.hour_minute_mode != old_widget.hour_minute_mode,
        ) || changed(
            TimePickerAspect::OnHourMinuteModeChanged,
            !Rc::ptr_eq(
                &self.on_hour_minute_mode_changed,
                &old_widget.on_hour_minute_mode_changed,
            ),
        )
            // Dart compares `onHourMinuteModeChanged` against the old double-tap callbacks for
            // the next two aspects, a typo that makes them notify whenever it differs from them.
            || changed(TimePickerAspect::OnHourDoubleTapped, true)
            || changed(TimePickerAspect::OnMinuteDoubleTapped, true)
            || changed(
                TimePickerAspect::HourDialType,
                self.hour_dial_type != old_widget.hour_dial_type,
            )
            || changed(
                TimePickerAspect::SelectedTime,
                self.selected_time != old_widget.selected_time,
            )
            || changed(
                TimePickerAspect::OnSelectedTimeChanged,
                !Rc::ptr_eq(
                    &self.on_selected_time_changed,
                    &old_widget.on_selected_time_changed,
                ),
            )
            || changed(
                TimePickerAspect::Orientation,
                self.orientation != old_widget.orientation,
            )
            || changed(TimePickerAspect::Theme, self.theme != old_widget.theme)
            || changed(
                TimePickerAspect::DefaultTheme,
                !Rc::ptr_eq(&self.default_theme, &old_widget.default_theme),
            )
    }
}

/// Dart's `_debugDialTimePickerEntryMode`.
fn debug_dial_time_picker_entry_mode(app: &mut App, context: BuildContext) -> bool {
    let entry_mode = TimePickerModel::entry_mode_of(app, context);
    entry_mode == TimePickerEntryMode::Dial || entry_mode == TimePickerEntryMode::DialOnly
}

/// A style as a plain [`TextStyle`]: what Dart's `Text(style:)` merges when the style is a
/// `WidgetStateTextStyle`.
fn plain(style: &waft_painting::AnyTextStyle) -> TextStyle {
    style.value().clone()
}

/// Dart's `_DialTimePickerHeader`.
#[derive(Debug)]
struct DialTimePickerHeader {
    help_text: String,
}

impl StatelessWidget for DialTimePickerHeader {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_dial_time_picker_entry_mode(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let use_24_hour_format = TimePickerModel::use_24_hour_format_of(app, context);
        let time_of_day_format = localizations.time_of_day_format(use_24_hour_format);

        let default_theme = TimePickerModel::default_theme_of(app, context);
        let orientation = TimePickerModel::orientation_of(app, context);
        let day_period_height = if orientation == Orientation::Portrait {
            default_theme.day_period_portrait_size(app).height()
        } else {
            default_theme.day_period_landscape_size(app).height()
        };
        let min_interactive_vertical_padding = if orientation == Orientation::Portrait {
            (2.0 * K_MIN_INTERACTIVE_DIMENSION - day_period_height).max(0.0)
        } else {
            (K_MIN_INTERACTIVE_DIMENSION - day_period_height).max(0.0)
        };

        let hour_dial_type = TimePickerModel::hour_dial_type_of(app, context);
        let theme = TimePickerModel::theme_of(app, context);
        let help_text_style = plain(
            &theme
                .help_text_style
                .clone()
                .unwrap_or_else(|| default_theme.help_text_style(app)),
        );
        let hour_minute_row = || {
            Row::new()
                // Hour/minutes should not change positions in RTL locales.
                .text_direction(TextDirection::Ltr)
                .children([
                    Expanded::new(DialHourControl).into_widget(),
                    TimeSelectorSeparator { time_of_day_format }.into_widget(),
                    Expanded::new(DialMinuteControl).into_widget(),
                ])
        };
        let orientation_specific_header = match orientation {
            Orientation::Portrait => Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children([
                    Padding::new(
                        EdgeInsetsDirectional::only(
                            0.0,
                            0.0,
                            0.0,
                            (if TimePickerModel::use_material3_of(app, context) {
                                20.0
                            } else {
                                24.0
                            }) - min_interactive_vertical_padding / 2.0,
                        )
                        .into(),
                    )
                    .child(Text::new(self.help_text.clone()).style(help_text_style))
                    .into_widget(),
                    Row::new()
                        .text_direction(
                            if time_of_day_format == TimeOfDayFormat::a_space_h_colon_mm {
                                TextDirection::Rtl
                            } else {
                                TextDirection::Ltr
                            },
                        )
                        .spacing(12.0)
                        .children(
                            [Expanded::new(hour_minute_row()).into_widget()]
                                .into_iter()
                                .chain(
                                    (hour_dial_type == HourDialType::TwelveHour)
                                        .then(|| DayPeriodControl::default().into_widget()),
                                ),
                        )
                        .into_widget(),
                ])
                .into_widget(),
            Orientation::Landscape => SizedBox::new()
                .width(K_TIME_PICKER_HEADER_LANDSCAPE_WIDTH)
                .child(
                    Stack::new().children([
                        Text::new(self.help_text.clone())
                            .style(help_text_style)
                            .into_widget(),
                        Column::new()
                            .vertical_direction(
                                if time_of_day_format == TimeOfDayFormat::a_space_h_colon_mm {
                                    VerticalDirection::Up
                                } else {
                                    VerticalDirection::Down
                                },
                            )
                            .main_axis_alignment(MainAxisAlignment::Center)
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .spacing((16.0 - min_interactive_vertical_padding / 2.0).max(0.0))
                            .children(
                                [hour_minute_row().into_widget()].into_iter().chain(
                                    (hour_dial_type == HourDialType::TwelveHour)
                                        .then(|| DayPeriodControl::default().into_widget()),
                                ),
                            )
                            .into_widget(),
                    ]),
                )
                .into_widget(),
        };

        Semantics::new()
            .label(localizations.format_time_of_day(
                &TimePickerModel::selected_time_of(app, context),
                MediaQuery::always_use_24_hour_format_of(app, context),
            ))
            .child(orientation_specific_header)
            .into_widget()
    }
}

/// Dart's `_DialTimeSelectorControl`.
#[derive(Debug)]
struct DialTimeSelectorControl {
    text: String,
    on_tap: Listener,
    on_double_tap: Listener,
    is_selected: bool,
}

impl StatelessWidget for DialTimeSelectorControl {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_dial_time_picker_entry_mode(app, context));
        let time_picker_theme = TimePickerModel::theme_of(app, context);
        let default_theme = TimePickerModel::default_theme_of(app, context);
        let background_color = time_picker_theme
            .hour_minute_color
            .clone()
            .unwrap_or_else(|| default_theme.hour_minute_color(app));
        let shape = time_picker_theme
            .hour_minute_shape
            .clone()
            .unwrap_or_else(|| default_theme.hour_minute_shape(app));

        let mut states = WidgetStates::new();
        if self.is_selected {
            states.insert(WidgetState::Selected);
        }
        let effective_text_color = <dyn WidgetStateProperty<Color>>::resolve_as(
            &time_picker_theme
                .hour_minute_text_color
                .clone()
                .unwrap_or_else(|| default_theme.hour_minute_text_color(app)),
            &states,
        );
        let hour_minute_text_style = time_picker_theme
            .hour_minute_text_style
            .clone()
            .unwrap_or_else(|| default_theme.hour_minute_text_style(app));
        let effective_style =
            <dyn WidgetStateProperty<TextStyle>>::resolve_as(&hour_minute_text_style, &states)
                .value()
                .copy_with()
                .color(effective_text_color.color());

        let mut ink_well = InkWell::new().on_tap(self.on_tap.clone());
        if self.is_selected {
            ink_well = ink_well.on_double_tap(self.on_double_tap.clone());
        }
        let background_color =
            <dyn WidgetStateProperty<Color>>::resolve_as(&background_color, &states);
        SizedBox::new()
            .height(default_theme.hour_minute_size(app).height())
            .child(
                Material::new()
                    .color(background_color.color())
                    .clip_behavior(Clip::AntiAlias)
                    .shape(shape)
                    .child(
                        ink_well.child(
                            Center::new().child(
                                Text::new(self.text.clone())
                                    .style(effective_style)
                                    .text_scaler(TextScaler::NO_SCALING),
                            ),
                        ),
                    ),
            )
            .into_widget()
    }
}

/// Dart's `_DialHourControl`.
#[derive(Debug)]
struct DialHourControl;

impl StatelessWidget for DialHourControl {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_dial_time_picker_entry_mode(app, context));
        waft_widgets::debug_check_has_media_query(app, context);
        let always_use_24_hour_format = MediaQuery::always_use_24_hour_format_of(app, context);
        let selected_time = TimePickerModel::selected_time_of(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let formatted_hour = localizations.format_hour(
            &selected_time,
            TimePickerModel::use_24_hour_format_of(app, context),
        );

        let hour_dial_type = TimePickerModel::hour_dial_type_of(app, context);
        let hours_from_selected = |hours_to_add: i32| -> TimeOfDay {
            match hour_dial_type {
                HourDialType::TwentyFourHour | HourDialType::TwentyFourHourDoubleRing => {
                    let selected_hour = selected_time.hour;
                    selected_time.replacing(
                        Some((selected_hour + hours_to_add).rem_euclid(TimeOfDay::HOURS_PER_DAY)),
                        None,
                    )
                }
                HourDialType::TwelveHour => {
                    // Cycle 1 through 12 without changing day period.
                    let period_offset = selected_time.period_offset();
                    let hours = selected_time.hour_of_period();
                    selected_time.replacing(
                        Some(
                            period_offset
                                + (hours + hours_to_add).rem_euclid(TimeOfDay::HOURS_PER_PERIOD),
                        ),
                        None,
                    )
                }
            }
        };

        let next_hour = hours_from_selected(1);
        let formatted_next_hour = localizations.format_hour(&next_hour, always_use_24_hour_format);
        let previous_hour = hours_from_selected(-1);
        let formatted_previous_hour =
            localizations.format_hour(&previous_hour, always_use_24_hour_format);

        Semantics::new()
            .value(format!(
                "{} {}",
                localizations.time_picker_hour_mode_announcement(),
                formatted_hour
            ))
            .exclude_semantics(true)
            .increased_value(formatted_next_hour)
            .on_increase(std::rc::Rc::new(move |app: &mut App| {
                TimePickerModel::set_selected_time(app, context, next_hour);
            }))
            .decreased_value(formatted_previous_hour)
            .on_decrease(std::rc::Rc::new(move |app: &mut App| {
                TimePickerModel::set_selected_time(app, context, previous_hour);
            }))
            .child(DialTimeSelectorControl {
                is_selected: TimePickerModel::hour_minute_mode_of(app, context)
                    == HourMinuteMode::Hour,
                text: formatted_hour,
                on_tap: Listener::new(move |app| {
                    TimePickerModel::set_hour_minute_mode(app, context, HourMinuteMode::Hour);
                }),
                on_double_tap: TimePickerModel::on_hour_double_tapped_of(app, context),
            })
            .into_widget()
    }
}

/// Dart's `_TimeSelectorSeparator`.
#[derive(Debug)]
struct TimeSelectorSeparator {
    time_of_day_format: TimeOfDayFormat,
}

impl TimeSelectorSeparator {
    fn time_selector_separator_value(time_of_day_format: TimeOfDayFormat) -> &'static str {
        match time_of_day_format {
            TimeOfDayFormat::h_colon_mm_space_a
            | TimeOfDayFormat::a_space_h_colon_mm
            | TimeOfDayFormat::H_colon_mm
            | TimeOfDayFormat::HH_colon_mm => ":",
            TimeOfDayFormat::HH_dot_mm => ".",
            TimeOfDayFormat::FrenchCanadian => "h",
        }
    }
}

impl StatelessWidget for TimeSelectorSeparator {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let time_picker_theme = TimePickerTheme::of(app, context);
        let default_theme: std::rc::Rc<dyn TimePickerDefaults> = if theme.use_material3 {
            std::rc::Rc::new(TimePickerDefaultsM3::new(
                context,
                TimePickerEntryMode::Dial,
            ))
        } else {
            std::rc::Rc::new(TimePickerDefaultsM2::new(context))
        };
        let states = WidgetStates::new();

        let effective_text_color = <dyn WidgetStateProperty<Color>>::resolve_as(
            &time_picker_theme
                .time_selector_separator_color
                .as_ref()
                .and_then(|color| color.resolve(&states))
                .map(AnyColor::from)
                .or_else(|| time_picker_theme.hour_minute_text_color.clone())
                .or_else(|| {
                    default_theme
                        .time_selector_separator_color(app)
                        .and_then(|color| color.resolve(&states))
                        .map(AnyColor::from)
                })
                .unwrap_or_else(|| default_theme.hour_minute_text_color(app)),
            &states,
        );
        let separator_text_style = time_picker_theme
            .time_selector_separator_text_style
            .as_ref()
            .and_then(|style| style.resolve(&states))
            .map(waft_painting::AnyTextStyle::from)
            .or_else(|| time_picker_theme.hour_minute_text_style.clone())
            .or_else(|| {
                default_theme
                    .time_selector_separator_text_style(app)
                    .and_then(|style| style.resolve(&states))
                    .map(waft_painting::AnyTextStyle::from)
            })
            .unwrap_or_else(|| default_theme.hour_minute_text_style(app));
        let effective_style =
            <dyn WidgetStateProperty<TextStyle>>::resolve_as(&separator_text_style, &states)
                .value()
                .copy_with()
                .color(effective_text_color.color())
                .height(1.0);

        let height = match TimePickerModel::entry_mode_of(app, context) {
            TimePickerEntryMode::Dial | TimePickerEntryMode::DialOnly => {
                default_theme.hour_minute_size(app).height()
            }
            TimePickerEntryMode::Input | TimePickerEntryMode::InputOnly => {
                default_theme.hour_minute_input_size(app).height()
            }
        };

        ExcludeSemantics::new()
            .child(
                SizedBox::new()
                    .width(
                        if self.time_of_day_format == TimeOfDayFormat::FrenchCanadian {
                            36.0
                        } else {
                            24.0
                        },
                    )
                    .height(height)
                    .child(
                        Center::new().child(
                            Text::new(Self::time_selector_separator_value(self.time_of_day_format))
                                .style(effective_style)
                                .text_scaler(TextScaler::NO_SCALING),
                        ),
                    ),
            )
            .into_widget()
    }
}

/// Dart's `_DialMinuteControl`.
#[derive(Debug)]
struct DialMinuteControl;

impl StatelessWidget for DialMinuteControl {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_dial_time_picker_entry_mode(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let selected_time = TimePickerModel::selected_time_of(app, context);
        let formatted_minute = localizations.format_minute(&selected_time);
        let next_minute = selected_time.replacing(
            None,
            Some((selected_time.minute + 1).rem_euclid(TimeOfDay::MINUTES_PER_HOUR)),
        );
        let formatted_next_minute = localizations.format_minute(&next_minute);
        let previous_minute = selected_time.replacing(
            None,
            Some((selected_time.minute - 1).rem_euclid(TimeOfDay::MINUTES_PER_HOUR)),
        );
        let formatted_previous_minute = localizations.format_minute(&previous_minute);

        Semantics::new()
            .exclude_semantics(true)
            .value(format!(
                "{} {}",
                localizations.time_picker_minute_mode_announcement(),
                formatted_minute
            ))
            .increased_value(formatted_next_minute)
            .on_increase(std::rc::Rc::new(move |app: &mut App| {
                TimePickerModel::set_selected_time(app, context, next_minute);
            }))
            .decreased_value(formatted_previous_minute)
            .on_decrease(std::rc::Rc::new(move |app: &mut App| {
                TimePickerModel::set_selected_time(app, context, previous_minute);
            }))
            .child(DialTimeSelectorControl {
                is_selected: TimePickerModel::hour_minute_mode_of(app, context)
                    == HourMinuteMode::Minute,
                text: formatted_minute,
                on_tap: Listener::new(move |app| {
                    TimePickerModel::set_hour_minute_mode(app, context, HourMinuteMode::Minute);
                }),
                on_double_tap: TimePickerModel::on_minute_double_tapped_of(app, context),
            })
            .into_widget()
    }
}

/// Dart's `_DayPeriodControl`.
#[derive(Default)]
struct DayPeriodControl {
    /// Dart's `onPeriodChanged`.
    on_period_changed: Option<SelectedTimeChanged>,
}

impl std::fmt::Debug for DayPeriodControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DayPeriodControl").finish_non_exhaustive()
    }
}

impl DayPeriodControl {
    fn toggle_period(
        on_period_changed: Option<&SelectedTimeChanged>,
        app: &mut App,
        context: BuildContext,
    ) {
        let selected_time = TimePickerModel::selected_time_of(app, context);
        let new_hour =
            (selected_time.hour + TimeOfDay::HOURS_PER_PERIOD).rem_euclid(TimeOfDay::HOURS_PER_DAY);
        let new_time = selected_time.replacing(Some(new_hour), None);
        if let Some(on_period_changed) = on_period_changed {
            on_period_changed(app, new_time);
        } else {
            TimePickerModel::set_selected_time(app, context, new_time);
        }
    }

    fn set_am(
        on_period_changed: Option<&SelectedTimeChanged>,
        app: &mut App,
        context: BuildContext,
    ) {
        let selected_time = TimePickerModel::selected_time_of(app, context);
        if selected_time.period() == DayPeriod::Am {
            return;
        }
        Self::toggle_period(on_period_changed, app, context);
    }

    fn set_pm(
        on_period_changed: Option<&SelectedTimeChanged>,
        app: &mut App,
        context: BuildContext,
    ) {
        let selected_time = TimePickerModel::selected_time_of(app, context);
        if selected_time.period() == DayPeriod::Pm {
            return;
        }
        Self::toggle_period(on_period_changed, app, context);
    }
}

impl StatelessWidget for DayPeriodControl {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let material_localizations = <dyn MaterialLocalizations>::of(app, context);
        let time_picker_theme = TimePickerModel::theme_of(app, context);
        let default_theme = TimePickerModel::default_theme_of(app, context);
        let selected_time = TimePickerModel::selected_time_of(app, context);
        let am_selected = selected_time.period() == DayPeriod::Am;
        let pm_selected = !am_selected;
        let resolved_side = time_picker_theme
            .day_period_border_side
            .unwrap_or_else(|| default_theme.day_period_border_side(app));
        let resolved_shape: OutlinedBorderRef = time_picker_theme
            .day_period_shape
            .clone()
            .unwrap_or_else(|| default_theme.day_period_shape(app))
            .copy_with(Some(resolved_side));

        let (day_period_size, orientation) = match TimePickerModel::entry_mode_of(app, context) {
            TimePickerEntryMode::Dial | TimePickerEntryMode::DialOnly => {
                let orientation = TimePickerModel::orientation_of(app, context);
                (
                    match orientation {
                        Orientation::Portrait => default_theme.day_period_portrait_size(app),
                        Orientation::Landscape => default_theme.day_period_landscape_size(app),
                    },
                    orientation,
                )
            }
            TimePickerEntryMode::Input | TimePickerEntryMode::InputOnly => (
                default_theme.day_period_input_size(app),
                Orientation::Portrait,
            ),
        };

        let mut am_shape = resolved_shape.clone();
        let mut pm_shape = resolved_shape.clone();
        let rounded_border: Option<(RoundedRectangleBorder, BorderRadius)> = resolved_shape
            .as_any()
            .downcast_ref::<RoundedRectangleBorder>()
            .and_then(|border| match &border.border_radius {
                BorderRadiusGeometry::BorderRadius(radius) => Some((*border, *radius)),
                _ => None,
            });
        let with_radius = |border: &RoundedRectangleBorder, radius: BorderRadius| {
            OutlinedBorderRef::new(
                border.copy_with(None, Some(BorderRadiusGeometry::BorderRadius(radius))),
            )
        };

        // In order to respect Material touch target guidelines, the Semantics for
        // AM and PM buttons needs to expand out of the bounds of the buttons
        // (Similarly to Google Agenda).
        // To achieve this, instead of using a parent Material which clips
        // the Semantics, each button should manage its own shape.
        // The logic below "cuts" the given period selector shape in two parts,
        // one for the AM button, the other for the PM button. Each sub-shape
        // is obtained by removing some rounded corners from the original shape.
        let on_period_changed = self.on_period_changed.clone();
        let on_am = {
            let on_period_changed = on_period_changed.clone();
            Listener::new(move |app| Self::set_am(on_period_changed.as_ref(), app, context))
        };
        let on_pm =
            Listener::new(move |app| Self::set_pm(on_period_changed.as_ref(), app, context));

        match orientation {
            Orientation::Portrait => {
                if let Some((border, border_radius)) = rounded_border {
                    am_shape = with_radius(
                        &border,
                        BorderRadius::only(
                            border_radius.top_left,
                            border_radius.top_right,
                            waft_embedder::Radius::ZERO,
                            waft_embedder::Radius::ZERO,
                        ),
                    );
                    pm_shape = with_radius(
                        &border,
                        BorderRadius::only(
                            waft_embedder::Radius::ZERO,
                            waft_embedder::Radius::ZERO,
                            border_radius.bottom_left,
                            border_radius.bottom_right,
                        ),
                    );
                }

                let min_interactive_size = Size::new(
                    day_period_size.width(),
                    day_period_size
                        .height()
                        .max(2.0 * K_MIN_INTERACTIVE_DIMENSION),
                );

                let am_button = AmPmButton {
                    selected: am_selected,
                    on_pressed: on_am,
                    label: material_localizations.ante_meridiem_abbreviation(),
                    padding: EdgeInsets::only(
                        0.0,
                        (min_interactive_size.height() - day_period_size.height()) / 2.0,
                        0.0,
                        0.0,
                    ),
                    shape: am_shape,
                };
                let pm_button = AmPmButton {
                    selected: pm_selected,
                    on_pressed: on_pm,
                    label: material_localizations.post_meridiem_abbreviation(),
                    padding: EdgeInsets::only(
                        0.0,
                        0.0,
                        0.0,
                        (min_interactive_size.height() - day_period_size.height()) / 2.0,
                    ),
                    shape: pm_shape,
                };

                DayPeriodInputPadding {
                    min_size: min_interactive_size,
                    orientation,
                    child: Some(
                        SizedBox::new()
                            .width(min_interactive_size.width())
                            .height(min_interactive_size.height())
                            .child(Column::new().children([
                                Expanded::new(am_button).into_widget(),
                                Expanded::new(pm_button).into_widget(),
                            ]))
                            .into_widget(),
                    ),
                }
                .into_widget()
            }
            Orientation::Landscape => {
                if let Some((border, border_radius)) = rounded_border {
                    am_shape = with_radius(
                        &border,
                        BorderRadius::only(
                            border_radius.top_left,
                            waft_embedder::Radius::ZERO,
                            border_radius.bottom_left,
                            waft_embedder::Radius::ZERO,
                        ),
                    );
                    pm_shape = with_radius(
                        &border,
                        BorderRadius::only(
                            waft_embedder::Radius::ZERO,
                            border_radius.top_right,
                            waft_embedder::Radius::ZERO,
                            border_radius.bottom_right,
                        ),
                    );
                }

                let min_interactive_size = Size::new(
                    day_period_size.width(),
                    day_period_size.height().max(K_MIN_INTERACTIVE_DIMENSION),
                );
                let vertical = (min_interactive_size.height() - day_period_size.height()) / 2.0;

                let am_button = AmPmButton {
                    selected: am_selected,
                    on_pressed: on_am,
                    label: material_localizations.ante_meridiem_abbreviation(),
                    padding: EdgeInsets::symmetric(vertical, 0.0),
                    shape: am_shape,
                };
                let pm_button = AmPmButton {
                    selected: pm_selected,
                    on_pressed: on_pm,
                    label: material_localizations.post_meridiem_abbreviation(),
                    padding: EdgeInsets::symmetric(vertical, 0.0),
                    shape: pm_shape,
                };

                DayPeriodInputPadding {
                    min_size: min_interactive_size,
                    orientation,
                    child: Some(
                        SizedBox::new()
                            .height(min_interactive_size.height())
                            .child(Row::new().children([
                                Expanded::new(am_button).into_widget(),
                                Expanded::new(pm_button).into_widget(),
                            ]))
                            .into_widget(),
                    ),
                }
                .into_widget()
            }
        }
    }
}

/// Dart's `_AmPmButton`.
#[derive(Debug)]
struct AmPmButton {
    selected: bool,
    on_pressed: Listener,
    label: String,
    padding: EdgeInsets,
    shape: OutlinedBorderRef,
}

impl StatelessWidget for AmPmButton {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut states = WidgetStates::new();
        if self.selected {
            states.insert(WidgetState::Selected);
        }
        let time_picker_theme = TimePickerModel::theme_of(app, context);
        let default_theme = TimePickerModel::default_theme_of(app, context);
        let resolved_background_color = <dyn WidgetStateProperty<Color>>::resolve_as(
            &time_picker_theme
                .day_period_color_getter()
                .unwrap_or_else(|| default_theme.day_period_color(app)),
            &states,
        );
        let resolved_text_color = <dyn WidgetStateProperty<Color>>::resolve_as(
            &time_picker_theme
                .day_period_text_color
                .clone()
                .unwrap_or_else(|| default_theme.day_period_text_color(app)),
            &states,
        );
        let resolved_text_style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            &time_picker_theme
                .day_period_text_style
                .clone()
                .unwrap_or_else(|| default_theme.day_period_text_style(app)),
            &states,
        )
        .value()
        .copy_with()
        .color(resolved_text_color.color());
        let button_text_scaler = MediaQuery::text_scaler_of(app, context).clamp(0.0, 2.0);

        let is_ios = app.platform().target_platform() == TargetPlatform::IOS;
        let mut semantics = Semantics::new()
            // Platform-specific semantics vary slightly here on iOS.
            .in_mutually_exclusive_group(true)
            .button(true);
        semantics = if is_ios {
            semantics.selected(self.selected)
        } else {
            semantics.checked(self.selected)
        };
        semantics
            .child(
                Padding::new(self.padding.into()).child(
                    Material::new()
                        .clip_behavior(Clip::AntiAlias)
                        .color(resolved_background_color.color())
                        .shape(ShapeBorderRef::from(self.shape.clone()))
                        .child(
                            InkWell::new().on_tap(self.on_pressed.clone()).child(
                                Center::new().child(
                                    Text::new(self.label.clone())
                                        .style(resolved_text_style)
                                        .text_scaler(button_text_scaler),
                                ),
                            ),
                        ),
                ),
            )
            .into_widget()
    }
}

/// Dart's `_DayPeriodInputPadding`.
#[derive(Debug)]
struct DayPeriodInputPadding {
    child: Option<WidgetRef>,
    min_size: Size,
    orientation: Orientation,
}

impl RenderObjectWidget for DayPeriodInputPadding {
    type RenderObject = RenderInputPadding;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderInputPadding::new(app, self.min_size, self.orientation, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderInputPadding>,
    ) {
        render_object.set_min_size(app, self.min_size);
        render_object.set_orientation(app, self.orientation);
    }
}

impl SingleChildRenderObjectWidget for DayPeriodInputPadding {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderInputPadding`.
struct RenderInputPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    min_size: Size,
    orientation: Orientation,
}

impl RenderInputPadding {
    fn new(
        app: &mut App,
        min_size: Size,
        orientation: Orientation,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderInputPadding> {
        let this = RenderHandle::new_box(
            app,
            RenderInputPadding {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                min_size,
                orientation,
            },
        );
        this.set_child(app, child);
        this
    }

    fn min_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).min_size
    }

    fn set_min_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if self.get(app).min_size == value {
            return;
        }
        self.get_mut(app).min_size = value;
        self.mark_needs_layout(app);
    }

    fn orientation(self: RenderHandle<Self>, app: &App) -> Orientation {
        self.get(app).orientation
    }

    fn set_orientation(self: RenderHandle<Self>, app: &mut App, value: Orientation) {
        if self.get(app).orientation == value {
            return;
        }
        self.get_mut(app).orientation = value;
        self.mark_needs_layout(app);
    }

    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        if let Some(child) = self.child(app) {
            let child_size = layout_child(app, child, constraints);
            let min_size = self.min_size(app);
            let width = child_size.width().max(min_size.width());
            let height = child_size.height().max(min_size.height());
            return constraints.constrain(Size::new(width, height));
        }
        Size::ZERO
    }
}

impl RenderObjectWithChildMixin for RenderInputPadding {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for RenderInputPadding {}

impl RenderObject for RenderInputPadding {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
        if let Some(child) = self.child(app) {
            let child_size = child.size(app);
            child.parent_data_of_mut::<BoxParentData>(app).offset =
                Alignment::CENTER.along_offset(size - child_size);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderInputPadding {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let result = child.get_dry_baseline(app, constraints, baseline)?;
        let child_size = child.get_dry_layout(app, constraints);
        let size = self.as_box().get_dry_layout(app, constraints);
        Some(result + Alignment::CENTER.along_offset(size - child_size).dy())
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if RenderBoxBase::hit_test(self, app, result, position) {
            return true;
        }

        let child = self.child(app).expect("an input padding has a child");
        let child_size = child.size(app);
        let min_size = self.min_size(app);
        if position.dx() < 0.0
            || position.dx() > child_size.width().max(min_size.width())
            || position.dy() < 0.0
            || position.dy() > child_size.height().max(min_size.height())
        {
            return false;
        }

        let mut new_position = child_size.center(Offset::ZERO);
        new_position = new_position
            + match self.orientation(app) {
                Orientation::Portrait if position.dy() > new_position.dy() => Offset::new(0.0, 1.0),
                Orientation::Landscape if position.dx() > new_position.dx() => {
                    Offset::new(1.0, 0.0)
                }
                Orientation::Portrait => Offset::new(0.0, -1.0),
                Orientation::Landscape => Offset::new(-1.0, 0.0),
            };

        result.add_with_raw_transform(
            Some(force_to_point(new_position)),
            new_position,
            |result, position| {
                debug_assert!(
                    (position.dx() - new_position.dx()).abs()
                        <= new_position.dx().abs() * f32::EPSILON as f64
                        && (position.dy() - new_position.dy()).abs()
                            <= new_position.dy().abs() * f32::EPSILON as f64,
                    "the forced transform maps every hit to the new position"
                );
                child.hit_test(app, result, new_position)
            },
        )
    }
}

/// Dart's `_TappableLabel`.
struct TappableLabel {
    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    value: i32,

    /// Whether this is the inner ring of a double-ring dial.
    inner: bool,

    /// The text painter, laid out at its natural size.
    painter: RefCell<TextPainter>,

    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    on_tap: Listener,
}

/// Dart's `_DialPainter`.
struct DialPainter {
    primary_labels: Vec<TappableLabel>,
    selected_labels: Vec<TappableLabel>,
    background_color: Color,
    hand_color: Color,
    hand_width: f64,
    dot_color: Color,
    dot_radius: f64,
    center_radius: f64,
    theta: f64,
    radius: f64,
    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    text_direction: TextDirection,
    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    selected_value: i32,
    repaint: Rc<dyn waft_foundation::Listenable>,
}

impl DialPainter {
    #[allow(clippy::too_many_arguments)]
    fn new(
        app: &mut App,
        primary_labels: Vec<TappableLabel>,
        selected_labels: Vec<TappableLabel>,
        background_color: Color,
        hand_color: Color,
        hand_width: f64,
        dot_color: Color,
        dot_radius: f64,
        center_radius: f64,
        theta: f64,
        radius: f64,
        text_direction: TextDirection,
        selected_value: i32,
    ) -> DialPainter {
        let repaint: Rc<dyn waft_foundation::Listenable> =
            Rc::new(PaintingBinding::instance(app).system_fonts(app).keep(app));
        DialPainter {
            primary_labels,
            selected_labels,
            background_color,
            hand_color,
            hand_width,
            dot_color,
            dot_radius,
            center_radius,
            theta,
            radius,
            text_direction,
            selected_value,
            repaint,
        }
    }

    /// Dart's `dispose`.
    fn dispose(&mut self) {
        for label in self.primary_labels.drain(..) {
            label.painter.borrow_mut().dispose();
        }
        for label in self.selected_labels.drain(..) {
            label.painter.borrow_mut().dispose();
        }
    }

    fn get_offset_for_theta(center: Offset, theta: f64, radius: f64) -> Offset {
        center + Offset::new(radius * theta.cos(), -radius * theta.sin())
    }

    fn paint_labels(
        app: &mut App,
        canvas: &mut Canvas,
        center: Offset,
        labels: &[&TappableLabel],
        radius: f64,
    ) {
        if labels.is_empty() {
            return;
        }
        let label_theta_increment = -K_TWO_PI / labels.len() as f64;
        let mut label_theta = PI / 2.0;

        let fonts = PaintingBinding::instance(app).fonts(app);
        for label in labels {
            let mut label_painter = label.painter.borrow_mut();
            let label_offset =
                Offset::new(-label_painter.width() / 2.0, -label_painter.height() / 2.0);
            label_painter.paint(
                app.get_mut(fonts),
                canvas,
                Self::get_offset_for_theta(center, label_theta, radius) + label_offset,
            );
            label_theta += label_theta_increment;
        }
    }

    fn paint_inner_outer_labels(
        app: &mut App,
        canvas: &mut Canvas,
        center: Offset,
        labels: &[TappableLabel],
        label_radius: f64,
        inner_label_radius: f64,
    ) {
        let outer: Vec<&TappableLabel> = labels.iter().filter(|label| !label.inner).collect();
        let inner: Vec<&TappableLabel> = labels.iter().filter(|label| label.inner).collect();
        Self::paint_labels(app, canvas, center, &outer, label_radius);
        Self::paint_labels(app, canvas, center, &inner, inner_label_radius);
    }
}

impl CustomPainter for DialPainter {
    fn repaint(&self) -> Option<&Rc<dyn waft_foundation::Listenable>> {
        Some(&self.repaint)
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        let dial_radius =
            (size.shortest_side() / 2.0).max(K_TIME_PICKER_DIAL_MIN_RADIUS + self.dot_radius);
        let label_radius =
            (dial_radius - K_TIME_PICKER_DIAL_PADDING).max(K_TIME_PICKER_DIAL_MIN_RADIUS);
        let inner_label_radius = (label_radius - K_TIME_PICKER_INNER_DIAL_OFFSET).max(0.0);
        let handle_radius = (label_radius
            - (if self.radius < 0.5 { 1.0 } else { 0.0 }) * (label_radius - inner_label_radius))
            .max(K_TIME_PICKER_DIAL_MIN_RADIUS);
        let center = Offset::new(size.width() / 2.0, size.height() / 2.0);
        let center_point = center;
        canvas.draw_circle(
            center_point,
            dial_radius as f32,
            &Paint::from_color(self.background_color.into()),
        );

        Self::paint_inner_outer_labels(
            app,
            canvas,
            center,
            &self.primary_labels,
            label_radius,
            inner_label_radius,
        );

        let selector_paint = Paint::from_color(self.hand_color.into());
        let focused_point = Self::get_offset_for_theta(center, self.theta, handle_radius);
        canvas.draw_circle(center_point, self.center_radius as f32, &selector_paint);
        canvas.draw_circle(focused_point, self.dot_radius as f32, &selector_paint);
        let mut line_paint = Paint::from_color(self.hand_color.into());
        line_paint.style =
            waft_embedder::PaintStyle::Stroke(waft_embedder::Stroke::new(self.hand_width as f32));
        draw_line(canvas, center_point, focused_point, &line_paint);

        // Add a dot inside the selector but only when it isn't over the labels.
        // This checks that the selector's theta is between two labels. A remainder
        // between 0.1 and 0.45 indicates that the selector is roughly not above any
        // labels. The values were derived by manually testing the dial.
        let label_theta_increment = -K_TWO_PI / self.primary_labels.len() as f64;
        let remainder = dart_remainder(self.theta, label_theta_increment);
        if remainder > 0.1 && remainder < 0.45 {
            let mut dot_paint = Paint::from_color(self.dot_color.into());
            dot_paint.style = waft_embedder::PaintStyle::Stroke(waft_embedder::Stroke::new(
                self.hand_width as f32,
            ));
            canvas.draw_circle(focused_point, 2.0, &dot_paint);
        }

        let mut oval = PathBuilder::new();
        oval.arc(
            focused_point,
            self.dot_radius as f32,
            0.0,
            std::f32::consts::TAU,
        );
        let focused_path = Path::new(oval.build());
        canvas.save();
        canvas.clip_path(
            focused_path.as_valo(),
            focused_path.fill_type,
            ClipOp::Intersect,
            true,
        );
        Self::paint_inner_outer_labels(
            app,
            canvas,
            center,
            &self.selected_labels,
            label_radius,
            inner_label_radius,
        );
        canvas.restore();
    }

    fn should_repaint(&self, _app: &App, _old_painter: &dyn CustomPainter) -> bool {
        // Dart compares the label lists by identity, and every build makes new ones.
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
impl DialPainter {
    /// The text of the primary labels, which the tests read through `painter.text.text`.
    fn primary_label_texts(&self) -> Vec<String> {
        Self::label_texts(&self.primary_labels)
    }

    /// The text of the selected labels.
    fn selected_label_texts(&self) -> Vec<String> {
        Self::label_texts(&self.selected_labels)
    }

    /// Whether each primary label is on the inner ring.
    fn primary_label_inner(&self) -> Vec<bool> {
        self.primary_labels
            .iter()
            .map(|label| label.inner)
            .collect()
    }

    /// Whether each selected label is on the inner ring.
    fn selected_label_inner(&self) -> Vec<bool> {
        self.selected_labels
            .iter()
            .map(|label| label.inner)
            .collect()
    }

    /// The style of the first primary label's text, which the tests read through
    /// `primaryLabels.first.painter.text.style`.
    fn first_primary_label_style(&self) -> Option<waft_painting::TextStyle> {
        Self::first_label_style(&self.primary_labels)
    }

    /// The style of the first selected label's text.
    fn first_selected_label_style(&self) -> Option<waft_painting::TextStyle> {
        Self::first_label_style(&self.selected_labels)
    }

    fn first_label_style(labels: &[TappableLabel]) -> Option<waft_painting::TextStyle> {
        labels[0]
            .painter
            .borrow()
            .text()
            .and_then(|span| span.style().cloned())
    }

    fn label_texts(labels: &[TappableLabel]) -> Vec<String> {
        labels
            .iter()
            .map(|label| label.painter.borrow_mut().plain_text().to_string())
            .collect()
    }
}

/// Dart's `%` on doubles: the result has the sign of the divisor's magnitude, never negative.
fn dart_remainder(a: f64, b: f64) -> f64 {
    a.rem_euclid(b.abs())
}

/// Dart's `_Dial`.
struct Dial {
    selected_time: TimeOfDay,
    hour_minute_mode: HourMinuteMode,
    hour_dial_type: HourDialType,
    on_changed: Option<SelectedTimeChanged>,
    on_hour_selected: Option<VoidCallback>,
}

impl std::fmt::Debug for Dial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dial")
            .field("selected_time", &self.selected_time)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Dial {
    type State = DialState;

    fn create_state(&self) -> DialState {
        DialState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            theme_data: None,
            localizations: None,
            painter: None,
            animation_controller: None,
            theta_tween: None,
            theta: None,
            radius_tween: None,
            radius: None,
            dragging: false,
            position: None,
            center: None,
            dial_size: None,
        }
    }
}

/// Dart's `_DialState`.
struct DialState {
    state: StateData<Dial>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    theme_data: Option<Rc<ThemeData>>,
    localizations: Option<Rc<dyn MaterialLocalizations>>,
    painter: Option<Rc<RefCell<DialPainter>>>,
    animation_controller: Option<Ref<AnimationController>>,
    theta_tween: Option<Ref<Tween<f64>>>,
    theta: Option<AnimationRef<f64>>,
    radius_tween: Option<Ref<Tween<f64>>>,
    radius: Option<AnimationRef<f64>>,
    dragging: bool,
    position: Option<Offset>,
    center: Option<Offset>,
    dial_size: Option<Size>,
}

impl SingleTickerProviderStateMixin for DialState {
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

impl waft_scheduler::TickerProviderObject for DialState {
    fn create_ticker(
        self: Handle<Self>,
        app: &mut App,
        on_tick: waft_scheduler::TickerCallback,
    ) -> Ref<waft_scheduler::Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl DialState {
    fn animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .animation_controller
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn theta_tween(self: Handle<Self>, app: &App) -> Handle<Tween<f64>> {
        app.get(self)
            .theta_tween
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn radius_tween(self: Handle<Self>, app: &App) -> Handle<Tween<f64>> {
        app.get(self)
            .radius_tween
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn theta_value(self: Handle<Self>, app: &App) -> f64 {
        app.get(self)
            .theta
            .as_ref()
            .expect("init_state ran")
            .value(app)
    }

    fn radius_value(self: Handle<Self>, app: &App) -> f64 {
        app.get(self)
            .radius
            .as_ref()
            .expect("init_state ran")
            .value(app)
    }

    fn handle_animation_value_changed(self: Handle<Self>, app: &mut App) {
        // _theta.value or _radius.value has changed
        self.set_state(app, |_state| {});
    }

    fn nearest(target: f64, a: f64, b: f64) -> f64 {
        if (target - a).abs() < (target - b).abs() {
            a
        } else {
            b
        }
    }

    fn animate_to(self: Handle<Self>, app: &mut App, target_theta: f64, target_radius: f64) {
        #[allow(clippy::too_many_arguments)]
        fn animate_to_value(
            app: &mut App,
            target: f64,
            animation_value: f64,
            tween: Handle<Tween<f64>>,
            controller: Handle<AnimationController>,
            min: f64,
            max: f64,
        ) {
            let mut begin_value = DialState::nearest(target, animation_value, max);
            begin_value = DialState::nearest(target, begin_value, min);
            tween.set_begin(app, Some(begin_value));
            tween.set_end(app, Some(target));
            controller.set_value(app, 0.0);
            controller.forward(app, None);
        }

        let theta_value = self.theta_value(app);
        animate_to_value(
            app,
            target_theta,
            theta_value,
            self.theta_tween(app),
            self.animation_controller(app),
            theta_value - K_TWO_PI,
            theta_value + K_TWO_PI,
        );
        let radius_value = self.radius_value(app);
        animate_to_value(
            app,
            target_radius,
            radius_value,
            self.radius_tween(app),
            self.animation_controller(app),
            0.0,
            1.0,
        );
    }

    fn get_radius_for_time(self: Handle<Self>, app: &App, time: TimeOfDay) -> f64 {
        let widget = self.widget(app);
        match widget.hour_minute_mode {
            HourMinuteMode::Hour => match widget.hour_dial_type {
                HourDialType::TwentyFourHourDoubleRing => {
                    if time.hour >= 12 {
                        0.0
                    } else {
                        1.0
                    }
                }
                HourDialType::TwentyFourHour | HourDialType::TwelveHour => 1.0,
            },
            HourMinuteMode::Minute => 1.0,
        }
    }

    fn get_theta_for_time(self: Handle<Self>, app: &App, time: TimeOfDay) -> f64 {
        let widget = self.widget(app);
        let hours_factor = match widget.hour_dial_type {
            HourDialType::TwentyFourHour => TimeOfDay::HOURS_PER_DAY,
            HourDialType::TwentyFourHourDoubleRing => TimeOfDay::HOURS_PER_PERIOD,
            HourDialType::TwelveHour => TimeOfDay::HOURS_PER_PERIOD,
        };
        let fraction = match widget.hour_minute_mode {
            HourMinuteMode::Hour => {
                (f64::from(time.hour) / f64::from(hours_factor)).rem_euclid(f64::from(hours_factor))
            }
            HourMinuteMode::Minute => (f64::from(time.minute)
                / f64::from(TimeOfDay::MINUTES_PER_HOUR))
            .rem_euclid(f64::from(TimeOfDay::MINUTES_PER_HOUR)),
        };
        (PI / 2.0 - fraction * K_TWO_PI).rem_euclid(K_TWO_PI)
    }

    fn get_time_for_theta(
        self: Handle<Self>,
        app: &App,
        theta: f64,
        round_minutes: bool,
        radius: f64,
    ) -> TimeOfDay {
        let widget = self.widget(app);
        let fraction = (0.25 - theta.rem_euclid(K_TWO_PI) / K_TWO_PI).rem_euclid(1.0);
        match widget.hour_minute_mode {
            HourMinuteMode::Hour => {
                let new_hour = match widget.hour_dial_type {
                    HourDialType::TwentyFourHour => {
                        dart_round(fraction * f64::from(TimeOfDay::HOURS_PER_DAY))
                            .rem_euclid(TimeOfDay::HOURS_PER_DAY)
                    }
                    HourDialType::TwentyFourHourDoubleRing => {
                        let mut new_hour =
                            dart_round(fraction * f64::from(TimeOfDay::HOURS_PER_PERIOD))
                                .rem_euclid(TimeOfDay::HOURS_PER_PERIOD);
                        if radius < 0.5 {
                            new_hour += TimeOfDay::HOURS_PER_PERIOD;
                        }
                        new_hour
                    }
                    HourDialType::TwelveHour => {
                        let new_hour =
                            dart_round(fraction * f64::from(TimeOfDay::HOURS_PER_PERIOD))
                                .rem_euclid(TimeOfDay::HOURS_PER_PERIOD);
                        new_hour + widget.selected_time.period_offset()
                    }
                };
                widget.selected_time.replacing(Some(new_hour), None)
            }
            HourMinuteMode::Minute => {
                let mut minute = dart_round(fraction * f64::from(TimeOfDay::MINUTES_PER_HOUR))
                    .rem_euclid(TimeOfDay::MINUTES_PER_HOUR);
                if round_minutes {
                    // Round the minutes to nearest 5 minute interval.
                    minute = ((minute + 2) / 5 * 5).rem_euclid(TimeOfDay::MINUTES_PER_HOUR);
                }
                widget.selected_time.replacing(None, Some(minute))
            }
        }
    }

    fn notify_on_changed_if_needed(
        self: Handle<Self>,
        app: &mut App,
        round_minutes: bool,
    ) -> TimeOfDay {
        let current = self.get_time_for_theta(
            app,
            self.theta_value(app),
            round_minutes,
            self.radius_value(app),
        );
        let widget = self.widget(app);
        let Some(on_changed) = widget.on_changed.clone() else {
            return current;
        };
        let selected_time = widget.selected_time;
        if current != selected_time {
            on_changed(app, current);
        }
        current
    }

    fn update_theta_for_pan(self: Handle<Self>, app: &mut App, round_minutes: bool) {
        self.set_state(app, |_state| {});
        let state = app.get(self);
        let offset = state.position.expect("a pan has a position")
            - state.center.expect("a pan has a center");
        let label_radius = state
            .dial_size
            .expect("a pan has a dial size")
            .shortest_side()
            / 2.0
            - K_TIME_PICKER_DIAL_PADDING;
        let inner_radius = label_radius - K_TIME_PICKER_INNER_DIAL_OFFSET;
        let mut angle = (offset.dx().atan2(offset.dy()) - PI / 2.0).rem_euclid(K_TWO_PI);
        let radius =
            ((offset.distance() - inner_radius) / K_TIME_PICKER_INNER_DIAL_OFFSET).clamp(0.0, 1.0);
        if round_minutes {
            let time = self.get_time_for_theta(app, angle, round_minutes, radius);
            angle = self.get_theta_for_time(app, time);
        }
        // The controller doesn't animate during the pan gesture.
        self.theta_tween(app).set_begin(app, Some(angle));
        self.theta_tween(app).set_end(app, Some(angle));
        self.radius_tween(app).set_begin(app, Some(radius));
        self.radius_tween(app).set_end(app, Some(radius));
    }

    fn handle_pan_start(
        self: Handle<Self>,
        app: &mut App,
        details: &waft_gestures::DragStartDetails,
    ) {
        debug_assert!(!app.get(self).dragging);
        app.get_mut(self).dragging = true;
        let context = self.context(app);
        let render_box = context
            .find_render_object(app)
            .expect("the dial has a render object")
            .as_box()
            .expect("the dial is a box");
        let position = render_box.global_to_local(app, details.global_position, None);
        let size = render_box.size(app);
        {
            let state = app.get_mut(self);
            state.position = Some(position);
            state.dial_size = Some(size);
            state.center = Some(size.center(Offset::ZERO));
        }
        self.update_theta_for_pan(app, false);
        self.notify_on_changed_if_needed(app, false);
    }

    fn handle_pan_update(
        self: Handle<Self>,
        app: &mut App,
        details: &waft_gestures::DragUpdateDetails,
    ) {
        let state = app.get_mut(self);
        state.position = Some(state.position.expect("a pan has a position") + details.delta);
        self.update_theta_for_pan(app, false);
        self.notify_on_changed_if_needed(app, false);
    }

    fn handle_pan_end(self: Handle<Self>, app: &mut App, _details: &waft_gestures::DragEndDetails) {
        debug_assert!(app.get(self).dragging);
        {
            let state = app.get_mut(self);
            state.dragging = false;
            state.position = None;
            state.center = None;
            state.dial_size = None;
        }
        let selected_time = self.widget(app).selected_time;
        self.animate_to(
            app,
            self.get_theta_for_time(app, selected_time),
            self.get_radius_for_time(app, selected_time),
        );
        let widget = self.widget(app);
        if widget.hour_minute_mode == HourMinuteMode::Hour
            && let Some(on_hour_selected) = widget.on_hour_selected.clone()
        {
            on_hour_selected(app);
        }
    }

    fn handle_tap_up(self: Handle<Self>, app: &mut App, details: &waft_gestures::TapUpDetails) {
        let context = self.context(app);
        let render_box = context
            .find_render_object(app)
            .expect("the dial has a render object")
            .as_box()
            .expect("the dial is a box");
        let position = render_box.global_to_local(app, details.global_position, None);
        let size = render_box.size(app);
        {
            let state = app.get_mut(self);
            state.position = Some(position);
            state.center = Some(size.center(Offset::ZERO));
            state.dial_size = Some(size);
        }
        self.update_theta_for_pan(app, true);
        self.notify_on_changed_if_needed(app, true);
        let widget = self.widget(app);
        if widget.hour_minute_mode == HourMinuteMode::Hour
            && let Some(on_hour_selected) = widget.on_hour_selected.clone()
        {
            on_hour_selected(app);
        }
        let time =
            self.get_time_for_theta(app, self.theta_value(app), true, self.radius_value(app));
        self.animate_to(
            app,
            self.get_theta_for_time(app, time),
            self.get_radius_for_time(app, time),
        );
        let state = app.get_mut(self);
        state.dragging = false;
        state.position = None;
        state.center = None;
        state.dial_size = None;
    }

    fn select_hour(self: Handle<Self>, app: &mut App, hour: i32) {
        let widget = self.widget(app);
        let selected_time = widget.selected_time;
        let get_am_pm_time = || match selected_time.period() {
            DayPeriod::Am => TimeOfDay::new(hour, selected_time.minute),
            DayPeriod::Pm => {
                TimeOfDay::new(hour + TimeOfDay::HOURS_PER_PERIOD, selected_time.minute)
            }
        };
        let time = match widget.hour_minute_mode {
            HourMinuteMode::Hour => match widget.hour_dial_type {
                HourDialType::TwentyFourHour | HourDialType::TwentyFourHourDoubleRing => {
                    TimeOfDay::new(hour, selected_time.minute)
                }
                HourDialType::TwelveHour => get_am_pm_time(),
            },
            HourMinuteMode::Minute => get_am_pm_time(),
        };
        let angle = self.get_theta_for_time(app, time);
        self.theta_tween(app).set_begin(app, Some(angle));
        self.theta_tween(app).set_end(app, Some(angle));
        self.notify_on_changed_if_needed(app, false);
    }

    fn select_minute(self: Handle<Self>, app: &mut App, minute: i32) {
        let time = TimeOfDay::new(self.widget(app).selected_time.hour, minute);
        let angle = self.get_theta_for_time(app, time);
        self.theta_tween(app).set_begin(app, Some(angle));
        self.theta_tween(app).set_end(app, Some(angle));
        self.notify_on_changed_if_needed(app, false);
    }

    #[allow(clippy::too_many_arguments)]
    fn build_tappable_label(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        text_style: &TextStyle,
        value: i32,
        inner: bool,
        label: String,
        on_tap: Listener,
    ) -> TappableLabel {
        let mut painter = TextPainter::new();
        painter.set_text(Some(Rc::new(
            TextSpan::new().style(text_style.clone()).text(label),
        )));
        painter.set_text_direction(Some(TextDirection::Ltr));
        painter.set_text_scaler(MediaQuery::text_scaler_of(app, context).clamp(0.0, 2.0));
        let fonts = PaintingBinding::instance(app).fonts(app);
        painter.layout(app.get_mut(fonts), 0.0, f64::INFINITY);
        TappableLabel {
            value,
            inner,
            painter: RefCell::new(painter),
            on_tap,
        }
    }

    fn select_hour_listener(self: Handle<Self>, hour: i32) -> Listener {
        Listener::new(move |app| self.select_hour(app, hour))
    }

    fn select_minute_listener(self: Handle<Self>, minute: i32) -> Listener {
        Listener::new(move |app| self.select_minute(app, minute))
    }

    fn build_24_hour_ring(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        text_style: &TextStyle,
    ) -> Vec<TappableLabel> {
        let theme_data = Rc::clone(app.get(self).theme_data.as_ref().expect("dependencies ran"));
        let localizations = Rc::clone(
            app.get(self)
                .localizations
                .as_ref()
                .expect("dependencies ran"),
        );
        let mut labels = Vec::new();
        if theme_data.use_material3 {
            for hour in 0..24 {
                let time_of_day = TimeOfDay::new(hour, 0);
                let label =
                    // The M3 specs for 24-hour ring show 0 hour as 00, but for 1-9,
                    // the specs show single digit.
                    if time_of_day.hour != 0 {
                        localizations.format_decimal(i64::from(time_of_day.hour))
                    } else {
                        localizations.format_hour(&time_of_day, true)
                    };
                labels.push(self.build_tappable_label(
                    app,
                    context,
                    text_style,
                    time_of_day.hour,
                    time_of_day.hour >= 12,
                    label,
                    self.select_hour_listener(time_of_day.hour),
                ));
            }
        } else {
            // On M2, there's no inner ring of numbers.
            for hour in (0..24).step_by(2) {
                let time_of_day = TimeOfDay::new(hour, 0);
                labels.push(self.build_tappable_label(
                    app,
                    context,
                    text_style,
                    time_of_day.hour,
                    false,
                    localizations.format_hour(&time_of_day, true),
                    self.select_hour_listener(time_of_day.hour),
                ));
            }
        }
        labels
    }

    fn build_12_hour_ring(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        text_style: &TextStyle,
    ) -> Vec<TappableLabel> {
        let localizations = Rc::clone(
            app.get(self)
                .localizations
                .as_ref()
                .expect("dependencies ran"),
        );
        let always_use_24_hour_format = MediaQuery::always_use_24_hour_format_of(app, context);
        // 12, then 1 through 11.
        (0..12)
            .map(|index| {
                let time_of_day = TimeOfDay::new(if index == 0 { 12 } else { index }, 0);
                self.build_tappable_label(
                    app,
                    context,
                    text_style,
                    time_of_day.hour,
                    false,
                    localizations.format_hour(&time_of_day, always_use_24_hour_format),
                    self.select_hour_listener(time_of_day.hour),
                )
            })
            .collect()
    }

    fn build_minutes(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        text_style: &TextStyle,
    ) -> Vec<TappableLabel> {
        let localizations = Rc::clone(
            app.get(self)
                .localizations
                .as_ref()
                .expect("dependencies ran"),
        );
        (0..12)
            .map(|index| {
                let time_of_day = TimeOfDay::new(0, index * 5);
                self.build_tappable_label(
                    app,
                    context,
                    text_style,
                    time_of_day.minute,
                    false,
                    localizations.format_minute(&time_of_day),
                    self.select_minute_listener(time_of_day.minute),
                )
            })
            .collect()
    }
}

/// Dart's `double.round()`: half away from zero.
fn dart_round(value: f64) -> i32 {
    value.round() as i32
}

impl State for DialState {
    type Widget = Dial;
    waft_widgets::state_accessors!();

    #[allow(deprecated)]
    fn init_state(self: Handle<Self>, app: &mut App) {
        let animation_controller = AnimationController::new(
            app,
            None,
            Some(K_DIAL_ANIMATE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let selected_time = self.widget(app).selected_time;
        let theta_tween = Tween::new(app, Some(self.get_theta_for_time(app, selected_time)), None);
        let radius_tween = Tween::new(
            app,
            Some(self.get_radius_for_time(app, selected_time)),
            None,
        );
        let eased = CurveTween::new(app, standard_easing());
        let eased = animation_controller.drive(app, eased);
        let theta = eased.drive(app, theta_tween.clone());
        theta.add_listener(
            app,
            Listener::handle_method(self, Self::handle_animation_value_changed),
        );
        let eased = CurveTween::new(app, standard_easing());
        let eased = animation_controller.drive(app, eased);
        let radius = eased.drive(app, radius_tween.clone());
        radius.add_listener(
            app,
            Listener::handle_method(self, Self::handle_animation_value_changed),
        );
        let state = app.get_mut(self);
        state.animation_controller = Some(animation_controller);
        state.theta_tween = Some(theta_tween);
        state.theta = Some(theta);
        state.radius_tween = Some(radius_tween);
        state.radius = Some(radius);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        waft_widgets::debug_check_has_media_query(app, context);
        let theme_data = Theme::of(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let state = app.get_mut(self);
        state.theme_data = Some(theme_data);
        state.localizations = Some(localizations);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Dial) {
        let widget = self.widget(app);
        if widget.hour_minute_mode != old_widget.hour_minute_mode
            || widget.selected_time != old_widget.selected_time
        {
            let selected_time = widget.selected_time;
            if !app.get(self).dragging {
                self.animate_to(
                    app,
                    self.get_theta_for_time(app, selected_time),
                    self.get_radius_for_time(app, selected_time),
                );
            }
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.animation_controller(app).dispose(app);
        if let Some(painter) = app.get_mut(self).painter.take() {
            painter.borrow_mut().dispose();
        }
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_dial_time_picker_entry_mode(app, context));
        let theme = Theme::of(app, context);
        let time_picker_theme = TimePickerTheme::of(app, context);
        let default_theme: Rc<dyn TimePickerDefaults> = if theme.use_material3 {
            Rc::new(TimePickerDefaultsM3::new(
                context,
                TimePickerEntryMode::Dial,
            ))
        } else {
            Rc::new(TimePickerDefaultsM2::new(context))
        };
        let background_color = time_picker_theme
            .dial_background_color
            .clone()
            .unwrap_or_else(|| default_theme.dial_background_color(app))
            .color();
        let dial_hand_color = time_picker_theme
            .dial_hand_color
            .clone()
            .unwrap_or_else(|| default_theme.dial_hand_color(app))
            .color();
        let label_style = time_picker_theme
            .dial_text_style
            .clone()
            .unwrap_or_else(|| default_theme.dial_text_style(app))
            .value()
            .clone();
        let dial_text_color = time_picker_theme
            .dial_text_color
            .clone()
            .unwrap_or_else(|| default_theme.dial_text_color(app));
        let dial_text_unselected_color =
            <dyn WidgetStateProperty<Color>>::resolve_as(&dial_text_color, &WidgetStates::new())
                .color();
        let mut selected_states = WidgetStates::new();
        selected_states.insert(WidgetState::Selected);
        let dial_text_selected_color =
            <dyn WidgetStateProperty<Color>>::resolve_as(&dial_text_color, &selected_states)
                .color();
        let resolved_unselected_label_style =
            label_style.copy_with().color(dial_text_unselected_color);
        let resolved_selected_label_style = label_style.copy_with().color(dial_text_selected_color);
        let dot_color = dial_text_selected_color;

        let widget = self.widget(app);
        let (hour_minute_mode, hour_dial_type, selected_time) = (
            widget.hour_minute_mode,
            widget.hour_dial_type,
            widget.selected_time,
        );
        let (primary_labels, selected_labels, selected_dial_value, radius_value) =
            match hour_minute_mode {
                HourMinuteMode::Hour => match hour_dial_type {
                    HourDialType::TwentyFourHour | HourDialType::TwentyFourHourDoubleRing => (
                        self.build_24_hour_ring(app, context, &resolved_unselected_label_style),
                        self.build_24_hour_ring(app, context, &resolved_selected_label_style),
                        selected_time.hour,
                        if theme.use_material3 {
                            self.radius_value(app)
                        } else {
                            1.0
                        },
                    ),
                    HourDialType::TwelveHour => (
                        self.build_12_hour_ring(app, context, &resolved_unselected_label_style),
                        self.build_12_hour_ring(app, context, &resolved_selected_label_style),
                        selected_time.hour_of_period(),
                        1.0,
                    ),
                },
                HourMinuteMode::Minute => (
                    self.build_minutes(app, context, &resolved_unselected_label_style),
                    self.build_minutes(app, context, &resolved_selected_label_style),
                    selected_time.minute,
                    1.0,
                ),
            };
        if let Some(painter) = app.get_mut(self).painter.take() {
            painter.borrow_mut().dispose();
        }
        let (hand_width, dot_radius, center_radius) = (
            default_theme.hand_width(app),
            default_theme.dot_radius(app),
            default_theme.center_radius(app),
        );
        let theta = self.theta_value(app);
        let text_direction = Directionality::of(app, context);
        let painter = Rc::new(RefCell::new(DialPainter::new(
            app,
            primary_labels,
            selected_labels,
            background_color,
            dial_hand_color,
            hand_width,
            dot_color,
            dot_radius,
            center_radius,
            theta,
            radius_value,
            text_direction,
            selected_dial_value,
        )));
        app.get_mut(self).painter = Some(Rc::clone(&painter));

        GestureDetector::new()
            .on_pan_start(Rc::new(move |app, details| {
                self.handle_pan_start(app, &details)
            }))
            .on_pan_update(Rc::new(move |app, details| {
                self.handle_pan_update(app, &details)
            }))
            .on_pan_end(Rc::new(move |app, details| {
                self.handle_pan_end(app, &details)
            }))
            .on_tap_up(Rc::new(move |app, details| {
                self.handle_tap_up(app, &details)
            }))
            .child(CustomPaint::new().painter(SharedDialPainter(painter)))
            .into_widget()
    }
}

/// The painter object the `CustomPaint` holds, which `_DialState` also keeps to dispose.
struct SharedDialPainter(Rc<RefCell<DialPainter>>);

impl CustomPainter for SharedDialPainter {
    fn repaint(&self) -> Option<&Rc<dyn waft_foundation::Listenable>> {
        None
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        self.0.borrow().paint(app, canvas, size);
    }

    fn should_repaint(&self, app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<SharedDialPainter>()
            .expect("Dart's covariant parameter");
        let old_painter = old_painter.0.borrow();
        self.0.borrow().should_repaint(app, &*old_painter)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `_TimePickerInput`.
#[derive(Clone, Debug)]
struct TimePickerInput {
    initial_selected_time: TimeOfDay,
    error_invalid_text: Option<String>,
    hour_label_text: Option<String>,
    minute_label_text: Option<String>,
    help_text: String,
    autofocus_hour: Option<bool>,
    autofocus_minute: Option<bool>,
    restoration_id: Option<String>,
    empty_initial_time: bool,
}

impl StatefulWidget for TimePickerInput {
    type State = TimePickerInputState;

    fn create_state(&self) -> TimePickerInputState {
        TimePickerInputState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            selected_time: None,
            hour_has_error: None,
            minute_has_error: None,
            cached_numbers: None,
            cached_locale: None,
        }
    }
}

/// Dart's `_TimePickerInputState`.
struct TimePickerInputState {
    state: StateData<TimePickerInput>,
    restoration: RestorationMixinData,
    selected_time: Option<Ref<RestorableTimeOfDay>>,
    hour_has_error: Option<Ref<RestorableBool>>,
    minute_has_error: Option<Ref<RestorableBool>>,
    cached_numbers: Option<HashMap<String, i32>>,
    cached_locale: Option<Locale>,
}

impl TimePickerInputState {
    fn selected_time(self: Handle<Self>, app: &App) -> Handle<RestorableTimeOfDay> {
        app.get(self)
            .selected_time
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn hour_has_error(self: Handle<Self>, app: &App) -> Handle<RestorableBool> {
        app.get(self)
            .hour_has_error
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn minute_has_error(self: Handle<Self>, app: &App) -> Handle<RestorableBool> {
        app.get(self)
            .minute_has_error
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn selected_time_value(self: Handle<Self>, app: &App) -> TimeOfDay {
        *self.selected_time(app).value(app)
    }

    fn parse_hour(self: Handle<Self>, app: &mut App, value: Option<&str>) -> Option<i32> {
        let value = value?;
        let mut new_hour = self.try_parse_localized_number(app, value)?;

        let context = self.context(app);
        if TimePickerModel::hour_dial_type_of(app, context) != HourDialType::TwelveHour {
            if (0..24).contains(&new_hour) {
                return Some(new_hour);
            }
        } else if new_hour > 0 && new_hour < 13 {
            let period = self.selected_time_value(app).period();
            if (period == DayPeriod::Pm && new_hour != 12)
                || (period == DayPeriod::Am && new_hour == 12)
            {
                new_hour =
                    (new_hour + TimeOfDay::HOURS_PER_PERIOD).rem_euclid(TimeOfDay::HOURS_PER_DAY);
            }
            return Some(new_hour);
        }
        None
    }

    fn parse_minute(self: Handle<Self>, app: &mut App, value: Option<&str>) -> Option<i32> {
        let value = value?;
        let new_minute = self.try_parse_localized_number(app, value)?;

        if (0..60).contains(&new_minute) {
            return Some(new_minute);
        }
        None
    }

    fn try_parse_localized_number(self: Handle<Self>, app: &mut App, number: &str) -> Option<i32> {
        if let Ok(parsed_number) = number.parse::<i32>() {
            return Some(parsed_number);
        }

        if app.get(self).cached_numbers.is_none() {
            let numbers = self.generate_localized_numbers(app);
            app.get_mut(self).cached_numbers = Some(numbers);
        }

        let trimmed_number = self.remove_leading_zeros(app, number);

        app.get(self)
            .cached_numbers
            .as_ref()
            .and_then(|numbers| numbers.get(&trimmed_number).copied())
    }

    fn generate_localized_numbers(self: Handle<Self>, app: &mut App) -> HashMap<String, i32> {
        let context = self.context(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);

        // Supports 0 - 59.
        (0..60)
            .map(|i| (localizations.format_decimal(i64::from(i)), i))
            .collect()
    }

    fn remove_leading_zeros(self: Handle<Self>, app: &mut App, value: &str) -> String {
        let context = self.context(app);
        let localized_zero = <dyn MaterialLocalizations>::of(app, context).format_decimal(0);
        let mut value = value;
        while value.starts_with(&localized_zero) && value.len() > localized_zero.len() {
            value = &value[localized_zero.len()..];
        }
        value.to_string()
    }

    fn handle_hour_saved_submitted(self: Handle<Self>, app: &mut App, value: Option<&str>) {
        let new_hour = self.parse_hour(app, value);
        if let Some(new_hour) = new_hour {
            let minute = self.selected_time_value(app).minute;
            self.selected_time(app)
                .set_value(app, TimeOfDay::new(new_hour, minute));
            let context = self.context(app);
            let selected_time = self.selected_time_value(app);
            TimePickerModel::set_selected_time(app, context, selected_time);
            FocusScope::of(app, context, false).request_focus(app, None);
        }
    }

    fn handle_hour_changed(self: Handle<Self>, app: &mut App, value: &str) {
        let new_hour = self.parse_hour(app, Some(value));
        if new_hour.is_some() && value.chars().count() == 2 {
            // If a valid hour is typed, move focus to the minute TextField.
            let context = self.context(app);
            FocusScope::of(app, context, false)
                .as_node()
                .next_focus(app);
        }
    }

    fn handle_minute_saved_submitted(self: Handle<Self>, app: &mut App, value: Option<&str>) {
        let new_minute = self.parse_minute(app, value);
        if let Some(new_minute) = new_minute {
            let hour = self.selected_time_value(app).hour;
            self.selected_time(app)
                .set_value(app, TimeOfDay::new(hour, new_minute));
            let context = self.context(app);
            let selected_time = self.selected_time_value(app);
            TimePickerModel::set_selected_time(app, context, selected_time);
            FocusScope::of(app, context, false).unfocus(app, UnfocusDisposition::Scope);
        }
    }

    fn handle_day_period_changed(self: Handle<Self>, app: &mut App, value: TimeOfDay) {
        self.selected_time(app).set_value(app, value);
        let context = self.context(app);
        TimePickerModel::set_selected_time(app, context, value);
    }

    fn validate_hour(self: Handle<Self>, app: &mut App, value: Option<&str>) -> Option<String> {
        let new_hour = self.parse_hour(app, value);
        self.set_state(app, |_state| {});
        self.hour_has_error(app).set_value(app, new_hour.is_none());
        // This is used as the validator for the [TextFormField].
        // Returning an empty string allows the field to go into an error state.
        // Returning null means no error in the validation of the entered text.
        new_hour.is_none().then(String::new)
    }

    fn validate_minute(self: Handle<Self>, app: &mut App, value: Option<&str>) -> Option<String> {
        let new_minute = self.parse_minute(app, value);
        self.set_state(app, |_state| {});
        self.minute_has_error(app)
            .set_value(app, new_minute.is_none());
        // This is used as the validator for the [TextFormField].
        // Returning an empty string allows the field to go into an error state.
        // Returning null means no error in the validation of the entered text.
        new_minute.is_none().then(String::new)
    }
}

impl RestorationMixin for TimePickerInputState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<waft_widgets::RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let selected_time = self.selected_time(app);
        let hour_has_error = self.hour_has_error(app);
        let minute_has_error = self.minute_has_error(app);
        self.register_for_restoration(app, selected_time.as_property(), "selected_time");
        self.register_for_restoration(app, hour_has_error.as_property(), "hour_has_error");
        self.register_for_restoration(app, minute_has_error.as_property(), "minute_has_error");
    }
}

impl State for TimePickerInputState {
    type Widget = TimePickerInput;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let initial_selected_time = self.widget(app).initial_selected_time;
        let selected_time = RestorableTimeOfDay::new(app, initial_selected_time);
        let hour_has_error = RestorableBool::new(app, false);
        let minute_has_error = RestorableBool::new(app, false);
        let state = app.get_mut(self);
        state.selected_time = Some(selected_time);
        state.hour_has_error = Some(hour_has_error);
        state.minute_has_error = Some(minute_has_error);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
        let context = self.context(app);
        let locale = Localizations::locale_of(app, context);
        if app.get(self).cached_locale.as_ref() != Some(&locale) {
            let state = app.get_mut(self);
            state.cached_numbers = None;
            state.cached_locale = Some(locale);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &TimePickerInput) {
        self.did_update_restoration_id(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.selected_time(app).dispose(app);
        self.hour_has_error(app).dispose(app);
        self.minute_has_error(app).dispose(app);
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        waft_widgets::debug_check_has_media_query(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let time_of_day_format =
            localizations.time_of_day_format(TimePickerModel::use_24_hour_format_of(app, context));
        let use_24_hour_dials = hour_format(time_of_day_format) != HourFormat::h;
        let theme = Theme::of(app, context);
        let time_picker_theme = TimePickerModel::theme_of(app, context);
        let default_theme = TimePickerModel::default_theme_of(app, context);
        let hour_minute_style = time_picker_theme
            .hour_minute_text_style
            .clone()
            .unwrap_or_else(|| default_theme.hour_minute_text_style(app));

        let min_interactive_vertical_padding = (2.0 * K_MIN_INTERACTIVE_DIMENSION
            - default_theme.day_period_input_size(app).height())
        .max(0.0);

        let widget = self.widget(app).clone();
        let use_material3 = TimePickerModel::use_material3_of(app, context);
        let help_text_style = time_picker_theme
            .help_text_style
            .clone()
            .unwrap_or_else(|| default_theme.help_text_style(app))
            .value()
            .clone();
        let hour_has_error = *self.hour_has_error(app).value(app);
        let minute_has_error = *self.minute_has_error(app).value(app);
        let selected_time = self.selected_time_value(app);
        let day_period_control = move || {
            DayPeriodControl {
                on_period_changed: Some(Rc::new(move |app: &mut App, value: TimeOfDay| {
                    self.handle_day_period_changed(app, value);
                })),
            }
            .into_widget()
        };

        let mut top_row_children: Vec<WidgetRef> = Vec::new();
        if !use_24_hour_dials && time_of_day_format == TimeOfDayFormat::a_space_h_colon_mm {
            top_row_children.push(
                Padding::new(EdgeInsetsDirectional::only(0.0, 0.0, 12.0, 0.0).into())
                    .child(day_period_control())
                    .into_widget(),
            );
        }
        let error_free = !hour_has_error && !minute_has_error;
        let label_text_style = theme.text_theme.body_small.clone();
        let hour_column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children(
                [Padding::new(EdgeInsets::only(0.0, 0.0, 0.0, 10.0).into())
                    .child(HourTextField {
                        restoration_id: Some("hour_text_field".to_string()),
                        selected_time,
                        style: hour_minute_style.clone(),
                        autofocus: widget.autofocus_hour,
                        input_action: TextInputAction::Next,
                        validator: Rc::new(move |app, value| self.validate_hour(app, value)),
                        on_saved_submitted: Rc::new(move |app, value| {
                            self.handle_hour_saved_submitted(app, value);
                        }),
                        on_changed: Rc::new(move |app, value| self.handle_hour_changed(app, value)),
                        hour_label_text: widget.hour_label_text.clone(),
                        empty_initial_time: widget.empty_initial_time,
                    })
                    .into_widget()]
                .into_iter()
                .chain(error_free.then(|| {
                    let mut text = Text::new(
                        widget
                            .hour_label_text
                            .clone()
                            .unwrap_or_else(|| localizations.time_picker_hour_label()),
                    )
                    .max_lines(1)
                    .overflow(TextOverflow::Ellipsis);
                    if let Some(style) = label_text_style.clone() {
                        text = text.style(style);
                    }
                    ExcludeSemantics::new().child(text).into_widget()
                })),
            );
        let minute_column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children(
                [Padding::new(EdgeInsets::only(0.0, 0.0, 0.0, 10.0).into())
                    .child(MinuteTextField {
                        restoration_id: Some("minute_text_field".to_string()),
                        selected_time,
                        style: hour_minute_style,
                        autofocus: widget.autofocus_minute,
                        input_action: TextInputAction::Done,
                        validator: Rc::new(move |app, value| self.validate_minute(app, value)),
                        on_saved_submitted: Rc::new(move |app, value| {
                            self.handle_minute_saved_submitted(app, value);
                        }),
                        minute_label_text: widget.minute_label_text.clone(),
                        empty_initial_time: widget.empty_initial_time,
                    })
                    .into_widget()]
                .into_iter()
                .chain(error_free.then(|| {
                    let mut text = Text::new(
                        widget
                            .minute_label_text
                            .clone()
                            .unwrap_or_else(|| localizations.time_picker_minute_label()),
                    )
                    .max_lines(1)
                    .overflow(TextOverflow::Ellipsis);
                    if let Some(style) = label_text_style.clone() {
                        text = text.style(style);
                    }
                    ExcludeSemantics::new().child(text).into_widget()
                })),
            );
        top_row_children.push(
            Expanded::new(
                Padding::new(
                    EdgeInsetsDirectional::only(
                        0.0,
                        min_interactive_vertical_padding / 2.0,
                        0.0,
                        0.0,
                    )
                    .into(),
                )
                .child(
                    Row::new()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        // Hour/minutes should not change positions in RTL locales.
                        .text_direction(waft_painting::TextDirection::Ltr)
                        .children([
                            Expanded::new(hour_column).into_widget(),
                            TimeSelectorSeparator { time_of_day_format }.into_widget(),
                            Expanded::new(minute_column).into_widget(),
                        ]),
                ),
            )
            .into_widget(),
        );
        if !use_24_hour_dials && time_of_day_format != TimeOfDayFormat::a_space_h_colon_mm {
            top_row_children.push(
                Padding::new(EdgeInsetsDirectional::only(12.0, 0.0, 0.0, 0.0).into())
                    .child(day_period_control())
                    .into_widget(),
            );
        }

        Padding::new(if use_material3 {
            EdgeInsetsGeometry::from(EdgeInsets::ZERO)
        } else {
            EdgeInsets::symmetric(0.0, 16.0).into()
        })
        .child(
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children([
                    Padding::new(
                        EdgeInsetsDirectional::only(
                            0.0,
                            0.0,
                            0.0,
                            (if use_material3 { 20.0 } else { 24.0 })
                                - min_interactive_vertical_padding / 2.0,
                        )
                        .into(),
                    )
                    .child(Text::new(widget.help_text.clone()).style(help_text_style))
                    .into_widget(),
                    Row::new()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .children(top_row_children)
                        .into_widget(),
                    if hour_has_error || minute_has_error {
                        let mut error_style: TextStyle = theme
                            .text_theme
                            .body_medium
                            .clone()
                            .expect("the text theme has bodyMedium");
                        error_style = error_style.copy_with().color(theme.color_scheme.error);
                        Text::new(
                            widget
                                .error_invalid_text
                                .clone()
                                .unwrap_or_else(|| localizations.invalid_time_label()),
                        )
                        .style(error_style)
                        .into_widget()
                    } else {
                        SizedBox::new().height(2.0).into_widget()
                    },
                ]),
        )
        .into_widget()
    }
}

type FormFieldValidator = Rc<dyn Fn(&mut App, Option<&str>) -> Option<String>>;
type SavedSubmitted = Rc<dyn Fn(&mut App, Option<&str>)>;

/// Dart's `_HourTextField`.
struct HourTextField {
    selected_time: TimeOfDay,
    style: waft_painting::AnyTextStyle,
    autofocus: Option<bool>,
    input_action: TextInputAction,
    validator: FormFieldValidator,
    on_saved_submitted: SavedSubmitted,
    on_changed: StringChanged,
    hour_label_text: Option<String>,
    restoration_id: Option<String>,
    empty_initial_time: bool,
}

impl std::fmt::Debug for HourTextField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HourTextField").finish_non_exhaustive()
    }
}

impl StatelessWidget for HourTextField {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        HourMinuteTextField {
            restoration_id: self.restoration_id.clone(),
            selected_time: self.selected_time,
            is_hour: true,
            autofocus: self.autofocus,
            input_action: self.input_action,
            style: self.style.clone(),
            semantic_hint_text: self.hour_label_text.clone().unwrap_or_else(|| {
                <dyn MaterialLocalizations>::of(app, context).time_picker_hour_label()
            }),
            validator: Rc::clone(&self.validator),
            on_saved_submitted: Rc::clone(&self.on_saved_submitted),
            empty_initial_time: self.empty_initial_time,
            on_changed: Some(Rc::clone(&self.on_changed)),
        }
        .into_widget()
    }
}

/// Dart's `_MinuteTextField`.
struct MinuteTextField {
    selected_time: TimeOfDay,
    style: waft_painting::AnyTextStyle,
    autofocus: Option<bool>,
    input_action: TextInputAction,
    validator: FormFieldValidator,
    on_saved_submitted: SavedSubmitted,
    minute_label_text: Option<String>,
    restoration_id: Option<String>,
    empty_initial_time: bool,
}

impl std::fmt::Debug for MinuteTextField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinuteTextField").finish_non_exhaustive()
    }
}

impl StatelessWidget for MinuteTextField {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        HourMinuteTextField {
            restoration_id: self.restoration_id.clone(),
            selected_time: self.selected_time,
            is_hour: false,
            autofocus: self.autofocus,
            input_action: self.input_action,
            style: self.style.clone(),
            semantic_hint_text: self.minute_label_text.clone().unwrap_or_else(|| {
                <dyn MaterialLocalizations>::of(app, context).time_picker_minute_label()
            }),
            validator: Rc::clone(&self.validator),
            empty_initial_time: self.empty_initial_time,
            on_saved_submitted: Rc::clone(&self.on_saved_submitted),
            on_changed: None,
        }
        .into_widget()
    }
}

/// Dart's `_HourMinuteTextField`.
#[derive(Clone)]
struct HourMinuteTextField {
    selected_time: TimeOfDay,
    is_hour: bool,
    autofocus: Option<bool>,
    input_action: TextInputAction,
    style: waft_painting::AnyTextStyle,
    semantic_hint_text: String,
    validator: FormFieldValidator,
    on_saved_submitted: SavedSubmitted,
    on_changed: Option<StringChanged>,
    restoration_id: Option<String>,
    empty_initial_time: bool,
}

impl std::fmt::Debug for HourMinuteTextField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HourMinuteTextField")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for HourMinuteTextField {
    type State = HourMinuteTextFieldState;

    fn create_state(&self) -> HourMinuteTextFieldState {
        HourMinuteTextFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            controller: None,
            controller_has_been_set: None,
            focus_node: None,
        }
    }
}

/// Dart's `_HourMinuteTextFieldState`.
struct HourMinuteTextFieldState {
    state: StateData<HourMinuteTextField>,
    restoration: RestorationMixinData,
    controller: Option<Ref<RestorableTextEditingController>>,
    controller_has_been_set: Option<Ref<RestorableBool>>,
    focus_node: Option<Ref<FocusNode>>,
}

impl HourMinuteTextFieldState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<RestorableTextEditingController> {
        app.get(self)
            .controller
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn controller_has_been_set(self: Handle<Self>, app: &App) -> Handle<RestorableBool> {
        app.get(self)
            .controller_has_been_set
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn focus_node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self)
            .focus_node
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        // Rebuild when focus changes.
        self.set_state(app, |_state| {});
        if waft_foundation::K_IS_WEB && self.focus_node(app).has_focus(app) {
            let primary_focus_context = waft_widgets::primary_focus(app)
                .and_then(|primary_focus| primary_focus.context(app));
            if let Some(context) = primary_focus_context {
                waft_widgets::Actions::maybe_invoke(
                    app,
                    context,
                    &waft_widgets::SelectAllTextIntent::new(
                        waft_services::SelectionChangedCause::Keyboard,
                    ),
                );
            }
        }
    }

    fn formatted_value(self: Handle<Self>, app: &mut App) -> String {
        let context = self.context(app);
        let always_use_24_hour_format = MediaQuery::always_use_24_hour_format_of(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let widget = self.widget(app);
        if !widget.is_hour {
            localizations.format_minute(&widget.selected_time)
        } else {
            localizations.format_hour(&widget.selected_time, always_use_24_hour_format)
        }
    }
}

impl RestorationMixin for HourMinuteTextFieldState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<waft_widgets::RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let controller = self.controller(app);
        let controller_has_been_set = self.controller_has_been_set(app);
        self.register_for_restoration(app, controller.as_property(), "text_editing_controller");
        self.register_for_restoration(
            app,
            controller_has_been_set.as_property(),
            "has_controller_been_set",
        );
    }
}

impl State for HourMinuteTextFieldState {
    type Widget = HourMinuteTextField;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = RestorableTextEditingController::new(app);
        let controller_has_been_set = RestorableBool::new(app, false);
        let focus_node = FocusNode::new(app);
        focus_node.add_listener(
            app,
            Listener::handle_method(self, Self::handle_focus_changed),
        );
        let state = app.get_mut(self);
        state.controller = Some(controller);
        state.controller_has_been_set = Some(controller_has_been_set);
        state.focus_node = Some(focus_node);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
        // Only set the text value if it has not been populated with a localized
        // version yet.
        // If emptyInitialTime is true, set it to an empty string to indicate no
        // initial time.
        if !*self.controller_has_been_set(app).value(app) {
            self.controller_has_been_set(app).set_value(app, true);
            let initial_text_value = if self.widget(app).empty_initial_time {
                String::new()
            } else {
                self.formatted_value(app)
            };
            let controller = self.controller(app).value(app).handle();
            controller.set_value(
                app,
                TextEditingValue {
                    text: initial_text_value,
                    ..TextEditingValue::default()
                },
            );
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &HourMinuteTextField) {
        self.did_update_restoration_id(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        RestorableProperty::dispose(self.controller(app), app);
        self.controller_has_been_set(app).dispose(app);
        self.focus_node(app).dispose(app);
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let time_picker_theme = TimePickerTheme::of(app, context);
        let default_theme: Rc<dyn TimePickerDefaults> = if theme.use_material3 {
            Rc::new(TimePickerDefaultsM3::new(
                context,
                crate::time_picker::TimePickerEntryMode::Dial,
            ))
        } else {
            Rc::new(TimePickerDefaultsM2::new(context))
        };
        let always_use_24_hour_format = MediaQuery::always_use_24_hour_format_of(app, context);

        let input_decoration_theme = time_picker_theme
            .input_decoration_theme
            .clone()
            .unwrap_or_else(|| default_theme.input_decoration_theme(app));
        let mut decoration = InputDecoration::new();
        // Prevent the error text from appearing when
        // timePickerTheme.inputDecorationTheme is used.
        if let Some(error_style) = default_theme.input_decoration_theme(app).error_style {
            decoration = decoration.error_style(error_style);
        }
        let mut input_decoration = decoration.apply_defaults(input_decoration_theme);
        // Remove the hint text when focused because the centered cursor
        // appears odd above the hint text.
        // Clear the hint text when emptyInitialTime is true.
        let has_focus = self.focus_node(app).has_focus(app);
        let empty_initial_time = self.widget(app).empty_initial_time;
        let hint_text = if has_focus || empty_initial_time {
            None
        } else {
            Some(self.formatted_value(app))
        };

        // Because the fill color is specified in both the inputDecorationTheme and
        // the TimePickerTheme, if there's one in the user's input decoration theme,
        // use that. If not, but there's one in the user's
        // timePickerTheme.hourMinuteColor, use that, and otherwise use the default.
        // We ignore the value in the fillColor of the input decoration in the
        // default theme here, but it's the same as the hourMinuteColor.
        let starting_fill_color = time_picker_theme
            .input_decoration_theme
            .as_ref()
            .and_then(|theme| theme.fill_color.clone())
            .or_else(|| time_picker_theme.hour_minute_color.clone())
            .unwrap_or_else(|| default_theme.hour_minute_color(app));
        let fill_color: AnyColor = if theme.use_material3 {
            let mut states = WidgetStates::new();
            if has_focus {
                states.insert(WidgetState::Focused);
                states.insert(WidgetState::Selected);
            }
            <dyn WidgetStateProperty<Color>>::resolve_as(&starting_fill_color, &states)
        } else if has_focus {
            Colors::TRANSPARENT.into()
        } else {
            starting_fill_color
        };

        if let Some(hint_text) = hint_text {
            input_decoration = input_decoration.hint_text(hint_text);
        }
        input_decoration = input_decoration.fill_color(fill_color);

        let mut states = WidgetStates::new();
        if has_focus {
            states.insert(WidgetState::Focused);
            states.insert(WidgetState::Selected);
        }
        let effective_text_color = <dyn WidgetStateProperty<Color>>::resolve_as(
            &time_picker_theme
                .hour_minute_text_color
                .clone()
                .unwrap_or_else(|| default_theme.hour_minute_text_color(app)),
            &states,
        );
        let widget = self.widget(app).clone();
        let effective_style =
            <dyn WidgetStateProperty<TextStyle>>::resolve_as(&widget.style, &states)
                .value()
                .copy_with()
                .color(effective_text_color.color());

        let size = if always_use_24_hour_format {
            default_theme.hour_minute_input_size_24_hour(app)
        } else {
            default_theme.hour_minute_input_size(app)
        };
        let controller = self.controller(app).value(app).clone();
        let controller_ref = controller.erase();
        let on_saved_submitted = Rc::clone(&widget.on_saved_submitted);
        let on_editing_complete: Rc<dyn Fn(&mut App)> = {
            let on_saved_submitted = Rc::clone(&on_saved_submitted);
            Rc::new(move |app: &mut App| {
                let text = controller.value(app).text.clone();
                on_saved_submitted(app, Some(&text));
            })
        };
        let mut field = TextFormField::new()
            .restoration_id("hour_minute_text_form_field")
            .autofocus(widget.autofocus.unwrap_or(false))
            .expands(true)
            .max_lines(None)
            .input_formatters(vec![TextInputFormatterRef(Rc::new(
                LengthLimitingTextInputFormatter::new(Some(2)),
            ))])
            .focus_node(
                app.get(self)
                    .focus_node
                    .as_ref()
                    .expect("init_state ran")
                    .erase(),
            )
            .text_align(TextAlign::Center)
            .text_input_action(widget.input_action)
            .keyboard_type(TextInputType::NUMBER)
            .style(effective_style)
            .controller(controller_ref)
            .decoration(Some(input_decoration))
            .validator({
                let validator = Rc::clone(&widget.validator);
                move |app, value| validator(app, value.map(String::as_str))
            })
            .on_editing_complete(move |app| on_editing_complete(app))
            .on_saved({
                let on_saved_submitted = Rc::clone(&on_saved_submitted);
                move |app, value| on_saved_submitted(app, value.as_deref())
            })
            .on_field_submitted({
                let on_saved_submitted = Rc::clone(&on_saved_submitted);
                move |app, value| on_saved_submitted(app, Some(&value))
            });
        if let Some(on_changed) = widget.on_changed.clone() {
            field = field.on_changed(move |app, value| on_changed(app, &value));
        }

        let restoration_bucket = self.bucket(app).and_then(|bucket| bucket.upgrade(app));
        SizedBox::new()
            .width(size.width())
            .height(size.height())
            .child(MediaQuery::with_no_text_scaling(
                None,
                UnmanagedRestorationScope::new(
                    Semantics::new()
                        .label(widget.semantic_hint_text.clone())
                        .child(field),
                )
                .bucket(restoration_bucket),
            ))
            .into_widget()
    }
}

/// Dart's `EntryModeChangeCallback`: called when the user switches the entry mode of a time
/// picker.
pub type EntryModeChangeCallback =
    std::rc::Rc<dyn Fn(&mut waft_foundation::App, TimePickerEntryMode)>;

const TIME_PICKER_ENTRY_MODES: [TimePickerEntryMode; 4] = [
    TimePickerEntryMode::Dial,
    TimePickerEntryMode::Input,
    TimePickerEntryMode::DialOnly,
    TimePickerEntryMode::InputOnly,
];
const AUTOVALIDATE_MODES: [AutovalidateMode; 4] = [
    AutovalidateMode::Disabled,
    AutovalidateMode::Always,
    AutovalidateMode::OnUserInteraction,
    AutovalidateMode::OnUnfocus,
];
const ORIENTATIONS: [Orientation; 2] = [Orientation::Portrait, Orientation::Landscape];
const HOUR_MINUTE_MODES: [HourMinuteMode; 2] = [HourMinuteMode::Hour, HourMinuteMode::Minute];

/// A Material Design time picker designed to appear inside a popup dialog.
///
/// Pass this widget to [`showDialog`](crate::show_dialog). The value returned by
/// [`showDialog`](crate::show_dialog) is the selected [`TimeOfDay`] if the user taps the "OK"
/// button, or null if the user taps the "CANCEL" button. The selected time is reported by
/// popping the navigator with the selected time.
///
/// Use [`show_time_picker`](crate::show_time_picker) to show the dialog.
#[derive(Clone)]
pub struct TimePickerDialog {
    pub key: Option<waft_widgets::KeyRef>,

    /// The time initially selected when the dialog is shown.
    pub initial_time: TimeOfDay,

    /// Optionally provide your own text for the cancel button.
    ///
    /// If null, the button uses [`MaterialLocalizations::cancel_button_label`].
    pub cancel_text: Option<String>,

    /// Optionally provide your own text for the confirm button.
    ///
    /// If null, the button uses [`MaterialLocalizations::ok_button_label`].
    pub confirm_text: Option<String>,

    /// Optionally provide your own help text to the header of the time picker.
    pub help_text: Option<String>,

    /// Optionally provide your own validation error text.
    pub error_invalid_text: Option<String>,

    /// Optionally provide your own hour label text.
    pub hour_label_text: Option<String>,

    /// Optionally provide your own minute label text.
    pub minute_label_text: Option<String>,

    /// Restoration ID to save and restore the state of the [`TimePickerDialog`].
    ///
    /// If it is non-null, the time picker will persist and restore the selected time and
    /// whether the dialog was in the dial or input entry mode.
    ///
    /// The state of this widget is persisted in a [`RestorationBucket`] claimed from
    /// the surrounding [`RestorationScope`] using the provided restoration ID.
    pub restoration_id: Option<String>,

    /// The entry mode for the picker. Whether it's text input or a dial.
    pub initial_entry_mode: TimePickerEntryMode,

    /// The optional [`orientation`](Self::orientation) parameter sets the
    /// [`Orientation`] to use when displaying the dialog.
    ///
    /// By default, the orientation is derived from the [`MediaQueryData::size`] of the ambient
    /// [`MediaQuery`]. If the aspect of the size is tall, then [`Orientation::Portrait`] is
    /// used, if the size is wide, then [`Orientation::Landscape`] is used.
    ///
    /// Use this parameter to override the default and force the dialog to appear in either
    /// portrait or landscape mode regardless of the aspect of the [`MediaQueryData::size`].
    pub orientation: Option<Orientation>,

    /// Callback called when the selected entry mode is changed.
    pub on_entry_mode_changed: Option<EntryModeChangeCallback>,

    /// Optionally provide your own icon for the icon button used to switch the mode to input.
    pub switch_to_input_entry_mode_icon: Option<WidgetRef>,

    /// Optionally provide your own icon for the icon button used to switch the mode to dial.
    pub switch_to_timer_entry_mode_icon: Option<WidgetRef>,

    /// Whether the input mode should start empty.
    ///
    /// If true, the hour and minute fields start empty and display hints for the expected
    /// values.
    pub empty_initial_input: bool,
}

impl std::fmt::Debug for TimePickerDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimePickerDialog")
            .field("initial_time", &self.initial_time)
            .field("initial_entry_mode", &self.initial_entry_mode)
            .finish_non_exhaustive()
    }
}

impl TimePickerDialog {
    /// Creates a Material Design time picker.
    ///
    /// Dart's named arguments are the setters.
    pub fn new(initial_time: TimeOfDay) -> TimePickerDialog {
        TimePickerDialog {
            key: None,
            initial_time,
            cancel_text: None,
            confirm_text: None,
            help_text: None,
            error_invalid_text: None,
            hour_label_text: None,
            minute_label_text: None,
            restoration_id: None,
            initial_entry_mode: TimePickerEntryMode::Dial,
            orientation: None,
            on_entry_mode_changed: None,
            switch_to_input_entry_mode_icon: None,
            switch_to_timer_entry_mode_icon: None,
            empty_initial_input: false,
        }
    }

    /// Dart `TimePickerDialog(key:)`.
    pub fn key(mut self, key: waft_widgets::KeyRef) -> TimePickerDialog {
        self.key = Some(key);
        self
    }

    /// Dart `TimePickerDialog(cancelText:)`.
    pub fn cancel_text(mut self, cancel_text: impl Into<String>) -> TimePickerDialog {
        self.cancel_text = Some(cancel_text.into());
        self
    }

    /// Dart `TimePickerDialog(confirmText:)`.
    pub fn confirm_text(mut self, confirm_text: impl Into<String>) -> TimePickerDialog {
        self.confirm_text = Some(confirm_text.into());
        self
    }

    /// Dart `TimePickerDialog(helpText:)`.
    pub fn help_text(mut self, help_text: impl Into<String>) -> TimePickerDialog {
        self.help_text = Some(help_text.into());
        self
    }

    /// Dart `TimePickerDialog(errorInvalidText:)`.
    pub fn error_invalid_text(mut self, error_invalid_text: impl Into<String>) -> TimePickerDialog {
        self.error_invalid_text = Some(error_invalid_text.into());
        self
    }

    /// Dart `TimePickerDialog(hourLabelText:)`.
    pub fn hour_label_text(mut self, hour_label_text: impl Into<String>) -> TimePickerDialog {
        self.hour_label_text = Some(hour_label_text.into());
        self
    }

    /// Dart `TimePickerDialog(minuteLabelText:)`.
    pub fn minute_label_text(mut self, minute_label_text: impl Into<String>) -> TimePickerDialog {
        self.minute_label_text = Some(minute_label_text.into());
        self
    }

    /// Dart `TimePickerDialog(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> TimePickerDialog {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `TimePickerDialog(initialEntryMode:)`.
    pub fn initial_entry_mode(
        mut self,
        initial_entry_mode: TimePickerEntryMode,
    ) -> TimePickerDialog {
        self.initial_entry_mode = initial_entry_mode;
        self
    }

    /// Dart `TimePickerDialog(orientation:)`.
    pub fn orientation(mut self, orientation: Orientation) -> TimePickerDialog {
        self.orientation = Some(orientation);
        self
    }

    /// Dart `TimePickerDialog(onEntryModeChanged:)`.
    pub fn on_entry_mode_changed(
        mut self,
        on_entry_mode_changed: EntryModeChangeCallback,
    ) -> TimePickerDialog {
        self.on_entry_mode_changed = Some(on_entry_mode_changed);
        self
    }

    /// Dart `TimePickerDialog(switchToInputEntryModeIcon:)`.
    pub fn switch_to_input_entry_mode_icon(mut self, icon: Icon) -> TimePickerDialog {
        self.switch_to_input_entry_mode_icon = Some(icon.into_widget());
        self
    }

    /// Dart `TimePickerDialog(switchToTimerEntryModeIcon:)`.
    pub fn switch_to_timer_entry_mode_icon(mut self, icon: Icon) -> TimePickerDialog {
        self.switch_to_timer_entry_mode_icon = Some(icon.into_widget());
        self
    }

    /// Dart `TimePickerDialog(emptyInitialInput:)`.
    pub fn empty_initial_input(mut self, empty_initial_input: bool) -> TimePickerDialog {
        self.empty_initial_input = empty_initial_input;
        self
    }
}

impl StatefulWidget for TimePickerDialog {
    type State = TimePickerDialogState;

    fn key(&self) -> Option<&waft_widgets::KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TimePickerDialogState {
        TimePickerDialogState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            entry_mode: None,
            selected_time: None,
            form_key: GlobalKey::new(),
            autovalidate_mode: None,
            orientation: None,
        }
    }
}

/// Dart's `_TimePickerDialogState`.
pub struct TimePickerDialogState {
    state: StateData<TimePickerDialog>,
    restoration: RestorationMixinData,
    entry_mode: Option<Ref<RestorableEnum<TimePickerEntryMode>>>,
    selected_time: Option<Ref<RestorableTimeOfDay>>,
    form_key: GlobalKey,
    autovalidate_mode: Option<Ref<RestorableEnum<AutovalidateMode>>>,
    orientation: Option<Ref<RestorableEnumN<Orientation>>>,
}

impl TimePickerDialogState {
    // Base sizes
    const K_TIME_PICKER_PORTRAIT_SIZE: Size = Size::new(310.0, 468.0);
    const K_TIME_PICKER_LANDSCAPE_SIZE: Size = Size::new(524.0, 342.0);
    const K_TIME_PICKER_LANDSCAPE_SIZE_M2: Size = Size::new(508.0, 300.0);
    const K_TIME_PICKER_INPUT_SIZE: Size = Size::new(312.0, 252.0);
    const K_TIME_PICKER_INPUT_MINIMUM_HEIGHT: f64 = 216.0;

    // Absolute minimum dialog sizes, which is the point at which it begins
    // scrolling to fit everything in.
    const K_TIME_PICKER_MIN_PORTRAIT_SIZE: Size = Size::new(238.0, 326.0);
    const K_TIME_PICKER_MIN_LANDSCAPE_SIZE: Size = Size::new(416.0, 248.0);
    const K_TIME_PICKER_MIN_INPUT_SIZE: Size = Size::new(312.0, 196.0);

    fn entry_mode(self: Handle<Self>, app: &App) -> Handle<RestorableEnum<TimePickerEntryMode>> {
        app.get(self)
            .entry_mode
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn selected_time(self: Handle<Self>, app: &App) -> Handle<RestorableTimeOfDay> {
        app.get(self)
            .selected_time
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn autovalidate_mode(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<RestorableEnum<AutovalidateMode>> {
        app.get(self)
            .autovalidate_mode
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn orientation(self: Handle<Self>, app: &App) -> Handle<RestorableEnumN<Orientation>> {
        app.get(self)
            .orientation
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn handle_time_changed(self: Handle<Self>, app: &mut App, value: TimeOfDay) {
        if value != *self.selected_time(app).value(app) {
            self.set_state(app, |_state| {});
            self.selected_time(app).set_value(app, value);
        }
    }

    fn handle_entry_mode_changed(self: Handle<Self>, app: &mut App, value: TimePickerEntryMode) {
        if value != *self.entry_mode(app).value(app) {
            self.set_state(app, |_state| {});
            match *self.entry_mode(app).value(app) {
                TimePickerEntryMode::Dial => {
                    self.autovalidate_mode(app)
                        .set_value(app, AutovalidateMode::Disabled);
                }
                TimePickerEntryMode::Input => {
                    let form_key = app.get(self).form_key.clone();
                    form_key
                        .current_state::<FormState>(app)
                        .expect("the form is mounted")
                        .save(app);
                }
                TimePickerEntryMode::DialOnly | TimePickerEntryMode::InputOnly => {}
            }
            self.entry_mode(app).set_value(app, value);
            if let Some(on_entry_mode_changed) = self.widget(app).on_entry_mode_changed.clone() {
                on_entry_mode_changed(app, value);
            }
        }
    }

    fn toggle_entry_mode(self: Handle<Self>, app: &mut App) {
        match *self.entry_mode(app).value(app) {
            TimePickerEntryMode::Dial => {
                self.handle_entry_mode_changed(app, TimePickerEntryMode::Input);
            }
            TimePickerEntryMode::Input => {
                self.handle_entry_mode_changed(app, TimePickerEntryMode::Dial);
            }
            TimePickerEntryMode::DialOnly | TimePickerEntryMode::InputOnly => {
                // Dart builds a `FlutterError` here and drops it without throwing.
            }
        }
    }

    fn handle_cancel(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        Navigator::pop(app, context, None);
    }

    fn handle_ok(self: Handle<Self>, app: &mut App) {
        let entry_mode = *self.entry_mode(app).value(app);
        if entry_mode == TimePickerEntryMode::Input || entry_mode == TimePickerEntryMode::InputOnly
        {
            let form_key = app.get(self).form_key.clone();
            let form = form_key
                .current_state::<FormState>(app)
                .expect("the form is mounted");
            if !form.validate(app) {
                self.set_state(app, |_state| {});
                self.autovalidate_mode(app)
                    .set_value(app, AutovalidateMode::Always);
                return;
            }
            form.save(app);
        }
        let context = self.context(app);
        let selected_time = *self.selected_time(app).value(app);
        Navigator::pop(app, context, Some(Rc::new(selected_time)));
    }

    fn effective_orientation(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> Orientation {
        self.orientation(app)
            .value(app)
            .unwrap_or_else(|| MediaQuery::orientation_of(app, context))
    }

    fn min_dialog_size(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        use_material3: bool,
    ) -> Size {
        let orientation = self.effective_orientation(app, context);

        match *self.entry_mode(app).value(app) {
            TimePickerEntryMode::Dial | TimePickerEntryMode::DialOnly => match orientation {
                Orientation::Portrait => Self::K_TIME_PICKER_MIN_PORTRAIT_SIZE,
                Orientation::Landscape => Self::K_TIME_PICKER_MIN_LANDSCAPE_SIZE,
            },
            TimePickerEntryMode::Input | TimePickerEntryMode::InputOnly => {
                let localizations = <dyn MaterialLocalizations>::of(app, context);
                let time_of_day_format = localizations
                    .time_of_day_format(MediaQuery::always_use_24_hour_format_of(app, context));
                let time_picker_width = match time_of_day_format {
                    TimeOfDayFormat::HH_colon_mm
                    | TimeOfDayFormat::HH_dot_mm
                    | TimeOfDayFormat::FrenchCanadian
                    | TimeOfDayFormat::H_colon_mm => {
                        let default_theme: Rc<dyn TimePickerDefaults> = if use_material3 {
                            Rc::new(TimePickerDefaultsM3::new(
                                context,
                                TimePickerEntryMode::Dial,
                            ))
                        } else {
                            Rc::new(TimePickerDefaultsM2::new(context))
                        };
                        Self::K_TIME_PICKER_MIN_INPUT_SIZE.width()
                            - default_theme.day_period_portrait_size(app).width()
                            - 12.0
                    }
                    TimeOfDayFormat::a_space_h_colon_mm | TimeOfDayFormat::h_colon_mm_space_a => {
                        Self::K_TIME_PICKER_MIN_INPUT_SIZE.width()
                            - if use_material3 { 32.0 } else { 0.0 }
                    }
                };
                Size::new(
                    time_picker_width,
                    Self::K_TIME_PICKER_MIN_INPUT_SIZE.height(),
                )
            }
        }
    }

    fn dialog_size(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        use_material3: bool,
    ) -> Size {
        let orientation = self.effective_orientation(app, context);
        // Constrain the textScaleFactor to prevent layout issues. Since only some
        // parts of the time picker scale up with textScaleFactor, we cap the factor
        // to 1.1 as that provides enough space to reasonably fit all the content.
        //
        // 14 is a common font size used to compute the effective text scale.
        const FONT_SIZE_TO_SCALE: f64 = 14.0;
        let text_scale_factor = MediaQuery::text_scaler_of(app, context)
            .clamp(0.0, 1.1)
            .scale(FONT_SIZE_TO_SCALE)
            / FONT_SIZE_TO_SCALE;

        let time_picker_size = match *self.entry_mode(app).value(app) {
            TimePickerEntryMode::Dial | TimePickerEntryMode::DialOnly => match orientation {
                Orientation::Portrait => Self::K_TIME_PICKER_PORTRAIT_SIZE,
                Orientation::Landscape => Size::new(
                    Self::K_TIME_PICKER_LANDSCAPE_SIZE.width() * text_scale_factor,
                    if use_material3 {
                        Self::K_TIME_PICKER_LANDSCAPE_SIZE.height()
                    } else {
                        Self::K_TIME_PICKER_LANDSCAPE_SIZE_M2.height()
                    },
                ),
            },
            TimePickerEntryMode::Input | TimePickerEntryMode::InputOnly => {
                let localizations = <dyn MaterialLocalizations>::of(app, context);
                let time_of_day_format = localizations
                    .time_of_day_format(MediaQuery::always_use_24_hour_format_of(app, context));
                let time_picker_width = match time_of_day_format {
                    TimeOfDayFormat::HH_colon_mm
                    | TimeOfDayFormat::HH_dot_mm
                    | TimeOfDayFormat::FrenchCanadian
                    | TimeOfDayFormat::H_colon_mm => {
                        let default_theme: Rc<dyn TimePickerDefaults> = if use_material3 {
                            Rc::new(TimePickerDefaultsM3::new(
                                context,
                                TimePickerEntryMode::Dial,
                            ))
                        } else {
                            Rc::new(TimePickerDefaultsM2::new(context))
                        };
                        Self::K_TIME_PICKER_INPUT_SIZE.width()
                            - default_theme.day_period_portrait_size(app).width()
                            - 12.0
                    }
                    TimeOfDayFormat::a_space_h_colon_mm | TimeOfDayFormat::h_colon_mm_space_a => {
                        Self::K_TIME_PICKER_INPUT_SIZE.width()
                            - if use_material3 { 32.0 } else { 0.0 }
                    }
                };
                Size::new(time_picker_width, Self::K_TIME_PICKER_INPUT_SIZE.height())
            }
        };
        Size::new(
            time_picker_size.width(),
            time_picker_size.height() * text_scale_factor,
        )
    }
}

impl RestorationMixin for TimePickerDialogState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<waft_widgets::RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let (selected_time, entry_mode, autovalidate_mode, orientation) = (
            self.selected_time(app),
            self.entry_mode(app),
            self.autovalidate_mode(app),
            self.orientation(app),
        );
        self.register_for_restoration(app, selected_time.as_property(), "selected_time");
        self.register_for_restoration(app, entry_mode.as_property(), "entry_mode");
        self.register_for_restoration(app, autovalidate_mode.as_property(), "autovalidate_mode");
        self.register_for_restoration(app, orientation.as_property(), "orientation");
    }
}

impl State for TimePickerDialogState {
    type Widget = TimePickerDialog;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let entry_mode =
            RestorableEnum::new(app, widget.initial_entry_mode, TIME_PICKER_ENTRY_MODES);
        let selected_time = RestorableTimeOfDay::new(app, widget.initial_time);
        let autovalidate_mode =
            RestorableEnum::new(app, AutovalidateMode::Disabled, AUTOVALIDATE_MODES);
        let orientation = RestorableEnumN::new(app, widget.orientation, ORIENTATIONS);
        let state = app.get_mut(self);
        state.entry_mode = Some(entry_mode);
        state.selected_time = Some(selected_time);
        state.autovalidate_mode = Some(autovalidate_mode);
        state.orientation = Some(orientation);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &TimePickerDialog) {
        self.did_update_restoration_id(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.selected_time(app).dispose(app);
        self.entry_mode(app).dispose(app);
        self.autovalidate_mode(app).dispose(app);
        self.orientation(app).dispose(app);
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        waft_widgets::debug_check_has_media_query(app, context);
        let theme = Theme::of(app, context);
        let picker_theme = TimePickerTheme::of(app, context);
        let default_theme: Rc<dyn TimePickerDefaults> = if theme.use_material3 {
            Rc::new(TimePickerDefaultsM3::new(
                context,
                TimePickerEntryMode::Dial,
            ))
        } else {
            Rc::new(TimePickerDefaultsM2::new(context))
        };
        let shape = picker_theme
            .shape
            .clone()
            .unwrap_or_else(|| default_theme.shape(app));
        let entry_mode_icon_color = picker_theme
            .entry_mode_icon_color
            .clone()
            .unwrap_or_else(|| default_theme.entry_mode_icon_color(app))
            .color();
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let widget = self.widget(app).clone();
        let entry_mode = *self.entry_mode(app).value(app);

        let mut action_children: Vec<WidgetRef> = Vec::new();
        if entry_mode == TimePickerEntryMode::Dial || entry_mode == TimePickerEntryMode::Input {
            let icon = if entry_mode == TimePickerEntryMode::Dial {
                widget
                    .switch_to_input_entry_mode_icon
                    .clone()
                    .unwrap_or_else(|| Icon::new(Some(Icons::keyboard_outlined())).into_widget())
            } else {
                widget
                    .switch_to_timer_entry_mode_icon
                    .clone()
                    .unwrap_or_else(|| Icon::new(Some(Icons::access_time())).into_widget())
            };
            let mut icon_button = IconButton::new(
                Some(Listener::new(move |app| self.toggle_entry_mode(app))),
                icon,
            )
            .tooltip(if entry_mode == TimePickerEntryMode::Dial {
                localizations.input_time_mode_button_label()
            } else {
                localizations.dial_mode_button_label()
            });
            // In material3 mode, we want to use the color as part of the
            // button style which applies its own opacity. In material2 mode,
            // we want to use the color as the color, which already includes
            // the opacity.
            if theme.use_material3 {
                icon_button = icon_button.style(
                    IconButton::style_from()
                        .foreground_color(entry_mode_icon_color)
                        .build(),
                );
            } else {
                icon_button = icon_button.color(entry_mode_icon_color);
            }
            action_children.push(icon_button.into_widget());
        }
        let cancel_button_style: ButtonStyle = picker_theme
            .cancel_button_style
            .clone()
            .unwrap_or_else(|| default_theme.cancel_button_style(app));
        let confirm_button_style: ButtonStyle = picker_theme
            .confirm_button_style
            .clone()
            .unwrap_or_else(|| default_theme.confirm_button_style(app));
        action_children.push(
            Expanded::new(
                ConstrainedBox::new(BoxConstraints {
                    min_height: 36.0,
                    ..BoxConstraints::new()
                })
                .child(
                    Align::new()
                        .alignment(AlignmentGeometry::from(AlignmentDirectional::CENTER_END))
                        .child(
                            OverflowBar::new()
                                .spacing(8.0)
                                .overflow_alignment(OverflowBarAlignment::End)
                                .children(vec![
                                    TextButton::new(
                                        Some(Listener::new(move |app| self.handle_cancel(app))),
                                        Some(Text::new(widget.cancel_text.clone().unwrap_or_else(
                                            || {
                                                if theme.use_material3 {
                                                    localizations.cancel_button_label()
                                                } else {
                                                    localizations
                                                        .cancel_button_label()
                                                        .to_uppercase()
                                                }
                                            },
                                        ))),
                                    )
                                    .style(cancel_button_style)
                                    .into_widget(),
                                    TextButton::new(
                                        Some(Listener::new(move |app| self.handle_ok(app))),
                                        Some(Text::new(
                                            widget
                                                .confirm_text
                                                .clone()
                                                .unwrap_or_else(|| localizations.ok_button_label()),
                                        )),
                                    )
                                    .style(confirm_button_style)
                                    .into_widget(),
                                ]),
                        ),
                ),
            )
            .into_widget(),
        );
        let actions = Padding::new(
            EdgeInsetsDirectional::only(if theme.use_material3 { 0.0 } else { 4.0 }, 0.0, 0.0, 0.0)
                .into(),
        )
        .child(Row::new().children(action_children))
        .into_widget();

        let tap_target_size_offset = match theme.material_tap_target_size {
            MaterialTapTargetSize::Padded => Offset::ZERO,
            // _dialogSize returns "padded" sizes.
            MaterialTapTargetSize::ShrinkWrap => Offset::new(0.0, -12.0),
        };
        let dialog_size =
            self.dialog_size(app, context, theme.use_material3) + tap_target_size_offset;
        let min_dialog_size =
            self.min_dialog_size(app, context, theme.use_material3) + tap_target_size_offset;
        let form_key = app.get(self).form_key.clone();
        let autovalidate_mode = *self.autovalidate_mode(app).value(app);
        let selected_initial_time = widget.initial_time;
        let padding: EdgeInsetsGeometry = picker_theme
            .padding
            .unwrap_or_else(|| default_theme.padding(app));
        let elevation = picker_theme
            .elevation
            .unwrap_or_else(|| default_theme.elevation(app));
        let background_color = picker_theme
            .background_color
            .clone()
            .unwrap_or_else(|| default_theme.background_color(app))
            .color();

        Dialog::new()
            .shape(shape)
            .elevation(elevation)
            .background_color(background_color)
            .inset_padding(EdgeInsets::symmetric(
                if entry_mode == TimePickerEntryMode::Input
                    || entry_mode == TimePickerEntryMode::InputOnly
                {
                    0.0
                } else {
                    24.0
                },
                16.0,
            ))
            .child(
                Padding::new(padding).child(
                    LayoutBuilder::new(
                    move |_app, _context, constraints| {
                        let constrained_size = constraints.constraints().constrain(dialog_size);
                        let allowed_size = Size::new(
                            if constrained_size.width() < min_dialog_size.width() {
                                min_dialog_size.width()
                            } else {
                                constrained_size.width()
                            },
                            if constrained_size.height() < min_dialog_size.height() {
                                min_dialog_size.height()
                            } else {
                                constrained_size.height()
                            },
                        );
                        let form_key = form_key.clone();
                        let widget = widget.clone();
                        let actions = actions.clone();
                        SingleChildScrollView::new()
                            .restoration_id("time_picker_scroll_view_horizontal")
                            .scroll_direction(Axis::Horizontal)
                            .child(
                                SingleChildScrollView::new()
                                    .restoration_id("time_picker_scroll_view_vertical")
                                    .child(
                                        waft_widgets::AnimatedContainer::new(
                                            K_DIALOG_SIZE_ANIMATION_DURATION,
                                        )
                                        .width(allowed_size.width())
                                        .curve(waft_animation::Curves::ease_in())
                                        .constraints(BoxConstraints {
                                            min_height:
                                                TimePickerDialogState::K_TIME_PICKER_INPUT_MINIMUM_HEIGHT,
                                            max_height: allowed_size.height(),
                                            ..BoxConstraints::new()
                                        })
                                        .child(
                                            Column::new()
                                                .main_axis_size(MainAxisSize::Min)
                                                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                                                .cross_axis_alignment(CrossAxisAlignment::Start)
                                                .children([
                                                    Builder::new(move |_app, _context| {
                                                        let child = Form::new(
                                                            TimePicker {
                                                                time: selected_initial_time,
                                                                on_time_changed: Some(Rc::new(
                                                                    move |app, value| {
                                                                        self.handle_time_changed(
                                                                            app, value,
                                                                        );
                                                                    },
                                                                )),
                                                                help_text: widget.help_text.clone(),
                                                                cancel_text: widget
                                                                    .cancel_text
                                                                    .clone(),
                                                                confirm_text: widget
                                                                    .confirm_text
                                                                    .clone(),
                                                                error_invalid_text: widget
                                                                    .error_invalid_text
                                                                    .clone(),
                                                                hour_label_text: widget
                                                                    .hour_label_text
                                                                    .clone(),
                                                                minute_label_text: widget
                                                                    .minute_label_text
                                                                    .clone(),
                                                                restoration_id: Some(
                                                                    "time_picker".to_string(),
                                                                ),
                                                                entry_mode,
                                                                orientation: widget.orientation,
                                                                on_entry_mode_changed: Some(
                                                                    Rc::new(move |app, value| {
                                                                        self.handle_entry_mode_changed(
                                                                            app, value,
                                                                        );
                                                                    }),
                                                                ),
                                                                switch_to_input_entry_mode_icon:
                                                                    widget
                                                                        .switch_to_input_entry_mode_icon
                                                                        .clone(),
                                                                switch_to_timer_entry_mode_icon:
                                                                    widget
                                                                        .switch_to_timer_entry_mode_icon
                                                                        .clone(),
                                                                empty_initial_input: widget
                                                                    .empty_initial_input,
                                                            },
                                                        )
                                                        .key(form_key.clone().into())
                                                        .autovalidate_mode(autovalidate_mode)
                                                        .into_widget();
                                                        if entry_mode != TimePickerEntryMode::Input
                                                            && entry_mode
                                                                != TimePickerEntryMode::InputOnly
                                                        {
                                                            return Flexible::new(child).into_widget();
                                                        }
                                                        child
                                                    })
                                                    .into_widget(),
                                                    actions,
                                                ]),
                                        ),
                                    ),
                            )
                            .into_widget()
                    },
                )
                .into_widget(),
                ),
            )
            .into_widget()
    }
}

/// Dart's `_TimePicker`.
#[derive(Clone)]
struct TimePicker {
    help_text: Option<String>,
    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    cancel_text: Option<String>,
    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    confirm_text: Option<String>,
    error_invalid_text: Option<String>,
    hour_label_text: Option<String>,
    minute_label_text: Option<String>,
    restoration_id: Option<String>,
    entry_mode: TimePickerEntryMode,
    time: TimeOfDay,
    on_time_changed: Option<SelectedTimeChanged>,
    orientation: Option<Orientation>,
    on_entry_mode_changed: Option<EntryModeChangeCallback>,
    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    switch_to_input_entry_mode_icon: Option<WidgetRef>,
    #[expect(dead_code, reason = "Dart's field, which only its constructor reads")]
    switch_to_timer_entry_mode_icon: Option<WidgetRef>,
    empty_initial_input: bool,
}

impl std::fmt::Debug for TimePicker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimePicker")
            .field("time", &self.time)
            .field("entry_mode", &self.entry_mode)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for TimePicker {
    type State = TimePickerState;

    fn create_state(&self) -> TimePickerState {
        TimePickerState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            vibrate_timer: None,
            localizations: None,
            hour_minute_mode: None,
            last_mode_announced: None,
            autofocus_hour: None,
            autofocus_minute: None,
            orientation: None,
            selected_time: None,
        }
    }
}

/// Dart's `_TimePickerState`.
struct TimePickerState {
    state: StateData<TimePicker>,
    restoration: RestorationMixinData,
    vibrate_timer: Option<Timer>,
    localizations: Option<Rc<dyn MaterialLocalizations>>,
    hour_minute_mode: Option<Ref<RestorableEnum<HourMinuteMode>>>,
    last_mode_announced: Option<Ref<RestorableEnumN<HourMinuteMode>>>,
    autofocus_hour: Option<Ref<RestorableBoolN>>,
    autofocus_minute: Option<Ref<RestorableBoolN>>,
    orientation: Option<Ref<RestorableEnumN<Orientation>>>,
    selected_time: Option<Ref<RestorableTimeOfDay>>,
}

impl TimePickerState {
    fn hour_minute_mode(self: Handle<Self>, app: &App) -> Handle<RestorableEnum<HourMinuteMode>> {
        app.get(self)
            .hour_minute_mode
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn last_mode_announced(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<RestorableEnumN<HourMinuteMode>> {
        app.get(self)
            .last_mode_announced
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn autofocus_hour(self: Handle<Self>, app: &App) -> Handle<RestorableBoolN> {
        app.get(self)
            .autofocus_hour
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn autofocus_minute(self: Handle<Self>, app: &App) -> Handle<RestorableBoolN> {
        app.get(self)
            .autofocus_minute
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn orientation(self: Handle<Self>, app: &App) -> Handle<RestorableEnumN<Orientation>> {
        app.get(self)
            .orientation
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    /// Dart's `selectedTime` getter: the restorable the dialog's tests read.
    fn selected_time(self: Handle<Self>, app: &App) -> Handle<RestorableTimeOfDay> {
        app.get(self)
            .selected_time
            .as_ref()
            .expect("init_state ran")
            .handle()
    }

    fn set_entry_mode(self: Handle<Self>, app: &mut App, mode: TimePickerEntryMode) {
        if let Some(on_entry_mode_changed) = self.widget(app).on_entry_mode_changed.clone() {
            on_entry_mode_changed(app, mode);
        }
    }

    fn vibrate(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        match Theme::of(app, context).platform {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                if let Some(timer) = app.get_mut(self).vibrate_timer.take() {
                    timer.cancel(app);
                }
                let timer = Timer::new(
                    app,
                    K_VIBRATE_COMMIT_DELAY,
                    Listener::new(move |app| {
                        HapticFeedback::vibrate(app);
                        app.get_mut(self).vibrate_timer = None;
                    }),
                );
                app.get_mut(self).vibrate_timer = Some(timer);
            }
            TargetPlatform::IOS | TargetPlatform::MacOS => {}
        }
    }

    fn handle_hour_minute_mode_changed(self: Handle<Self>, app: &mut App, mode: HourMinuteMode) {
        self.vibrate(app);
        self.set_state(app, |_state| {});
        self.hour_minute_mode(app).set_value(app, mode);
    }

    fn handle_entry_mode_toggle(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
        let entry_mode = self.widget(app).entry_mode;
        let new_mode = match entry_mode {
            TimePickerEntryMode::Dial => TimePickerEntryMode::Input,
            TimePickerEntryMode::Input => {
                self.autofocus_hour(app).set_value(app, Some(false));
                self.autofocus_minute(app).set_value(app, Some(false));
                TimePickerEntryMode::Dial
            }
            TimePickerEntryMode::DialOnly | TimePickerEntryMode::InputOnly => {
                // Dart builds a `FlutterError` here and drops it without throwing.
                entry_mode
            }
        };
        self.set_entry_mode(app, new_mode);
    }

    fn handle_time_changed(self: Handle<Self>, app: &mut App, value: TimeOfDay) {
        self.vibrate(app);
        self.set_state(app, |_state| {});
        self.selected_time(app).set_value(app, value);
        if let Some(on_time_changed) = self.widget(app).on_time_changed.clone() {
            on_time_changed(app, value);
        }
    }

    fn handle_hour_double_tapped(self: Handle<Self>, app: &mut App) {
        self.autofocus_hour(app).set_value(app, Some(true));
        self.handle_entry_mode_toggle(app);
    }

    fn handle_minute_double_tapped(self: Handle<Self>, app: &mut App) {
        self.autofocus_minute(app).set_value(app, Some(true));
        self.handle_entry_mode_toggle(app);
    }

    fn handle_hour_selected(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
        self.hour_minute_mode(app)
            .set_value(app, HourMinuteMode::Minute);
    }
}

impl RestorationMixin for TimePickerState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<waft_widgets::RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let (
            hour_minute_mode,
            last_mode_announced,
            autofocus_hour,
            autofocus_minute,
            selected_time,
            orientation,
        ) = (
            self.hour_minute_mode(app),
            self.last_mode_announced(app),
            self.autofocus_hour(app),
            self.autofocus_minute(app),
            self.selected_time(app),
            self.orientation(app),
        );
        self.register_for_restoration(app, hour_minute_mode.as_property(), "hour_minute_mode");
        self.register_for_restoration(
            app,
            last_mode_announced.as_property(),
            "last_mode_announced",
        );
        self.register_for_restoration(app, autofocus_hour.as_property(), "autofocus_hour");
        self.register_for_restoration(app, autofocus_minute.as_property(), "autofocus_minute");
        self.register_for_restoration(app, selected_time.as_property(), "selected_time");
        self.register_for_restoration(app, orientation.as_property(), "orientation");
    }
}

impl State for TimePickerState {
    type Widget = TimePicker;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let (orientation, time) = {
            let widget = self.widget(app);
            (widget.orientation, widget.time)
        };
        let hour_minute_mode = RestorableEnum::new(app, HourMinuteMode::Hour, HOUR_MINUTE_MODES);
        let last_mode_announced = RestorableEnumN::new(app, None, HOUR_MINUTE_MODES);
        let autofocus_hour = RestorableBoolN::new(app, None);
        let autofocus_minute = RestorableBoolN::new(app, None);
        let orientation = RestorableEnumN::new(app, orientation, ORIENTATIONS);
        let selected_time = RestorableTimeOfDay::new(app, time);
        let state = app.get_mut(self);
        state.hour_minute_mode = Some(hour_minute_mode);
        state.last_mode_announced = Some(last_mode_announced);
        state.autofocus_hour = Some(autofocus_hour);
        state.autofocus_minute = Some(autofocus_minute);
        state.orientation = Some(orientation);
        state.selected_time = Some(selected_time);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
        let context = self.context(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        app.get_mut(self).localizations = Some(localizations);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TimePicker) {
        self.did_update_restoration_id(app);
        let (orientation, time) = {
            let widget = self.widget(app);
            (widget.orientation, widget.time)
        };
        if old_widget.orientation != orientation {
            self.orientation(app).set_value(app, orientation);
        }
        if old_widget.time != time {
            self.selected_time(app).set_value(app, time);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).vibrate_timer.take() {
            timer.cancel(app);
        }
        self.orientation(app).dispose(app);
        self.selected_time(app).dispose(app);
        self.hour_minute_mode(app).dispose(app);
        self.last_mode_announced(app).dispose(app);
        self.autofocus_hour(app).dispose(app);
        self.autofocus_minute(app).dispose(app);
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        waft_widgets::debug_check_has_media_query(app, context);
        let localizations = Rc::clone(
            app.get(self)
                .localizations
                .as_ref()
                .expect("dependencies ran"),
        );
        let time_of_day_format = localizations
            .time_of_day_format(MediaQuery::always_use_24_hour_format_of(app, context));
        let theme = Theme::of(app, context);
        let widget = self.widget(app).clone();
        let default_theme: Rc<dyn TimePickerDefaults> = if theme.use_material3 {
            Rc::new(TimePickerDefaultsM3::new(context, widget.entry_mode))
        } else {
            Rc::new(TimePickerDefaultsM2::new(context))
        };
        let orientation = self
            .orientation(app)
            .value(app)
            .unwrap_or_else(|| MediaQuery::orientation_of(app, context));
        let time_of_day_hour = hour_format(time_of_day_format);
        let hour_mode = match time_of_day_hour {
            HourFormat::HH | HourFormat::H if theme.use_material3 => {
                HourDialType::TwentyFourHourDoubleRing
            }
            HourFormat::HH | HourFormat::H => HourDialType::TwentyFourHour,
            HourFormat::h => HourDialType::TwelveHour,
        };

        let hour_minute_mode = *self.hour_minute_mode(app).value(app);
        let selected_time = *self.selected_time(app).value(app);
        let picker: WidgetRef = match widget.entry_mode {
            TimePickerEntryMode::Dial | TimePickerEntryMode::DialOnly => {
                let help_text = widget.help_text.clone().unwrap_or_else(|| {
                    if theme.use_material3 {
                        localizations.time_picker_dial_help_text()
                    } else {
                        localizations.time_picker_dial_help_text().to_uppercase()
                    }
                });

                // The vertical adjustment used to make both AM/PM buttons accessible.
                // Because the period selector height is increased based on this value,
                // the dial padding has to be decreased of the same amount.
                let portrait_min_interactive_vertical_adjustment = (2.0
                    * K_MIN_INTERACTIVE_DIMENSION
                    - default_theme.day_period_portrait_size(app).height())
                .max(0.0);
                let dial_padding: EdgeInsetsGeometry = match orientation {
                    Orientation::Portrait => EdgeInsets::only(
                        12.0,
                        36.0 - portrait_min_interactive_vertical_adjustment / 2.0,
                        12.0,
                        0.0,
                    )
                    .into(),
                    Orientation::Landscape => {
                        EdgeInsetsDirectional::only(64.0, 0.0, 0.0, 0.0).into()
                    }
                };

                let dial_size = default_theme.dial_size(app);
                let dial = Padding::new(dial_padding)
                    .child(
                        ExcludeSemantics::new().child(
                            SizedBox::new()
                                .width(dial_size.width())
                                .height(dial_size.height())
                                .child(AspectRatio::new(1.0).child(Dial {
                                    hour_minute_mode,
                                    hour_dial_type: hour_mode,
                                    selected_time,
                                    on_changed: Some(Rc::new(move |app, value| {
                                        self.handle_time_changed(app, value);
                                    })),
                                    on_hour_selected: Some(Rc::new(move |app| {
                                        self.handle_hour_selected(app);
                                    })),
                                })),
                        ),
                    )
                    .into_widget();

                let horizontal = if theme.use_material3 { 0.0 } else { 16.0 };
                match orientation {
                    Orientation::Portrait => Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .children([
                            Padding::new(EdgeInsets::symmetric(0.0, horizontal).into())
                                .child(DialTimePickerHeader { help_text })
                                .into_widget(),
                            Expanded::new(
                                Column::new().main_axis_size(MainAxisSize::Min).children([
                                    // Dial grows and shrinks with the available space.
                                    Expanded::new(
                                        Padding::new(EdgeInsets::symmetric(0.0, horizontal).into())
                                            .child(dial),
                                    )
                                    .into_widget(),
                                ]),
                            )
                            .into_widget(),
                        ])
                        .into_widget(),
                    Orientation::Landscape => Column::new()
                        .children([Expanded::new(
                            Padding::new(EdgeInsets::symmetric(0.0, horizontal).into()).child(
                                Row::new()
                                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                    .children([
                                        DialTimePickerHeader { help_text }.into_widget(),
                                        Expanded::new(dial).into_widget(),
                                    ]),
                            ),
                        )
                        .into_widget()])
                        .into_widget(),
                }
            }
            TimePickerEntryMode::Input | TimePickerEntryMode::InputOnly => {
                let help_text = widget.help_text.clone().unwrap_or_else(|| {
                    if theme.use_material3 {
                        localizations.time_picker_input_help_text()
                    } else {
                        localizations.time_picker_input_help_text().to_uppercase()
                    }
                });

                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .children([TimePickerInput {
                        initial_selected_time: selected_time,
                        error_invalid_text: widget.error_invalid_text.clone(),
                        hour_label_text: widget.hour_label_text.clone(),
                        minute_label_text: widget.minute_label_text.clone(),
                        help_text,
                        autofocus_hour: *self.autofocus_hour(app).value(app),
                        autofocus_minute: *self.autofocus_minute(app).value(app),
                        restoration_id: Some("time_picker_input".to_string()),
                        empty_initial_time: widget.empty_initial_input,
                    }
                    .into_widget()])
                    .into_widget()
            }
        };
        let theme_data = TimePickerTheme::of(app, context);
        TimePickerModel {
            entry_mode: widget.entry_mode,
            selected_time,
            hour_minute_mode,
            orientation,
            on_hour_minute_mode_changed: Rc::new(move |app, mode| {
                self.handle_hour_minute_mode_changed(app, mode);
            }),
            on_hour_double_tapped: Listener::new(move |app| self.handle_hour_double_tapped(app)),
            on_minute_double_tapped: Listener::new(move |app| {
                self.handle_minute_double_tapped(app);
            }),
            hour_dial_type: hour_mode,
            on_selected_time_changed: Rc::new(move |app, value| {
                self.handle_time_changed(app, value);
            }),
            use_material3: theme.use_material3,
            use_24_hour_format: MediaQuery::always_use_24_hour_format_of(app, context),
            theme: theme_data,
            default_theme,
            child: picker,
        }
        .into_widget()
    }
}

/// Shows a dialog containing a Material Design time picker.
///
/// The returned future resolves when the dialog is dismissed: to the [`TimeOfDay`] the user
/// selected (inside the [`RouteResult`](waft_widgets::RouteResult)) if they tapped the "OK"
/// button, or to `None` if they dismissed the dialog or tapped "CANCEL".
///
/// The `builder` parameter can be used to wrap the dialog widget to add inherited widgets like
/// `Localizations.override`, `Directionality`, or `MediaQuery`.
///
/// The `initial_entry_mode` parameter can be used to determine the initial time entry
/// selection of the picker (either a clock dial or text input).
///
/// Optional strings for the `help_text`, `cancel_text`, `confirm_text`,
/// `error_invalid_text`, `hour_label_text` and `minute_label_text` allow you to override the
/// default text used for various parts of the dialog.
///
/// The `use_root_navigator` argument is used to determine whether to push the dialog to the
/// [`Navigator`](waft_widgets::Navigator) furthest from or nearest to the given `context`. By
/// default, `use_root_navigator` is true and the dialog route created by this method is pushed
/// to the root navigator.
///
/// The `orientation` parameter sets the [`Orientation`](waft_widgets::Orientation) to use
/// when displaying the dialog. By default the orientation is derived from the
/// [`MediaQueryData::size`](waft_widgets::MediaQueryData) of the ambient
/// [`MediaQuery`](waft_widgets::MediaQuery).
///
/// The `empty_initial_input` parameter starts the input mode with empty hour and minute fields.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_time_picker(
    app: &mut waft_foundation::App,
    context: waft_widgets::BuildContext,
    initial_time: TimeOfDay,
    builder: Option<waft_widgets::TransitionBuilder>,
    barrier_dismissible: bool,
    barrier_color: Option<waft_embedder::Color>,
    barrier_label: Option<String>,
    use_root_navigator: bool,
    initial_entry_mode: TimePickerEntryMode,
    cancel_text: Option<String>,
    confirm_text: Option<String>,
    help_text: Option<String>,
    error_invalid_text: Option<String>,
    hour_label_text: Option<String>,
    minute_label_text: Option<String>,
    route_settings: Option<waft_widgets::RouteSettingsRef>,
    on_entry_mode_changed: Option<EntryModeChangeCallback>,
    orientation: Option<waft_widgets::Orientation>,
    switch_to_input_entry_mode_icon: Option<waft_widgets::Icon>,
    switch_to_timer_entry_mode_icon: Option<waft_widgets::Icon>,
    empty_initial_input: bool,
) -> waft_foundation::CompleterFuture<waft_widgets::RouteResult> {
    debug_assert!(crate::debug::debug_check_has_material_localizations(
        app, context
    ));

    let mut dialog = TimePickerDialog::new(initial_time)
        .initial_entry_mode(initial_entry_mode)
        .empty_initial_input(empty_initial_input);
    dialog.cancel_text = cancel_text;
    dialog.confirm_text = confirm_text;
    dialog.help_text = help_text;
    dialog.error_invalid_text = error_invalid_text;
    dialog.hour_label_text = hour_label_text;
    dialog.minute_label_text = minute_label_text;
    dialog.orientation = orientation;
    dialog.on_entry_mode_changed = on_entry_mode_changed;
    if let Some(icon) = switch_to_input_entry_mode_icon {
        dialog = dialog.switch_to_input_entry_mode_icon(icon);
    }
    if let Some(icon) = switch_to_timer_entry_mode_icon {
        dialog = dialog.switch_to_timer_entry_mode_icon(icon);
    }
    let dialog = waft_widgets::IntoWidget::into_widget(dialog);
    crate::dialog::show_dialog(
        app,
        context,
        std::rc::Rc::new(move |app, context| match &builder {
            None => dialog.clone(),
            Some(builder) => builder(app, context, Some(&dialog)),
        }),
        barrier_dismissible,
        barrier_color,
        barrier_label,
        true,
        use_root_navigator,
        route_settings,
        None,
        false,
        None,
        None,
    )
}

/// Dart's `_TimePickerDefaults`: the defaults of a [`TimePickerThemeData`], each getter
/// non-null, with the sizes the theme does not expose. A getter reads the theme only when it is
/// asked, as Dart's do.
pub(crate) trait TimePickerDefaults {
    fn background_color(&self, app: &mut App) -> AnyColor;
    fn cancel_button_style(&self, app: &mut App) -> ButtonStyle;
    fn confirm_button_style(&self, app: &mut App) -> ButtonStyle;
    fn day_period_border_side(&self, app: &mut App) -> BorderSide;
    fn day_period_color(&self, app: &mut App) -> AnyColor;
    fn day_period_shape(&self, app: &mut App) -> OutlinedBorderRef;
    fn day_period_input_size(&self, app: &mut App) -> Size;
    fn day_period_landscape_size(&self, app: &mut App) -> Size;
    fn day_period_portrait_size(&self, app: &mut App) -> Size;
    fn day_period_text_color(&self, app: &mut App) -> AnyColor;
    fn day_period_text_style(&self, app: &mut App) -> AnyTextStyle;
    fn dial_background_color(&self, app: &mut App) -> AnyColor;
    fn dial_hand_color(&self, app: &mut App) -> AnyColor;
    fn dial_size(&self, app: &mut App) -> Size;
    fn hand_width(&self, app: &mut App) -> f64;
    fn dot_radius(&self, app: &mut App) -> f64;
    fn center_radius(&self, app: &mut App) -> f64;
    fn dial_text_color(&self, app: &mut App) -> AnyColor;
    fn dial_text_style(&self, app: &mut App) -> AnyTextStyle;
    fn elevation(&self, app: &mut App) -> f64;
    fn entry_mode_icon_color(&self, app: &mut App) -> AnyColor;
    fn help_text_style(&self, app: &mut App) -> AnyTextStyle;
    fn hour_minute_color(&self, app: &mut App) -> AnyColor;
    fn hour_minute_shape(&self, app: &mut App) -> ShapeBorderRef;
    fn hour_minute_size(&self, app: &mut App) -> Size;
    fn hour_minute_size_24_hour(&self, app: &mut App) -> Size;
    fn hour_minute_input_size(&self, app: &mut App) -> Size;
    fn hour_minute_input_size_24_hour(&self, app: &mut App) -> Size;
    fn hour_minute_text_color(&self, app: &mut App) -> AnyColor;
    fn hour_minute_text_style(&self, app: &mut App) -> AnyTextStyle;
    fn input_decoration_theme(&self, app: &mut App) -> InputDecorationThemeData;
    fn padding(&self, app: &mut App) -> EdgeInsetsGeometry;
    fn shape(&self, app: &mut App) -> ShapeBorderRef;

    /// Dart's `TimePickerThemeData.timeSelectorSeparatorColor`, which only the M3 defaults set.
    fn time_selector_separator_color(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        None
    }

    /// Dart's `TimePickerThemeData.timeSelectorSeparatorTextStyle`, which only the M3 defaults
    /// set.
    fn time_selector_separator_text_style(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        None
    }
}

pub(crate) fn state_color(callback: impl Fn(&WidgetStates) -> Color + 'static) -> AnyColor {
    AnyColor::from(WidgetStateColor::resolve_with(callback))
}

/// Dart's `_TimePickerDefaultsM2`: the Material 2 values, which are not generated because the
/// spec is not expected to change.
struct TimePickerDefaultsM2 {
    context: DefaultsContext,
}

#[allow(deprecated)]
impl TimePickerDefaultsM2 {
    fn new(context: BuildContext) -> TimePickerDefaultsM2 {
        TimePickerDefaultsM2 {
            context: DefaultsContext::new(context),
        }
    }

    fn default_shape() -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(4.0))
    }

    fn hour_minute_input_color(&self, app: &mut App) -> AnyColor {
        let on_surface = self.context.colors(app).on_surface;
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                Colors::TRANSPARENT
            } else {
                on_surface.with_opacity(0.12)
            }
        })
    }
}

#[allow(deprecated)]
impl TimePickerDefaults for TimePickerDefaultsM2 {
    fn background_color(&self, app: &mut App) -> AnyColor {
        self.context.colors(app).surface.into()
    }

    fn cancel_button_style(&self, _app: &mut App) -> ButtonStyle {
        TextButton::style_from().build()
    }

    fn confirm_button_style(&self, _app: &mut App) -> ButtonStyle {
        TextButton::style_from().build()
    }

    fn day_period_border_side(&self, app: &mut App) -> BorderSide {
        let colors = self.context.colors(app);
        BorderSide {
            color: Color::alpha_blend(colors.on_surface.with_opacity(0.38), colors.surface),
            ..BorderSide::default()
        }
    }

    fn day_period_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (primary, is_dark) = (colors.primary, colors.brightness == Brightness::Dark);
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                return primary.with_opacity(if is_dark { 0.24 } else { 0.12 });
            }
            // The unselected day period should match the overall picker dialog color.
            // Making it transparent enables that without being redundant and allows
            // the optional elevation overlay for dark mode to be visible.
            Colors::TRANSPARENT
        })
    }

    fn day_period_shape(&self, _app: &mut App) -> OutlinedBorderRef {
        OutlinedBorderRef::new(Self::default_shape())
    }

    fn day_period_portrait_size(&self, _app: &mut App) -> Size {
        Size::new(52.0, 80.0)
    }

    fn day_period_landscape_size(&self, _app: &mut App) -> Size {
        Size::new(0.0, 40.0)
    }

    fn day_period_input_size(&self, _app: &mut App) -> Size {
        Size::new(52.0, 70.0)
    }

    fn day_period_text_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (primary, on_surface) = (colors.primary, colors.on_surface);
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                primary
            } else {
                on_surface.with_opacity(0.60)
            }
        })
    }

    fn day_period_text_style(&self, app: &mut App) -> AnyTextStyle {
        let color = self.day_period_text_color(app);
        self.context
            .text_theme(app)
            .title_medium
            .as_ref()
            .expect("the text theme has titleMedium")
            .copy_with()
            .color(color)
            .into()
    }

    fn dial_background_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        colors
            .on_surface
            .with_opacity(if colors.brightness == Brightness::Dark {
                0.12
            } else {
                0.08
            })
            .into()
    }

    fn dial_hand_color(&self, app: &mut App) -> AnyColor {
        self.context.colors(app).primary.into()
    }

    fn dial_size(&self, _app: &mut App) -> Size {
        Size::square(280.0)
    }

    fn hand_width(&self, _app: &mut App) -> f64 {
        2.0
    }

    fn dot_radius(&self, _app: &mut App) -> f64 {
        22.0
    }

    fn center_radius(&self, _app: &mut App) -> f64 {
        4.0
    }

    fn dial_text_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (surface, on_surface) = (colors.surface, colors.on_surface);
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                surface
            } else {
                on_surface
            }
        })
    }

    fn dial_text_style(&self, app: &mut App) -> AnyTextStyle {
        self.context
            .text_theme(app)
            .body_large
            .clone()
            .expect("the text theme has bodyLarge")
            .into()
    }

    fn elevation(&self, _app: &mut App) -> f64 {
        6.0
    }

    fn entry_mode_icon_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        colors
            .on_surface
            .with_opacity(if colors.brightness == Brightness::Dark {
                1.0
            } else {
                0.6
            })
            .into()
    }

    fn help_text_style(&self, app: &mut App) -> AnyTextStyle {
        self.context
            .text_theme(app)
            .label_small
            .clone()
            .expect("the text theme has labelSmall")
            .into()
    }

    fn hour_minute_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (primary, on_surface, is_dark) = (
            colors.primary,
            colors.on_surface,
            colors.brightness == Brightness::Dark,
        );
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                primary.with_opacity(if is_dark { 0.24 } else { 0.12 })
            } else {
                on_surface.with_opacity(0.12)
            }
        })
    }

    fn hour_minute_shape(&self, _app: &mut App) -> ShapeBorderRef {
        ShapeBorderRef::new(Self::default_shape())
    }

    fn hour_minute_size(&self, _app: &mut App) -> Size {
        Size::new(96.0, 80.0)
    }

    fn hour_minute_size_24_hour(&self, _app: &mut App) -> Size {
        Size::new(114.0, 80.0)
    }

    fn hour_minute_input_size(&self, _app: &mut App) -> Size {
        Size::new(96.0, 70.0)
    }

    fn hour_minute_input_size_24_hour(&self, _app: &mut App) -> Size {
        Size::new(114.0, 70.0)
    }

    fn hour_minute_text_color(&self, app: &mut App) -> AnyColor {
        let colors = self.context.colors(app);
        let (primary, on_surface) = (colors.primary, colors.on_surface);
        state_color(move |states| {
            if states.contains(&WidgetState::Selected) {
                primary
            } else {
                on_surface
            }
        })
    }

    fn hour_minute_text_style(&self, app: &mut App) -> AnyTextStyle {
        self.context
            .text_theme(app)
            .display_medium
            .clone()
            .expect("the text theme has displayMedium")
            .into()
    }

    fn input_decoration_theme(&self, app: &mut App) -> InputDecorationThemeData {
        let fill_color = self.hour_minute_input_color(app);
        let hint_style = self
            .hour_minute_text_style(app)
            .value()
            .copy_with()
            .color(self.context.colors(app).on_surface.with_opacity(0.36));
        let colors = self.context.colors(app);
        let (error, primary) = (colors.error, colors.primary);
        InputDecorationThemeData::new()
            .content_padding(EdgeInsetsGeometry::from(EdgeInsets::ZERO))
            .filled(true)
            .fill_color(fill_color)
            .focus_color(Colors::TRANSPARENT)
            .enabled_border(OutlineInputBorder::new().border_side(BorderSide {
                color: Colors::TRANSPARENT,
                ..BorderSide::default()
            }))
            .error_border(OutlineInputBorder::new().border_side(BorderSide {
                color: error,
                width: 2.0,
                ..BorderSide::default()
            }))
            .focused_border(OutlineInputBorder::new().border_side(BorderSide {
                color: primary,
                width: 2.0,
                ..BorderSide::default()
            }))
            .focused_error_border(OutlineInputBorder::new().border_side(BorderSide {
                color: error,
                width: 2.0,
                ..BorderSide::default()
            }))
            .hint_style(hint_style)
            // Prevent the error text from appearing.
            .error_style(TextStyle::new().font_size(0.0).height(1.0))
    }

    fn padding(&self, _app: &mut App) -> EdgeInsetsGeometry {
        EdgeInsets::from_ltrb(8.0, 18.0, 8.0, 8.0).into()
    }

    fn shape(&self, _app: &mut App) -> ShapeBorderRef {
        ShapeBorderRef::new(Self::default_shape())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! time_picker_test.dart.

    use waft_widgets::testing::{
        TestPointer, WidgetTester, assert_f32_equals, assert_more_or_less_equals_within, find,
        test_widgets,
    };

    use super::*;
    use crate::dialog::Dialog;
    use crate::material::Material;
    use crate::test_support::{PaintInspection, material_app};
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;
    use crate::time::TimeOfDay;
    use crate::tooltip::tests::{m2, m3};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;
    use waft_embedder::{Offset, Size, TargetPlatform};
    use waft_embedder::{TextAffinity, TextEditingValue, TextSelection};
    use waft_foundation::Listener;
    use waft_painting::{EdgeInsets, TextScaler, TextStyle};
    use waft_rendering::testing::{PaintCall, assert_paints};
    use waft_services::TextInputClient;
    use waft_widgets::{
        AnyElement, BuildContext, Builder, Icon, IntoWidget, MediaQuery, MediaQueryData,
        Orientation, SizedBox, Text,
    };
    use waft_widgets::{EditableText, EditableTextState};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum MaterialType {
        Material2,
        Material3,
    }

    const OK_STRING: &str = "OK";
    const AM_STRING: &str = "AM";

    /// Dart's `mediaQueryBoilerplate` parameters.
    #[derive(Clone)]
    struct Boilerplate {
        always_use_24_hour_format: bool,
        initial_time: TimeOfDay,
        text_scaler: TextScaler,
        entry_mode: TimePickerEntryMode,
        help_text: Option<String>,
        hour_label_text: Option<String>,
        minute_label_text: Option<String>,
        error_invalid_text: Option<String>,
        accessible_navigation: bool,
        on_entry_mode_change: Option<EntryModeChangeCallback>,
        tap_button: bool,
        material_type: MaterialType,
        orientation: Option<Orientation>,
    }

    impl Boilerplate {
        fn new(material_type: MaterialType) -> Boilerplate {
            Boilerplate {
                always_use_24_hour_format: false,
                initial_time: TimeOfDay::new(7, 0),
                text_scaler: TextScaler::NO_SCALING,
                entry_mode: TimePickerEntryMode::Dial,
                help_text: None,
                hour_label_text: None,
                minute_label_text: None,
                error_invalid_text: None,
                accessible_navigation: false,
                on_entry_mode_change: None,
                tap_button: true,
                material_type,
                orientation: None,
            }
        }
    }

    /// Dart's `tester.view.physicalSize = size; tester.view.devicePixelRatio = 1`.
    fn tester_of_size(size: Size) -> WidgetTester {
        WidgetTester::with_view(
            waft_test::TestPlatform::new().on(TargetPlatform::Android),
            waft_test::TestView::with_pixel_ratio(size.width(), size.height(), 1.0),
        )
    }

    fn theme_of(tester: &WidgetTester, material_type: MaterialType) -> ThemeData {
        match material_type {
            MaterialType::Material2 => m2(&tester.cell.borrow()),
            MaterialType::Material3 => m3(&tester.cell.borrow()),
        }
    }

    /// Dart's `mediaQueryBoilerplate`: an app with a button that shows the time picker.
    fn media_query_boilerplate(tester: &mut WidgetTester, size: Size, boilerplate: Boilerplate) {
        let theme = theme_of(tester, boilerplate.material_type);
        let launcher = {
            let boilerplate = boilerplate.clone();
            Builder::new(move |_app, context| {
                let boilerplate = boilerplate.clone();
                TextButton::new(
                    Some(Listener::new(move |app| {
                        drop(show_time_picker(
                            app,
                            context,
                            boilerplate.initial_time,
                            None,
                            true,
                            None,
                            None,
                            true,
                            boilerplate.entry_mode,
                            None,
                            None,
                            boilerplate.help_text.clone(),
                            boilerplate.error_invalid_text.clone(),
                            boilerplate.hour_label_text.clone(),
                            boilerplate.minute_label_text.clone(),
                            None,
                            boilerplate.on_entry_mode_change.clone(),
                            boilerplate.orientation,
                            None,
                            None,
                            false,
                        ));
                    })),
                    Some(Text::new("X")),
                )
                .into_widget()
            })
        };
        let root = crate::app::MaterialApp::new()
            .theme(theme)
            .builder(move |_app, _context, child| {
                MediaQuery::new(
                    MediaQueryData::new()
                        .always_use_24_hour_format(boilerplate.always_use_24_hour_format)
                        .text_scaler(boilerplate.text_scaler.clone())
                        .accessible_navigation(boilerplate.accessible_navigation)
                        .size(size),
                    child.expect("a MaterialApp child").clone(),
                )
                .into_widget()
            })
            .home(launcher);
        let root = IntoWidget::into_widget(root);
        tester.pump_widget(root);
        if boilerplate.tap_button {
            tester.tap(find::text("X"));
        }
        tester.pump_and_settle();
    }

    fn dialog_material_size(tester: &WidgetTester) -> Size {
        let material = tester.element(
            find::descendant(find::by_type::<Dialog>().at(0), find::by_type::<Material>()).first(),
        );
        tester.get_size(material)
    }

    #[test]
    fn material2_dialog_size_dial_mode() {
        test_widgets(|_| {
            const TIME_PICKER_PORTRAIT_SIZE: Size = Size::new(310.0, 468.0);
            const TIME_PICKER_LANDSCAPE_SIZE: Size = Size::new(524.0, 342.0);
            const TIME_PICKER_LANDSCAPE_SIZE_M2: Size = Size::new(508.0, 300.0);
            let padding = EdgeInsets::from_ltrb(8.0, 18.0, 8.0, 8.0);

            // portrait
            let size = Size::new(800.0, 800.5);
            let mut tester = tester_of_size(size);
            let tester = &mut tester;
            media_query_boilerplate(tester, size, Boilerplate::new(MaterialType::Material2));

            let width = TIME_PICKER_PORTRAIT_SIZE.width() + padding.horizontal();
            let height = TIME_PICKER_PORTRAIT_SIZE.height() + padding.vertical();
            assert_eq!(dialog_material_size(tester), Size::new(width, height));

            tester.tap(find::text(OK_STRING)); // dismiss the dialog
            tester.pump_and_settle();

            // landscape
            let size = Size::new(800.5, 800.0);
            let mut tester = tester_of_size(size);
            let tester = &mut tester;
            let mut boilerplate = Boilerplate::new(MaterialType::Material2);
            boilerplate.always_use_24_hour_format = true;
            media_query_boilerplate(tester, size, boilerplate);

            let width = TIME_PICKER_LANDSCAPE_SIZE.width() + padding.horizontal();
            let height = TIME_PICKER_LANDSCAPE_SIZE_M2.height() + padding.vertical();
            assert_eq!(dialog_material_size(tester), Size::new(width, height));
        });
    }

    fn material_dialog_default_size() -> Size {
        Size::new(800.0, 600.0)
    }

    /// Dart's `startPicker`'s parameters.
    #[derive(Clone)]
    struct StartPicker {
        entry_mode: TimePickerEntryMode,
        material_type: MaterialType,
        empty_initial_input: bool,
        always_use_24_hour_format: Option<bool>,
    }

    impl StartPicker {
        fn new(material_type: MaterialType) -> StartPicker {
            StartPicker {
                entry_mode: TimePickerEntryMode::Dial,
                material_type,
                empty_initial_input: false,
                always_use_24_hour_format: None,
            }
        }
    }

    type PickerResult =
        Rc<RefCell<Option<waft_foundation::CompleterFuture<waft_widgets::RouteResult>>>>;

    /// What the picker completed with, as Dart's `onChanged(await showTimePicker(...))` sees it.
    fn picked_time(result: &PickerResult) -> Option<TimeOfDay> {
        result
            .borrow()
            .as_ref()
            .and_then(|future| future.peek())
            .flatten()
            .and_then(|value| value.downcast_ref::<TimeOfDay>().copied())
    }

    /// Dart's `startPicker`: an app with a button that shows the time picker at 7:00, which is
    /// tapped. Returns the center of the dial, if the picker shows one.
    fn start_picker(
        tester: &mut WidgetTester,
        result: &PickerResult,
        parameters: StartPicker,
    ) -> Option<Offset> {
        let theme = theme_of(tester, parameters.material_type);
        let result_slot = Rc::clone(result);
        let launcher = {
            let parameters = parameters.clone();
            Builder::new(move |_app, _context| {
                let parameters = parameters.clone();
                let result_slot = Rc::clone(&result_slot);
                Material::new()
                    .child(
                        waft_widgets::Center::new().child(Builder::new(move |_app, context| {
                            let parameters = parameters.clone();
                            let result_slot = Rc::clone(&result_slot);
                            crate::elevated_button::ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    let future = show_time_picker(
                                        app,
                                        context,
                                        TimeOfDay::new(7, 0),
                                        None,
                                        true,
                                        None,
                                        None,
                                        true,
                                        parameters.entry_mode,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        parameters.empty_initial_input,
                                    );
                                    *result_slot.borrow_mut() = Some(future);
                                })),
                                Some(Text::new("X")),
                            )
                            .into_widget()
                        })),
                    )
                    .into_widget()
            })
        };
        let always_use_24_hour_format = parameters.always_use_24_hour_format;
        let mut app = crate::app::MaterialApp::new().theme(theme).home(launcher);
        if let Some(always_use_24_hour_format) = always_use_24_hour_format {
            app = app.builder(move |app, context, child| {
                MediaQuery::new(
                    MediaQuery::of(app, context)
                        .copy_with()
                        .always_use_24_hour_format(always_use_24_hour_format),
                    child.expect("a MaterialApp child").clone(),
                )
                .into_widget()
            });
        }
        tester.pump_widget(IntoWidget::into_widget(app));
        tester.tap(find::text("X"));
        tester.pump_and_settle_by(Duration::from_secs(1));
        (parameters.entry_mode == TimePickerEntryMode::Dial).then(|| {
            let dial = tester.element(find::by_type::<Dial>().at(0));
            tester.get_center(dial)
        })
    }

    /// Dart's `finishPicker`.
    fn finish_picker(tester: &mut WidgetTester) {
        tester.tap(find::text(OK_STRING));
        tester.pump_and_settle_by(Duration::from_secs(1));
    }

    fn pickers() -> [MaterialType; 2] {
        [MaterialType::Material2, MaterialType::Material3]
    }

    /// Dart's `findDialPaint`: the `CustomPaint` under the private `_Dial`, which
    /// time_picker_theme_test also finds by the widget's type name.
    pub(crate) fn dial_paint(tester: &WidgetTester) -> AnyElement {
        tester.element(
            find::descendant(
                find::by_type::<Dial>().at(0),
                find::by_type::<waft_widgets::CustomPaint>(),
            )
            .first(),
        )
    }

    /// Dart's `dialPainter.primaryLabels.first.painter.text.style` and
    /// `selectedLabels.first.painter.text.style`, which time_picker_theme_test reads through
    /// `dynamic`.
    pub(crate) fn dial_label_styles(tester: &WidgetTester) -> (TextStyle, TextStyle) {
        let painter = dial_painter(tester);
        let painter = painter.borrow();
        (
            painter
                .first_primary_label_style()
                .expect("a primary label"),
            painter
                .first_selected_label_style()
                .expect("a selected label"),
        )
    }

    fn dial_painter(tester: &WidgetTester) -> Rc<RefCell<DialPainter>> {
        tester.widget(dial_paint(tester), |paint: &waft_widgets::CustomPaint| {
            let painter = paint.painter.as_ref().expect("the dial paints");
            Rc::clone(
                &painter
                    .as_any()
                    .downcast_ref::<SharedDialPainter>()
                    .expect("a dial painter")
                    .0,
            )
        })
    }

    #[test]
    fn material3_dialog_size_dial_mode() {
        test_widgets(|_| {
            const TIME_PICKER_PORTRAIT_SIZE: Size = Size::new(310.0, 468.0);
            const TIME_PICKER_LANDSCAPE_SIZE: Size = Size::new(524.0, 342.0);
            let padding = EdgeInsets::all(24.0);

            // portrait
            let size = Size::new(800.0, 800.5);
            let mut tester = tester_of_size(size);
            let tester = &mut tester;
            media_query_boilerplate(tester, size, Boilerplate::new(MaterialType::Material3));

            let width = TIME_PICKER_PORTRAIT_SIZE.width() + padding.horizontal();
            let height = TIME_PICKER_PORTRAIT_SIZE.height() + padding.vertical();
            assert_eq!(dialog_material_size(tester), Size::new(width, height));

            tester.tap(find::text(OK_STRING)); // dismiss the dialog
            tester.pump_and_settle();

            // landscape
            let size = Size::new(800.5, 800.0);
            let mut tester = tester_of_size(size);
            let tester = &mut tester;
            let mut boilerplate = Boilerplate::new(MaterialType::Material3);
            boilerplate.always_use_24_hour_format = true;
            media_query_boilerplate(tester, size, boilerplate);

            let width = TIME_PICKER_LANDSCAPE_SIZE.width() + padding.horizontal();
            let height = TIME_PICKER_LANDSCAPE_SIZE.height() + padding.vertical();
            assert_eq!(dialog_material_size(tester), Size::new(width, height));
        });
    }

    #[test]
    fn material2_dialog_size_input_mode() {
        test_widgets(|tester| {
            const TIME_PICKER_INPUT_SIZE: Size = Size::new(312.0, 252.0);
            const DAY_PERIOD_PORTRAIT_SIZE: Size = Size::new(52.0, 80.0);
            let padding = EdgeInsets::from_ltrb(8.0, 18.0, 8.0, 8.0);
            let height = TIME_PICKER_INPUT_SIZE.height() + padding.vertical();

            let size = material_dialog_default_size();
            let mut boilerplate = Boilerplate::new(MaterialType::Material2);
            boilerplate.entry_mode = TimePickerEntryMode::Input;
            media_query_boilerplate(tester, size, boilerplate.clone());

            let width = TIME_PICKER_INPUT_SIZE.width() + padding.horizontal();
            let dialog_size = dialog_material_size(tester);
            assert_eq!(dialog_size.width(), width);
            assert!(dialog_size.height() < height);

            tester.tap(find::text(OK_STRING)); // dismiss the dialog
            tester.pump_and_settle();

            boilerplate.always_use_24_hour_format = true;
            media_query_boilerplate(tester, size, boilerplate);
            let width = TIME_PICKER_INPUT_SIZE.width() - DAY_PERIOD_PORTRAIT_SIZE.width() - 12.0
                + padding.horizontal()
                + 16.0;
            let dialog_size = dialog_material_size(tester);
            assert_eq!(dialog_size.width(), width);
            assert!(dialog_size.height() < height);
        });
    }

    #[test]
    fn material3_dialog_size_input_mode() {
        test_widgets(|tester| {
            const TIME_PICKER_MIN_INPUT_SIZE: Size = Size::new(312.0, 252.0);
            const DAY_PERIOD_PORTRAIT_SIZE: Size = Size::new(52.0, 80.0);
            let padding = EdgeInsets::all(24.0);
            let text_scale_factor = 1.0;
            let height =
                TIME_PICKER_MIN_INPUT_SIZE.height() * text_scale_factor + padding.vertical();

            let size = material_dialog_default_size();
            let mut boilerplate = Boilerplate::new(MaterialType::Material3);
            boilerplate.entry_mode = TimePickerEntryMode::Input;
            media_query_boilerplate(tester, size, boilerplate.clone());

            let width = TIME_PICKER_MIN_INPUT_SIZE.width() - 32.0 + padding.horizontal();
            let dialog_size = dialog_material_size(tester);
            assert_eq!(dialog_size.width(), width);
            assert!(dialog_size.height() < height);

            tester.tap(find::text(OK_STRING)); // dismiss the dialog
            tester.pump_and_settle();

            boilerplate.always_use_24_hour_format = true;
            media_query_boilerplate(tester, size, boilerplate);
            let width =
                TIME_PICKER_MIN_INPUT_SIZE.width() - DAY_PERIOD_PORTRAIT_SIZE.width() - 12.0
                    + padding.horizontal();
            let dialog_size = dialog_material_size(tester);
            assert_eq!(dialog_size.width(), width);
            assert!(dialog_size.height() < height);
        });
    }

    #[test]
    fn material2_respects_media_query_data_always_use_24_hour_format_true() {
        test_widgets(|tester| {
            let size = material_dialog_default_size();
            let mut boilerplate = Boilerplate::new(MaterialType::Material2);
            boilerplate.always_use_24_hour_format = true;
            media_query_boilerplate(tester, size, boilerplate);

            let labels_00_to_22: Vec<String> =
                (0..12).map(|index| format!("{:02}", index * 2)).collect();
            let painter = dial_painter(tester);
            assert_eq!(painter.borrow().primary_label_texts(), labels_00_to_22);
            assert_eq!(painter.borrow().selected_label_texts(), labels_00_to_22);
        });
    }

    #[test]
    fn material3_respects_media_query_data_always_use_24_hour_format_true() {
        test_widgets(|tester| {
            let size = material_dialog_default_size();
            let mut boilerplate = Boilerplate::new(MaterialType::Material3);
            boilerplate.always_use_24_hour_format = true;
            media_query_boilerplate(tester, size, boilerplate);

            let labels_00_to_23: Vec<String> = (0..24)
                .map(|index| {
                    if index == 0 {
                        "00".to_string()
                    } else {
                        index.to_string()
                    }
                })
                .collect();
            let inner_0_to_23: Vec<bool> = (0..24).map(|index| index >= 12).collect();

            let painter = dial_painter(tester);
            assert_eq!(painter.borrow().primary_label_texts(), labels_00_to_23);
            assert_eq!(painter.borrow().primary_label_inner(), inner_0_to_23);
            assert_eq!(painter.borrow().selected_label_texts(), labels_00_to_23);
            assert_eq!(painter.borrow().selected_label_inner(), inner_0_to_23);
        });
    }

    #[test]
    fn dial_tap_select_an_hour() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                tester.tap_at(Offset::new(center.dx(), center.dy() - 50.0), 1); // 12:00 AM
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(0, 0)));

                center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                tester.tap_at(Offset::new(center.dx() + 50.0, center.dy()), 1);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(3, 0)));

                center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                tester.tap_at(Offset::new(center.dx(), center.dy() + 50.0), 1);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(6, 0)));

                center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                tester.tap_at(Offset::new(center.dx(), center.dy() + 50.0), 1);
                tester.tap_at(Offset::new(center.dx() - 50.0, center.dy()), 1);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(9, 0)));
            });
        }
    }

    fn drag(tester: &mut WidgetTester, from: Offset, to: Offset) {
        let mut gesture = TestPointer::touch(1);
        tester.down(&mut gesture, from);
        tester.move_to(&mut gesture, to, true);
        tester.up(&gesture);
    }

    #[test]
    fn dial_drag_select_an_hour() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                let hour0 = Offset::new(center.dx(), center.dy() - 50.0); // 12:00 AM
                let hour3 = Offset::new(center.dx() + 50.0, center.dy());
                let hour6 = Offset::new(center.dx(), center.dy() + 50.0);
                let hour9 = Offset::new(center.dx() - 50.0, center.dy());

                drag(tester, hour3, hour0);
                finish_picker(tester);
                assert_eq!(picked_time(&result).map(|time| time.hour), Some(0));

                assert_eq!(
                    start_picker(tester, &result, StartPicker::new(material_type)),
                    Some(center)
                );
                drag(tester, hour0, hour3);
                finish_picker(tester);
                assert_eq!(picked_time(&result).map(|time| time.hour), Some(3));

                assert_eq!(
                    start_picker(tester, &result, StartPicker::new(material_type)),
                    Some(center)
                );
                drag(tester, hour3, hour6);
                finish_picker(tester);
                assert_eq!(picked_time(&result).map(|time| time.hour), Some(6));

                assert_eq!(
                    start_picker(tester, &result, StartPicker::new(material_type)),
                    Some(center)
                );
                drag(tester, hour6, hour9);
                finish_picker(tester);
                assert_eq!(picked_time(&result).map(|time| time.hour), Some(9));
            });
        }
    }

    #[test]
    fn dial_tap_select_switches_from_hour_to_minute() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                let hour6 = Offset::new(center.dx(), center.dy() + 50.0); // 6:00
                let min45 = Offset::new(center.dx() - 50.0, center.dy()); // 45 mins (or 9:00 hours)

                tester.tap_at(hour6, 1);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(min45, 1);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(6, 45)));
            });
        }
    }

    #[test]
    fn dial_drag_select_switches_from_hour_to_minute() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                let hour3 = Offset::new(center.dx() + 50.0, center.dy());
                let hour6 = Offset::new(center.dx(), center.dy() + 50.0);
                let hour9 = Offset::new(center.dx() - 50.0, center.dy());

                drag(tester, hour6, hour9);
                tester.pump_by(Duration::from_millis(50));
                drag(tester, hour6, hour3);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(9, 15)));
            });
        }
    }

    #[test]
    fn dial_tap_select_rounds_down_to_nearest_5_minute_increment() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                let hour6 = Offset::new(center.dx(), center.dy() + 50.0); // 6:00
                let min46 = Offset::new(center.dx() - 50.0, center.dy() - 5.0); // 46 mins

                tester.tap_at(hour6, 1);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(min46, 1);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(6, 45)));
            });
        }
    }

    #[test]
    fn dial_tap_select_rounds_up_to_nearest_5_minute_increment() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let center =
                    start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
                let hour6 = Offset::new(center.dx(), center.dy() + 50.0); // 6:00
                let min48 = Offset::new(center.dx() - 50.0, center.dy() - 15.0); // 48 mins

                tester.tap_at(hour6, 1);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(min48, 1);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(6, 50)));
            });
        }
    }

    fn only_text(tester: &WidgetTester, text: &str) -> AnyElement {
        let found = find::text(text).evaluate(tester);
        assert_eq!(found.len(), 1, "one {text:?}");
        found[0]
    }

    #[test]
    fn material2_widgets_have_correct_label_capitalization() {
        test_widgets(|tester| {
            let result: PickerResult = Rc::default();
            start_picker(tester, &result, StartPicker::new(MaterialType::Material2));
            only_text(tester, "SELECT TIME");
            only_text(tester, "CANCEL");
        });
    }

    #[test]
    fn material3_widgets_have_correct_label_capitalization() {
        test_widgets(|tester| {
            let result: PickerResult = Rc::default();
            start_picker(tester, &result, StartPicker::new(MaterialType::Material3));
            only_text(tester, "Select time");
            only_text(tester, "Cancel");
        });
    }

    #[test]
    fn material2_widgets_have_correct_label_capitalization_in_input_mode() {
        test_widgets(|tester| {
            let result: PickerResult = Rc::default();
            let mut parameters = StartPicker::new(MaterialType::Material2);
            parameters.entry_mode = TimePickerEntryMode::Input;
            start_picker(tester, &result, parameters);
            only_text(tester, "ENTER TIME");
            only_text(tester, "CANCEL");
        });
    }

    #[test]
    fn material3_widgets_have_correct_label_capitalization_in_input_mode() {
        test_widgets(|tester| {
            let result: PickerResult = Rc::default();
            let mut parameters = StartPicker::new(MaterialType::Material3);
            parameters.entry_mode = TimePickerEntryMode::Input;
            start_picker(tester, &result, parameters);
            only_text(tester, "Enter time");
            only_text(tester, "Cancel");
        });
    }

    #[test]
    fn respects_media_query_data_always_use_24_hour_format_false() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                media_query_boilerplate(tester, size, Boilerplate::new(material_type));
                let labels_12_to_11: Vec<String> = [
                    "12", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11",
                ]
                .iter()
                .map(ToString::to_string)
                .collect();

                let painter = dial_painter(tester);
                assert_eq!(painter.borrow().primary_label_texts(), labels_12_to_11);
                assert_eq!(painter.borrow().selected_label_texts(), labels_12_to_11);
            });
        }
    }

    fn day_period_input_padding_orientation(tester: &WidgetTester) -> Orientation {
        let element = tester.element(find::by_type::<DayPeriodInputPadding>().at(0));
        tester.widget(element, |padding: &DayPeriodInputPadding| {
            padding.orientation
        })
    }

    #[test]
    fn when_change_orientation_should_reflect_in_render_objects() {
        for material_type in pickers() {
            test_widgets(|_| {
                // portrait
                let size = Size::new(800.0, 800.5);
                let mut tester = tester_of_size(size);
                let tester = &mut tester;
                media_query_boilerplate(tester, size, Boilerplate::new(material_type));
                assert_eq!(
                    day_period_input_padding_orientation(tester),
                    Orientation::Portrait
                );

                // landscape
                let size = Size::new(800.5, 800.0);
                let mut tester = tester_of_size(size);
                let tester = &mut tester;
                media_query_boilerplate(tester, size, Boilerplate::new(material_type));
                assert_eq!(
                    day_period_input_padding_orientation(tester),
                    Orientation::Landscape
                );
            });
        }
    }

    #[test]
    fn setting_orientation_should_override_media_query_orientation() {
        for material_type in pickers() {
            test_widgets(|_| {
                // portrait media query
                let size = Size::new(800.0, 800.5);
                let mut tester = tester_of_size(size);
                let tester = &mut tester;
                let mut boilerplate = Boilerplate::new(material_type);
                boilerplate.orientation = Some(Orientation::Landscape);
                media_query_boilerplate(tester, size, boilerplate);
                assert_eq!(
                    day_period_input_padding_orientation(tester),
                    Orientation::Landscape
                );
            });
        }
    }

    /// Dart's `PickerObserver.pickerCount`: how many time pickers the navigator shows.
    fn picker_count(tester: &WidgetTester) -> usize {
        find::by_type::<TimePickerDialog>().evaluate(tester).len()
    }

    /// An app with a button that shows a time picker with the given arguments.
    fn open_picker_app(
        tester: &mut WidgetTester,
        show: impl Fn(&mut waft_foundation::App, BuildContext) + 'static,
    ) {
        let theme = m3(&tester.cell.borrow());
        let show = Rc::new(show);
        let home = Material::new()
            .child(
                waft_widgets::Center::new().child(Builder::new(move |_app, context| {
                    let show = Rc::clone(&show);
                    crate::elevated_button::ElevatedButton::new(
                        Some(Listener::new(move |app| show(app, context))),
                        Some(Text::new("X")),
                    )
                    .into_widget()
                })),
            )
            .into_widget();
        tester.pump_widget(material_app(theme, home));
    }

    fn show_default(
        app: &mut waft_foundation::App,
        context: BuildContext,
    ) -> waft_foundation::CompleterFuture<waft_widgets::RouteResult> {
        show_time_picker(
            app,
            context,
            TimeOfDay::new(7, 0),
            None,
            true,
            None,
            None,
            true,
            TimePickerEntryMode::Dial,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        )
    }

    #[test]
    fn barrier_is_dismissible_with_default_parameter() {
        test_widgets(|tester| {
            open_picker_app(tester, |app, context| drop(show_default(app, context)));

            // Open the dialog.
            tester.tap(find::text("X"));
            tester.pump_and_settle();
            assert_eq!(picker_count(tester), 1);

            // Tap on the barrier.
            tester.tap_at(Offset::new(10.0, 10.0), 1);
            tester.pump_and_settle();
            assert_eq!(picker_count(tester), 0);
        });
    }

    #[test]
    fn barrier_is_not_dismissible_with_barrier_dismissible_is_false() {
        test_widgets(|tester| {
            open_picker_app(tester, |app, context| {
                drop(show_time_picker(
                    app,
                    context,
                    TimeOfDay::new(7, 0),
                    None,
                    false,
                    None,
                    None,
                    true,
                    TimePickerEntryMode::Dial,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                ));
            });

            // Open the dialog.
            tester.tap(find::text("X"));
            tester.pump_and_settle();
            assert_eq!(picker_count(tester), 1);

            // Tap on the barrier, which shouldn't do anything this time.
            tester.tap_at(Offset::new(10.0, 10.0), 1);
            tester.pump_and_settle();
            assert_eq!(picker_count(tester), 1);
        });
    }

    fn last_modal_barrier<R>(
        tester: &WidgetTester,
        read: impl FnOnce(&waft_widgets::ModalBarrier) -> R,
    ) -> R {
        let barrier = *find::by_type::<waft_widgets::ModalBarrier>()
            .evaluate(tester)
            .last()
            .expect("a ModalBarrier");
        tester.widget(barrier, read)
    }

    #[test]
    fn barrier_color() {
        test_widgets(|tester| {
            open_picker_app(tester, |app, context| drop(show_default(app, context)));

            // Open the dialog.
            tester.tap(find::text("X"));
            tester.pump_and_settle();
            assert_eq!(
                last_modal_barrier(tester, |barrier| barrier.color),
                Some(crate::colors::Colors::BLACK54)
            );

            // Dismiss the dialog.
            tester.tap_at(Offset::new(10.0, 10.0), 1);

            open_picker_app(tester, |app, context| {
                drop(show_time_picker(
                    app,
                    context,
                    TimeOfDay::new(7, 0),
                    None,
                    true,
                    Some(crate::colors::Colors::PINK.primary()),
                    None,
                    true,
                    TimePickerEntryMode::Dial,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                ));
            });

            // Open the dialog.
            tester.tap(find::text("X"));
            tester.pump_and_settle();
            assert_eq!(
                last_modal_barrier(tester, |barrier| barrier.color),
                Some(crate::colors::Colors::PINK.primary())
            );
        });
    }

    #[test]
    fn barrier_label() {
        test_widgets(|tester| {
            open_picker_app(tester, |app, context| {
                drop(show_time_picker(
                    app,
                    context,
                    TimeOfDay::new(7, 0),
                    None,
                    true,
                    None,
                    Some("Custom Label".to_string()),
                    true,
                    TimePickerEntryMode::Dial,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                ));
            });

            // Open the dialog.
            tester.tap(find::text("X"));
            tester.pump_and_settle();
            assert_eq!(
                last_modal_barrier(tester, |barrier| barrier.semantics_label.clone()),
                Some("Custom Label".to_string())
            );
        });
    }

    #[test]
    fn builder_parameter() {
        test_widgets(|tester| {
            let build_frame =
                |tester: &mut WidgetTester, text_direction: waft_painting::TextDirection| {
                    open_picker_app(tester, move |app, context| {
                        let builder: waft_widgets::TransitionBuilder =
                            Rc::new(move |_app, _context, child| {
                                waft_widgets::Directionality::new(
                                    text_direction,
                                    child.expect("a dialog child").clone(),
                                )
                                .into_widget()
                            });
                        drop(show_time_picker(
                            app,
                            context,
                            TimeOfDay::new(7, 0),
                            Some(builder),
                            true,
                            None,
                            None,
                            true,
                            TimePickerEntryMode::Dial,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            false,
                        ));
                    });
                };

            build_frame(tester, waft_painting::TextDirection::Ltr);
            tester.tap(find::text("X"));
            tester.pump_and_settle();
            let ok = only_text(tester, OK_STRING);
            let ltr_ok_right = tester.get_rect(ok).right;

            tester.tap(find::text(OK_STRING)); // dismiss the dialog
            tester.pump_and_settle();

            build_frame(tester, waft_painting::TextDirection::Rtl);
            tester.tap(find::text("X"));
            tester.pump_and_settle();

            // Verify that the time picker is being laid out RTL.
            // We expect the left edge of the 'OK' button in the RTL
            // layout to match the gap between right edge of the 'OK'
            // button and the right edge of the 800 wide view.
            let ok = only_text(tester, OK_STRING);
            let rtl_ok_left = tester.get_rect(ok).left;
            // Dart's `rtlOkLeft == 800 - ltrOkRight`, compared at the f32 rounding of the 800
            // wide view the two edges add up to.
            assert_f32_equals(rtl_ok_left + ltr_ok_right, 800.0);
        });
    }

    #[test]
    fn optional_text_parameters_are_utilized() {
        test_widgets(|tester| {
            open_picker_app(tester, |app, context| {
                drop(show_time_picker(
                    app,
                    context,
                    TimeOfDay::new(7, 0),
                    None,
                    true,
                    None,
                    None,
                    true,
                    TimePickerEntryMode::Dial,
                    Some("Custom Cancel".to_string()),
                    Some("Custom OK".to_string()),
                    Some("Custom Help".to_string()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                ));
            });

            // Open the picker.
            tester.tap(find::text("X"));
            tester.pump_and_settle_by(Duration::from_secs(1));

            only_text(tester, "Custom Cancel");
            only_text(tester, "Custom OK");
            only_text(tester, "Custom Help");
        });
    }

    fn open_directional_picker(
        tester: &mut WidgetTester,
        text_direction: waft_painting::TextDirection,
    ) {
        open_picker_app(tester, move |app, context| {
            let builder: waft_widgets::TransitionBuilder = Rc::new(move |_app, _context, child| {
                waft_widgets::Directionality::new(
                    text_direction,
                    child.expect("a dialog child").clone(),
                )
                .into_widget()
            });
            drop(show_time_picker(
                app,
                context,
                TimeOfDay::new(7, 0),
                Some(builder),
                true,
                None,
                None,
                true,
                TimePickerEntryMode::Dial,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                false,
            ));
        });
    }

    #[test]
    fn material2_ok_cancel_button_and_help_text_layout() {
        test_widgets(|tester| {
            const SELECT_TIME_STRING: &str = "SELECT TIME";
            const CANCEL_STRING: &str = "CANCEL";
            let m2_app = |tester: &mut WidgetTester,
                          text_direction: waft_painting::TextDirection| {
                let theme = m2(&tester.cell.borrow());
                let home = Material::new()
                    .child(
                        waft_widgets::Center::new().child(Builder::new(move |_app, context| {
                            crate::elevated_button::ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    let builder: waft_widgets::TransitionBuilder =
                                        Rc::new(move |_app, _context, child| {
                                            waft_widgets::Directionality::new(
                                                text_direction,
                                                child.expect("a dialog child").clone(),
                                            )
                                            .into_widget()
                                        });
                                    drop(show_time_picker(
                                        app,
                                        context,
                                        TimeOfDay::new(7, 0),
                                        Some(builder),
                                        true,
                                        None,
                                        None,
                                        true,
                                        TimePickerEntryMode::Dial,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        false,
                                    ));
                                })),
                                Some(Text::new("X")),
                            )
                            .into_widget()
                        })),
                    )
                    .into_widget();
                tester.pump_widget(material_app(theme, home));
            };

            m2_app(tester, waft_painting::TextDirection::Ltr);
            tester.tap(find::text("X"));
            tester.pump_and_settle();

            let select_time = tester.get_rect(find::text(SELECT_TIME_STRING));
            assert_eq!(select_time.top_left(), Offset::new(154.0, 155.0));
            assert_eq!(select_time.bottom_right(), Offset::new(280.5, 165.0));
            assert_eq!(tester.get_rect(find::text(OK_STRING)).right, 644.0);
            assert_eq!(tester.get_rect(find::text(OK_STRING)).left, 616.0);
            assert_eq!(tester.get_rect(find::text(CANCEL_STRING)).right, 582.0);

            tester.tap(find::text(OK_STRING));
            tester.pump_and_settle();

            m2_app(tester, waft_painting::TextDirection::Rtl);
            tester.tap(find::text("X"));
            tester.pump_and_settle();

            let select_time = tester.get_rect(find::text(SELECT_TIME_STRING));
            assert_eq!(select_time.top_left(), Offset::new(519.5, 155.0));
            assert_eq!(select_time.bottom_right(), Offset::new(646.0, 165.0));
            assert_eq!(tester.get_rect(find::text(OK_STRING)).left, 156.0);
            assert_eq!(tester.get_rect(find::text(OK_STRING)).right, 184.0);
            assert_eq!(tester.get_rect(find::text(CANCEL_STRING)).left, 218.0);

            tester.tap(find::text(OK_STRING));
            tester.pump_and_settle();
        });
    }

    #[test]
    fn material3_ok_cancel_button_and_help_text_layout() {
        test_widgets(|tester| {
            const SELECT_TIME_STRING: &str = "Select time";
            const CANCEL_STRING: &str = "Cancel";

            open_directional_picker(tester, waft_painting::TextDirection::Ltr);
            tester.tap(find::text("X"));
            tester.pump_and_settle();

            let select_time = tester.get_rect(find::text(SELECT_TIME_STRING));
            assert_eq!(select_time.top_left(), Offset::new(138.0, 129.0));
            assert_eq!(select_time.bottom_right(), Offset::new(294.75, 149.0));
            assert_more_or_less_equals_within(
                tester.get_rect(find::text(OK_STRING)).left,
                615.9,
                0.001,
            );
            assert_f32_equals(tester.get_rect(find::text(CANCEL_STRING)).right, 578.0);

            tester.tap(find::text(OK_STRING));
            tester.pump_and_settle();

            open_directional_picker(tester, waft_painting::TextDirection::Rtl);
            tester.tap(find::text("X"));
            tester.pump_and_settle();

            let select_time = tester.get_rect(find::text(SELECT_TIME_STRING));
            assert_eq!(select_time.top_left(), Offset::new(505.25, 129.0));
            assert_eq!(select_time.bottom_right(), Offset::new(662.0, 149.0));
            assert_more_or_less_equals_within(
                tester.get_rect(find::text(OK_STRING)).left,
                155.9,
                0.001,
            );
            assert_more_or_less_equals_within(
                tester.get_rect(find::text(OK_STRING)).right,
                184.1,
                0.001,
            );
            assert_f32_equals(tester.get_rect(find::text(CANCEL_STRING)).left, 222.0);

            tester.tap(find::text(OK_STRING));
            tester.pump_and_settle();
        });
    }

    #[test]
    fn text_scale_affects_certain_elements_and_not_others() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = Boilerplate::new(material_type);
                boilerplate.initial_time = TimeOfDay::new(7, 41);
                media_query_boilerplate(tester, size, boilerplate.clone());

                let minutes_display_height = tester.get_rect(find::text("41")).height();
                let am_height = tester.get_rect(find::text(AM_STRING)).height();

                tester.tap(find::text(OK_STRING)); // dismiss the dialog
                tester.pump_and_settle();

                // Verify that the time display is not affected by text scale.
                boilerplate.text_scaler = TextScaler::linear(2.0);
                media_query_boilerplate(tester, size, boilerplate.clone());

                let am_height_2x = tester.get_rect(find::text(AM_STRING)).height();
                assert_eq!(
                    tester.get_rect(find::text("41")).height(),
                    minutes_display_height
                );
                assert_eq!(am_height_2x, f64::min(38.0, am_height * 2.0));

                tester.tap(find::text(OK_STRING)); // dismiss the dialog
                tester.pump_and_settle();

                // Verify that text scale for AM/PM is at most 2x.
                boilerplate.text_scaler = TextScaler::linear(3.0);
                media_query_boilerplate(tester, size, boilerplate);

                assert_eq!(
                    tester.get_rect(find::text("41")).height(),
                    minutes_display_height
                );
                assert_eq!(
                    tester.get_rect(find::text(AM_STRING)).height(),
                    f64::min(38.0, am_height * 2.0)
                );
            });
        }
    }

    /// Records the haptic feedback the platform is asked for: Dart's `FeedbackTester`.
    #[derive(Default)]
    struct FeedbackTester {
        haptics: RefCell<Vec<waft_embedder::HapticFeedbackType>>,
    }

    impl waft_embedder::Haptics for FeedbackTester {
        fn feedback(&self, kind: waft_embedder::HapticFeedbackType) {
            self.haptics.borrow_mut().push(kind);
        }
    }

    impl FeedbackTester {
        fn haptic_count(&self) -> usize {
            self.haptics.borrow().len()
        }
    }

    fn feedback_tester() -> (WidgetTester, Rc<FeedbackTester>) {
        let feedback = Rc::new(FeedbackTester::default());
        let platform = waft_test::TestPlatform::new()
            .on(TargetPlatform::Android)
            .with_haptics(Rc::clone(&feedback) as Rc<dyn waft_embedder::Haptics>);
        (WidgetTester::with_platform(platform), feedback)
    }

    const FAST_FEEDBACK_INTERVAL: Duration = Duration::from_millis(10);
    const SLOW_FEEDBACK_INTERVAL: Duration = Duration::from_millis(200);

    fn haptic_case(case: impl Fn(&mut WidgetTester, Offset, &FeedbackTester)) {
        for material_type in pickers() {
            let (mut tester, feedback) = feedback_tester();
            let tester = &mut tester;
            let result: PickerResult = Rc::default();
            let center =
                start_picker(tester, &result, StartPicker::new(material_type)).expect("a dial");
            case(tester, center, &feedback);
        }
    }

    #[test]
    fn dial_haptic_feedback_tap_select_vibrates_once() {
        test_widgets(|_| {
            haptic_case(|tester, center, feedback| {
                tester.tap_at(Offset::new(center.dx(), center.dy() - 50.0), 1);
                finish_picker(tester);
                assert_eq!(feedback.haptic_count(), 1);
            });
        });
    }

    #[test]
    fn dial_haptic_feedback_quick_successive_tap_selects_vibrate_once() {
        test_widgets(|_| {
            haptic_case(|tester, center, feedback| {
                tester.tap_at(Offset::new(center.dx(), center.dy() - 50.0), 1);
                tester.pump_by(FAST_FEEDBACK_INTERVAL);
                tester.tap_at(Offset::new(center.dx(), center.dy() + 50.0), 1);
                finish_picker(tester);
                assert_eq!(feedback.haptic_count(), 1);
            });
        });
    }

    #[test]
    fn dial_haptic_feedback_slow_successive_tap_selects_vibrate_once_per_tap() {
        test_widgets(|_| {
            haptic_case(|tester, center, feedback| {
                tester.tap_at(Offset::new(center.dx(), center.dy() - 50.0), 1);
                tester.pump_by(SLOW_FEEDBACK_INTERVAL);
                tester.tap_at(Offset::new(center.dx(), center.dy() + 50.0), 1);
                tester.pump_by(SLOW_FEEDBACK_INTERVAL);
                tester.tap_at(Offset::new(center.dx(), center.dy() - 50.0), 1);
                finish_picker(tester);
                assert_eq!(feedback.haptic_count(), 3);
            });
        });
    }

    #[test]
    fn dial_haptic_feedback_drag_select_vibrates_once() {
        test_widgets(|_| {
            haptic_case(|tester, center, feedback| {
                let hour0 = Offset::new(center.dx(), center.dy() - 50.0);
                let hour3 = Offset::new(center.dx() + 50.0, center.dy());
                drag(tester, hour3, hour0);
                finish_picker(tester);
                assert_eq!(feedback.haptic_count(), 1);
            });
        });
    }

    #[test]
    fn dial_haptic_feedback_quick_drag_select_vibrates_once() {
        test_widgets(|_| {
            haptic_case(|tester, center, feedback| {
                let hour0 = Offset::new(center.dx(), center.dy() - 50.0);
                let hour3 = Offset::new(center.dx() + 50.0, center.dy());
                let mut gesture = TestPointer::touch(1);
                tester.down(&mut gesture, hour3);
                tester.move_to(&mut gesture, hour0, true);
                tester.pump_by(FAST_FEEDBACK_INTERVAL);
                tester.move_to(&mut gesture, hour3, true);
                tester.pump_by(FAST_FEEDBACK_INTERVAL);
                tester.move_to(&mut gesture, hour0, true);
                tester.up(&gesture);
                finish_picker(tester);
                assert_eq!(feedback.haptic_count(), 1);
            });
        });
    }

    #[test]
    fn dial_haptic_feedback_slow_drag_select_vibrates_once() {
        test_widgets(|_| {
            haptic_case(|tester, center, feedback| {
                let hour0 = Offset::new(center.dx(), center.dy() - 50.0);
                let hour3 = Offset::new(center.dx() + 50.0, center.dy());
                let mut gesture = TestPointer::touch(1);
                tester.down(&mut gesture, hour3);
                tester.move_to(&mut gesture, hour0, true);
                tester.pump_by(SLOW_FEEDBACK_INTERVAL);
                tester.move_to(&mut gesture, hour3, true);
                tester.pump_by(SLOW_FEEDBACK_INTERVAL);
                tester.move_to(&mut gesture, hour0, true);
                tester.up(&gesture);
                finish_picker(tester);
                assert_eq!(feedback.haptic_count(), 3);
            });
        });
    }

    /// Dart's `tester.enterText(find.byType(TextField).at(index), text)`.
    fn enter_text_at(tester: &mut WidgetTester, index: usize, text: &str) {
        let editable = tester.element(find::by_type::<EditableText>().at(index));
        let state = editable
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state");
        state.request_keyboard(&mut tester.cell.borrow_mut());
        tester.pump();
        let length = text.encode_utf16().count() as i32;
        TextInputClient::update_editing_value(
            state,
            &mut tester.cell.borrow_mut(),
            TextEditingValue::new()
                .text(text)
                .selection(TextSelection::collapsed(length, TextAffinity::Downstream)),
        );
        tester.cell.checkpoint();
    }

    fn text_field_count(tester: &WidgetTester) -> usize {
        find::by_type::<crate::text_field::TextField>()
            .evaluate(tester)
            .len()
    }

    fn input_boilerplate(material_type: MaterialType) -> Boilerplate {
        let mut boilerplate = Boilerplate::new(material_type);
        boilerplate.always_use_24_hour_format = true;
        boilerplate.entry_mode = TimePickerEntryMode::Input;
        boilerplate
    }

    fn tap_icon(tester: &mut WidgetTester, icon: &waft_widgets::IconData) {
        let element = tester.element(find::by_icon(icon).first());
        let center = tester.get_center(element);
        tester.tap_at(center, 1);
    }

    fn has_icon(tester: &WidgetTester, icon: &waft_widgets::IconData) -> bool {
        find::by_type::<waft_widgets::Icon>()
            .evaluate(tester)
            .into_iter()
            .any(|element| {
                tester.widget(element, |widget: &waft_widgets::Icon| {
                    widget.icon.as_ref() == Some(icon)
                })
            })
    }

    #[test]
    fn input_initial_entry_mode_is_used() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                media_query_boilerplate(tester, size, input_boilerplate(material_type));
                assert_eq!(text_field_count(tester), 2);
            });
        }
    }

    #[test]
    fn input_initial_time_is_the_default() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(7, 0)));
            });
        }
    }

    #[test]
    fn input_help_text_is_used() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = input_boilerplate(material_type);
                boilerplate.help_text = Some("help".to_string());
                media_query_boilerplate(tester, size, boilerplate);
                only_text(tester, "help");
            });
        }
    }

    #[test]
    fn input_hour_label_text_is_used() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = input_boilerplate(material_type);
                boilerplate.hour_label_text = Some("Custom hour label".to_string());
                media_query_boilerplate(tester, size, boilerplate);
                only_text(tester, "Custom hour label");
            });
        }
    }

    #[test]
    fn input_minute_label_text_is_used() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = input_boilerplate(material_type);
                boilerplate.minute_label_text = Some("Custom minute label".to_string());
                media_query_boilerplate(tester, size, boilerplate);
                only_text(tester, "Custom minute label");
            });
        }
    }

    #[test]
    fn input_invalid_error_text_is_used() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = input_boilerplate(material_type);
                boilerplate.error_invalid_text = Some("Custom validation error".to_string());
                media_query_boilerplate(tester, size, boilerplate);
                // Input invalid time (hour) to force validation error
                enter_text_at(tester, 0, "88");
                // Tap the ok button to trigger the validation error with custom translation
                tester.tap(find::text(OK_STRING));
                tester.pump_and_settle_by(Duration::from_secs(1));
                only_text(tester, "Custom validation error");
            });
        }
    }

    #[test]
    fn time_picker_default_entry_icons() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                TimePickerDialog::new(TimeOfDay::new(12, 0)).into_widget(),
            ));

            // Check that the default icon for the dial mode is displayed.
            assert!(has_icon(tester, &crate::icons::Icons::keyboard_outlined()));
            assert!(!has_icon(tester, &crate::icons::Icons::access_time()));
            // Tap the icon to switch to input mode.
            tap_icon(tester, &crate::icons::Icons::keyboard_outlined());
            tester.pump_and_settle();
            // Check that the icon for the input mode is displayed.
            assert!(has_icon(tester, &crate::icons::Icons::access_time()));
            assert!(!has_icon(tester, &crate::icons::Icons::keyboard_outlined()));
        });
    }

    #[test]
    fn can_override_time_picker_entry_icons() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let custom_input_icon = crate::icons::Icons::text_fields();
            let custom_timer_icon = crate::icons::Icons::watch();
            tester.pump_widget(material_app(
                theme,
                TimePickerDialog::new(TimeOfDay::new(12, 0))
                    .switch_to_input_entry_mode_icon(Icon::new(Some(custom_input_icon.clone())))
                    .switch_to_timer_entry_mode_icon(Icon::new(Some(custom_timer_icon.clone())))
                    .into_widget(),
            ));

            // Check that the custom icons are displayed.
            assert!(has_icon(tester, &custom_input_icon));
            assert!(!has_icon(tester, &custom_timer_icon));
            // Tap the custom icon to switch to input mode.
            tap_icon(tester, &custom_input_icon);
            tester.pump_and_settle();
            // Check that the custom icon for the input mode is displayed.
            assert!(!has_icon(tester, &custom_input_icon));
            assert!(has_icon(tester, &custom_timer_icon));
        });
    }

    #[test]
    fn can_switch_from_input_to_dial_entry_mode() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                media_query_boilerplate(tester, size, input_boilerplate(material_type));
                tap_icon(tester, &crate::icons::Icons::access_time());
                tester.pump_and_settle();
                assert_eq!(text_field_count(tester), 0);
            });
        }
    }

    #[test]
    fn can_switch_from_dial_to_input_entry_mode() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = Boilerplate::new(material_type);
                boilerplate.always_use_24_hour_format = true;
                media_query_boilerplate(tester, size, boilerplate);
                tap_icon(tester, &crate::icons::Icons::keyboard_outlined());
                tester.pump_and_settle();
                assert!(text_field_count(tester) > 0);
            });
        }
    }

    #[test]
    fn can_not_switch_out_of_input_only_mode() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = input_boilerplate(material_type);
                boilerplate.entry_mode = TimePickerEntryMode::InputOnly;
                media_query_boilerplate(tester, size, boilerplate);
                assert!(text_field_count(tester) > 0);
                assert!(!has_icon(tester, &crate::icons::Icons::access_time()));
            });
        }
    }

    #[test]
    fn can_not_switch_out_of_dial_only_mode() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                let mut boilerplate = Boilerplate::new(material_type);
                boilerplate.always_use_24_hour_format = true;
                boilerplate.entry_mode = TimePickerEntryMode::DialOnly;
                media_query_boilerplate(tester, size, boilerplate);
                assert_eq!(text_field_count(tester), 0);
                assert!(!has_icon(tester, &crate::icons::Icons::keyboard_outlined()));
            });
        }
    }

    #[test]
    fn switching_to_dial_entry_mode_triggers_entry_callback() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let triggered_callback = Rc::new(Cell::new(false));
                let size = material_dialog_default_size();
                let mut boilerplate = input_boilerplate(material_type);
                boilerplate.on_entry_mode_change = Some({
                    let triggered_callback = Rc::clone(&triggered_callback);
                    Rc::new(move |_app, mode| {
                        if mode == TimePickerEntryMode::Dial {
                            triggered_callback.set(true);
                        }
                    })
                });
                media_query_boilerplate(tester, size, boilerplate);

                tap_icon(tester, &crate::icons::Icons::access_time());
                tester.pump_and_settle();
                assert!(triggered_callback.get());
            });
        }
    }

    #[test]
    fn switching_to_input_entry_mode_triggers_entry_callback() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let triggered_callback = Rc::new(Cell::new(false));
                let size = material_dialog_default_size();
                let mut boilerplate = Boilerplate::new(material_type);
                boilerplate.always_use_24_hour_format = true;
                boilerplate.on_entry_mode_change = Some({
                    let triggered_callback = Rc::clone(&triggered_callback);
                    Rc::new(move |_app, mode| {
                        if mode == TimePickerEntryMode::Input {
                            triggered_callback.set(true);
                        }
                    })
                });
                media_query_boilerplate(tester, size, boilerplate);

                tap_icon(tester, &crate::icons::Icons::keyboard_outlined());
                tester.pump_and_settle();
                assert!(triggered_callback.get());
            });
        }
    }

    /// Dart's `find.ancestor(of: find.text(text), matching: find.byType(InkWell))`.
    fn ink_well_of_text(tester: &WidgetTester, text: &str) -> AnyElement {
        tester.element(
            find::ancestor(
                find::text(text),
                find::by_type::<crate::ink_well::InkWell>(),
            )
            .first(),
        )
    }

    fn tap_element(tester: &mut WidgetTester, element: AnyElement) {
        let center = tester.get_center(element);
        tester.tap_at(center, 1);
    }

    #[test]
    fn can_not_double_tap_hours_when_not_selected_to_enter_input_mode() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                media_query_boilerplate(tester, size, Boilerplate::new(material_type));
                let hour_finder = ink_well_of_text(tester, "7");
                let minute_finder = ink_well_of_text(tester, "00");

                assert_eq!(text_field_count(tester), 0);

                // Switch to minutes mode.
                tap_element(tester, minute_finder);
                tester.pump_and_settle();

                // Double tap the hour.
                tap_element(tester, hour_finder);
                tester.pump_by(Duration::from_millis(100));
                tap_element(tester, hour_finder);
                tester.pump_and_settle();

                assert_eq!(text_field_count(tester), 0);
            });
        }
    }

    #[test]
    fn can_not_double_tap_minutes_when_not_selected_to_enter_input_mode() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let size = material_dialog_default_size();
                media_query_boilerplate(tester, size, Boilerplate::new(material_type));
                let minute_finder = ink_well_of_text(tester, "00");

                assert_eq!(text_field_count(tester), 0);

                // Double tap the minutes.
                tap_element(tester, minute_finder);
                tester.pump_by(Duration::from_millis(100));
                tap_element(tester, minute_finder);
                tester.pump_and_settle();

                assert_eq!(text_field_count(tester), 0);
            });
        }
    }

    #[test]
    fn input_entered_text_returns_time() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);
                enter_text_at(tester, 0, "9");
                enter_text_at(tester, 1, "12");
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(9, 12)));
            });
        }
    }

    #[test]
    fn input_toggle_to_dial_mode_keeps_selected_time() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);
                enter_text_at(tester, 0, "8");
                enter_text_at(tester, 1, "15");
                tap_icon(tester, &crate::icons::Icons::access_time());
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(8, 15)));
            });
        }
    }

    #[test]
    fn input_invalid_text_prevents_dismissing() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);

                // Invalid hour.
                enter_text_at(tester, 0, "88");
                enter_text_at(tester, 1, "15");
                finish_picker(tester);
                assert_eq!(picked_time(&result), None);

                // Invalid minute.
                enter_text_at(tester, 0, "8");
                enter_text_at(tester, 1, "95");
                finish_picker(tester);
                assert_eq!(picked_time(&result), None);

                enter_text_at(tester, 0, "8");
                enter_text_at(tester, 1, "15");
                finish_picker(tester);
                assert_eq!(picked_time(&result), Some(TimeOfDay::new(8, 15)));
            });
        }
    }

    fn text_field_texts(tester: &WidgetTester) -> Vec<(String, Option<String>)> {
        find::by_type::<crate::text_field::TextField>()
            .evaluate(tester)
            .into_iter()
            .map(|field| {
                tester.widget(field, |field: &crate::text_field::TextField| {
                    (
                        field
                            .controller
                            .as_ref()
                            .map(|controller| controller.value(&tester.cell.borrow()).text.clone())
                            .unwrap_or_default(),
                        field
                            .decoration
                            .as_ref()
                            .and_then(|decoration| decoration.hint_text.clone()),
                    )
                })
            })
            .collect()
    }

    #[test]
    fn empty_initial_input_fields_are_empty_and_show_correct_hints_when_true() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                parameters.empty_initial_input = true;
                start_picker(tester, &result, parameters);
                tester.pump();

                let fields = text_field_texts(tester);
                assert_eq!(fields[0], (String::new(), None)); // hour
                assert_eq!(fields[1], (String::new(), None)); // minute
                finish_picker(tester);
            });
        }
    }

    #[test]
    fn empty_initial_input_user_sets_hour_minute_after_initially_empty_fields() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                parameters.empty_initial_input = true;
                start_picker(tester, &result, parameters);

                let fields = text_field_texts(tester);
                assert_eq!(fields[0], (String::new(), None)); // hour
                assert_eq!(fields[1], (String::new(), None)); // minute

                enter_text_at(tester, 0, "11");
                enter_text_at(tester, 1, "30");
                finish_picker(tester);

                assert_eq!(picked_time(&result), Some(TimeOfDay::new(11, 30)));
            });
        }
    }

    #[test]
    fn empty_initial_input_user_overrides_default_values_when_false() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);

                let fields = text_field_texts(tester);
                assert_eq!(fields[0].0, "7"); // hour
                assert_eq!(fields[1].0, "00"); // minute

                enter_text_at(tester, 0, "8");
                enter_text_at(tester, 1, "15");
                tester.pump();
                finish_picker(tester);

                assert_eq!(picked_time(&result), Some(TimeOfDay::new(8, 15)));
            });
        }
    }

    fn separator_style(tester: &WidgetTester) -> TextStyle {
        let separator = only_text(tester, ":");
        tester.widget(separator, |text: &Text| {
            text.style.clone().expect("a styled separator")
        })
    }

    #[test]
    fn material3_time_selector_separator_default_text_style() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let result: PickerResult = Rc::default();
            start_picker(tester, &result, StartPicker::new(MaterialType::Material3));

            let style = separator_style(tester);
            assert_eq!(
                style.color.map(|color| color.color()),
                Some(theme.color_scheme.on_surface)
            );
            assert_eq!(style.font_size, Some(57.0));
        });
    }

    #[test]
    fn material2_time_selector_separator_default_text_style() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());
            let result: PickerResult = Rc::default();
            start_picker(tester, &result, StartPicker::new(MaterialType::Material2));

            let style = separator_style(tester);
            assert_eq!(
                style.color.map(|color| color.color()),
                Some(theme.color_scheme.on_surface)
            );
            assert_eq!(style.font_size, Some(56.0));
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

    #[test]
    fn time_picker_hour_minute_does_not_resize_on_error() {
        test_widgets(|tester| {
            // Dart's cases give no `materialType`, which `startPicker` takes as Material 2, and
            // the explicit Material 2 one.
            {
                let material_type = MaterialType::Material2;
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);

                assert_eq!(border_painter_size(tester), Size::new(96.0, 70.0));

                // Enter invalid hour.
                enter_text_at(tester, 0, "AB");
                tester.tap(find::text(OK_STRING));

                assert_eq!(border_painter_size(tester), Size::new(96.0, 70.0));
            }
        });
    }

    #[test]
    fn time_picker_dialog_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                waft_widgets::Center::new()
                    .child(
                        SizedBox::new()
                            .width(0.0)
                            .height(0.0)
                            .child(TimePickerDialog::new(TimeOfDay::new(10, 12))),
                    )
                    .into_widget(),
            ));
            let dialog = tester.element(find::by_type::<TimePickerDialog>().at(0));
            assert_eq!(tester.get_size(dialog), Size::ZERO);
        });
    }

    /// What the dial's `CustomPaint` paints: Dart's `paints` on its render object.
    fn dial_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
        let paint = tester.element(
            find::descendant(
                find::by_type::<Dial>().at(0),
                find::by_type::<waft_widgets::CustomPaint>(),
            )
            .first(),
        );
        tester.painted_ops_deep(tester.render_object(paint))
    }

    fn open_default_picker(tester: &mut WidgetTester, theme: ThemeData) {
        let home = Material::new()
            .child(
                waft_widgets::Center::new().child(Builder::new(move |_app, context| {
                    crate::elevated_button::ElevatedButton::new(
                        Some(Listener::new(move |app| {
                            drop(show_default(app, context));
                        })),
                        Some(Text::new("X")),
                    )
                    .into_widget()
                })),
            )
            .into_widget();
        tester.pump_widget(material_app(theme, home));
        tester.tap(find::text("X"));
        tester.pump_and_settle();
    }

    #[test]
    #[allow(deprecated)]
    fn material3_dial_background_uses_correct_default_color() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());

            open_default_picker(tester, theme.clone());
            // Test default dial background color.
            assert_paints(
                &dial_ops(tester),
                &[
                    PaintCall::circle(Some(theme.color_scheme.surface_container_highest())),
                    PaintCall::circle(Some(theme.color_scheme.primary)),
                ],
            );

            tester.tap(find::text(OK_STRING)); // dismiss the dialog
            tester.pump_and_settle();

            // Test dial background color when theme color scheme is changed.
            let theme = theme
                .copy_with()
                .color_scheme(
                    theme
                        .color_scheme
                        .copy_with()
                        .with_surface_variant(waft_embedder::Color::new(0xffff0000)),
                )
                .build();
            open_default_picker(tester, theme.clone());
            assert_paints(
                &dial_ops(tester),
                &[
                    PaintCall::circle(Some(theme.color_scheme.surface_container_highest())),
                    PaintCall::circle(Some(theme.color_scheme.primary)),
                ],
            );
        });
    }

    /// Dart's `PickerObserver`: counts the dialog routes pushed.
    #[derive(Default)]
    struct PickerObserver {
        navigator_observer: waft_widgets::NavigatorObserverData,
        picker_count: Cell<u32>,
    }

    impl waft_widgets::NavigatorObserver for PickerObserver {
        waft_widgets::navigator_observer_accessors!();

        fn did_push(
            self: waft_foundation::Handle<Self>,
            app: &mut waft_foundation::App,
            route: waft_widgets::AnyRoute,
            _previous: Option<waft_widgets::AnyRoute>,
        ) {
            if route.downcast::<crate::dialog::DialogRoute>(app).is_some() {
                let count = &app.get(self).picker_count;
                count.set(count.get() + 1);
            }
        }
    }

    /// The pickers the root and the nested navigator's observers saw.
    fn pickers_seen_by_root_and_nested(
        tester: &mut WidgetTester,
        use_root_navigator: bool,
    ) -> (u32, u32) {
        let (root_observer, nested_observer) = {
            let mut app = tester.cell.borrow_mut();
            (
                app.create(PickerObserver::default()),
                app.create(PickerObserver::default()),
            )
        };
        let root = {
            let app = tester.cell.borrow();
            crate::app::MaterialApp::new()
                .theme(m3(&app))
                .navigator_observers([root_observer.erase::<waft_widgets::AnyNavigatorObserver>()])
                .home(
                    waft_widgets::Navigator::new()
                        .observers([nested_observer.erase::<waft_widgets::AnyNavigatorObserver>()])
                        .on_generate_route(move |app, _settings| {
                            let route = crate::page::MaterialPageRoute::new(
                                app,
                                Rc::new(move |_app, context| {
                                    crate::elevated_button::ElevatedButton::new(
                                        Some(Listener::new(move |app| {
                                            drop(show_time_picker(
                                                app,
                                                context,
                                                TimeOfDay::new(7, 0),
                                                None,
                                                true,
                                                None,
                                                None,
                                                use_root_navigator,
                                                TimePickerEntryMode::Dial,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                false,
                                            ));
                                        })),
                                        Some(Text::new("Show Picker")),
                                    )
                                    .into_widget()
                                }),
                            );
                            Some(route.erase())
                        }),
                )
                .into_widget()
        };
        tester.pump_widget(root);

        // Open the dialog.
        let button =
            tester.element(find::by_type::<crate::elevated_button::ElevatedButton>().at(0));
        let center = tester.get_center(button);
        tester.tap_at(center, 1);

        let app = tester.cell.borrow();
        (
            app.get(&root_observer).picker_count.get(),
            app.get(&nested_observer).picker_count.get(),
        )
    }

    #[test]
    fn time_picker_uses_root_navigator_by_default() {
        test_widgets(|tester| {
            assert_eq!(pickers_seen_by_root_and_nested(tester, true), (1, 0));
        });
    }

    #[test]
    fn time_picker_uses_nested_navigator_if_use_root_navigator_is_false() {
        test_widgets(|tester| {
            assert_eq!(pickers_seen_by_root_and_nested(tester, false), (0, 1));
        });
    }

    /// Dart's `find.ancestor(of: find.text(text), matching: find.byType(InkWell))`, which finds
    /// one ink well: the header's, not a dial label's.
    fn ink_well_above_text(tester: &WidgetTester, text: &str) -> AnyElement {
        let wells: Vec<AnyElement> = find::text(text)
            .evaluate(tester)
            .into_iter()
            .filter_map(|found| {
                let ancestors = tester.ancestors(found);
                let app = tester.cell.borrow();
                ancestors.into_iter().find(|ancestor| {
                    waft_widgets::downcast_widget::<crate::ink_well::InkWell>(
                        &**ancestor.widget(&app),
                    )
                    .is_some()
                })
            })
            .collect();
        assert_eq!(wells.len(), 1, "one ink well above {text:?}");
        wells[0]
    }

    // 'header touch regions are large enough'.
    #[test]
    fn header_touch_regions_are_large_enough() {
        for material_type in pickers() {
            test_widgets(|_| {
                // Ensure picker is displayed in portrait mode.
                let size = Size::new(400.0, 800.0);
                let mut tester = tester_of_size(size);
                let tester = &mut tester;
                media_query_boilerplate(tester, size, Boilerplate::new(material_type));

                let day_period_control_size =
                    tester.get_size(tester.element(find::by_type::<DayPeriodControl>().at(0)));
                assert!(day_period_control_size.width() >= 48.0, "{material_type:?}");
                assert!(
                    day_period_control_size.height() >= 80.0,
                    "{material_type:?}"
                );

                let hour_size = tester.get_size(ink_well_above_text(tester, "7"));
                assert!(hour_size.width() >= 48.0, "{material_type:?}");
                assert!(hour_size.height() >= 48.0, "{material_type:?}");

                let minute_size = tester.get_size(ink_well_above_text(tester, "00"));
                assert!(minute_size.width() >= 48.0, "{material_type:?}");
                assert!(minute_size.height() >= 48.0, "{material_type:?}");
            });
        }
    }

    #[test]
    fn time_picker_draws_without_overflows_at_various_view_sizes() {
        for material_type in pickers() {
            test_widgets(|_| {
                for size in [
                    Size::new(100.0, 100.0),
                    Size::new(300.0, 300.0),
                    Size::new(800.0, 600.0),
                ] {
                    for entry_mode in [TimePickerEntryMode::Dial, TimePickerEntryMode::Input] {
                        // Dart's layout asserts here too (the input mode's minimum height of 196
                        // is below its constraints' minimum of 216); its test hides the assertion
                        // with `isNot(throwsAssertionError)` on the exception object.
                        if entry_mode == TimePickerEntryMode::Input && size.height() < 216.0 {
                            continue;
                        }
                        // Dart sets `physicalSize = size` at flutter_test's device pixel ratio,
                        // a view of size / 3 logical pixels, whose layout asserts (hidden by
                        // `isNot(throwsAssertionError)`, which always holds) panic here: the
                        // sizes are the logical ones.
                        let mut tester = tester_of_size(size);
                        let tester = &mut tester;
                        let mut boilerplate = Boilerplate::new(material_type);
                        boilerplate.entry_mode = entry_mode;
                        media_query_boilerplate(tester, size, boilerplate);
                        tester.pump_and_settle();
                        assert_eq!(
                            find::by_type::<TimePickerDialog>().evaluate(tester).len(),
                            1,
                            "{material_type:?} {entry_mode:?} at {size:?}"
                        );
                    }
                }
            });
        }
    }

    #[test]
    fn hour_and_minute_fields_are_top_aligned_with_the_separator() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);

                let top = |element: AnyElement| {
                    crate::chip::tests::rect_of(tester, element).top_left().dy()
                };
                let hour_field_top = top(tester.element(find::by_type::<HourTextField>().at(0)));
                let minute_field_top =
                    top(tester.element(find::by_type::<MinuteTextField>().at(0)));
                let separator_top =
                    top(tester.element(find::by_type::<TimeSelectorSeparator>().at(0)));
                assert_eq!(hour_field_top, separator_top, "{material_type:?}");
                assert_eq!(minute_field_top, separator_top, "{material_type:?}");
            });
        }
    }

    #[test]
    fn can_switch_between_hour_minute_fields_using_keyboard_input_action() {
        for material_type in pickers() {
            test_widgets(|tester| {
                let result: PickerResult = Rc::default();
                let mut parameters = StartPicker::new(material_type);
                parameters.entry_mode = TimePickerEntryMode::Input;
                start_picker(tester, &result, parameters);

                let field = |tester: &WidgetTester, index: usize| {
                    tester.widget(
                        tester.element(find::by_type::<crate::text_field::TextField>().at(index)),
                        |field: &crate::text_field::TextField| field.clone(),
                    )
                };
                let has_focus = |tester: &WidgetTester, index: usize| {
                    let node = field(tester, index).focus_node.expect("a focus node");
                    node.has_focus(&mut tester.cell.borrow_mut())
                };
                let last = text_field_count(tester) - 1;

                let hour = tester.element(find::by_type::<crate::text_field::TextField>().at(0));
                let center = tester.get_center(hour);
                tester.tap_at(center, 1);
                assert!(has_focus(tester, 0), "{material_type:?}");

                enter_text_at(tester, 0, "08");
                assert!(!has_focus(tester, 0), "{material_type:?}");
                assert!(has_focus(tester, last), "{material_type:?}");

                assert_eq!(
                    field(tester, last).text_input_action,
                    Some(waft_services::TextInputAction::Done)
                );
                let minute_editable = tester.element(find::by_type::<EditableText>().at(last));
                let state = minute_editable
                    .state_handle::<EditableTextState>(&tester.cell.borrow())
                    .expect("an EditableText state");
                TextInputClient::perform_action(
                    state,
                    &mut tester.cell.borrow_mut(),
                    waft_services::TextInputAction::Done,
                );
                tester.cell.checkpoint();
                assert!(!has_focus(tester, 0), "{material_type:?}");
                assert!(!has_focus(tester, last), "{material_type:?}");
            });
        }
    }
}
