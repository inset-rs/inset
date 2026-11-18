//! Composition helpers shared by the gallery's pages.
//!
//! Nothing here imitates a Material widget: these only assemble the real ones the same way on
//! every page, so a page module is a list of the things it demonstrates and nothing else.

use waft_foundation::App;
use waft_material::{AppBar, Scaffold, Theme};
use waft_painting::{EdgeInsetsGeometry, TextAlign};
use waft_rendering::MainAxisSize;
use waft_widgets::{
    BuildContext, Center, Column, Icon, IconData, IntoWidget, Padding, PreferredSizeWidgetRef,
    Text, WidgetRef, Wrap,
};

/// The shell most gallery pages wear: a scaffold whose app bar shows `title` and, because the
/// page was pushed, the automatic back button.
pub fn page<K>(title: &str, body: impl IntoWidget<K>) -> Scaffold {
    Scaffold::new()
        .app_bar(PreferredSizeWidgetRef::new(
            AppBar::new().title(Text::new(title)),
        ))
        .body(body)
}

/// The label over a group of demos, in the scheme's primary colour, as Material's list
/// subheaders are.
pub fn section_header(app: &mut App, context: BuildContext, text: &str) -> WidgetRef {
    let theme = Theme::of(app, context);
    let style = theme
        .text_theme
        .title_small
        .clone()
        .unwrap_or_default()
        .color(theme.color_scheme.primary());
    Padding::new(EdgeInsetsGeometry::from_ltrb(16.0, 24.0, 16.0, 8.0))
        .child(Text::new(text).style(style))
        .into_widget()
}

/// Demos laid out side by side, wrapping onto the next run where the page is narrow.
pub fn showcase(children: Vec<WidgetRef>) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(4.0, 16.0))
        .child(Wrap::new().spacing(8.0).run_spacing(8.0).children(children))
        .into_widget()
}

/// A line of explanation or state under a group of demos.
pub fn caption(app: &mut App, context: BuildContext, text: impl Into<String>) -> WidgetRef {
    let theme = Theme::of(app, context);
    let style = theme
        .text_theme
        .body_medium
        .clone()
        .unwrap_or_default()
        .color(theme.color_scheme.on_surface_variant());
    Padding::new(EdgeInsetsGeometry::symmetric(8.0, 16.0))
        .child(Text::new(text).style(style))
        .into_widget()
}

/// What a pane shows when it has nothing to show: an icon, a title and a line saying what to do,
/// centred together on the pane's surface.
pub fn empty_state(
    app: &mut App,
    context: BuildContext,
    icon: IconData,
    title: &str,
    message: &str,
) -> WidgetRef {
    let theme = Theme::of(app, context);
    let muted = theme.color_scheme.on_surface_variant();
    let title_style = theme.text_theme.title_large.clone().unwrap_or_default();
    let message_style = theme
        .text_theme
        .body_medium
        .clone()
        .unwrap_or_default()
        .color(muted);
    let content = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .spacing(8.0)
        .children([
            Padding::new(EdgeInsetsGeometry::only(0.0, 0.0, 0.0, 8.0))
                .child(Icon::new(Some(icon)).size(48.0).color(muted))
                .into_widget(),
            Text::new(title).style(title_style).into_widget(),
            Text::new(message)
                .style(message_style)
                .text_align(TextAlign::Center)
                .into_widget(),
        ]);
    Scaffold::new()
        .body(Center::new().child(Padding::new(EdgeInsetsGeometry::all(24.0)).child(content)))
        .into_widget()
}
