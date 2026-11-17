//! `ReplyEmailListItem.kt`: one mail in the inbox, as a card.

use waft_embedder::Clip;
use waft_foundation::{App, Listener};
use waft_material::{Card, IconButton, Icons, InkWell, Theme};
use waft_painting::{EdgeInsetsGeometry, TextOverflow};
use waft_rendering::{CrossAxisAlignment, MainAxisAlignment};
use waft_widgets::{
    BuildContext, Center, Column, Expanded, GestureDetector, Icon, IntoWidget, Padding, Row,
    SizedBox, Text, WidgetRef,
};

use crate::reply::components::profile_image::{PROFILE_IMAGE_SIZE, profile_image};
use crate::reply::data::Email;

/// The card for `email`: tapping opens it, a long press or a tap on the avatar picks it.
///
/// A picked mail is filled with the primary container and shows a check in place of its
/// avatar; the open one is filled with the secondary container, and the rest with the surface
/// variant, which Flutter deprecates and Reply's scheme still sets.
#[allow(deprecated)]
pub fn email_list_item(
    app: &mut App,
    context: BuildContext,
    email: &Email,
    is_opened: bool,
    is_selected: bool,
    navigate_to_detail: Listener,
    toggle_selection: Listener,
) -> WidgetRef {
    let color_scheme = Theme::of(app, context).color_scheme.clone();
    let text_theme = Theme::of(app, context).text_theme.clone();
    let (container_color, content_color) = if is_selected {
        (
            color_scheme.primary_container(),
            color_scheme.on_primary_container(),
        )
    } else if is_opened {
        (
            color_scheme.secondary_container(),
            color_scheme.on_secondary_container(),
        )
    } else {
        (
            color_scheme.surface_variant(),
            color_scheme.on_surface_variant(),
        )
    };
    let label = text_theme
        .label_medium
        .clone()
        .unwrap_or_default()
        .color(content_color);

    let avatar = if is_selected {
        selected_profile_image(app, context)
    } else {
        profile_image(email.sender.avatar, PROFILE_IMAGE_SIZE)
    };
    let header = Row::new().children([
        GestureDetector::new()
            .on_tap(toggle_selection.clone())
            .child(avatar)
            .into_widget(),
        Expanded::new(
            Padding::new(EdgeInsetsGeometry::symmetric(4.0, 12.0)).child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .children([
                        Text::new(email.sender.first_name)
                            .style(label.clone())
                            .into_widget(),
                        Text::new(email.created_at).style(label).into_widget(),
                    ]),
            ),
        )
        .into_widget(),
        favorite_button(
            color_scheme.surface_container_high(),
            color_scheme.outline(),
        ),
    ]);
    let content = Padding::new(EdgeInsetsGeometry::all(20.0)).child(
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children([
                header.into_widget(),
                Padding::new(EdgeInsetsGeometry::only(0.0, 12.0, 0.0, 8.0))
                    .child(
                        Text::new(email.subject).style(
                            text_theme
                                .body_large
                                .clone()
                                .unwrap_or_default()
                                .color(content_color),
                        ),
                    )
                    .into_widget(),
                Text::new(email.body)
                    .style(
                        text_theme
                            .body_medium
                            .clone()
                            .unwrap_or_default()
                            .color(content_color),
                    )
                    .max_lines(2)
                    .overflow(TextOverflow::Ellipsis)
                    .into_widget(),
            ]),
    );
    Card::filled()
        .color(container_color)
        .margin(EdgeInsetsGeometry::symmetric(4.0, 16.0))
        .clip_behavior(Clip::AntiAlias)
        .child(
            InkWell::new()
                .on_tap(navigate_to_detail)
                .on_long_press(toggle_selection)
                .child(content),
        )
        .into_widget()
}

/// `SelectedProfileImage`: a check on the primary colour, where the avatar was.
fn selected_profile_image(app: &mut App, context: BuildContext) -> WidgetRef {
    let color_scheme = Theme::of(app, context).color_scheme.clone();
    SizedBox::new()
        .width(PROFILE_IMAGE_SIZE)
        .height(PROFILE_IMAGE_SIZE)
        .child(
            waft_widgets::DecoratedBox::new(
                waft_painting::BoxDecoration::new()
                    .color(color_scheme.primary)
                    .shape(waft_painting::BoxShape::Circle),
            )
            .child(
                Center::new().child(
                    Icon::new(Some(Icons::check()))
                        .size(24.0)
                        .color(color_scheme.on_primary),
                ),
            ),
        )
        .into_widget()
}

/// The star at the card's end, on a circle of `container_color`.
pub fn favorite_button(
    container_color: waft_embedder::Color,
    icon_color: waft_embedder::Color,
) -> WidgetRef {
    IconButton::new(
        Some(Listener::new(|_app: &mut App| {})),
        Icon::new(Some(Icons::star_border())).color(icon_color),
    )
    .tooltip("Favorite")
    .style(
        IconButton::style_from()
            .background_color(container_color)
            .build(),
    )
    .into_widget()
}
