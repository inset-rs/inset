//! `ReplyNavigationComponents.kt` and `ReplyNavigationActions.kt`: the top-level destinations,
//! and what shows them at each window size.
//!
//! The sample lays its own bar, rail and drawers out; here the [`NavigationSuiteScaffold`] does,
//! and Reply's pieces go where the suite takes them: the rail's menu and compose button lead the
//! rail, the drawer's title and compose button head the drawer, and the compose button floats
//! over the content above a bar.

use waft_foundation::{App, Listener, ValueChanged};
use waft_material::{
    FloatingActionButton, IconButton, Icons, NavigationDrawer, NavigationDrawerDestination,
    NavigationItemVerticalArrangement, NavigationSuiteColors, NavigationSuiteDestination,
    NavigationSuiteScaffold, NavigationSuiteType, Scaffold, Theme, WindowHeightSizeClass,
    WindowSizeClass, WindowWidthSizeClass,
};
use waft_painting::EdgeInsetsGeometry;
use waft_rendering::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize};
use waft_widgets::{
    BuildContext, Builder, Column, Expanded, Icon, IconData, IntoWidget, Padding, Row, SizedBox,
    Text, WidgetRef,
};

/// One of Reply's top-level destinations: `ReplyTopLevelDestination`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Inbox,
    Articles,
    DirectMessages,
    Groups,
}

impl Route {
    /// `TOP_LEVEL_DESTINATIONS`, in order.
    pub const ALL: [Route; 4] = [
        Route::Inbox,
        Route::Articles,
        Route::DirectMessages,
        Route::Groups,
    ];

