//! `ReplyAppBars.kt`: the search bar over the inbox and the bar over an open mail.

use std::rc::Rc;

use waft_foundation::{App, Entity, Handle, Listener, Task};
use waft_material::{AppBar, IconButton, Icons, ListTile, SearchAnchor, SearchController, Theme};
use waft_painting::EdgeInsetsGeometry;
use waft_rendering::{CrossAxisAlignment, MainAxisSize};
use waft_widgets::{
    BuildContext, Column, Icon, IntoWidget, Padding, Text, TextEditingControllerLeaf as _,
    WidgetRef, WidgetStatePropertyAll,
};

use crate::reply::components::profile_image::profile_image;
use crate::reply::data::{Email, PROFILE_AVATAR};
use crate::reply::state::{ReplyHome, open_email};

/// `ReplyDockedSearchBar`: searches the inbox by the start of a subject or a sender's name, and
/// opens the mail picked from the results.
///
/// `leading` replaces the search icon: the gallery puts its way back there.
pub fn reply_search_bar(home: Entity<ReplyHome>, leading: Option<WidgetRef>) -> WidgetRef {
    let leading = leading.unwrap_or_else(|| Icon::new(Some(Icons::search())).into_widget());
    SearchAnchor::bar(move |app, context, controller: Handle<SearchController>| {
        let query = controller.text_value(app).to_owned();
        let results = search_results(app, context, &home, controller, &query);
        Task::ready(results)
    })
    .bar_hint_text("Search emails")
    // Compose's search bar lies flat; Flutter's floats at elevation 6.
    .bar_elevation(Rc::new(WidgetStatePropertyAll::new(Some(0.0))))
    .bar_leading(leading)
    .bar_trailing(vec![profile_image(PROFILE_AVATAR, 32.0)])
    .into_search_anchor()
    .into_widget()
}

/// The rows under the search bar for `query`.
fn search_results(
    app: &mut App,
    _context: BuildContext,
    home: &Entity<ReplyHome>,
    controller: Handle<SearchController>,
    query: &str,
) -> Vec<WidgetRef> {
    if query.is_empty() {
        return vec![message("No search history")];
    }
    let matches: Vec<Email> = app
        .read(home)
        .emails
        .iter()
        .filter(|email| matches_query(email, query))
        .cloned()
        .collect();
    if matches.is_empty() {
        return vec![message("No item found")];
    }
    matches
        .into_iter()
        .map(|email| {
            let home = home.clone();
            let id = email.id;
            ListTile::new()
                .leading(profile_image(email.sender.avatar, 32.0))
                .title(Text::new(email.subject))
                .subtitle(Text::new(email.sender.full_name()))
                .on_tap(Listener::new(move |app: &mut App| {
                    controller.close_view(app, Some(String::new()));
                    open_email(app, &home, id);
                }))
                .into_widget()
        })
        .collect()
}

/// Whether `email`'s subject or its sender's full name starts with `query`, ignoring case.
fn matches_query(email: &Email, query: &str) -> bool {
    let query = query.to_lowercase();
    email.subject.to_lowercase().starts_with(&query)
        || email.sender.full_name().to_lowercase().starts_with(&query)
}

fn message(text: &str) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::all(16.0))
        .child(Text::new(text))
        .into_widget()
}

/// `EmailDetailAppBar`: the open mail's subject and how many messages its thread has, with a
/// back button when the mail has a page of its own.
pub fn email_detail_app_bar(
    app: &mut App,
    context: BuildContext,
    email: &Email,
    is_full_screen: bool,
    on_back_pressed: Listener,
) -> AppBar {
    let theme = Theme::of(app, context);
    let color_scheme = theme.color_scheme.clone();
    let text_theme = theme.text_theme.clone();
    let alignment = if is_full_screen {
        CrossAxisAlignment::Center
    } else {
        CrossAxisAlignment::Start
    };
    let title = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(alignment)
        .children([
            Text::new(email.subject)
                .style(
                    text_theme
                        .title_medium
                        .clone()
                        .unwrap_or_default()
                        .color(color_scheme.on_surface_variant()),
                )
                .into_widget(),
            Padding::new(EdgeInsetsGeometry::only(0.0, 4.0, 0.0, 0.0))
                .child(
                    Text::new(format!("{} Messages", email.threads.len())).style(
                        text_theme
                            .label_medium
                            .clone()
                            .unwrap_or_default()
                            .color(color_scheme.outline()),
                    ),
                )
                .into_widget(),
        ]);
    let mut app_bar = AppBar::new()
        .background_color(color_scheme.on_inverse_surface())
        .center_title(is_full_screen)
        .automatically_imply_leading(false)
        .title(title)
        .actions(vec![
            IconButton::new(
                Some(Listener::new(|_app: &mut App| {})),
                Icon::new(Some(Icons::more_vert())).color(color_scheme.on_surface_variant()),
            )
            .tooltip("More options")
            .into_widget(),
        ]);
    if is_full_screen {
        app_bar = app_bar.leading(
            Padding::new(EdgeInsetsGeometry::all(8.0)).child(
                IconButton::filled(
                    Some(on_back_pressed),
                    Icon::new(Some(Icons::arrow_back())).size(14.0),
                )
                .tooltip("Back")
                .style(
                    IconButton::style_from()
                        .background_color(color_scheme.surface)
                        .foreground_color(color_scheme.on_surface)
                        .build(),
                ),
            ),
        );
    }
    app_bar
}
