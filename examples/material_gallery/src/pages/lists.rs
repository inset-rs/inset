//! `ListTile`: selected tiles, one, two and three lines, leading and trailing widgets, dense
//! and disabled tiles, and the two ways to rule them apart — a `Divider` between groups and
//! `ListTile::divide_tiles` between the tiles of one.

use waft_foundation::{App, Handle, Listener};
use waft_material::{Divider, Icons, ListTile};
use waft_widgets::{
    BuildContext, Icon, IntoWidget, ListView, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

/// The tiles of the selectable group, in order.
const MAILBOXES: [&str; 3] = ["Inbox", "Starred", "Sent"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    ListsDemo.into_widget()
}

/// The selection lives here, in the smallest widget that renders it.
#[derive(Debug)]
pub struct ListsDemo;

/// Dart would spell this `_ListsDemoState`.
pub struct ListsDemoState {
    state: StateData<ListsDemo>,
    selected: usize,
}

impl StatefulWidget for ListsDemo {
    type State = ListsDemoState;

    fn create_state(&self) -> ListsDemoState {
        ListsDemoState {
            state: StateData::new(),
            selected: 0,
        }
    }
}

impl State for ListsDemoState {
    type Widget = ListsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let selected = app.get(self).selected;
        let mailboxes: Vec<WidgetRef> = MAILBOXES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                ListTile::new()
                    .leading(Icon::new(Some(mailbox_icon(index))))
                    .title(Text::new(*name))
                    .selected(index == selected)
                    .on_tap(Listener::new(move |app: &mut App| {
                        self.set_state(app, |state| state.selected = index);
                    }))
                    .into_widget()
            })
            .collect();

        let mut children = vec![
            section_header(app, context, "Selected"),
            caption(app, context, format!("Selected: {}", MAILBOXES[selected])),
        ];
        children.extend(mailboxes);
        children.push(Divider::new().into_widget());
        children.push(section_header(app, context, "Lines"));
        children.extend(lines());
        children.push(Divider::new().into_widget());
        children.push(section_header(app, context, "Dense and disabled"));
        children.extend(dense_and_disabled());
        children.push(Divider::new().into_widget());
        children.push(section_header(app, context, "Divided tiles"));
        children.extend(ListTile::divide_tiles(
            app,
            Some(context),
            ["Alpha", "Beta", "Gamma"]
                .map(|name| ListTile::new().title(Text::new(name)).into_widget()),
            None,
        ));

        page(Entry::Lists.title(), ListView::new().children(children)).into_widget()
    }
}

/// One, two and three lines, each with something leading and trailing.
fn lines() -> Vec<WidgetRef> {
    vec![
        ListTile::new().title(Text::new("Title only")).into_widget(),
        ListTile::new()
            .leading(Icon::new(Some(Icons::person())))
            .title(Text::new("Leading and trailing"))
            .trailing(Icon::new(Some(Icons::more_vert())))
            .into_widget(),
        ListTile::new()
            .leading(Icon::new(Some(Icons::mail())))
            .title(Text::new("Two lines"))
            .subtitle(Text::new("A subtitle under the title"))
            .trailing(Text::new("9:41"))
            .into_widget(),
        ListTile::new()
            .leading(Icon::new(Some(Icons::bookmark())))
            .title(Text::new("Three lines"))
            .subtitle(Text::new(
                "A subtitle long enough to need its second line, which is what \
                 is_three_line makes room for",
            ))
            .trailing(Icon::new(Some(Icons::star())))
            .is_three_line(true)
            .into_widget(),
    ]
}

fn dense_and_disabled() -> Vec<WidgetRef> {
    vec![
        ListTile::new()
            .leading(Icon::new(Some(Icons::call())))
            .title(Text::new("Dense"))
            .subtitle(Text::new("Smaller text, shorter tile"))
            .dense(true)
            .into_widget(),
        ListTile::new()
            .leading(Icon::new(Some(Icons::delete())))
            .title(Text::new("Disabled"))
            .subtitle(Text::new("Greyed, and ignores taps"))
            .enabled(false)
            .on_tap(Listener::new(|_app: &mut App| {}))
            .into_widget(),
    ]
}

fn mailbox_icon(index: usize) -> waft_widgets::IconData {
    match index {
        0 => Icons::inbox(),
        1 => Icons::star(),
        _ => Icons::send(),
    }
}
