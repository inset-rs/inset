//! Flutter counterpart: material_ui's `time.dart`.
//!
//! [`TimeOfDayFormat`] and [`HourFormat`] keep Dart's names: they are ICU patterns, where
//! `H` and `h` are different hours.

use std::cmp::Ordering;
use std::fmt;

use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, DateTime, Handle, Ref};
use waft_services::RestorationData;
use waft_widgets::{
    BuildContext, MediaQuery, RestorableProperty, RestorablePropertyData, RestorableValue,
    RestorableValueData, debug_check_has_media_query,
};

use crate::debug::debug_check_has_material_localizations;
use crate::material_localizations::MaterialLocalizations;

/// Whether the [`TimeOfDay`] is before or after noon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DayPeriod {
    /// Ante meridiem (before noon).
    Am,

    /// Post meridiem (after noon).
    Pm,
}

/// A value representing a time during the day, independent of the date that
/// day might fall on or the time zone.
///
/// The time is represented by [`hour`](Self::hour) and [`minute`](Self::minute) pair. Once
/// created, both values cannot be changed.
///
/// You can create TimeOfDay using the constructor which requires both hour and
/// minute or using [`DateTime`] object.
/// Hours are specified between 0 and 23, as in a 24-hour clock.
///
/// ```
/// # use waft_foundation::DateTime;
/// # use waft_material::TimeOfDay;
/// let release_time = TimeOfDay::new(15, 0); // 3:00pm
/// let room_booked = TimeOfDay::from_date_time(&DateTime::new_with_time(2018, 10, 20, 16, 30, 4, 0, 0)); // 4:30pm
/// ```
///
/// See also:
///
///  * `showTimePicker`, which returns this type.
///  * [`MaterialLocalizations`], which provides methods for formatting values of
///    this type according to the chosen `Locale`.
///  * [`DateTime`], which represents date and time, and is subject to eras and
///    time zones.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeOfDay {
    /// The selected hour, in 24 hour time from 0..23.
    pub hour: i32,

    /// The selected minute.
    pub minute: i32,
}

impl TimeOfDay {
    /// Creates a time of day.
    ///
    /// The `hour` argument must be between 0 and 23, inclusive. The `minute`
    /// argument must be between 0 and 59, inclusive.
    pub const fn new(hour: i32, minute: i32) -> TimeOfDay {
        TimeOfDay { hour, minute }
    }

    /// Creates a time of day based on the current time, in the local time zone.
    pub fn now() -> TimeOfDay {
        TimeOfDay::from_date_time(&DateTime::now())
    }

    /// Creates a time of day based on the given time.
    ///
    /// The [`hour`](Self::hour) is set to the time's hour and the [`minute`](Self::minute) is
    /// set to the time's minute in the timezone of the given [`DateTime`].
    pub fn from_date_time(time: &DateTime) -> TimeOfDay {
        TimeOfDay {
            hour: time.hour(),
            minute: time.minute(),
        }
    }

    /// The number of hours in one day, i.e. 24.
    pub const HOURS_PER_DAY: i32 = 24;

    /// The number of hours in one day period (see also [`DayPeriod`]), i.e. 12.
    pub const HOURS_PER_PERIOD: i32 = 12;

    /// The number of minutes in one hour, i.e. 60.
    pub const MINUTES_PER_HOUR: i32 = 60;

    /// Returns a new TimeOfDay with the hour and/or minute replaced.
    pub fn replacing(&self, hour: Option<i32>, minute: Option<i32>) -> TimeOfDay {
        debug_assert!(hour.is_none_or(|hour| (0..Self::HOURS_PER_DAY).contains(&hour)));
        debug_assert!(minute.is_none_or(|minute| (0..Self::MINUTES_PER_HOUR).contains(&minute)));
        TimeOfDay::new(hour.unwrap_or(self.hour), minute.unwrap_or(self.minute))
    }

    /// Whether this time of day is before or after noon.
    pub fn period(&self) -> DayPeriod {
        if self.hour < Self::HOURS_PER_PERIOD {
            DayPeriod::Am
        } else {
            DayPeriod::Pm
        }
    }

