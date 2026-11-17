//! Date and time pickers: a button for each way a dialog opens — the calendar and the text
//! entry of `showDatePicker`, `showDateRangePicker`, and the dial and the text entry of
//! `showTimePicker` — and a caption with what was chosen.
//!
//! A dialog answers through the route's result, so each button hands its dialog's future a
//! callback that stores the answer in the page's state; a dismissed dialog answers nothing and
//! leaves the caption alone.

use std::rc::Rc;

use waft_embedder::TextInputType;
use waft_foundation::{App, DateTime, Handle, Listener};
use waft_material::{
    DatePickerEntryMode, DatePickerMode, DateTimeRange, FilledButton, GregorianCalendarDelegate,
    OutlinedButton, ShowDatePicker, ShowDateRangePicker, TimeOfDay, TimePickerEntryMode,
    show_time_picker,
};
use waft_widgets::{
    BuildContext, Builder, IntoWidget, ListView, RouteResult, State, StateData, StatefulWidget,
    Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

const NOTHING_CHOSEN: &str = "Nothing chosen yet";

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    PickersDemo.into_widget()
}

/// What the dialogs last answered lives here, in the smallest widget that renders it.
#[derive(Debug)]
pub struct PickersDemo;

/// Dart would spell this `_PickersDemoState`.
pub struct PickersDemoState {
    state: StateData<PickersDemo>,
    date: String,
    range: String,
    time: String,
}

impl StatefulWidget for PickersDemo {
    type State = PickersDemoState;

    fn create_state(&self) -> PickersDemoState {
        PickersDemoState {
            state: StateData::new(),
            date: NOTHING_CHOSEN.to_owned(),
            range: NOTHING_CHOSEN.to_owned(),
            time: NOTHING_CHOSEN.to_owned(),
        }
    }
}

impl State for PickersDemoState {
    type Widget = PickersDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (date, range, time) = {
            let state = app.get(self);
            (state.date.clone(), state.range.clone(), state.time.clone())
        };
        let content = ListView::new().children(vec![
            section_header(app, context, "Date picker"),
            caption(app, context, format!("Date: {date}")),
            showcase(vec![
                date_button(self, "Calendar", DatePickerEntryMode::Calendar),
                date_button(self, "Text entry", DatePickerEntryMode::Input),
                date_button(self, "Calendar only", DatePickerEntryMode::CalendarOnly),
            ]),
            section_header(app, context, "Date range picker"),
            caption(app, context, format!("Range: {range}")),
            showcase(vec![
                range_button(self, "Calendar", DatePickerEntryMode::Calendar),
                range_button(self, "Text entry", DatePickerEntryMode::Input),
            ]),
            section_header(app, context, "Time picker"),
            caption(app, context, format!("Time: {time}")),
            showcase(vec![
                time_button(self, "Dial", TimePickerEntryMode::Dial),
                time_button(self, "Text entry", TimePickerEntryMode::Input),
                time_button(self, "Dial only", TimePickerEntryMode::DialOnly),
            ]),
        ]);
        page(Entry::Pickers.title(), content).into_widget()
    }
}

/// The answer a dialog's future gave, if it was not dismissed.
fn answer<T: Clone + 'static>(result: &RouteResult) -> Option<T> {
    result.as_ref()?.downcast_ref::<T>().cloned()
}

fn date_button(
    demo: Handle<PickersDemoState>,
    label: &'static str,
    entry_mode: DatePickerEntryMode,
) -> WidgetRef {
    Builder::new(move |_app: &mut App, context: BuildContext| {
        FilledButton::tonal(
            Some(Listener::new(move |app: &mut App| {
                let future =
                    ShowDatePicker::new(DateTime::utc(2000, 1, 1), DateTime::utc(2100, 12, 31))
                        .initial_date(DateTime::now())
                        .initial_entry_mode(entry_mode)
                        .calendar_delegate(Rc::new(GregorianCalendarDelegate))
                        .show(app, context);
                future.then(app, move |app, result| {
                    if let Some(date) = answer::<DateTime>(&result) {
                        let text = format!(
                            "{}-{:02}-{:02}",
                            date.year(app),
                            date.month(app),
                            date.day(app)
                        );
                        demo.set_state(app, |state| state.date = text);
                    }
                });
            })),
            Some(Text::new(label)),
        )
        .into_widget()
    })
    .into_widget()
}

fn range_button(
    demo: Handle<PickersDemoState>,
    label: &'static str,
    entry_mode: DatePickerEntryMode,
) -> WidgetRef {
    Builder::new(move |_app: &mut App, context: BuildContext| {
        OutlinedButton::new(
            Some(Listener::new(move |app: &mut App| {
                let future = ShowDateRangePicker::new(
                    DateTime::utc(2000, 1, 1),
                    DateTime::utc(2100, 12, 31),
                )
                .initial_entry_mode(entry_mode)
                .calendar_delegate(Rc::new(GregorianCalendarDelegate))
                .show(app, context);
                future.then(app, move |app, result| {
                    if let Some(range) = answer::<DateTimeRange>(&result) {
                        let text = format!(
                            "{}-{:02}-{:02} to {}-{:02}-{:02}",
                            range.start.year(app),
                            range.start.month(app),
                            range.start.day(app),
                            range.end.year(app),
                            range.end.month(app),
                            range.end.day(app)
                        );
                        demo.set_state(app, |state| state.range = text);
                    }
                });
            })),
            Some(Text::new(label)),
        )
        .into_widget()
    })
    .into_widget()
}

fn time_button(
    demo: Handle<PickersDemoState>,
    label: &'static str,
    entry_mode: TimePickerEntryMode,
) -> WidgetRef {
    Builder::new(move |_app: &mut App, context: BuildContext| {
        FilledButton::new(
            Some(Listener::new(move |app: &mut App| {
                let future = show_time_picker(
                    app,
                    context,
                    TimeOfDay::new(9, 30),
                    None,
                    true,
                    None,
                    None,
                    true,
                    entry_mode,
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
                );
                future.then(app, move |app, result| {
                    if let Some(time) = answer::<TimeOfDay>(&result) {
                        let text = format!("{:02}:{:02}", time.hour, time.minute);
                        demo.set_state(app, |state| state.time = text);
                    }
                });
            })),
            Some(Text::new(label)),
        )
        .into_widget()
    })
    .into_widget()
}
