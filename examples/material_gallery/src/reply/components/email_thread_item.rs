//! `ReplyEmailThreadItem.kt`: one message of an open mail's thread, as a card.

use waft_foundation::{App, Listener};
use waft_material::{Card, FilledButton, Theme};
use waft_painting::EdgeInsetsGeometry;
use waft_rendering::{CrossAxisAlignment, MainAxisAlignment};
use waft_widgets::{BuildContext, Column, Expanded, IntoWidget, Padding, Row, Text, WidgetRef};

use crate::reply::components::email_list_item::favorite_button;
use crate::reply::components::profile_image::{PROFILE_IMAGE_SIZE, profile_image};
use crate::reply::data::Email;

/// The card for one message: who sent it, its subject and body, and the buttons to answer it.
pub fn email_thread_item(app: &mut App, context: BuildContext, email: &Email) -> WidgetRef {
    let theme = Theme::of(app, context);
    let color_scheme = theme.color_scheme.clone();
    let text_theme = theme.text_theme.clone();
    let label = text_theme.label_medium.clone().unwrap_or_default();

    let header = Row::new().children([
        profile_image(email.sender.avatar, PROFILE_IMAGE_SIZE),
        Expanded::new(
            Padding::new(EdgeInsetsGeometry::symmetric(4.0, 12.0)).child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .children([
                        Text::new(email.sender.first_name)
                            .style(label.clone().color(color_scheme.on_surface()))
                            .into_widget(),
                        Text::new("20 mins ago")
                            .style(label.color(color_scheme.outline()))
                            .into_widget(),
                    ]),
            ),
        )
        .into_widget(),
        favorite_button(color_scheme.surface_container(), color_scheme.outline()),
    ]);
    let answers = Padding::new(EdgeInsetsGeometry::only(0.0, 20.0, 0.0, 8.0)).child(
        Row::new().spacing(12.0).children([
            Expanded::new(answer_button(app, context, "Reply")).into_widget(),
            Expanded::new(answer_button(app, context, "Reply All")).into_widget(),
        ]),
    );
    Card::filled()
        .color(color_scheme.surface_container_high())
        .margin(EdgeInsetsGeometry::symmetric(4.0, 16.0))
        .child(
            Padding::new(EdgeInsetsGeometry::all(20.0)).child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .children([
                        header.into_widget(),
                        Padding::new(EdgeInsetsGeometry::only(0.0, 12.0, 0.0, 8.0))
                            .child(
                                Text::new(email.subject).style(
                                    text_theme
                                        .body_medium
                                        .clone()
                                        .unwrap_or_default()
                                        .color(color_scheme.outline()),
                                ),
                            )
                            .into_widget(),
                        Text::new(email.body)
                            .style(
                                text_theme
                                    .body_large
                                    .clone()
                                    .unwrap_or_default()
                                    .color(color_scheme.on_surface_variant()),
                            )
                            .into_widget(),
                        answers.into_widget(),
                    ]),
            ),
        )
        .into_widget()
}

/// A filled button on the bright surface, with its label in the colour on the surface.
fn answer_button(app: &mut App, context: BuildContext, label: &str) -> WidgetRef {
    let color_scheme = Theme::of(app, context).color_scheme.clone();
    FilledButton::new(
        Some(Listener::new(|_app: &mut App| {})),
        Some(Text::new(label)),
    )
    .style(
        FilledButton::style_from()
            .background_color(color_scheme.surface_bright())
            .foreground_color(color_scheme.on_surface())
            .build(),
    )
    .into_widget()
}