    /// Which hour of the current period (e.g., am or pm) this time is.
    ///
    /// For 12AM (midnight) and 12PM (noon) this returns 12.
    pub fn hour_of_period(&self) -> i32 {
        if self.hour == 0 || self.hour == 12 {
            12
        } else {
            self.hour - self.period_offset()
        }
    }

    /// The hour at which the current period starts.
    pub fn period_offset(&self) -> i32 {
        if self.period() == DayPeriod::Am {
            0
        } else {
            Self::HOURS_PER_PERIOD
        }
    }

    /// Returns the localized string representation of this time of day.
    ///
    /// This is a shortcut for [`MaterialLocalizations::format_time_of_day`].
    pub fn format(&self, app: &mut App, context: BuildContext) -> String {
        debug_assert!(debug_check_has_media_query(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        localizations
            .format_time_of_day(self, MediaQuery::always_use_24_hour_format_of(app, context))
    }

    /// Whether this [`TimeOfDay`] occurs earlier than `other`.
    ///
    /// Does not account for day or sub-minute differences. This means
    /// that "00:00" of the next day is still before "23:00" of this day.
    pub fn is_before(&self, other: &TimeOfDay) -> bool {
        self.compare_to(other) == Ordering::Less
    }

    /// Whether this [`TimeOfDay`] occurs later than `other`.
    ///
    /// Does not account for day or sub-minute differences. This means
    /// that "00:00" of the next day is still before "23:00" of this day.
    pub fn is_after(&self, other: &TimeOfDay) -> bool {
        self.compare_to(other) == Ordering::Greater
    }

    /// Whether this [`TimeOfDay`] occurs at the same time as `other`.
    ///
    /// Does not account for day or sub-minute differences. This means
    /// that "00:00" of the next day is still before "23:00" of this day.
    pub fn is_at_same_time_as(&self, other: &TimeOfDay) -> bool {
        self.compare_to(other) == Ordering::Equal
    }

    /// Compares this [`TimeOfDay`] object to `other` independent of date.
    ///
    /// Does not account for day or sub-minute differences. This means
    /// that "00:00" of the next day is still before "23:00" of this day.
    ///
    /// A [`compare_to`](Self::compare_to) function returns:
    ///  * [`Ordering::Less`] if this TimeOfDay [`is_before`](Self::is_before) `other`.
    ///  * [`Ordering::Equal`] if this DateTime [`is_at_same_time_as`](Self::is_at_same_time_as)
    ///    `other`, and
    ///  * [`Ordering::Greater`] otherwise (when this TimeOfDay [`is_after`](Self::is_after)
    ///    `other`).
    pub fn compare_to(&self, other: &TimeOfDay) -> Ordering {
        let hour_comparison = self.hour.cmp(&other.hour);
        if hour_comparison == Ordering::Equal {
            self.minute.cmp(&other.minute)
        } else {
            hour_comparison
        }
    }
}

/// Dart's `implements Comparable<TimeOfDay>`: the order of [`TimeOfDay::compare_to`].
impl Ord for TimeOfDay {
    fn cmp(&self, other: &TimeOfDay) -> Ordering {
        self.compare_to(other)
    }
}

impl PartialOrd for TimeOfDay {
    fn partial_cmp(&self, other: &TimeOfDay) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn add_leading_zero_if_needed(value: i32) -> String {
            if value < 10 {
                return format!("0{value}");
            }
            value.to_string()
        }

        let hour_label = add_leading_zero_if_needed(self.hour);
        let minute_label = add_leading_zero_if_needed(self.minute);

        write!(f, "TimeOfDay({hour_label}:{minute_label})")
    }
}

/// A [`RestorableValue`] that knows how to save and restore [`TimeOfDay`].
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given
/// in the constructor.
pub struct RestorableTimeOfDay {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<TimeOfDay>,
    default_value: TimeOfDay,
}

impl RestorableTimeOfDay {
    /// Creates a [`RestorableTimeOfDay`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: TimeOfDay) -> Ref<RestorableTimeOfDay> {
        app.create(RestorableTimeOfDay {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            default_value,
        })
    }
}

