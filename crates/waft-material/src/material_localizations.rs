//! Flutter counterpart: material_ui's `material_localizations.dart`.
//!
//! US English only: [`DefaultMaterialLocalizations`] is the one implementation, and its
//! delegate supports `en`.

use std::fmt;
use std::rc::Rc;

use waft_embedder::Locale;
use waft_foundation::{App, CompleterFuture, DateTime};
use waft_widgets::{BuildContext, Localizations, LocalizationsDelegate, LocalizationsDelegateRef};

use crate::debug::debug_check_has_material_localizations;
use crate::time::{DayPeriod, TimeOfDay, TimeOfDayFormat};
use crate::typography::ScriptCategory;

/// Defines the localized resource values used by the Material widgets.
///
/// See also:
///
///  * [`DefaultMaterialLocalizations`], the default, English-only, implementation
///    of this interface.
///  * `GlobalMaterialLocalizations`, which provides material localizations for
///    many languages.
pub trait MaterialLocalizations: 'static {
    /// The tooltip for the leading `AppBar` menu (a.k.a. 'hamburger') button.
    fn open_app_drawer_tooltip(&self) -> String;

    /// The `BackButton`'s tooltip.
    fn back_button_tooltip(&self) -> String;

    /// The tooltip for the clear button to clear text on `SearchAnchor`'s search view.
    fn clear_button_tooltip(&self) -> String;

    /// The `CloseButton`'s tooltip.
    fn close_button_tooltip(&self) -> String;

    /// The tooltip for the delete button on a `Chip`.
    fn delete_button_tooltip(&self) -> String;

    /// The tooltip for the more button on an overflowing text selection menu.
    fn more_button_tooltip(&self) -> String;

    /// The tooltip for the `CalendarDatePicker`'s "next month" button.
    fn next_month_tooltip(&self) -> String;

    /// The tooltip for the `CalendarDatePicker`'s "previous month" button.
    fn previous_month_tooltip(&self) -> String;

    /// The tooltip for the `PaginatedDataTable`'s "first page" button.
    fn first_page_tooltip(&self) -> String;

    /// The tooltip for the `PaginatedDataTable`'s "last page" button.
    fn last_page_tooltip(&self) -> String;

    /// The tooltip for the `PaginatedDataTable`'s "next page" button.
    fn next_page_tooltip(&self) -> String;

    /// The tooltip for the `PaginatedDataTable`'s "previous page" button.
    fn previous_page_tooltip(&self) -> String;

    /// The default `PopupMenuButton` tooltip.
    fn show_menu_tooltip(&self) -> String;

    /// The default title for `AboutListTile`.
    fn about_list_tile_title(&self, application_name: &str) -> String;

    /// Title for the `LicensePage` widget.
    fn licenses_page_title(&self) -> String;

    /// Subtitle for a package in the `LicensePage` widget.
    fn licenses_package_detail_text(&self, license_count: i32) -> String;

    /// Title for the `PaginatedDataTable`'s row info footer.
    fn page_rows_info_title(
        &self,
        first_row: i32,
        last_row: i32,
        row_count: i32,
        row_count_is_approximate: bool,
    ) -> String;

    /// Title for the `PaginatedDataTable`'s "rows per page" footer.
    fn rows_per_page_title(&self) -> String;

    /// The accessibility label used on a tab in a `TabBar`.
    ///
    /// This message describes the index of the selected tab and how many tabs
    /// there are, e.g. 'Tab 1 of 2' in United States English.
    ///
    /// `tab_index` and `tab_count` must be greater than or equal to one.
    fn tab_label(&self, tab_index: i32, tab_count: i32) -> String;

    /// Title for the `PaginatedDataTable`'s selected row count header.
    fn selected_row_count_title(&self, selected_row_count: i32) -> String;

    /// Label for "cancel" buttons and menu items.
    fn cancel_button_label(&self) -> String;

    /// Label for "close" buttons and menu items.
    fn close_button_label(&self) -> String;

    /// Label for "continue" buttons and menu items.
    fn continue_button_label(&self) -> String;

    /// Label for "copy" edit buttons and menu items.
    fn copy_button_label(&self) -> String;

    /// Label for "cut" edit buttons and menu items.
    fn cut_button_label(&self) -> String;

    /// Label for "scan text" OCR edit buttons and menu items.
    fn scan_text_button_label(&self) -> String;

    /// Label for OK buttons and menu items.
    fn ok_button_label(&self) -> String;

    /// Label for "paste" edit buttons and menu items.
    fn paste_button_label(&self) -> String;

    /// Label for "select all" edit buttons and menu items.
    fn select_all_button_label(&self) -> String;

    /// Label for "look up" edit buttons and menu items.
    fn look_up_button_label(&self) -> String;

    /// Label for "search web" edit buttons and menu items.
    fn search_web_button_label(&self) -> String;

    /// Label for "share" edit buttons and menu items.
    fn share_button_label(&self) -> String;

    /// Label for the `AboutDialog` button that shows the `LicensePage`.
    fn view_licenses_button_label(&self) -> String;

    /// The abbreviation for ante meridiem (before noon) shown in the time picker.
    fn ante_meridiem_abbreviation(&self) -> String;

    /// The abbreviation for post meridiem (after noon) shown in the time picker.
    fn post_meridiem_abbreviation(&self) -> String;

    /// The text-to-speech announcement made when a time picker invoked using
    /// `showTimePicker` is set to the hour picker mode.
    fn time_picker_hour_mode_announcement(&self) -> String;

    /// The text-to-speech announcement made when a time picker invoked using
    /// `showTimePicker` is set to the minute picker mode.
    fn time_picker_minute_mode_announcement(&self) -> String;

    /// Label read out by accessibility tools (TalkBack or VoiceOver) for a modal
    /// barrier to indicate that a tap dismisses the barrier.
    ///
    /// A modal barrier can for example be found behind an alert or popup to block
    /// user interaction with elements behind it.
    fn modal_barrier_dismiss_label(&self) -> String;

    /// Label read out by accessibility tools (TalkBack or VoiceOver) for a
    /// context menu to indicate that a tap dismisses the context menu.
    fn menu_dismiss_label(&self) -> String;

    /// Label read out by accessibility tools (TalkBack or VoiceOver) when a
    /// drawer widget is opened.
    fn drawer_label(&self) -> String;

    /// Label read out by accessibility tools (TalkBack or VoiceOver) when a
    /// popup menu widget is opened.
    fn popup_menu_label(&self) -> String;

    /// Label read out by accessibility tools (TalkBack or VoiceOver) when a
    /// MenuBarMenu widget is opened.
    fn menu_bar_menu_label(&self) -> String;

    /// Label read out by accessibility tools (TalkBack or VoiceOver) when a
    /// dialog widget is opened.
    fn dialog_label(&self) -> String;

    /// Label read out by accessibility tools (TalkBack or VoiceOver) when an
    /// alert dialog widget is opened.
    fn alert_dialog_label(&self) -> String;

    /// Label indicating that a text field is a search field. This will be used
    /// as a hint text in the text field.
    fn search_field_label(&self) -> String;

    /// Label indicating that a given date is the current date.
    fn current_date_label(&self) -> String;

    /// The semantics label to describe the selected date in the calendar picker
    /// invoked using `showDatePicker`.
    fn selected_date_label(&self) -> String;

    /// Label for the scrim rendered underneath a `BottomSheet`.
    fn scrim_label(&self) -> String;

    /// Label for a `BottomSheet`, used as the `modalRouteContentName` of the
    /// [`scrim_on_tap_hint`](Self::scrim_on_tap_hint).
    fn bottom_sheet_label(&self) -> String;

    /// Hint text announced when tapping on the scrim underneath the content of
    /// a modal route.
    fn scrim_on_tap_hint(&self, modal_route_content_name: &str) -> String;

    /// The format used to lay out the time picker.
    ///
    /// The documentation for [`TimeOfDayFormat`] enum values provides details on
    /// each supported layout.
    fn time_of_day_format(&self, always_use_24_hour_format: bool) -> TimeOfDayFormat;

    /// Defines the localized `TextStyle` geometry for `ThemeData.textTheme`.
    ///
    /// The [`script_category`](Self::script_category) defines the overall geometry of a `TextTheme` for
    /// the `Typography.geometryThemeFor` method in terms of the
    /// three language categories defined in <https://material.io/go/design-typography>.
    ///
    /// Generally speaking, font sizes for [`ScriptCategory::Tall`] and
    /// [`ScriptCategory::Dense`] scripts - for text styles that are smaller than the
    /// title style - are one unit larger than they are for
    /// [`ScriptCategory::EnglishLike`] scripts.
    fn script_category(&self) -> ScriptCategory;

    /// Formats `number` as a decimal, inserting locale-appropriate thousands
    /// separators as necessary.
    fn format_decimal(&self, number: i64) -> String;

    /// Formats [`hour`](TimeOfDay::hour) in the given time of day according to the value
    /// of [`time_of_day_format`](Self::time_of_day_format).
    ///
    /// If `always_use_24_hour_format` is true, formats hour using [`HourFormat::HH`](crate::HourFormat::HH)
    /// rather than the default for the current locale.
    fn format_hour(&self, time_of_day: &TimeOfDay, always_use_24_hour_format: bool) -> String;

    /// Formats [`minute`](TimeOfDay::minute) in the given time of day according to the value
    /// of [`time_of_day_format`](Self::time_of_day_format).
    fn format_minute(&self, time_of_day: &TimeOfDay) -> String;

    /// Formats `time_of_day` according to the value of [`time_of_day_format`](Self::time_of_day_format).
    ///
    /// If `always_use_24_hour_format` is true, formats hour using [`HourFormat::HH`](crate::HourFormat::HH)
    /// rather than the default for the current locale. This value is usually
    /// passed from `MediaQueryData.alwaysUse24HourFormat`, which has platform-
    /// specific behavior.
    fn format_time_of_day(
        &self,
        time_of_day: &TimeOfDay,
        always_use_24_hour_format: bool,
    ) -> String;

    /// Full unabbreviated year format, e.g. 2017 rather than 17.
    fn format_year(&self, date: &DateTime) -> String;

    /// Formats the date in a compact format.
    ///
    /// Usually just the numeric values for the for day, month and year are used.
    ///
    /// Examples:
    ///
    /// - US English: 02/21/2019
    /// - Russian: 21.02.2019
    ///
    /// See also:
    ///   * [`parse_compact_date`](Self::parse_compact_date), which will convert a compact date string to a [`DateTime`].
    fn format_compact_date(&self, date: &DateTime) -> String;

    /// Formats the date using a short-width format.
    ///
    /// Includes the abbreviation of the month, the day and year.
    ///
    /// Examples:
    ///
    /// - US English: Feb 21, 2019
    /// - Russian: 21 февр. 2019 г.
    fn format_short_date(&self, date: &DateTime) -> String;

    /// Formats the date using a medium-width format.
    ///
    /// Abbreviates month and days of week. This appears in the header of the date
    /// picker invoked using `showDatePicker`.
    ///
    /// Examples:
    ///
    /// - US English: Wed, Sep 27
    /// - Russian: ср, сент. 27
    fn format_medium_date(&self, date: &DateTime) -> String;

    /// Formats day of week, month, day of month and year in a long-width format.
    ///
    /// Does not abbreviate names. Appears in spoken announcements of the date
    /// picker invoked using `showDatePicker`, when accessibility mode is on.
    ///
    /// Examples:
    ///
    /// - US English: Wednesday, September 27, 2017
    /// - Russian: Среда, Сентябрь 27, 2017
    fn format_full_date(&self, date: &DateTime) -> String;

    /// Formats the month and the year of the given `date`.
    ///
    /// The returned string does not contain the day of the month. This appears
    /// in the date picker invoked using `showDatePicker`.
    fn format_month_year(&self, date: &DateTime) -> String;

    /// Formats the month and day of the given `date`.
    ///
    /// Examples:
    ///
    /// - US English: Feb 21
    /// - Russian: 21 февр.
    fn format_short_month_day(&self, date: &DateTime) -> String;

    /// Converts the given compact date formatted string into a [`DateTime`].
    ///
    /// The format of the string must be a valid compact date format for the
    /// given locale. If the text doesn't represent a valid date, `None` will be
    /// returned.
    ///
    /// See also:
    ///   * [`format_compact_date`](Self::format_compact_date), which will convert a [`DateTime`] into a string in the compact format.
    fn parse_compact_date(&self, input_string: Option<&str>) -> Option<DateTime>;

    /// List of week day names in narrow format, usually 1- or 2-letter
    /// abbreviations of full names.
    ///
    /// The list begins with the value corresponding to Sunday and ends with
    /// Saturday. Use [`first_day_of_week_index`](Self::first_day_of_week_index) to find the first day of week in this
    /// list.
    ///
    /// Examples:
    ///
    /// - US English: S, M, T, W, T, F, S
    /// - Russian: вс, пн, вт, ср, чт, пт, сб - notice that the list begins with
    ///   вс (Sunday) even though the first day of week for Russian is Monday.
    fn narrow_weekdays(&self) -> Vec<String>;

    /// Index of the first day of week, where 0 points to Sunday, and 6 points to
    /// Saturday.
    ///
    /// This getter is compatible with [`narrow_weekdays`](Self::narrow_weekdays). For example:
    ///
    /// ```text
    /// let localizations = <dyn MaterialLocalizations>::of(app, context);
    /// // The name of the first day of week for the current locale.
    /// let first_day_of_week = &localizations.narrow_weekdays()[localizations.first_day_of_week_index() as usize];
    /// ```
    fn first_day_of_week_index(&self) -> i32;

    /// The character string used to separate the parts of a compact date format
    /// (i.e. mm/dd/yyyy has a separator of '/').
    fn date_separator(&self) -> String;

    /// The help text used on an empty `InputDatePickerFormField` to indicate
    /// to the user the date format being asked for.
    fn date_help_text(&self) -> String;

    /// The semantic label used to announce when the user has entered the year
    /// selection mode of the `CalendarDatePicker` which is used in the data picker
    /// dialog created with `showDatePicker`.
    fn select_year_semantics_label(&self) -> String;

    /// The label used to indicate a date that has not been entered or selected
    /// yet in the date picker.
    fn unspecified_date(&self) -> String;

    /// The label used to indicate a date range that has not been entered or
    /// selected yet in the date range picker.
    fn unspecified_date_range(&self) -> String;

    /// The label used to describe the text field used in an `InputDatePickerFormField`.
    fn date_input_label(&self) -> String;

    /// The label used for the starting date input field in the date range picker
    /// created with `showDateRangePicker`.
    fn date_range_start_label(&self) -> String;

    /// The label used for the ending date input field in the date range picker
    /// created with `showDateRangePicker`.
    fn date_range_end_label(&self) -> String;

    /// The semantics label used for the selected start date in the date range
    /// picker's day grid.
    fn date_range_start_date_semantic_label(&self, formatted_date: &str) -> String;

    /// The semantics label used for the selected end date in the date range
    /// picker's day grid.
    fn date_range_end_date_semantic_label(&self, formatted_date: &str) -> String;

    /// Error message displayed to the user when they have entered a text string
    /// in an `InputDatePickerFormField` that is not in a valid date format.
    fn invalid_date_format_label(&self) -> String;

    /// Error message displayed to the user when they have entered an invalid
    /// date range in the input mode of the date range picker created with
    /// `showDateRangePicker`.
    fn invalid_date_range_label(&self) -> String;

    /// Error message displayed to the user when they have entered a date that
    /// is outside the valid range for the date picker.
    /// `showDateRangePicker`.
    fn date_out_of_range_label(&self) -> String;

    /// Label for a 'SAVE' button. Currently used by the full screen mode of the
    /// date range picker.
    fn save_button_label(&self) -> String;

    /// Label used in the header of the date picker dialog created with
    /// `showDatePicker`.
    fn date_picker_help_text(&self) -> String;

    /// Label used in the header of the date range picker dialog created with
    /// `showDateRangePicker`.
    fn date_range_picker_help_text(&self) -> String;

    /// Tooltip used for the calendar mode button of the date pickers.
    fn calendar_mode_button_label(&self) -> String;

    /// Tooltip used for the text input mode button of the date pickers.
    fn input_date_mode_button_label(&self) -> String;

    /// Label used in the header of the time picker dialog created with
    /// `showTimePicker` when in `TimePickerEntryMode.dial`.
    fn time_picker_dial_help_text(&self) -> String;

    /// Label used in the header of the time picker dialog created with
    /// `showTimePicker` when in `TimePickerEntryMode.input`.
    fn time_picker_input_help_text(&self) -> String;

    /// Label used below the hour text field of the time picker dialog created
    /// with `showTimePicker` when in `TimePickerEntryMode.input`.
    fn time_picker_hour_label(&self) -> String;

    /// Label used below the minute text field of the time picker dialog created
    /// with `showTimePicker` when in `TimePickerEntryMode.input`.
    fn time_picker_minute_label(&self) -> String;

    /// Error message for the time picker dialog created with `showTimePicker`
    /// when in `TimePickerEntryMode.input`.
    fn invalid_time_label(&self) -> String;

    /// Tooltip used to put the time picker into `TimePickerEntryMode.dial`.
    fn dial_mode_button_label(&self) -> String;

    /// Tooltip used to put the time picker into `TimePickerEntryMode.input`.
    fn input_time_mode_button_label(&self) -> String;

    /// The semantics label used to indicate which account is signed in the
    /// `UserAccountsDrawerHeader` widget.
    fn signed_in_label(&self) -> String;

    /// The semantics label used for the button on `UserAccountsDrawerHeader` that
    /// hides the list of accounts.
    fn hide_accounts_label(&self) -> String;

    /// The semantics label used for the button on `UserAccountsDrawerHeader` that
    /// shows the list of accounts.
    fn show_accounts_label(&self) -> String;

    /// The semantics label used for `ReorderableListView` to reorder an item in the
    /// list to the start of the list.
    #[deprecated(
        note = "Use the reorderItemToStart from WidgetsLocalizations instead. This feature was deprecated after v3.10.0-2.0.pre."
    )]
    fn reorder_item_to_start(&self) -> String;

    /// The semantics label used for `ReorderableListView` to reorder an item in the
    /// list to the end of the list.
    #[deprecated(
        note = "Use the reorderItemToEnd from WidgetsLocalizations instead. This feature was deprecated after v3.10.0-2.0.pre."
    )]
    fn reorder_item_to_end(&self) -> String;

    /// The semantics label used for `ReorderableListView` to reorder an item in the
    /// list one space up the list.
    #[deprecated(
        note = "Use the reorderItemUp from WidgetsLocalizations instead. This feature was deprecated after v3.10.0-2.0.pre."
    )]
    fn reorder_item_up(&self) -> String;

    /// The semantics label used for `ReorderableListView` to reorder an item in the
    /// list one space down the list.
    #[deprecated(
        note = "Use the reorderItemDown from WidgetsLocalizations instead. This feature was deprecated after v3.10.0-2.0.pre."
    )]
    fn reorder_item_down(&self) -> String;

    /// The semantics label used for `ReorderableListView` to reorder an item in the
    /// list one space left in the list.
    #[deprecated(
        note = "Use the reorderItemLeft from WidgetsLocalizations instead. This feature was deprecated after v3.10.0-2.0.pre."
    )]
    fn reorder_item_left(&self) -> String;

    /// The semantics label used for `ReorderableListView` to reorder an item in the
    /// list one space right in the list.
    #[deprecated(
        note = "Use the reorderItemRight from WidgetsLocalizations instead. This feature was deprecated after v3.10.0-2.0.pre."
    )]
    fn reorder_item_right(&self) -> String;

    /// The semantics hint to describe the tap action on an expanded `ExpandIcon`.
    fn expanded_icon_tap_hint(&self) -> String {
        String::from("Collapse")
    }

    /// The semantics hint to describe the tap action on a collapsed `ExpandIcon`.
    fn collapsed_icon_tap_hint(&self) -> String {
        String::from("Expand")
    }

    /// The semantics hint to describe the tap action on an expanded
    /// `ExpansionTile` on iOS and macOS. This is appended to the [`collapsed_hint`](Self::collapsed_hint)
    /// hint to provide a more detailed description of the action, e.g. "Expanded
    /// double tap to collapse".
    fn expansion_tile_expanded_hint(&self) -> String {
        String::from("double tap to collapse")
    }

    /// The semantics hint to describe the tap action on a collapsed
    /// `ExpansionTile` on iOS and macOS. This is appended to the [`expanded_hint`](Self::expanded_hint)
    /// hint to provide a more detailed description of the action, e.g. "Collapsed
    /// double tap to expand".
    fn expansion_tile_collapsed_hint(&self) -> String {
        String::from("double tap to expand")
    }

    /// The semantics hint to describe the tap action on an expanded `ExpansionTile`.
    fn expansion_tile_expanded_tap_hint(&self) -> String {
        String::from("Collapse")
    }

    /// The semantics hint to describe the tap action on a collapsed `ExpansionTile`.
    fn expansion_tile_collapsed_tap_hint(&self) -> String {
        String::from("Expand for more details")
    }

    /// The semantics hint to describe the `ExpansionTile` expanded state.
    fn expanded_hint(&self) -> String {
        String::from("Collapsed")
    }

    /// The semantics hint to describe the `ExpansionTile` collapsed state.
    fn collapsed_hint(&self) -> String {
        String::from("Expanded")
    }

    /// The label for the `TextField`'s character counter.
    fn remaining_text_field_character_count(&self, remaining: i32) -> String;

    /// The default semantics label for a `RefreshIndicator`.
    fn refresh_indicator_semantic_label(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.alt`.
    fn keyboard_key_alt(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.altGraph`.
    fn keyboard_key_alt_graph(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.backspace`.
    fn keyboard_key_backspace(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.capsLock`.
    fn keyboard_key_caps_lock(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.channelDown`.
    fn keyboard_key_channel_down(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.channelUp`.
    fn keyboard_key_channel_up(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.control`.
    fn keyboard_key_control(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.delete`.
    fn keyboard_key_delete(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.eject`.
    fn keyboard_key_eject(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.end`.
    fn keyboard_key_end(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.escape`.
    fn keyboard_key_escape(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.fn`.
    fn keyboard_key_fn(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.home`.
    fn keyboard_key_home(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.insert`.
    fn keyboard_key_insert(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.meta`.
    fn keyboard_key_meta(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.meta` on macOS.
    fn keyboard_key_meta_mac_os(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.meta` on Windows.
    fn keyboard_key_meta_windows(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numLock`.
    fn keyboard_key_num_lock(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad1`.
    fn keyboard_key_numpad1(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad2`.
    fn keyboard_key_numpad2(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad3`.
    fn keyboard_key_numpad3(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad4`.
    fn keyboard_key_numpad4(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad5`.
    fn keyboard_key_numpad5(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad6`.
    fn keyboard_key_numpad6(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad7`.
    fn keyboard_key_numpad7(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad8`.
    fn keyboard_key_numpad8(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad9`.
    fn keyboard_key_numpad9(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpad0`.
    fn keyboard_key_numpad0(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadAdd`.
    fn keyboard_key_numpad_add(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadComma`.
    fn keyboard_key_numpad_comma(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadDecimal`.
    fn keyboard_key_numpad_decimal(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadDivide`.
    fn keyboard_key_numpad_divide(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadEnter`.
    fn keyboard_key_numpad_enter(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadEqual`.
    fn keyboard_key_numpad_equal(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadMultiply`.
    fn keyboard_key_numpad_multiply(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadParenLeft`.
    fn keyboard_key_numpad_paren_left(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadParenRight`.
    fn keyboard_key_numpad_paren_right(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.numpadSubtract`.
    fn keyboard_key_numpad_subtract(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.pageDown`.
    fn keyboard_key_page_down(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.pageUp`.
    fn keyboard_key_page_up(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.power`.
    fn keyboard_key_power(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.powerOff`.
    fn keyboard_key_power_off(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.printScreen`.
    fn keyboard_key_print_screen(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.scrollLock`.
    fn keyboard_key_scroll_lock(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.select`.
    fn keyboard_key_select(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.shift`.
    fn keyboard_key_shift(&self) -> String;

    /// The shortcut label for the keyboard key `LogicalKeyboardKey.space`.
    fn keyboard_key_space(&self) -> String;
}

impl dyn MaterialLocalizations {
    /// The `MaterialLocalizations` from the closest [`Localizations`] instance
    /// that encloses the given context.
    ///
    /// If no [`MaterialLocalizations`] are available in the given `context`, this
    /// method throws an exception.
    ///
    /// This method is just a convenient shorthand for:
    /// `Localizations::of::<dyn MaterialLocalizations>(app, context)`.
    ///
    /// References to the localized resources defined by this class are typically
    /// written in terms of this method. For example:
    ///
    /// ```text
    /// tooltip: <dyn MaterialLocalizations>::of(app, context).back_button_tooltip(),
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> Rc<dyn MaterialLocalizations> {
        debug_assert!(debug_check_has_material_localizations(app, context));
        Localizations::of::<dyn MaterialLocalizations>(app, context)
            .expect("No MaterialLocalizations found.")
    }
}

struct MaterialLocalizationsDelegate;

impl LocalizationsDelegate<dyn MaterialLocalizations> for MaterialLocalizationsDelegate {
    fn is_supported(&self, locale: &Locale) -> bool {
        locale.language_code == "en"
    }

    fn load(
        &self,
        _app: &mut App,
        locale: &Locale,
    ) -> CompleterFuture<Rc<dyn MaterialLocalizations>> {
        DefaultMaterialLocalizations::load(locale)
    }

    fn should_reload(&self, _old: &Self) -> bool {
        false
    }
}

impl fmt::Debug for MaterialLocalizationsDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DefaultMaterialLocalizations.delegate(en_US)")
    }
}

/// US English strings for the material widgets.
///
/// See also:
///
///  * `GlobalMaterialLocalizations`, which provides material localizations for
///    many languages.
///  * `MaterialApp.localizationsDelegates`, which automatically includes
///    [`DefaultMaterialLocalizations::delegate`] by default.
#[derive(Debug, Default)]
pub struct DefaultMaterialLocalizations;

impl DefaultMaterialLocalizations {
    /// Constructs an object that defines the material widgets' localized strings
    /// for US English (only).
    ///
    /// [`LocalizationsDelegate`] implementations typically call the static [`load`](Self::load)
    /// function, rather than constructing this class directly.
    pub const fn new() -> DefaultMaterialLocalizations {
        DefaultMaterialLocalizations
    }

    // Ordered to match DateTime.monday=1, DateTime.sunday=6
    const SHORT_WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    // Ordered to match DateTime.monday=1, DateTime.sunday=6
    const WEEKDAYS: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];

    const NARROW_WEEKDAYS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];

    const SHORT_MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    /// Returns the number of days in a month, according to the proleptic
    /// Gregorian calendar.
    ///
    /// This applies the leap year logic introduced by the Gregorian reforms of
    /// 1582. It will not give valid results for dates prior to that time.
    fn get_days_in_month(year: i64, month: i64) -> i64 {
        if month == i64::from(DateTime::FEBRUARY) {
            let is_leap_year = (year % 4 == 0) && (year % 100 != 0) || (year % 400 == 0);
            if is_leap_year {
                return 29;
            }
            return 28;
        }
        const DAYS_IN_MONTH: [i64; 12] = [31, -1, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        DAYS_IN_MONTH[usize::try_from(month - 1).expect("a month in 1..=12")]
    }

    /// Formats `number` using two digits, assuming it's in the 0-99 inclusive
    /// range. Not designed to format values outside this range.
    fn format_two_digit_zero_pad(number: i32) -> String {
        debug_assert!((0..100).contains(&number));

        if number < 10 {
            return format!("0{number}");
        }

        number.to_string()
    }

    fn format_day_period(&self, time_of_day: &TimeOfDay) -> String {
        match time_of_day.period() {
            DayPeriod::Am => self.ante_meridiem_abbreviation(),
            DayPeriod::Pm => self.post_meridiem_abbreviation(),
        }
    }

    /// Creates an object that provides US English resource values for the material
    /// library widgets.
    ///
    /// The `locale` parameter is ignored.
    ///
    /// This method is typically used to create a [`LocalizationsDelegate`].
    /// The `MaterialApp` does so by default.
    pub fn load(_locale: &Locale) -> CompleterFuture<Rc<dyn MaterialLocalizations>> {
        CompleterFuture::ready(Rc::new(DefaultMaterialLocalizations::new()))
    }

    /// A [`LocalizationsDelegate`] that uses [`DefaultMaterialLocalizations::load`]
    /// to create an instance of this class.
    ///
    /// `MaterialApp` automatically adds this value to `MaterialApp.localizationsDelegates`.
    pub fn delegate() -> LocalizationsDelegateRef {
        LocalizationsDelegateRef::new(MaterialLocalizationsDelegate)
    }
}

/// The index of `date`'s month in a table that starts with January.
fn month_index(date: &DateTime) -> usize {
    usize::try_from(date.month() - DateTime::JANUARY).expect("a month in 1..=12")
}

/// The index of `date`'s weekday in a table that starts with Monday.
fn weekday_index(date: &DateTime) -> usize {
    usize::try_from(date.weekday() - DateTime::MONDAY).expect("a weekday in 1..=7")
}

/// Dart's `int.tryParse(source, radix: 10)`: surrounding whitespace (Dart's, which counts
/// U+FEFF) and a sign are accepted, and a value outside 64 bits is `None`.
fn try_parse_int(source: &str) -> Option<i64> {
    source
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
        .parse()
        .ok()
}

/// Dart's `DateTime(year, month, day)` for a valid date from year 1 on, with the
/// `ArgumentError` it throws past the last day a `DateTime` can represent as `None`.
fn date_time_or_none(year: i64, month: i64, day: i64) -> Option<DateTime> {
    // 100,000,000 days after the epoch.
    const LAST_REPRESENTABLE_DAY: (i64, i64, i64) = (275_760, 9, 13);
    if (year, month, day) > LAST_REPRESENTABLE_DAY {
        return None;
    }
    let field = |value: i64| i32::try_from(value).expect("within the representable range");
    Some(DateTime::utc(field(year), field(month), field(day)))
}

impl MaterialLocalizations for DefaultMaterialLocalizations {
    fn format_hour(&self, time_of_day: &TimeOfDay, always_use_24_hour_format: bool) -> String {
        let format = self.time_of_day_format(always_use_24_hour_format);
        match format {
            TimeOfDayFormat::h_colon_mm_space_a => {
                self.format_decimal(i64::from(if time_of_day.hour_of_period() == 0 {
                    12
                } else {
                    time_of_day.hour_of_period()
                }))
            }
            TimeOfDayFormat::HH_colon_mm => Self::format_two_digit_zero_pad(time_of_day.hour),
            TimeOfDayFormat::a_space_h_colon_mm
            | TimeOfDayFormat::FrenchCanadian
            | TimeOfDayFormat::H_colon_mm
            | TimeOfDayFormat::HH_dot_mm => {
                panic!("DefaultMaterialLocalizations does not support {format:?}.")
            }
        }
    }

    fn format_minute(&self, time_of_day: &TimeOfDay) -> String {
        let minute = time_of_day.minute;
        if minute < 10 {
            format!("0{minute}")
        } else {
            minute.to_string()
        }
    }

    fn format_year(&self, date: &DateTime) -> String {
        date.year().to_string()
    }

    fn format_compact_date(&self, date: &DateTime) -> String {
        // Assumes US mm/dd/yyyy format
        let month = Self::format_two_digit_zero_pad(date.month());
        let day = Self::format_two_digit_zero_pad(date.day());
        let year = format!("{:0>4}", date.year().to_string());
        format!("{month}/{day}/{year}")
    }

    fn format_short_date(&self, date: &DateTime) -> String {
        let month = Self::SHORT_MONTHS[month_index(date)];
        format!("{month} {}, {}", date.day(), date.year())
    }

    fn format_medium_date(&self, date: &DateTime) -> String {
        let day = Self::SHORT_WEEKDAYS[weekday_index(date)];
        let month = Self::SHORT_MONTHS[month_index(date)];
        format!("{day}, {month} {}", date.day())
    }

    fn format_full_date(&self, date: &DateTime) -> String {
        let month = Self::MONTHS[month_index(date)];
        format!(
            "{}, {month} {}, {}",
            Self::WEEKDAYS[weekday_index(date)],
            date.day(),
            date.year()
        )
    }

    fn format_month_year(&self, date: &DateTime) -> String {
        let year = self.format_year(date);
        let month = Self::MONTHS[month_index(date)];
        format!("{month} {year}")
    }

    fn format_short_month_day(&self, date: &DateTime) -> String {
        let month = Self::SHORT_MONTHS[month_index(date)];
        format!("{month} {}", date.day())
    }

    fn parse_compact_date(&self, input_string: Option<&str>) -> Option<DateTime> {
        let input_string = input_string?;

        // Assumes US mm/dd/yyyy format
        let input_parts: Vec<&str> = input_string.split('/').collect();
        if input_parts.len() != 3 {
            return None;
        }

        let year = try_parse_int(input_parts[2]).filter(|&year| year >= 1)?;

        let month = try_parse_int(input_parts[0]).filter(|month| (1..=12).contains(month))?;

        let day = try_parse_int(input_parts[1])
            .filter(|&day| day >= 1 && day <= Self::get_days_in_month(year, month))?;

        date_time_or_none(year, month, day)
    }

    fn narrow_weekdays(&self) -> Vec<String> {
        Self::NARROW_WEEKDAYS.map(String::from).to_vec()
    }

    fn first_day_of_week_index(&self) -> i32 {
        0 // narrow_weekdays[0] is 'S' for Sunday
    }

    fn date_separator(&self) -> String {
        String::from("/")
    }

    fn date_help_text(&self) -> String {
        String::from("mm/dd/yyyy")
    }

    fn select_year_semantics_label(&self) -> String {
        String::from("Select year")
    }

    fn unspecified_date(&self) -> String {
        String::from("Date")
    }

    fn unspecified_date_range(&self) -> String {
        String::from("Date Range")
    }

    fn date_input_label(&self) -> String {
        String::from("Enter Date")
    }

    fn date_range_start_label(&self) -> String {
        String::from("Start Date")
    }

    fn date_range_end_label(&self) -> String {
        String::from("End Date")
    }

    fn date_range_start_date_semantic_label(&self, formatted_date: &str) -> String {
        format!("Start date {formatted_date}")
    }

    fn date_range_end_date_semantic_label(&self, formatted_date: &str) -> String {
        format!("End date {formatted_date}")
    }

    fn invalid_date_format_label(&self) -> String {
        String::from("Invalid format.")
    }

    fn invalid_date_range_label(&self) -> String {
        String::from("Invalid range.")
    }

    fn date_out_of_range_label(&self) -> String {
        String::from("Out of range.")
    }

    fn save_button_label(&self) -> String {
        String::from("Save")
    }

    fn date_picker_help_text(&self) -> String {
        String::from("Select date")
    }

    fn date_range_picker_help_text(&self) -> String {
        String::from("Select range")
    }

    fn calendar_mode_button_label(&self) -> String {
        String::from("Switch to calendar")
    }

    fn input_date_mode_button_label(&self) -> String {
        String::from("Switch to input")
    }

    fn time_picker_dial_help_text(&self) -> String {
        String::from("Select time")
    }

    fn time_picker_input_help_text(&self) -> String {
        String::from("Enter time")
    }

    fn time_picker_hour_label(&self) -> String {
        String::from("Hour")
    }

    fn time_picker_minute_label(&self) -> String {
        String::from("Minute")
    }

    fn invalid_time_label(&self) -> String {
        String::from("Enter a valid time")
    }

    fn dial_mode_button_label(&self) -> String {
        String::from("Switch to dial picker mode")
    }

    fn input_time_mode_button_label(&self) -> String {
        String::from("Switch to text input mode")
    }

    fn format_decimal(&self, number: i64) -> String {
        if number > -1000 && number < 1000 {
            return number.to_string();
        }

        // Dart's 64-bit `abs`, under which the smallest int stays negative.
        let digits = number.wrapping_abs().to_string();
        let mut result = String::from(if number < 0 { "-" } else { "" });
        let max_digit_index = digits.len() - 1;
        for (i, digit) in digits.chars().enumerate() {
            result.push(digit);
            if i < max_digit_index && (max_digit_index - i).is_multiple_of(3) {
                result.push(',');
            }
        }
        result
    }

    fn format_time_of_day(
        &self,
        time_of_day: &TimeOfDay,
        always_use_24_hour_format: bool,
    ) -> String {
        // Not using intl.DateFormat for two reasons:
        //
        // - DateFormat supports more formats than our material time picker does,
        //   and we want to be consistent across time picker format and the string
        //   formatting of the time of day.
        // - DateFormat operates on DateTime, which is sensitive to time eras and
        //   time zones, while here we want to format hour and minute within one day
        //   no matter what date the day falls on.
        let mut buffer = String::new();

        // Add hour:minute.
        buffer.push_str(&self.format_hour(time_of_day, always_use_24_hour_format));
        buffer.push(':');
        buffer.push_str(&self.format_minute(time_of_day));

        if always_use_24_hour_format {
            // There's no AM/PM indicator in 24-hour format.
            return buffer;
        }

        // Add AM/PM indicator.
        buffer.push(' ');
        buffer.push_str(&self.format_day_period(time_of_day));
        buffer
    }

    fn open_app_drawer_tooltip(&self) -> String {
        String::from("Open navigation menu")
    }

    fn back_button_tooltip(&self) -> String {
        String::from("Back")
    }

    fn clear_button_tooltip(&self) -> String {
        String::from("Clear text")
    }

    fn close_button_tooltip(&self) -> String {
        String::from("Close")
    }

    fn delete_button_tooltip(&self) -> String {
        String::from("Delete")
    }

    fn more_button_tooltip(&self) -> String {
        String::from("More")
    }

    fn next_month_tooltip(&self) -> String {
        String::from("Next month")
    }

    fn previous_month_tooltip(&self) -> String {
        String::from("Previous month")
    }

    fn next_page_tooltip(&self) -> String {
        String::from("Next page")
    }

    fn previous_page_tooltip(&self) -> String {
        String::from("Previous page")
    }

    fn first_page_tooltip(&self) -> String {
        String::from("First page")
    }

    fn last_page_tooltip(&self) -> String {
        String::from("Last page")
    }

    fn show_menu_tooltip(&self) -> String {
        String::from("Show menu")
    }

    fn drawer_label(&self) -> String {
        String::from("Navigation menu")
    }

    fn menu_bar_menu_label(&self) -> String {
        String::from("Menu bar menu")
    }

    fn popup_menu_label(&self) -> String {
        String::from("Popup menu")
    }

    fn dialog_label(&self) -> String {
        String::from("Dialog")
    }

    fn alert_dialog_label(&self) -> String {
        String::from("Alert")
    }

    fn search_field_label(&self) -> String {
        String::from("Search")
    }

    fn current_date_label(&self) -> String {
        String::from("Today")
    }

    fn selected_date_label(&self) -> String {
        String::from("Selected")
    }

    fn scrim_label(&self) -> String {
        String::from("Scrim")
    }

    fn bottom_sheet_label(&self) -> String {
        String::from("Bottom Sheet")
    }

    fn scrim_on_tap_hint(&self, modal_route_content_name: &str) -> String {
        format!("Close {modal_route_content_name}")
    }

    fn about_list_tile_title(&self, application_name: &str) -> String {
        format!("About {application_name}")
    }

    fn licenses_page_title(&self) -> String {
        String::from("Licenses")
    }

    fn licenses_package_detail_text(&self, license_count: i32) -> String {
        debug_assert!(license_count >= 0);
        match license_count {
            0 => String::from("No licenses."),
            1 => String::from("1 license."),
            _ => format!("{license_count} licenses."),
        }
    }

    fn page_rows_info_title(
        &self,
        first_row: i32,
        last_row: i32,
        row_count: i32,
        row_count_is_approximate: bool,
    ) -> String {
        if row_count_is_approximate {
            format!("{first_row}–{last_row} of about {row_count}")
        } else {
            format!("{first_row}–{last_row} of {row_count}")
        }
    }

    fn rows_per_page_title(&self) -> String {
        String::from("Rows per page:")
    }

    fn tab_label(&self, tab_index: i32, tab_count: i32) -> String {
        debug_assert!(tab_index >= 1);
        debug_assert!(tab_count >= 1);
        format!("Tab {tab_index} of {tab_count}")
    }

    fn selected_row_count_title(&self, selected_row_count: i32) -> String {
        match selected_row_count {
            0 => String::from("No items selected"),
            1 => String::from("1 item selected"),
            _ => format!("{selected_row_count} items selected"),
        }
    }

    fn cancel_button_label(&self) -> String {
        String::from("Cancel")
    }

    fn close_button_label(&self) -> String {
        String::from("Close")
    }

    fn continue_button_label(&self) -> String {
        String::from("Continue")
    }

    fn copy_button_label(&self) -> String {
        String::from("Copy")
    }

    fn cut_button_label(&self) -> String {
        String::from("Cut")
    }

    fn scan_text_button_label(&self) -> String {
        String::from("Scan text")
    }

    fn ok_button_label(&self) -> String {
        String::from("OK")
    }

    fn paste_button_label(&self) -> String {
        String::from("Paste")
    }

    fn select_all_button_label(&self) -> String {
        String::from("Select all")
    }

    fn look_up_button_label(&self) -> String {
        String::from("Look Up")
    }

    fn search_web_button_label(&self) -> String {
        String::from("Search Web")
    }

    fn share_button_label(&self) -> String {
        String::from("Share")
    }

    fn view_licenses_button_label(&self) -> String {
        String::from("View licenses")
    }

    fn ante_meridiem_abbreviation(&self) -> String {
        String::from("AM")
    }

    fn post_meridiem_abbreviation(&self) -> String {
        String::from("PM")
    }

    fn time_picker_hour_mode_announcement(&self) -> String {
        String::from("Select hours")
    }

    fn time_picker_minute_mode_announcement(&self) -> String {
        String::from("Select minutes")
    }

    fn modal_barrier_dismiss_label(&self) -> String {
        String::from("Dismiss")
    }

    fn menu_dismiss_label(&self) -> String {
        String::from("Dismiss menu")
    }

    fn script_category(&self) -> ScriptCategory {
        ScriptCategory::EnglishLike
    }

    fn time_of_day_format(&self, always_use_24_hour_format: bool) -> TimeOfDayFormat {
        if always_use_24_hour_format {
            TimeOfDayFormat::HH_colon_mm
        } else {
            TimeOfDayFormat::h_colon_mm_space_a
        }
    }

    fn signed_in_label(&self) -> String {
        String::from("Signed in")
    }

    fn hide_accounts_label(&self) -> String {
        String::from("Hide accounts")
    }

    fn show_accounts_label(&self) -> String {
        String::from("Show accounts")
    }

    fn reorder_item_up(&self) -> String {
        String::from("Move up")
    }

    fn reorder_item_down(&self) -> String {
        String::from("Move down")
    }

    fn reorder_item_left(&self) -> String {
        String::from("Move left")
    }

    fn reorder_item_right(&self) -> String {
        String::from("Move right")
    }

    fn reorder_item_to_end(&self) -> String {
        String::from("Move to the end")
    }

    fn reorder_item_to_start(&self) -> String {
        String::from("Move to the start")
    }

    fn expanded_icon_tap_hint(&self) -> String {
        String::from("Collapse")
    }

    fn collapsed_icon_tap_hint(&self) -> String {
        String::from("Expand")
    }

    fn expansion_tile_expanded_hint(&self) -> String {
        String::from("double tap to collapse")
    }

    fn expansion_tile_collapsed_hint(&self) -> String {
        String::from("double tap to expand")
    }

    fn expansion_tile_expanded_tap_hint(&self) -> String {
        String::from("Collapse")
    }

    fn expansion_tile_collapsed_tap_hint(&self) -> String {
        String::from("Expand for more details")
    }

    fn expanded_hint(&self) -> String {
        String::from("Collapsed")
    }

    fn collapsed_hint(&self) -> String {
        String::from("Expanded")
    }

    fn refresh_indicator_semantic_label(&self) -> String {
        String::from("Refresh")
    }

    fn remaining_text_field_character_count(&self, remaining: i32) -> String {
        match remaining {
            0 => String::from("No characters remaining"),
            1 => String::from("1 character remaining"),
            _ => format!("{remaining} characters remaining"),
        }
    }

    fn keyboard_key_alt(&self) -> String {
        String::from("Alt")
    }

    fn keyboard_key_alt_graph(&self) -> String {
        String::from("AltGr")
    }

    fn keyboard_key_backspace(&self) -> String {
        String::from("Backspace")
    }

    fn keyboard_key_caps_lock(&self) -> String {
        String::from("Caps Lock")
    }

    fn keyboard_key_channel_down(&self) -> String {
        String::from("Channel Down")
    }

    fn keyboard_key_channel_up(&self) -> String {
        String::from("Channel Up")
    }

    fn keyboard_key_control(&self) -> String {
        String::from("Ctrl")
    }

    fn keyboard_key_delete(&self) -> String {
        String::from("Del")
    }

    fn keyboard_key_eject(&self) -> String {
        String::from("Eject")
    }

    fn keyboard_key_end(&self) -> String {
        String::from("End")
    }

    fn keyboard_key_escape(&self) -> String {
        String::from("Esc")
    }

    fn keyboard_key_fn(&self) -> String {
        String::from("Fn")
    }

    fn keyboard_key_home(&self) -> String {
        String::from("Home")
    }

    fn keyboard_key_insert(&self) -> String {
        String::from("Insert")
    }

    fn keyboard_key_meta(&self) -> String {
        String::from("Meta")
    }

    fn keyboard_key_meta_mac_os(&self) -> String {
        String::from("Command")
    }

    fn keyboard_key_meta_windows(&self) -> String {
        String::from("Win")
    }

    fn keyboard_key_num_lock(&self) -> String {
        String::from("Num Lock")
    }

    fn keyboard_key_numpad1(&self) -> String {
        String::from("Num 1")
    }

    fn keyboard_key_numpad2(&self) -> String {
        String::from("Num 2")
    }

    fn keyboard_key_numpad3(&self) -> String {
        String::from("Num 3")
    }

    fn keyboard_key_numpad4(&self) -> String {
        String::from("Num 4")
    }

    fn keyboard_key_numpad5(&self) -> String {
        String::from("Num 5")
    }

    fn keyboard_key_numpad6(&self) -> String {
        String::from("Num 6")
    }

    fn keyboard_key_numpad7(&self) -> String {
        String::from("Num 7")
    }

    fn keyboard_key_numpad8(&self) -> String {
        String::from("Num 8")
    }

    fn keyboard_key_numpad9(&self) -> String {
        String::from("Num 9")
    }

    fn keyboard_key_numpad0(&self) -> String {
        String::from("Num 0")
    }

    fn keyboard_key_numpad_add(&self) -> String {
        String::from("Num +")
    }

    fn keyboard_key_numpad_comma(&self) -> String {
        String::from("Num ,")
    }

    fn keyboard_key_numpad_decimal(&self) -> String {
        String::from("Num .")
    }

    fn keyboard_key_numpad_divide(&self) -> String {
        String::from("Num /")
    }

    fn keyboard_key_numpad_enter(&self) -> String {
        String::from("Num Enter")
    }

    fn keyboard_key_numpad_equal(&self) -> String {
        String::from("Num =")
    }

    fn keyboard_key_numpad_multiply(&self) -> String {
        String::from("Num *")
    }

    fn keyboard_key_numpad_paren_left(&self) -> String {
        String::from("Num (")
    }

    fn keyboard_key_numpad_paren_right(&self) -> String {
        String::from("Num )")
    }

    fn keyboard_key_numpad_subtract(&self) -> String {
        String::from("Num -")
    }

    fn keyboard_key_page_down(&self) -> String {
        String::from("PgDown")
    }

    fn keyboard_key_page_up(&self) -> String {
        String::from("PgUp")
    }

    fn keyboard_key_power(&self) -> String {
        String::from("Power")
    }

    fn keyboard_key_power_off(&self) -> String {
        String::from("Power Off")
    }

    fn keyboard_key_print_screen(&self) -> String {
        String::from("Print Screen")
    }

    fn keyboard_key_scroll_lock(&self) -> String {
        String::from("Scroll Lock")
    }

    fn keyboard_key_select(&self) -> String {
        String::from("Select")
    }

    fn keyboard_key_shift(&self) -> String {
        String::from("Shift")
    }

    fn keyboard_key_space(&self) -> String {
        String::from("Space")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;

    use waft_widgets::{Builder, DefaultWidgetsLocalizations, IntoWidget, SizedBox, WidgetRef};

    use super::*;
    use crate::test_support::{build, test_cell};

    /// `child` under the US English widgets and Material localizations, as `MaterialApp`
    /// provides them.
    pub(crate) fn us_english<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Localizations::new(
            Locale::new("en").country_code("US"),
            vec![
                DefaultWidgetsLocalizations::delegate(),
                DefaultMaterialLocalizations::delegate(),
            ],
        )
        .child(child)
        .into_widget()
    }

    // localizations_test.dart 'English translations exist for all MaterialLocalizations
    // properties', with the strings themselves.
    #[test]
    #[allow(deprecated)]
    fn the_default_strings_are_the_us_english_ones() {
        let strings: &dyn MaterialLocalizations = &DefaultMaterialLocalizations::new();

        assert_eq!(strings.script_category(), ScriptCategory::EnglishLike);
        assert_eq!(strings.date_separator(), "/");
        assert_eq!(strings.date_help_text(), "mm/dd/yyyy");
        assert_eq!(strings.select_year_semantics_label(), "Select year");
        assert_eq!(strings.unspecified_date(), "Date");
        assert_eq!(strings.unspecified_date_range(), "Date Range");
        assert_eq!(strings.date_input_label(), "Enter Date");
        assert_eq!(strings.date_range_start_label(), "Start Date");
        assert_eq!(strings.date_range_end_label(), "End Date");
        assert_eq!(strings.invalid_date_format_label(), "Invalid format.");
        assert_eq!(strings.invalid_date_range_label(), "Invalid range.");
        assert_eq!(strings.date_out_of_range_label(), "Out of range.");
        assert_eq!(strings.save_button_label(), "Save");
        assert_eq!(strings.date_picker_help_text(), "Select date");
        assert_eq!(strings.date_range_picker_help_text(), "Select range");
        assert_eq!(strings.calendar_mode_button_label(), "Switch to calendar");
        assert_eq!(strings.input_date_mode_button_label(), "Switch to input");
        assert_eq!(strings.time_picker_dial_help_text(), "Select time");
        assert_eq!(strings.time_picker_input_help_text(), "Enter time");
        assert_eq!(strings.time_picker_hour_label(), "Hour");
        assert_eq!(strings.time_picker_minute_label(), "Minute");
        assert_eq!(strings.invalid_time_label(), "Enter a valid time");
        assert_eq!(
            strings.dial_mode_button_label(),
            "Switch to dial picker mode"
        );
        assert_eq!(
            strings.input_time_mode_button_label(),
            "Switch to text input mode"
        );
        assert_eq!(strings.open_app_drawer_tooltip(), "Open navigation menu");
        assert_eq!(strings.back_button_tooltip(), "Back");
        assert_eq!(strings.clear_button_tooltip(), "Clear text");
        assert_eq!(strings.close_button_tooltip(), "Close");
        assert_eq!(strings.delete_button_tooltip(), "Delete");
        assert_eq!(strings.more_button_tooltip(), "More");
        assert_eq!(strings.next_month_tooltip(), "Next month");
        assert_eq!(strings.previous_month_tooltip(), "Previous month");
        assert_eq!(strings.next_page_tooltip(), "Next page");
        assert_eq!(strings.previous_page_tooltip(), "Previous page");
        assert_eq!(strings.first_page_tooltip(), "First page");
        assert_eq!(strings.last_page_tooltip(), "Last page");
        assert_eq!(strings.show_menu_tooltip(), "Show menu");
        assert_eq!(strings.drawer_label(), "Navigation menu");
        assert_eq!(strings.menu_bar_menu_label(), "Menu bar menu");
        assert_eq!(strings.popup_menu_label(), "Popup menu");
        assert_eq!(strings.dialog_label(), "Dialog");
        assert_eq!(strings.alert_dialog_label(), "Alert");
        assert_eq!(strings.search_field_label(), "Search");
        assert_eq!(strings.current_date_label(), "Today");
        assert_eq!(strings.selected_date_label(), "Selected");
        assert_eq!(strings.scrim_label(), "Scrim");
        assert_eq!(strings.bottom_sheet_label(), "Bottom Sheet");
        assert_eq!(strings.licenses_page_title(), "Licenses");
        assert_eq!(strings.rows_per_page_title(), "Rows per page:");
        assert_eq!(strings.cancel_button_label(), "Cancel");
        assert_eq!(strings.close_button_label(), "Close");
        assert_eq!(strings.continue_button_label(), "Continue");
        assert_eq!(strings.copy_button_label(), "Copy");
        assert_eq!(strings.cut_button_label(), "Cut");
        assert_eq!(strings.scan_text_button_label(), "Scan text");
        assert_eq!(strings.ok_button_label(), "OK");
        assert_eq!(strings.paste_button_label(), "Paste");
        assert_eq!(strings.select_all_button_label(), "Select all");
        assert_eq!(strings.look_up_button_label(), "Look Up");
        assert_eq!(strings.search_web_button_label(), "Search Web");
        assert_eq!(strings.share_button_label(), "Share");
        assert_eq!(strings.view_licenses_button_label(), "View licenses");
        assert_eq!(strings.ante_meridiem_abbreviation(), "AM");
        assert_eq!(strings.post_meridiem_abbreviation(), "PM");
        assert_eq!(strings.time_picker_hour_mode_announcement(), "Select hours");
        assert_eq!(
            strings.time_picker_minute_mode_announcement(),
            "Select minutes"
        );
        assert_eq!(strings.modal_barrier_dismiss_label(), "Dismiss");
        assert_eq!(strings.menu_dismiss_label(), "Dismiss menu");
        assert_eq!(strings.signed_in_label(), "Signed in");
        assert_eq!(strings.hide_accounts_label(), "Hide accounts");
        assert_eq!(strings.show_accounts_label(), "Show accounts");
        assert_eq!(strings.reorder_item_up(), "Move up");
        assert_eq!(strings.reorder_item_down(), "Move down");
        assert_eq!(strings.reorder_item_left(), "Move left");
        assert_eq!(strings.reorder_item_right(), "Move right");
        assert_eq!(strings.reorder_item_to_end(), "Move to the end");
        assert_eq!(strings.reorder_item_to_start(), "Move to the start");
        assert_eq!(strings.expanded_icon_tap_hint(), "Collapse");
        assert_eq!(strings.collapsed_icon_tap_hint(), "Expand");
        assert_eq!(
            strings.expansion_tile_expanded_hint(),
            "double tap to collapse"
        );
        assert_eq!(
            strings.expansion_tile_collapsed_hint(),
            "double tap to expand"
        );
        assert_eq!(strings.expansion_tile_expanded_tap_hint(), "Collapse");
        assert_eq!(
            strings.expansion_tile_collapsed_tap_hint(),
            "Expand for more details"
        );
        assert_eq!(strings.expanded_hint(), "Collapsed");
        assert_eq!(strings.collapsed_hint(), "Expanded");
        assert_eq!(strings.refresh_indicator_semantic_label(), "Refresh");
        assert_eq!(strings.keyboard_key_alt(), "Alt");
        assert_eq!(strings.keyboard_key_alt_graph(), "AltGr");
        assert_eq!(strings.keyboard_key_backspace(), "Backspace");
        assert_eq!(strings.keyboard_key_caps_lock(), "Caps Lock");
        assert_eq!(strings.keyboard_key_channel_down(), "Channel Down");
        assert_eq!(strings.keyboard_key_channel_up(), "Channel Up");
        assert_eq!(strings.keyboard_key_control(), "Ctrl");
        assert_eq!(strings.keyboard_key_delete(), "Del");
        assert_eq!(strings.keyboard_key_eject(), "Eject");
        assert_eq!(strings.keyboard_key_end(), "End");
        assert_eq!(strings.keyboard_key_escape(), "Esc");
        assert_eq!(strings.keyboard_key_fn(), "Fn");
        assert_eq!(strings.keyboard_key_home(), "Home");
        assert_eq!(strings.keyboard_key_insert(), "Insert");
        assert_eq!(strings.keyboard_key_meta(), "Meta");
        assert_eq!(strings.keyboard_key_meta_mac_os(), "Command");
        assert_eq!(strings.keyboard_key_meta_windows(), "Win");
        assert_eq!(strings.keyboard_key_num_lock(), "Num Lock");
        assert_eq!(strings.keyboard_key_numpad1(), "Num 1");
        assert_eq!(strings.keyboard_key_numpad2(), "Num 2");
        assert_eq!(strings.keyboard_key_numpad3(), "Num 3");
        assert_eq!(strings.keyboard_key_numpad4(), "Num 4");
        assert_eq!(strings.keyboard_key_numpad5(), "Num 5");
        assert_eq!(strings.keyboard_key_numpad6(), "Num 6");
        assert_eq!(strings.keyboard_key_numpad7(), "Num 7");
        assert_eq!(strings.keyboard_key_numpad8(), "Num 8");
        assert_eq!(strings.keyboard_key_numpad9(), "Num 9");
        assert_eq!(strings.keyboard_key_numpad0(), "Num 0");
        assert_eq!(strings.keyboard_key_numpad_add(), "Num +");
        assert_eq!(strings.keyboard_key_numpad_comma(), "Num ,");
        assert_eq!(strings.keyboard_key_numpad_decimal(), "Num .");
        assert_eq!(strings.keyboard_key_numpad_divide(), "Num /");
        assert_eq!(strings.keyboard_key_numpad_enter(), "Num Enter");
        assert_eq!(strings.keyboard_key_numpad_equal(), "Num =");
        assert_eq!(strings.keyboard_key_numpad_multiply(), "Num *");
        assert_eq!(strings.keyboard_key_numpad_paren_left(), "Num (");
        assert_eq!(strings.keyboard_key_numpad_paren_right(), "Num )");
        assert_eq!(strings.keyboard_key_numpad_subtract(), "Num -");
        assert_eq!(strings.keyboard_key_page_down(), "PgDown");
        assert_eq!(strings.keyboard_key_page_up(), "PgUp");
        assert_eq!(strings.keyboard_key_power(), "Power");
        assert_eq!(strings.keyboard_key_power_off(), "Power Off");
        assert_eq!(strings.keyboard_key_print_screen(), "Print Screen");
        assert_eq!(strings.keyboard_key_scroll_lock(), "Scroll Lock");
        assert_eq!(strings.keyboard_key_select(), "Select");
        assert_eq!(strings.keyboard_key_shift(), "Shift");
        assert_eq!(strings.keyboard_key_space(), "Space");
        assert_eq!(strings.script_category(), ScriptCategory::EnglishLike);
        assert_eq!(strings.date_separator(), "/");
        assert_eq!(strings.first_day_of_week_index(), 0);
        assert_eq!(
            strings.narrow_weekdays(),
            ["S", "M", "T", "W", "T", "F", "S"]
        );
        assert_eq!(
            strings.time_of_day_format(false),
            TimeOfDayFormat::h_colon_mm_space_a
        );
        assert_eq!(
            strings.time_of_day_format(true),
            TimeOfDayFormat::HH_colon_mm
        );
    }

    // localizations_test.dart: the strings that take an argument hold it, and no Dart
    // placeholder is left in them.
    #[test]
    fn the_strings_with_arguments_fill_them_in_and_pluralize() {
        let strings = DefaultMaterialLocalizations::new();

        assert_eq!(strings.scrim_on_tap_hint("FOO"), "Close FOO");
        assert_eq!(strings.about_list_tile_title("FOO"), "About FOO");

        assert_eq!(strings.selected_row_count_title(0), "No items selected");
        assert_eq!(strings.selected_row_count_title(1), "1 item selected");
        assert_eq!(strings.selected_row_count_title(2), "2 items selected");
        assert_eq!(strings.selected_row_count_title(100), "100 items selected");

        assert_eq!(
            strings.page_rows_info_title(1, 10, 100, true),
            "1–10 of about 100"
        );
        assert_eq!(
            strings.page_rows_info_title(1, 10, 100, false),
            "1–10 of 100"
        );

        assert_eq!(strings.licenses_package_detail_text(0), "No licenses.");
        assert_eq!(strings.licenses_package_detail_text(1), "1 license.");
        assert_eq!(strings.licenses_package_detail_text(2), "2 licenses.");
        assert_eq!(strings.licenses_package_detail_text(100), "100 licenses.");

        assert_eq!(
            strings.remaining_text_field_character_count(0),
            "No characters remaining"
        );
        assert_eq!(
            strings.remaining_text_field_character_count(1),
            "1 character remaining"
        );
        assert_eq!(
            strings.remaining_text_field_character_count(5),
            "5 characters remaining"
        );

        assert_eq!(strings.tab_label(1, 2), "Tab 1 of 2");
        assert_eq!(
            strings.date_range_start_date_semantic_label("Feb 21"),
            "Start date Feb 21"
        );
        assert_eq!(
            strings.date_range_end_date_semantic_label("Feb 21"),
            "End date Feb 21"
        );
    }

    #[test]
    fn a_time_of_day_formats_in_twelve_or_twenty_four_hours() {
        let strings = DefaultMaterialLocalizations::new();
        let format = |hour, minute, always_use_24_hour_format| {
            strings.format_time_of_day(&TimeOfDay::new(hour, minute), always_use_24_hour_format)
        };

        assert_eq!(format(7, 0, false), "7:00 AM");
        assert_eq!(format(7, 0, true), "07:00");
        assert_eq!(format(0, 5, false), "12:05 AM");
        assert_eq!(format(0, 5, true), "00:05");
        assert_eq!(format(12, 0, false), "12:00 PM");
        assert_eq!(format(23, 59, false), "11:59 PM");
        assert_eq!(format(23, 59, true), "23:59");

        assert_eq!(strings.format_hour(&TimeOfDay::new(13, 0), false), "1");
        assert_eq!(strings.format_hour(&TimeOfDay::new(13, 0), true), "13");
        assert_eq!(strings.format_minute(&TimeOfDay::new(9, 5)), "05");
        assert_eq!(strings.format_minute(&TimeOfDay::new(9, 45)), "45");
    }

    // The expected values are the Dart VM's, from the Dart code of `formatDecimal`.
    #[test]
    fn format_decimal_inserts_thousands_separators() {
        let strings = DefaultMaterialLocalizations::new();

        assert_eq!(strings.format_decimal(0), "0");
        assert_eq!(strings.format_decimal(999), "999");
        assert_eq!(strings.format_decimal(-999), "-999");
        assert_eq!(strings.format_decimal(1000), "1,000");
        assert_eq!(strings.format_decimal(-1000), "-1,000");
        assert_eq!(strings.format_decimal(100000), "100,000");
        assert_eq!(strings.format_decimal(1234567), "1,234,567");
        assert_eq!(strings.format_decimal(-1234567), "-1,234,567");
        assert_eq!(
            strings.format_decimal(i64::MAX),
            "9,223,372,036,854,775,807"
        );
        assert_eq!(
            strings.format_decimal(i64::MIN),
            "--9,223,372,036,854,775,808"
        );
    }

    #[test]
    fn a_date_formats_in_us_english() {
        let strings = DefaultMaterialLocalizations::new();
        let date = DateTime::utc(2019, 2, 21);

        assert_eq!(strings.format_year(&date), "2019");
        assert_eq!(strings.format_compact_date(&date), "02/21/2019");
        assert_eq!(
            strings.format_compact_date(&DateTime::utc(5, 1, 2)),
            "01/02/0005"
        );
        assert_eq!(strings.format_short_date(&date), "Feb 21, 2019");
        assert_eq!(strings.format_medium_date(&date), "Thu, Feb 21");
        assert_eq!(
            strings.format_full_date(&date),
            "Thursday, February 21, 2019"
        );
        assert_eq!(strings.format_month_year(&date), "February 2019");
        assert_eq!(strings.format_short_month_day(&date), "Feb 21");

        // The examples of the interface's documentation.
        let example = DateTime::utc(2017, 9, 27);
        assert_eq!(strings.format_medium_date(&example), "Wed, Sep 27");
        assert_eq!(
            strings.format_full_date(&example),
            "Wednesday, September 27, 2017"
        );
    }

    // localizations_test.dart "parseCompactDate doesn't throw an exception on invalid text",
    // and the Dart VM's answers for the other strings.
    #[test]
    fn parse_compact_date_reads_a_us_date_and_rejects_the_rest() {
        let strings = DefaultMaterialLocalizations::new();
        let parse = |text| strings.parse_compact_date(Some(text));

        assert_eq!(parse("10/05/2023"), Some(DateTime::utc(2023, 10, 5)));
        assert_eq!(parse("10/05/2023666777889"), None);

        assert_eq!(strings.parse_compact_date(None), None);
        assert_eq!(parse("02/21/2019"), Some(DateTime::utc(2019, 2, 21)));
        assert_eq!(parse(" 02/ 21/+2019"), Some(DateTime::utc(2019, 2, 21)));
        assert_eq!(parse("1/1/1"), Some(DateTime::utc(1, 1, 1)));
        assert_eq!(parse("02/21"), None);
        assert_eq!(parse("a/b/c"), None);
        assert_eq!(parse("0x1/01/2019"), None);
        assert_eq!(parse("13/01/2019"), None);
        assert_eq!(parse("01/01/0"), None);
        assert_eq!(parse("02/29/2019"), None);
        assert_eq!(parse("02/29/2020"), Some(DateTime::utc(2020, 2, 29)));
        assert_eq!(parse("02/29/1900"), None);
        assert_eq!(parse("02/29/2000"), Some(DateTime::utc(2000, 2, 29)));
        assert_eq!(parse("09/13/275760"), Some(DateTime::utc(275760, 9, 13)));
        assert_eq!(parse("09/14/275760"), None);
    }

    #[test]
    fn the_delegate_supports_english_only_and_prints_as_dart_does() {
        let delegate = DefaultMaterialLocalizations::delegate();
        assert!(delegate.is_supported(&Locale::new("en").country_code("GB")));
        assert!(!delegate.is_supported(&Locale::new("fr")));
        assert_eq!(
            format!("{delegate:?}"),
            "DefaultMaterialLocalizations.delegate(en_US)"
        );
    }

    #[test]
    fn load_completes_synchronously() {
        let strings = DefaultMaterialLocalizations::load(&Locale::new("en"));
        assert!(strings.is_completed(), "Dart's SynchronousFuture");
        assert_eq!(
            strings.peek().expect("complete").back_button_tooltip(),
            "Back"
        );
    }

    // localizations_test.dart 'MaterialLocalizations.of throws', the context with
    // localizations.
    #[test]
    fn of_finds_the_strings_through_a_localizations_ancestor() {
        let cell = test_cell();
        let seen = Rc::new(RefCell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() =
                    Some(<dyn MaterialLocalizations>::of(app, context).back_button_tooltip());
                SizedBox::shrink().into_widget()
            }
        });
        build(&cell, us_english(probe));
        assert_eq!(seen.borrow().as_deref(), Some("Back"));
    }

    // localizations_test.dart 'MaterialLocalizations.of throws', the context without
    // localizations.
    #[test]
    #[should_panic(expected = "No MaterialLocalizations found")]
    fn of_panics_without_a_localizations_ancestor() {
        let cell = test_cell();
        let probe = Builder::new(|app, context| {
            <dyn MaterialLocalizations>::of(app, context);
            SizedBox::shrink().into_widget()
        });
        build(&cell, probe.into_widget());
    }
}
