//! Material 3's segmented buttons, single and multiple selection with icons and labels, one
//! disabled segment, and the older toggle buttons.
//!
//! Each demo keeps its own selection and a caption states it.

use waft_foundation::{App, Handle};
use waft_material::{ButtonSegment, Icons, SegmentedButton, ToggleButtons};
use waft_widgets::{
    BuildContext, Icon, IntoWidget, ListView, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

const VIEWS: [&str; 3] = ["Day", "Week", "Month"];
const TOGGLES: [&str; 3] = ["Bold", "Italic", "Underline"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Segments.title(), SegmentsDemo.into_widget()).into_widget()
}

/// The selections live here, in the smallest widget that renders them.
#[derive(Debug)]
pub struct SegmentsDemo;

pub struct SegmentsDemoState {
    state: StateData<SegmentsDemo>,
    view: usize,
    days: Vec<usize>,
    toggles: [bool; TOGGLES.len()],
}

impl StatefulWidget for SegmentsDemo {
    type State = SegmentsDemoState;

    fn create_state(&self) -> SegmentsDemoState {
        SegmentsDemoState {
            state: StateData::new(),
            view: 1,
            days: vec![0, 2],
            toggles: [true, false, false],
        }
    }
}

impl State for SegmentsDemoState {
    type Widget = SegmentsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let children = vec![
            section_header(app, context, "Single selection"),
            showcase(vec![self.single_selection(app)]),
            caption(
                app,
                context,
                format!("Selected: {}", VIEWS[app.get(self).view]),
            ),
            section_header(app, context, "Multiple selection"),
            showcase(vec![self.multiple_selection(app)]),
            caption(
                app,
                context,
                format!("Selected: {} of 3", app.get(self).days.len()),
            ),
            section_header(app, context, "Disabled"),
            showcase(vec![Self::disabled_segment()]),
            section_header(app, context, "Toggle buttons"),
            showcase(vec![self.toggle_buttons(app)]),
            caption(
                app,
                context,
                format!(
                    "On: {}",
                    TOGGLES
                        .iter()
                        .zip(app.get(self).toggles)
                        .filter(|(_, on)| *on)
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
        ];
        ListView::new().children(children).into_widget()
    }
}

impl SegmentsDemoState {
    fn single_selection(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let segments = VIEWS
            .iter()
            .enumerate()
            .map(|(index, name)| ButtonSegment::new(index).label(Text::new(*name)))
            .collect();
        SegmentedButton::new(segments, vec![app.get(self).view])
            .on_selection_changed(move |app: &mut App, selection: Vec<usize>| {
                if let Some(view) = selection.first().copied() {
                    self.set_state(app, |state| state.view = view);
                }
            })
            .into_widget()
    }

    fn multiple_selection(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let icons = [Icons::today(), Icons::view_week(), Icons::calendar_month()];
        let segments = VIEWS
            .iter()
            .zip(icons)
            .enumerate()
            .map(|(index, (name, icon))| {
                ButtonSegment::new(index)
                    .icon(Icon::new(Some(icon)))
                    .label(Text::new(*name))
            })
            .collect();
        SegmentedButton::new(segments, app.get(self).days.clone())
            .multi_selection_enabled(true)
            .empty_selection_allowed(true)
            .on_selection_changed(move |app: &mut App, selection: Vec<usize>| {
                self.set_state(app, |state| state.days = selection);
            })
            .into_widget()
    }

    fn disabled_segment() -> WidgetRef {
        SegmentedButton::new(
            vec![
                ButtonSegment::new(0usize).label(Text::new("Enabled")),
                ButtonSegment::new(1usize)
                    .label(Text::new("Disabled"))
                    .enabled(false),
            ],
            vec![0],
        )
        .on_selection_changed(|_app: &mut App, _selection: Vec<usize>| {})
        .into_widget()
    }

    fn toggle_buttons(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let children = TOGGLES
            .iter()
            .map(|name| Text::new(*name).into_widget())
            .collect();
        ToggleButtons::new(children, app.get(self).toggles.to_vec())
            .on_pressed(move |app: &mut App, index: usize| {
                self.set_state(app, |state| state.toggles[index] = !state.toggles[index]);
            })
            .into_widget()
    }
}
