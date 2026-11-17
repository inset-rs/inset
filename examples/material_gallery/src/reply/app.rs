//! `ReplyApp.kt`: the theme, the inbox's state, and the top-level destination shown.

use std::rc::Rc;

use waft_foundation::{App, Entity, Handle, Listener, ValueChanged};
use waft_material::{BackButton, Theme};
use waft_widgets::{
    AnyModalRoute, BuildContext, Builder, IntoWidget, Navigator, State, StateData, StatefulWidget,
    WidgetRef,
};

use crate::reply::empty_coming_soon::empty_coming_soon;
use crate::reply::inbox::reply_inbox_screen;
use crate::reply::navigation::{Route, reply_navigation_wrapper};
use crate::reply::state::ReplyHome;
use crate::reply::theme::reply_theme;

/// The whole Reply app, in the brightness of the theme around it.
///
/// Pushed over another page, it shows a back button where the search bar's icon is, since the
/// sample, a whole app, has no way out of itself.
#[derive(Debug, Default)]
pub struct ReplyApp;

impl StatefulWidget for ReplyApp {
    type State = ReplyAppState;

    fn create_state(&self) -> ReplyAppState {
        ReplyAppState {
            state: StateData::new(),
            home: None,
            route: Route::Inbox,
        }
    }
}

/// Dart would spell this `_ReplyAppState`.
pub struct ReplyAppState {
    state: StateData<ReplyApp>,
    home: Option<Entity<ReplyHome>>,
    route: Route,
}

impl State for ReplyAppState {
    type Widget = ReplyApp;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let home = ReplyHome::entity(app);
        app.get_mut(self).home = Some(home);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let brightness = Theme::of(app, context).brightness();
        let home = app.get(self).home.clone().expect("created in init_state");
        let route = app.get(self).route;
        let leave = way_back(app, context);
        let navigate: ValueChanged<Route> = Rc::new(move |app: &mut App, route: Route| {
            self.set_state(app, |state| state.route = route);
        });
        let screens = Builder::new(move |app: &mut App, context: BuildContext| {
            let content = match route {
                Route::Inbox => reply_inbox_screen(app, context, home.clone(), leave.clone()),
                _ => empty_coming_soon(app, context),
            };
            reply_navigation_wrapper(app, context, route, navigate.clone(), content)
        });
        Theme::new(reply_theme(app, brightness), screens).into_widget()
    }
}

/// A back button that pops the route Reply is on, when that route can be popped.
fn way_back(app: &mut App, context: BuildContext) -> Option<WidgetRef> {
    if AnyModalRoute::can_pop_of(app, context) != Some(true) {
        return None;
    }
    let pop = Listener::new(move |app: &mut App| {
        Navigator::maybe_pop(app, context, None);
    });
    Some(BackButton::new().on_pressed(pop).into_widget())
}
