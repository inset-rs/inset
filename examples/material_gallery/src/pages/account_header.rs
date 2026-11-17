//! The user accounts drawer header: a current account picture, other accounts, and the
//! account's name and email over the drawer's primary colour.

use waft_foundation::{App, Listener};
use waft_material::{CircleAvatar, Colors, Icons, UserAccountsDrawerHeader};
use waft_widgets::{BuildContext, Icon, IntoWidget, ListView, Text, WidgetRef};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = waft_widgets::Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Account header"),
                UserAccountsDrawerHeader::new(
                    Some(Text::new("Jane Doe").into_widget()),
                    Some(Text::new("jane@example.com").into_widget()),
                )
                .current_account_picture(
                    CircleAvatar::new()
                        .child(Icon::new(Some(Icons::person())))
                        .radius(36.0),
                )
                .other_accounts_pictures(vec![
                    CircleAvatar::new().child(Text::new("B")).into_widget(),
                    CircleAvatar::new().child(Text::new("C")).into_widget(),
                ])
                .on_details_pressed(Listener::new(|_app: &mut App| {}))
                .into_widget(),
                caption(
                    app,
                    context,
                    "Tapping the account row flips the arrow and reports the details press.",
                ),
                section_header(app, context, "Another colour"),
                UserAccountsDrawerHeader::new(
                    Some(Text::new("Inbox").into_widget()),
                    Some(Text::new("inbox@example.com").into_widget()),
                )
                .decoration(waft_painting::BoxDecoration::new().color(Colors::TEAL.shade700()))
                .arrow_color(Colors::AMBER.primary())
                .into_widget(),
            ])
            .into_widget()
    });
    page(Entry::AccountHeader.title(), content).into_widget()
}
