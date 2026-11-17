//! Pull to refresh: a scrollable that runs a refresh action and settles back.

use std::rc::Rc;

use waft_foundation::{App, Handle, Task};
use waft_material::{Material, RefreshIndicator};
use waft_widgets::{
    AlwaysScrollableScrollPhysics, BuildContext, IntoWidget, ListView, SizedBox, State, StateData,
    StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Refresh.title(), RefreshDemo).into_widget()
}

/// Counts the refreshes.
#[derive(Debug)]
pub struct RefreshDemo;

pub struct RefreshDemoState {
    state: StateData<RefreshDemo>,
    refreshes: usize,
}

impl StatefulWidget for RefreshDemo {
    type State = RefreshDemoState;

    fn create_state(&self) -> RefreshDemoState {
        RefreshDemoState {
            state: StateData::new(),
            refreshes: 0,
        }
    }
}

impl State for RefreshDemoState {
    type Widget = RefreshDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let refreshes = app.get(self).refreshes;
        let on_refresh: waft_material::RefreshCallback = Rc::new(move |app: &mut App| {
            self.set_state(app, |state| state.refreshes += 1);
            Task::ready(())
        });
        let items = (1..=6)
            .map(|index| {
                SizedBox::new()
                    .height(40.0)
                    .child(Text::new(format!("Item {index}")))
                    .into_widget()
            })
            .collect::<Vec<_>>();
        let list = ListView::new()
            .physics(Rc::new(AlwaysScrollableScrollPhysics::new()))
            .children(items);
        ListView::new()
            .children(vec![
                section_header(app, context, "Pull to refresh"),
                SizedBox::new()
                    .height(320.0)
                    .child(
                        Material::new()
                            .child(RefreshIndicator::new(on_refresh, list))
                            .into_widget(),
                    )
                    .into_widget(),
                caption(
                    app,
                    context,
                    match refreshes {
                        0 => "Drag down on the list to refresh.".to_string(),
                        count => format!("Refreshed {count} times"),
                    },
                ),
            ])
            .into_widget()
    }
}