impl ChangeNotifier for RestorableTimeOfDay {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableTimeOfDay {
    type Value = TimeOfDay;
    waft_widgets::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> TimeOfDay {
        app.get(self).default_value
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> TimeOfDay {
        let _ = app;
        let time_data = data
            .as_list()
            .expect("the restoration data of a TimeOfDay property is a list");
        let int_at = |index: usize| {
            let value = time_data[index]
                .as_int()
                .expect("the restoration data of a TimeOfDay property holds ints");
            i32::try_from(value).expect("an hour or a minute")
        };
        TimeOfDay::new(int_at(1), int_at(0))
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: TimeOfDay) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        let value = RestorableValue::value(self, app);
        RestorationData::List(vec![
            RestorationData::Int(i64::from(value.minute)),
            RestorationData::Int(i64::from(value.hour)),
        ])
    }
}

impl RestorableValue for RestorableTimeOfDay {
    waft_widgets::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<TimeOfDay>) {
        let _ = old_value;
        self.notify_listeners(app);
    }
}

/// Determines how the time picker invoked using `showTimePicker` formats and
/// lays out the time controls.
///
/// The time picker provides layout configurations optimized for each of the
/// enum values.
#[allow(
    non_camel_case_types,
    reason = "Dart's names are ICU patterns, where the case of a letter is its meaning"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeOfDayFormat {
    /// Corresponds to the ICU 'HH:mm' pattern.
    ///
    /// This format uses 24-hour two-digit zero-padded hours. Controls are always
    /// laid out horizontally. Hours are separated from minutes by one colon
    /// character.
    HH_colon_mm,

    /// Corresponds to the ICU 'HH.mm' pattern.
    ///
    /// This format uses 24-hour two-digit zero-padded hours. Controls are always
    /// laid out horizontally. Hours are separated from minutes by one dot
    /// character.
    HH_dot_mm,

    /// Corresponds to the ICU "HH 'h' mm" pattern used in Canadian French.
    ///
    /// This format uses 24-hour two-digit zero-padded hours. Controls are always
    /// laid out horizontally. Hours are separated from minutes by letter 'h'.
    FrenchCanadian,

    /// Corresponds to the ICU 'H:mm' pattern.
    ///
    /// This format uses 24-hour non-padded variable-length hours. Controls are
    /// always laid out horizontally. Hours are separated from minutes by one
    /// colon character.
    H_colon_mm,

    /// Corresponds to the ICU 'h:mm a' pattern.
    ///
    /// This format uses 12-hour non-padded variable-length hours with a day
    /// period. Controls are laid out horizontally in portrait mode. In landscape
    /// mode, the day period appears vertically after (consistent with the ambient
    /// `TextDirection`) hour-minute indicator. Hours are separated from minutes
    /// by one colon character.
    h_colon_mm_space_a,

    /// Corresponds to the ICU 'a h:mm' pattern.
    ///
    /// This format uses 12-hour non-padded variable-length hours with a day
    /// period. Controls are laid out horizontally in portrait mode. In landscape
    /// mode, the day period appears vertically before (consistent with the
    /// ambient `TextDirection`) hour-minute indicator. Hours are separated from
    /// minutes by one colon character.
    a_space_h_colon_mm,
}

/// Describes how hours are formatted.
#[allow(
    non_camel_case_types,
    clippy::upper_case_acronyms,
    reason = "Dart's names are ICU patterns, where `H` and `h` are different hours"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HourFormat {
    /// Zero-padded two-digit 24-hour format ranging from "00" to "23".
    HH,

    /// Non-padded variable-length 24-hour format ranging from "0" to "23".
    H,

    /// Non-padded variable-length hour in day period format ranging from "1" to
    /// "12".
    h,
}

