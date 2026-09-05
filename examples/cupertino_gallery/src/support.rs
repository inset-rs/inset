//! Composition helpers shared by the gallery's screens.
//!
//! Nothing here imitates a Cupertino widget: these only assemble the real ones the same way on
//! every screen, so a page module is a list of the things it demonstrates and nothing else.

use std::rc::Rc;

use reveal_cupertino::{
    CupertinoColors, CupertinoDynamicColor, CupertinoListSection, CupertinoNavigationBar,
    CupertinoPageScaffold, CupertinoTheme, ObstructingPreferredSizeWidgetRef,
};
use reveal_foundation::App;
use reveal_painting::{
    AlignmentGeometry, AnyColor, BorderRadiusGeometry, BoxDecoration, EdgeInsetsGeometry, TextStyle,
};
use reveal_widgets::{
    Align, BuildContext, Container, Icon, IconData, IntoWidget, ListView, MediaQuery, Padding,
    SizedBox, Text, WidgetRef,
};

/// The shell every gallery screen wears: a standard navigation bar whose middle and back label
/// the route supplies, over the grouped-list background iOS puts behind sections.
pub fn screen<K>(body: impl IntoWidget<K>) -> WidgetRef {
    screen_with_bar(CupertinoNavigationBar::new(), body)
}

/// The same, with a bar the screen configured itself.
pub fn screen_with_bar<K>(bar: CupertinoNavigationBar, body: impl IntoWidget<K>) -> WidgetRef {
    CupertinoPageScaffold::new(body)
        .navigation_bar(ObstructingPreferredSizeWidgetRef::new(bar))
        .background_color(CupertinoColors::SYSTEM_GROUPED_BACKGROUND)
        .into_widget()
}

/// A screen body that scrolls, with the top and bottom safe-area insets applied to its CONTENT
/// rather than to the viewport.
///
/// Not a `SafeArea` here, and do not unify this with a fixed body later: a `SafeArea` around a
/// scrollable shrinks the VIEWPORT to start below the bar, so nothing could ever pass under it
/// and the navigation bar's scrolled-under fade would never fire.
///
/// That is Flutter's own split, not a local trick: `SafeArea` for a fixed body, and
/// `ListView(padding: MediaQuery.of(context).padding)` or a `SliverSafeArea` inside a
/// `CustomScrollView` for a scrolling one. These two spacers are what that `padding:` amounts
/// to.
pub fn scrolling_body(children: Vec<WidgetRef>) -> WidgetRef {
    let children = Rc::new(children);
    let count = children.len() as i32 + 2;
    ListView::builder(move |app: &mut App, context: BuildContext, index: i32| {
        let last = children.len() as i32;
        Some(match index {
            0 => SizedBox::new()
                .height(MediaQuery::padding_of(app, context).top)
                .into_widget(),
            index if index <= last => children[index as usize - 1].clone(),
            _ => SizedBox::new()
                .height(MediaQuery::padding_of(app, context).bottom + 24.0)
                .into_widget(),
        })
    })
    .item_count(count)
    .build()
    .into_widget()
}

/// One inset-grouped section: the rounded card iOS Settings groups rows in.
pub fn section(header: &str, children: Vec<WidgetRef>) -> WidgetRef {
    CupertinoListSection::inset_grouped()
        .header(Text::new(header))
        .children(children)
        .into_widget()
}

/// The same, with the explanatory paragraph iOS puts under a group.
///
/// Keep the footer to one line. This builds an `inset_grouped` section, and that constructor
/// hands the footer the theme's text style unmodified — so a long footer renders at body size
/// and takes over the screen rather than reading as small print. The other section style
/// restyles its footer differently; check `list_section.rs` before carrying this rule anywhere
/// else.
pub fn section_with_footer(header: &str, footer: &str, children: Vec<WidgetRef>) -> WidgetRef {
    CupertinoListSection::inset_grouped()
        .header(Text::new(header))
        .footer(Text::new(footer))
        .children(children)
        .into_widget()
}

/// A section row that is not a list tile — a widget under demonstration, centered with the
/// breathing room a tile would have given it.
pub fn showcase<K>(child: impl IntoWidget<K>) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(14.0, 16.0))
        .child(
            Align::new()
                .alignment(AlignmentGeometry::CENTER)
                .child(child),
        )
        .into_widget()
}

/// The same, aligned to the reading start edge, for rows whose point is where they begin.
pub fn showcase_leading<K>(child: impl IntoWidget<K>) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(14.0, 16.0))
        .child(
            Align::new()
                .alignment(AlignmentGeometry::CENTER_START)
                .child(child),
        )
        .into_widget()
}

/// The rounded square an iOS Settings row puts its glyph in.
pub fn badge(icon: IconData, tint: AnyColor, size: f64) -> WidgetRef {
    Container::new()
        .decoration(
            BoxDecoration::new()
                .color(tint)
                .border_radius(BorderRadiusGeometry::circular(size * 0.23)),
        )
        .alignment(AlignmentGeometry::CENTER)
        .width(size)
        .height(size)
        .child(
            Icon::new(Some(icon))
                .size(size * 0.62)
                .color(CupertinoColors::WHITE),
        )
        .into_widget()
}

/// The greyed second line iOS uses for values and explanations.
pub fn secondary(app: &mut App, context: BuildContext, text: impl Into<String>) -> WidgetRef {
    let color = CupertinoDynamicColor::resolve(&CupertinoColors::SECONDARY_LABEL, app, context);
    Text::new(text)
        .style(body_style(app, context).color(color))
        .into_widget()
}

/// The theme's own body style — the same one a `CupertinoListTile` gives its title, so a demo
/// row and a tile read as the same typeface.
pub fn body_style(app: &mut App, context: BuildContext) -> TextStyle {
    CupertinoTheme::of(app, context).text_theme().text_style()
}
