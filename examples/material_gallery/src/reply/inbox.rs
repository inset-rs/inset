//! `ReplyListContent.kt`: the inbox, a list of mail beside the open one on a wide window and a
//! page per mail on a phone.
//!
//! Where the sample switches between its own one- and two-pane layouts, this is one
//! [`ThreePaneNavigator`] over the inbox's back stack: the pane scaffold decides how many panes
//! the window has room for, and a phone's open mail is a page with the platform's back.

use waft_foundation::{App, Entity, Listener};
use waft_material::{
    ListDetailPaneScaffoldRole, Pane, PanePreferredSize, PaneScaffoldDirective, Theme,
    ThreePaneNavigator, ThreePaneScaffold, ThreePaneScaffoldDestinationItem,
    ThreePaneSceneStrategy, WindowSizeClass,
};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{
    BuildContext, ColoredBox, Column, Expanded, IntoWidget, ListView, Navigator, Padding, WidgetRef,
};

use crate::reply::components::app_bars::{email_detail_app_bar, reply_search_bar};
use crate::reply::components::email_list_item::email_list_item;
use crate::reply::components::email_thread_item::email_thread_item;
use crate::reply::state::{
    InboxEntry, ReplyHome, close_detail_screen, open_email, toggle_selected_email,
};
use crate::reply::theme::{rounded, shapes};

/// The inbox. `leave` is the gallery's way back, shown in the search bar.
pub fn reply_inbox_screen(
    app: &mut App,
    context: BuildContext,
    home: Entity<ReplyHome>,
    leave: Option<WidgetRef>,
) -> WidgetRef {
    let directive =
        PaneScaffoldDirective::from_window_size_class(WindowSizeClass::of(app, context));
    let stack = app.read(&home).back_stack();
    let entry_home = home.clone();
    let entry_leave = leave.clone();
    let scene_home = home.clone();
    ThreePaneNavigator::new(
        ThreePaneSceneStrategy::list_detail(directive),
        stack,
        pane_of,
        move |app, context, index| match app.read(&entry_home).back_stack()[index] {
            InboxEntry::List => {
                reply_email_list(app, context, entry_home.clone(), entry_leave.clone())
            }
            InboxEntry::Detail(_) => {
                let back = Listener::new(move |app: &mut App| {
                    Navigator::maybe_pop(app, context, None);
                });
                reply_email_detail(app, context, &entry_home, true, back)
            }
        },
        move |app, context, scene| {
            let list = reply_email_list(app, context, scene_home.clone(), leave.clone());
            let detail = reply_email_detail(
                app,
                context,
                &scene_home,
                false,
                Listener::new(|_app: &mut App| {}),
            );
            ThreePaneScaffold::list_detail(
                scene.directive.clone(),
                scene.value,
                Pane::new(list).preferred_width(PanePreferredSize::Proportion(0.5)),
                detail_pane(app, context, detail),
            )
            .into_widget()
        },
        move |app, len| {
            if len <= 1 {
                close_detail_screen(app, &home);
            }
        },
    )
    .into_widget()
}

/// The pane `entry` fills.
fn pane_of(entry: &InboxEntry) -> Option<ThreePaneScaffoldDestinationItem<u64>> {
    Some(match *entry {
        InboxEntry::List => {
            ThreePaneScaffoldDestinationItem::new(ListDetailPaneScaffoldRole::LIST, None)
        }
        InboxEntry::Detail(id) => {
            ThreePaneScaffoldDestinationItem::new(ListDetailPaneScaffoldRole::DETAIL, Some(id))
        }
    })
}

/// The open mail beside the list: a rounded surface inside the window's margin, so the thread
/// reads as a sheet laid beside the inbox.
fn detail_pane(app: &mut App, context: BuildContext, detail: WidgetRef) -> Pane {
    let margin = WindowSizeClass::of(app, context).width.margin();
    Pane::new(detail)
        .margins(EdgeInsetsGeometry::all(margin))
        .shape(rounded(shapes::LARGE))
}

/// `ReplyEmailList`: the search bar over the mail cards.
fn reply_email_list(
    app: &mut App,
    context: BuildContext,
    home: Entity<ReplyHome>,
    leave: Option<WidgetRef>,
) -> WidgetRef {
    let state = app.read(&home);
    let emails = state.emails.clone();
    let opened = state.opened_email;
    let selected = state.selected_emails.clone();
    let items: Vec<WidgetRef> = emails
        .iter()
        .map(|email| {
            let id = email.id;
            let open_home = home.clone();
            let toggle_home = home.clone();
            email_list_item(
                app,
                context,
                email,
                opened == Some(id),
                selected.contains(&id),
                Listener::new(move |app: &mut App| open_email(app, &open_home, id)),
                Listener::new(move |app: &mut App| toggle_selected_email(app, &toggle_home, id)),
            )
        })
        .collect();
    Column::new()
        .children([
            Padding::new(EdgeInsetsGeometry::all(16.0))
                .child(reply_search_bar(home, leave))
                .into_widget(),
            Expanded::new(ListView::new().children(items)).into_widget(),
        ])
        .into_widget()
}

/// `ReplyEmailDetail`: the open mail's bar over its thread, on the inverse of the surface's
/// content colour.
fn reply_email_detail(
    app: &mut App,
    context: BuildContext,
    home: &Entity<ReplyHome>,
    is_full_screen: bool,
    on_back_pressed: Listener,
) -> WidgetRef {
    let email = app.read(home).opened_email().clone();
    let background = Theme::of(app, context).color_scheme.on_inverse_surface();
    let mut children = vec![
        email_detail_app_bar(app, context, &email, is_full_screen, on_back_pressed).into_widget(),
    ];
    children.extend(
        email
            .threads
            .iter()
            .map(|thread| email_thread_item(app, context, thread)),
    );
    ColoredBox::new(background)
        .child(ListView::new().children(children))
        .into_widget()
}