/// The [`HourFormat`] used for the given [`TimeOfDayFormat`].
pub fn hour_format(of: TimeOfDayFormat) -> HourFormat {
    match of {
        TimeOfDayFormat::h_colon_mm_space_a | TimeOfDayFormat::a_space_h_colon_mm => HourFormat::h,
        TimeOfDayFormat::H_colon_mm => HourFormat::H,
        TimeOfDayFormat::HH_dot_mm
        | TimeOfDayFormat::HH_colon_mm
        | TimeOfDayFormat::FrenchCanadian => HourFormat::HH,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Restoration, TargetPlatform};
    use waft_foundation::{AppCell, Listenable, Listener};
    use waft_services::{RestorationBucket, RestorationManager, RestorationMap, RestorationUpdate};
    use waft_test::TestPlatform;
    use waft_widgets::{
        Builder, GlobalKey, IntoWidget, KeyRef, MediaQueryData, RestorationMixin,
        RestorationMixinData, RootRestorationScope, SizedBox, State, StateData, StatefulWidget,
        WidgetRef,
    };

    use super::*;
    use crate::material_localizations::tests::us_english;
    use crate::test_support::{build, pump, test_cell, test_view};

    // time_test.dart 'hourFormat'.
    #[test]
    fn hour_format_names_the_hours_of_each_time_of_day_format() {
        for format in [
            TimeOfDayFormat::h_colon_mm_space_a,
            TimeOfDayFormat::a_space_h_colon_mm,
        ] {
            assert_eq!(hour_format(format), HourFormat::h, "for {format:?}");
        }

        assert_eq!(hour_format(TimeOfDayFormat::H_colon_mm), HourFormat::H);

        for format in [
            TimeOfDayFormat::HH_dot_mm,
            TimeOfDayFormat::HH_colon_mm,
            TimeOfDayFormat::FrenchCanadian,
        ] {
            assert_eq!(hour_format(format), HourFormat::HH, "for {format:?}");
        }
    }

    // time_test.dart 'TimeOfDay.format respects alwaysUse24HourFormat option'.
    #[test]
    fn format_respects_always_use_24_hour_format() {
        let pump_test = |always_use_24_hour_format: bool| {
            let cell = test_cell();
            let formatted_value = Rc::new(RefCell::new(String::new()));
            let probe = Builder::new({
                let formatted_value = Rc::clone(&formatted_value);
                move |app, context| {
                    *formatted_value.borrow_mut() = TimeOfDay::new(7, 0).format(app, context);
                    SizedBox::shrink().into_widget()
                }
            });
            build(
                &cell,
                us_english(
                    MediaQuery::new(
                        MediaQueryData::new().always_use_24_hour_format(always_use_24_hour_format),
                        probe,
                    )
                    .into_widget(),
                ),
            );
            formatted_value.take()
        };

        assert_eq!(pump_test(false), "7:00 AM");
        assert_eq!(pump_test(true), "07:00");
    }

    // time_test.dart 'hourOfPeriod returns correct value'.
    #[test]
    fn hour_of_period_counts_from_twelve() {
        let expected = [
            12, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
        ];
        for (hour, hour_of_period) in (0..24).zip(expected) {
            assert_eq!(
                TimeOfDay::new(hour, 0).hour_of_period(),
                hour_of_period,
                "hour {hour}"
            );
        }
    }

    // time_test.dart 'correctly compares to other TimeOfDays'.
    #[test]
    fn a_time_of_day_compares_by_hour_then_minute() {
        let time = TimeOfDay::new;

        assert_eq!(time(0, 0).compare_to(&time(1, 0)), Ordering::Less);
        assert_eq!(time(20, 0).compare_to(&time(20, 1)), Ordering::Less);
        assert_eq!(time(0, 0).compare_to(&time(0, 0)), Ordering::Equal);
        assert_eq!(time(1, 0).compare_to(&time(0, 0)), Ordering::Greater);
        assert_eq!(time(20, 1).compare_to(&time(20, 0)), Ordering::Greater);

        assert!(time(0, 0).is_before(&time(1, 0)));
        assert!(time(0, 0).is_before(&time(23, 0)));
        assert!(time(0, 0).is_before(&time(0, 10)));
        assert!(!time(0, 0).is_before(&time(0, 0)));
        assert!(!time(1, 0).is_before(&time(0, 0)));
        assert!(!time(23, 0).is_before(&time(0, 0)));
        assert!(!time(0, 10).is_before(&time(0, 0)));

        assert!(!time(0, 0).is_after(&time(1, 0)));
        assert!(!time(0, 0).is_after(&time(23, 0)));
        assert!(!time(0, 0).is_after(&time(0, 10)));
        assert!(!time(0, 0).is_after(&time(0, 0)));
        assert!(time(1, 0).is_after(&time(0, 0)));
        assert!(time(23, 0).is_after(&time(0, 0)));
        assert!(time(0, 10).is_after(&time(0, 0)));

        assert!(!time(0, 0).is_at_same_time_as(&time(1, 0)));
        assert!(!time(0, 0).is_at_same_time_as(&time(0, 10)));
        assert!(time(0, 0).is_at_same_time_as(&time(0, 0)));
        assert!(!time(1, 0).is_at_same_time_as(&time(0, 0)));
        assert!(!time(0, 10).is_at_same_time_as(&time(0, 0)));
    }

    #[test]
    fn a_time_of_day_sorts_as_it_compares() {
        let mut times = vec![
            TimeOfDay::new(23, 0),
            TimeOfDay::new(0, 10),
            TimeOfDay::new(12, 30),
            TimeOfDay::new(0, 0),
        ];
        times.sort();
        assert_eq!(
            times,
            [
                TimeOfDay::new(0, 0),
                TimeOfDay::new(0, 10),
                TimeOfDay::new(12, 30),
                TimeOfDay::new(23, 0),
            ]
        );
        assert!(TimeOfDay::new(20, 0) < TimeOfDay::new(20, 1));
        assert_eq!(times.iter().max(), Some(&TimeOfDay::new(23, 0)));
    }

    #[test]
    fn replacing_keeps_what_it_is_not_given_and_the_period_follows_the_hour() {
        let time = TimeOfDay::new(9, 30);
        assert_eq!(time.replacing(Some(21), None), TimeOfDay::new(21, 30));
        assert_eq!(time.replacing(None, Some(5)), TimeOfDay::new(9, 5));
        assert_eq!(time.replacing(None, None), time);
        assert_eq!(time.period(), DayPeriod::Am);
        assert_eq!(time.period_offset(), 0);
        assert_eq!(TimeOfDay::new(12, 0).period(), DayPeriod::Pm);
        assert_eq!(TimeOfDay::new(21, 0).period_offset(), 12);
        assert_eq!(
            TimeOfDay::from_date_time(&DateTime::new_with_time(2018, 10, 20, 16, 30, 4, 0, 0)),
            TimeOfDay::new(16, 30)
        );
        assert_eq!(format!("{:?}", TimeOfDay::new(7, 5)), "TimeOfDay(07:05)");
        assert_eq!(format!("{:?}", TimeOfDay::new(23, 45)), "TimeOfDay(23:45)");
    }

    /// A host that answers `Restoration::get` with what it was handed and records every
    /// `Restoration::put`.
    #[derive(Default)]
    struct RecordingRestoration {
        stored: RefCell<Option<RestorationUpdate>>,
        puts: RefCell<Vec<RestorationMap>>,
    }

    impl Restoration for RecordingRestoration {
        fn get(&self) -> Option<RestorationUpdate> {
            self.stored.borrow().clone()
        }

        fn put(&self, data: RestorationMap) {
            self.puts.borrow_mut().push(data);
        }
    }

    impl RecordingRestoration {
        fn last_put(&self) -> RestorationMap {
            self.puts.borrow().last().cloned().expect("a put")
        }
    }

    /// An app whose host restores `data`, with one 800x600 view at 2x.
    fn restoring_cell(data: Option<RestorationMap>) -> (Rc<AppCell>, Rc<RecordingRestoration>) {
        let restoration = Rc::new(RecordingRestoration::default());
        *restoration.stored.borrow_mut() = Some(RestorationUpdate {
            enabled: true,
            data,
        });
        let platform = TestPlatform::new()
            .on(TargetPlatform::Android)
            .with_view(test_view())
            .with_restoration(restoration.clone());
        (AppCell::with_platform(Rc::new(platform)), restoration)
    }

    /// time_test.dart's `_RestorableWidget`.
    #[derive(Debug)]
    struct RestorableWidget {
        key: KeyRef,
    }

    impl StatefulWidget for RestorableWidget {
        type State = RestorableWidgetState;

        fn key(&self) -> Option<&KeyRef> {
            Some(&self.key)
        }

        fn create_state(&self) -> RestorableWidgetState {
            RestorableWidgetState {
                state: StateData::new(),
                restoration: RestorationMixinData::new(),
                time_of_day: None,
            }
        }
    }

    struct RestorableWidgetState {
        state: StateData<RestorableWidget>,
        restoration: RestorationMixinData,
        time_of_day: Option<Ref<RestorableTimeOfDay>>,
    }

    impl RestorableWidgetState {
        fn time_of_day(self: Handle<Self>, app: &App) -> Ref<RestorableTimeOfDay> {
            app.get(self).time_of_day.clone().expect("init_state ran")
        }

        fn set_properties(self: Handle<Self>, app: &mut App, callback: impl FnOnce(&mut App)) {
            callback(app);
            self.set_state(app, |_| {});
        }
    }

    impl RestorationMixin for RestorableWidgetState {
        waft_widgets::restoration_mixin_accessors!();

        fn restoration_id(self: Handle<Self>, _app: &App) -> Option<&str> {
            Some("widget")
        }

        fn restore_state(
            self: Handle<Self>,
            app: &mut App,
            _old_bucket: Option<Handle<RestorationBucket>>,
            _initial_restore: bool,
        ) {
            let time_of_day = self.time_of_day(app);
            self.register_for_restoration(app, time_of_day.as_property(), "time_of_day");
        }
    }

    impl State for RestorableWidgetState {
        type Widget = RestorableWidget;
        waft_widgets::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let time_of_day = RestorableTimeOfDay::new(app, TimeOfDay::new(10, 5));
            app.get_mut(self).time_of_day = Some(time_of_day);
        }

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            self.did_change_dependencies_restoration(app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            let time_of_day = self.time_of_day(app);
            RestorableProperty::dispose(time_of_day.handle(), app);
            self.dispose_restoration(app);
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::shrink().into_widget()
        }
    }

    /// Mounts a `_RestorableWidget`, under a root restoration scope when `in_scope`, and hands
    /// back its state.
    fn mount(cell: &AppCell, in_scope: bool) -> Handle<RestorableWidgetState> {
        let key = GlobalKey::new();
        let widget = RestorableWidget {
            key: key.clone().into(),
        };
        let root = if in_scope {
            RootRestorationScope::new(Some("root-child".to_string()), widget).into_widget()
        } else {
            widget.into_widget()
        };
        build(cell, root);
        key.current_state::<RestorableWidgetState>(&cell.borrow_mut())
            .expect("a _RestorableWidgetState")
    }

    fn value(app: &App, state: Handle<RestorableWidgetState>) -> TimeOfDay {
        *RestorableValue::value(state.time_of_day(app).handle(), app)
    }

    // time_test.dart 'RestorableTimeOfDay tests: value is not accessible when not registered'.
    #[test]
    #[should_panic(expected = "is_registered")]
    fn value_is_not_accessible_when_not_registered() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let property = RestorableTimeOfDay::new(&mut app, TimeOfDay::new(20, 4));
        RestorableValue::value(property.handle(), &app);
    }

    // time_test.dart 'RestorableTimeOfDay tests: work when not in restoration scope'.
    #[test]
    fn a_property_works_when_not_in_a_restoration_scope() {
        let cell = test_cell();
        let state = mount(&cell, false);
        let app = &mut cell.borrow_mut();

        // Initialized to default values.
        assert_eq!(value(app, state), TimeOfDay::new(10, 5));

        // Modify values.
        state.set_properties(app, |app| {
            state.time_of_day(app).set_value(app, TimeOfDay::new(2, 2));
        });
        pump(app, Duration::ZERO);

        assert_eq!(value(app, state), TimeOfDay::new(2, 2));
    }

    // time_test.dart 'RestorableTimeOfDay tests: restart and restore'.
    #[test]
    fn a_restarted_app_restores_the_time_of_day() {
        let (cell, restoration) = restoring_cell(None);
        let state = mount(&cell, true);
        {
            let app = &mut cell.borrow_mut();

            // Initialized to default values.
            assert_eq!(value(app, state), TimeOfDay::new(10, 5));

            // Modify values.
            state.set_properties(app, |app| {
                state.time_of_day(app).set_value(app, TimeOfDay::new(2, 2));
            });
            pump(app, Duration::ZERO);

            assert_eq!(value(app, state), TimeOfDay::new(2, 2));
        }

        // Restores to previous values.
        let (restarted, _) = restoring_cell(Some(restoration.last_put()));
        let restarted_state = mount(&restarted, true);
        let app = &mut restarted.borrow_mut();
        assert_eq!(value(app, restarted_state), TimeOfDay::new(2, 2));
    }

    // time_test.dart 'RestorableTimeOfDay tests: restore to older state'.
    #[test]
    fn a_property_restores_to_older_data_and_to_its_default_from_empty_data() {
        let (cell, restoration) = restoring_cell(None);
        let state = mount(&cell, true);
        let app = &mut cell.borrow_mut();

        // Modify values.
        state.set_properties(app, |app| {
            state.time_of_day(app).set_value(app, TimeOfDay::new(2, 2));
        });
        pump(app, Duration::ZERO);

        let restoration_data = restoration.last_put();

        // Modify values.
        state.set_properties(app, |app| {
            state.time_of_day(app).set_value(app, TimeOfDay::new(4, 4));
        });
        pump(app, Duration::ZERO);

        // Restore to previous.
        let manager = RestorationManager::instance(app);
        manager.handle_restoration_update_from_engine(app, true, Some(restoration_data));
        pump(app, Duration::ZERO);
        assert_eq!(value(app, state), TimeOfDay::new(2, 2));

        // Restore to empty data will re-initialize to default values.
        manager.handle_restoration_update_from_engine(app, true, Some(RestorationMap::new()));
        pump(app, Duration::ZERO);
        assert_eq!(value(app, state), TimeOfDay::new(10, 5));
    }

    // time_test.dart 'RestorableTimeOfDay tests: call notifiers when value changes'.
    #[test]
    fn a_property_notifies_only_when_its_value_changes() {
        let (cell, _) = restoring_cell(None);
        let state = mount(&cell, true);
        let app = &mut cell.borrow_mut();

        let notify_log = Rc::new(RefCell::new(Vec::<&str>::new()));
        state.time_of_day(app).add_listener(app, {
            let notify_log = Rc::clone(&notify_log);
            Listener::new(move |_app| notify_log.borrow_mut().push("hello world"))
        });

        state.set_properties(app, |app| {
            state.time_of_day(app).set_value(app, TimeOfDay::new(2, 2));
        });
        assert_eq!(*notify_log.borrow(), ["hello world"]);
        notify_log.borrow_mut().clear();
        pump(app, Duration::ZERO);

        // Does not notify when set to same value.
        state.set_properties(app, |app| {
            state.time_of_day(app).set_value(app, TimeOfDay::new(2, 2));
        });

        assert!(notify_log.borrow().is_empty());
    }

    #[test]
    fn the_restoration_data_is_the_minute_then_the_hour() {
        let (cell, restoration) = restoring_cell(None);
        let state = mount(&cell, true);
        let app = &mut cell.borrow_mut();
        state.set_properties(app, |app| {
            state.time_of_day(app).set_value(app, TimeOfDay::new(2, 7));
        });
        pump(app, Duration::ZERO);
        assert_eq!(
            RestorableProperty::to_primitives(state.time_of_day(app).handle(), app),
            RestorationData::List(vec![RestorationData::Int(7), RestorationData::Int(2)])
        );
        assert!(!restoration.puts.borrow().is_empty());
    }
}