    fn icon(self) -> IconData {
        match self {
            Route::Inbox => Icons::inbox(),
            Route::Articles => Icons::article(),
            Route::DirectMessages => Icons::chat_bubble_outline(),
            Route::Groups => Icons::group(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Route::Inbox => "Inbox",
            Route::Articles => "Articles",
            Route::DirectMessages => "Direct Messages",
            Route::Groups => "Groups",
        }
    }
}

/// `ReplyNavigationWrapper`'s choice: a bar when the window is compact either way, a permanent
/// drawer from 1200 wide, and a rail otherwise.
pub fn reply_navigation_type(size_class: WindowSizeClass) -> NavigationSuiteType {
    if size_class.width == WindowWidthSizeClass::Compact
        || size_class.height == WindowHeightSizeClass::Compact
    {
        NavigationSuiteType::NavigationBar
    } else if size_class.width >= WindowWidthSizeClass::Large {
        NavigationSuiteType::NavigationDrawer
    } else {
        NavigationSuiteType::NavigationRail
    }
}

/// `ReplyNavigationWrapper`: `content` beside the destinations, in a scaffold whose modal drawer
/// the rail's menu button opens.
pub fn reply_navigation_wrapper(
    app: &mut App,
    context: BuildContext,
    current: Route,
    navigate_to_top_level_destination: ValueChanged<Route>,
    content: WidgetRef,
) -> WidgetRef {
    let size_class = WindowSizeClass::of(app, context);
    let navigation_type = reply_navigation_type(size_class);
    let color_scheme = Theme::of(app, context).color_scheme.clone();
    let arrangement = if size_class.height >= WindowHeightSizeClass::Medium {
        NavigationItemVerticalArrangement::Center
    } else {
        NavigationItemVerticalArrangement::Top
    };
    let destinations = Route::ALL
        .iter()
        .map(|route| NavigationSuiteDestination::new(Icon::new(Some(route.icon())), route.label()))
        .collect();
    let primary_action = match navigation_type {
        NavigationSuiteType::NavigationRail => rail_header(),
        NavigationSuiteType::NavigationDrawer => drawer_header(app, context, None),
        _ => compose_button(true),
    };
    let on_selected = navigate_to_top_level_destination.clone();
    let suite = NavigationSuiteScaffold::new(destinations, content)
        .navigation_suite_type(navigation_type)
        .selected_index(index_of(current))
        .on_destination_selected(std::rc::Rc::new(move |app: &mut App, index: usize| {
            on_selected(app, Route::ALL[index]);
        }))
        .navigation_suite_colors(NavigationSuiteColors {
            navigation_rail_container_color: Some(color_scheme.on_inverse_surface()),
            navigation_drawer_container_color: Some(color_scheme.surface_container_high()),
            ..NavigationSuiteColors::default()
        })
        .navigation_item_vertical_arrangement(arrangement)
        .primary_action(primary_action);
    Scaffold::new()
        .drawer(modal_drawer(current, navigate_to_top_level_destination))
        .body(suite)
        .into_widget()
}

fn index_of(route: Route) -> usize {
    Route::ALL
        .iter()
        .position(|candidate| *candidate == route)
        .expect("every route is a destination")
}

/// `ReplyNavigationRail`'s header: the menu that opens the modal drawer, over the compose
/// button.
fn rail_header() -> WidgetRef {
    Builder::new(|_app: &mut App, context: BuildContext| {
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .spacing(4.0)
            .children([
                IconButton::new(
                    Some(Listener::new(move |app: &mut App| {
                        Scaffold::of(app, context).open_drawer(app);
                    })),
                    Icon::new(Some(Icons::menu())),
                )
                .tooltip("Navigation Drawer")
                .into_widget(),
                Padding::new(EdgeInsetsGeometry::only(0.0, 8.0, 0.0, 32.0))
                    .child(compose_button(false))
                    .into_widget(),
            ])
            .into_widget()
    })
    .into_widget()
}

/// The compose button on the tertiary container: extended with its label, or the icon alone.
fn compose_button(extended: bool) -> WidgetRef {
    Builder::new(move |app: &mut App, context: BuildContext| {
        let color_scheme = Theme::of(app, context).color_scheme.clone();
        let on_pressed = Some(Listener::new(|_app: &mut App| {}));
        let button = if extended {
            FloatingActionButton::extended(on_pressed, Text::new("Compose"))
                .icon(Icon::new(Some(Icons::edit())))
        } else {
            FloatingActionButton::new(on_pressed).child(Icon::new(Some(Icons::edit())).size(18.0))
        };
        button
            .tooltip("Compose")
            .background_color(color_scheme.tertiary_container())
            .foreground_color(color_scheme.on_tertiary_container())
            .into_widget()
    })
    .into_widget()
}

/// The drawers' header: the app's name, the button that closes a modal drawer, and the compose
/// button across the drawer.
fn drawer_header(app: &mut App, context: BuildContext, close: Option<WidgetRef>) -> WidgetRef {
    let theme = Theme::of(app, context);
    let title = Text::new("REPLY").style(
        theme
            .text_theme
            .title_medium
            .clone()
            .unwrap_or_default()
            .color(theme.color_scheme.primary),
    );
    let mut title_row = vec![Expanded::new(title).into_widget()];
    title_row.extend(close);
    Padding::new(EdgeInsetsGeometry::all(16.0))
        .child(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children([
                    Padding::new(EdgeInsetsGeometry::all(16.0))
                        .child(
                            Row::new()
                                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                                .children(title_row),
                        )
                        .into_widget(),
                    Padding::new(EdgeInsetsGeometry::only(0.0, 8.0, 0.0, 40.0))
                        .child(compose_button(true))
                        .into_widget(),
                ]),
        )
        .into_widget()
}

/// `ModalNavigationDrawerContent`: the destinations in a drawer the rail's menu opens.
fn modal_drawer(current: Route, navigate: ValueChanged<Route>) -> WidgetRef {
    Builder::new(move |app: &mut App, context: BuildContext| {
        let close = IconButton::new(
            Some(Listener::new(move |app: &mut App| {
                Scaffold::of(app, context).close_drawer(app);
            })),
            Icon::new(Some(Icons::menu_open())),
        )
        .tooltip("Close drawer")
        .into_widget();
        let navigate = navigate.clone();
        let mut children = vec![drawer_header(app, context, Some(close))];
        children.extend(Route::ALL.iter().map(|route| {
            NavigationDrawerDestination::new(
                Icon::new(Some(route.icon())),
                Text::new(route.label()),
            )
            .into_widget()
        }));
        children.push(SizedBox::new().height(16.0).into_widget());
        NavigationDrawer::new(children)
            .background_color(Theme::of(app, context).color_scheme.on_inverse_surface())
            .selected_index(Some(index_of(current) as i32))
            .on_destination_selected(std::rc::Rc::new(move |app: &mut App, index: i32| {
                Scaffold::of(app, context).close_drawer(app);
                navigate(app, Route::ALL[index as usize]);
            }))
            .into_widget()
    })
    .into_widget()
}
