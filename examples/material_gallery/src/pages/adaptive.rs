//! Adaptive layout: a navigation suite over an inbox whose list and detail share a page on a
//! wide window, and where each mail is its own page on a phone.
//!
//! Resize the window: the bar becomes a rail and then an extended rail, and the inbox regroups.

use std::rc::Rc;

use waft_foundation::{App, Handle, Listener};
use waft_material::{
    AppBar, FloatingActionButton, Icons, ListDetailPaneScaffoldRole, ListTile, Material,
    NavigationSuiteDestination, NavigationSuiteScaffold, Pane, PanePreferredSize,
    PaneScaffoldDirective, Scaffold, ThreePaneNavigator, ThreePaneScaffold,
    ThreePaneScaffoldDestinationItem, ThreePaneSceneStrategy, WindowSizeClass,
};
use waft_widgets::{
    BuildContext, Center, Icon, IntoWidget, ListView, PreferredSizeWidgetRef, State, StateData,
    StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::page;

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Adaptive.title(), AdaptiveDemo.into_widget()).into_widget()
}

/// The demo's state: the selected destination and the inbox's back stack.
#[derive(Debug)]
pub struct AdaptiveDemo;

/// One entry of the inbox's own back stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InboxEntry {
    /// The mail list.
    List,
    /// One opened mail.
    Mail(u32),
}

impl InboxEntry {
    /// The pane this entry fills.
    fn pane(&self) -> Option<ThreePaneScaffoldDestinationItem<u32>> {
        match *self {
            InboxEntry::List => Some(ThreePaneScaffoldDestinationItem::new(
                ListDetailPaneScaffoldRole::LIST,
                None,
            )),
            InboxEntry::Mail(mail) => Some(ThreePaneScaffoldDestinationItem::new(
                ListDetailPaneScaffoldRole::DETAIL,
                Some(mail),
            )),
        }
    }
}

/// Dart would spell this `_AdaptiveDemoState`.
pub struct AdaptiveDemoState {
    state: StateData<AdaptiveDemo>,
    destination: usize,
    /// The list, then each opened mail.
    stack: Vec<InboxEntry>,
}

impl StatefulWidget for AdaptiveDemo {
    type State = AdaptiveDemoState;

    fn create_state(&self) -> AdaptiveDemoState {
        AdaptiveDemoState {
            state: StateData::new(),
            destination: 0,
            stack: vec![InboxEntry::List],
        }
    }
}

impl State for AdaptiveDemoState {
    type Widget = AdaptiveDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let destination = app.get(self).destination;
        let content = match destination {
            0 => inbox(self, app, context),
            1 => Center::new().child(Text::new("Starred")).into_widget(),
            _ => Center::new().child(Text::new("Settings")).into_widget(),
        };
        NavigationSuiteScaffold::new(
            vec![
                NavigationSuiteDestination::new(Icon::new(Some(Icons::inbox())), "Inbox"),
                NavigationSuiteDestination::new(Icon::new(Some(Icons::star_outline())), "Starred")
                    .selected_icon(Icon::new(Some(Icons::star()))),
                NavigationSuiteDestination::new(Icon::new(Some(Icons::settings())), "Settings"),
            ],
            content,
        )
        .selected_index(destination)
        .on_destination_selected(Rc::new(move |app, index| {
            self.set_state(app, |state| state.destination = index);
        }))
        .primary_action(FloatingActionButton::new(None).child(Icon::new(Some(Icons::edit()))))
        .into_widget()
    }
}

fn inbox(self_: Handle<AdaptiveDemoState>, app: &mut App, context: BuildContext) -> WidgetRef {
    let strategy = ThreePaneSceneStrategy::list_detail(
        PaneScaffoldDirective::from_window_size_class(WindowSizeClass::of(app, context)),
    );
    let stack = app.get(self_).stack.clone();
    ThreePaneNavigator::new(
        strategy,
        stack,
        InboxEntry::pane,
        move |app, context, index| match app.get(self_).stack[index] {
            InboxEntry::Mail(mail) => mail_detail(mail).into_widget(),
            InboxEntry::List => mail_list(self_, app, context, None),
        },
        move |app, context, scene| {
            let mail = scene
                .entry_for(ListDetailPaneScaffoldRole::DETAIL)
                .and_then(|index| open_mail(app, self_, index));
            ThreePaneScaffold::list_detail(
                scene.directive.clone(),
                scene.value,
                Pane::new(mail_list(self_, app, context, mail))
                    .preferred_width(PanePreferredSize::Fixed(300.0)),
                Pane::new(mail_detail_of(mail)),
            )
            .into_widget()
        },
        move |app, len| self_.set_state(app, |state| state.stack.truncate(len)),
    )
    .into_widget()
}

/// The mail `stack[index]` shows: `None` for the list.
fn open_mail(app: &App, self_: Handle<AdaptiveDemoState>, index: usize) -> Option<u32> {
    match app.get(self_).stack[index] {
        InboxEntry::Mail(mail) => Some(mail),
        InboxEntry::List => None,
    }
}

fn mail_list(
    self_: Handle<AdaptiveDemoState>,
    _app: &mut App,
    _context: BuildContext,
    selected: Option<u32>,
) -> WidgetRef {
    Material::new()
        .child(ListView::new().children((1..=8).map(|mail| {
            ListTile::new()
                .title(Text::new(format!("Mail {mail}")))
                .selected(selected == Some(mail))
                .on_tap(Listener::new(move |app| {
                    self_.set_state(app, |state| state.stack.push(InboxEntry::Mail(mail)));
                }))
                .into_widget()
        })))
        .into_widget()
}

fn mail_detail(mail: u32) -> Scaffold {
    Scaffold::new()
        .app_bar(PreferredSizeWidgetRef::new(
            AppBar::new().title(Text::new(format!("Mail {mail}"))),
        ))
        .body(
            Center::new()
                .child(Text::new(format!("Message {mail}")))
                .into_widget(),
        )
}

fn mail_detail_of(mail: Option<u32>) -> WidgetRef {
    match mail {
        Some(mail) => mail_detail(mail).into_widget(),
        None => Center::new()
            .child(Text::new("Select a mail"))
            .into_widget(),
    }
}
