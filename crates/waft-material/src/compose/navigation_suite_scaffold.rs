//! One list of destinations shown as a bar, a rail or a drawer for the window's size.

use std::rc::Rc;

use waft_embedder::{Color, TextDirection};
use waft_foundation::{App, ValueChanged};
use waft_painting::RoundedRectangleBorder;
use waft_widgets::{
    BuildContext, Column, Directionality, Expanded, IntoWidget, KeyRef, MediaQuery, Positioned,
    Row, SizedBox, Stack, StatelessWidget, Text, WidgetRef,
};

use crate::compose::{WindowHeightSizeClass, WindowSizeClass, WindowWidthSizeClass};
use crate::drawer_theme::DrawerTheme;
use crate::material::Material;
use crate::navigation_bar::{NavigationBar, NavigationDestination};
use crate::navigation_drawer::{NavigationDrawer, NavigationDrawerDestination};
use crate::navigation_rail::{NavigationRail, NavigationRailDestination, NavigationRailLabelType};
use crate::theme::Theme;

/// Compose's `PrimaryActionContentPadding`: the primary action's gap to the bar and the end edge.
const PRIMARY_ACTION_CONTENT_PADDING: f64 = 16.0;

/// What a [`NavigationSuiteScaffold`] shows its destinations in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationSuiteType {
    /// A bottom bar.
    NavigationBar,
    /// A rail with icons and optional labels.
    NavigationRail,
    /// Flutter's extended rail: icons with labels, 256 wide.
    ExtendedNavigationRail,
    /// A permanent drawer beside the content.
    NavigationDrawer,
    /// No navigation widget; the app shows its own.
    None,
}

impl NavigationSuiteType {
    /// A bar when the width or height is Compact; the extended rail from Large; the rail
    /// otherwise.
    pub fn from_window_size_class(size_class: WindowSizeClass) -> NavigationSuiteType {
        if size_class.width == WindowWidthSizeClass::Compact
            || size_class.height == WindowHeightSizeClass::Compact
        {
            NavigationSuiteType::NavigationBar
        } else {
            match size_class.width {
                WindowWidthSizeClass::Large | WindowWidthSizeClass::ExtraLarge => {
                    NavigationSuiteType::ExtendedNavigationRail
                }
                _ => NavigationSuiteType::NavigationRail,
            }
        }
    }
}

/// Compose's `NavigationSuiteColors`: the container colour of each navigation widget the suite
/// can show. `None` keeps the widget's own, from its theme or Material's defaults, as Compose's
/// `NavigationSuiteDefaults.colors()` does; `Colors::TRANSPARENT` shows the suite's container.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NavigationSuiteColors {
    /// The bar's background.
    pub navigation_bar_container_color: Option<Color>,
    /// The rail's background, extended or not.
    pub navigation_rail_container_color: Option<Color>,
    /// The permanent drawer's background.
    pub navigation_drawer_container_color: Option<Color>,
}

/// One destination in a navigation suite.
#[derive(Clone)]
pub struct NavigationSuiteDestination {
    /// The icon shown when the destination is not selected.
    pub icon: WidgetRef,
    /// The icon shown when it is selected.
    pub selected_icon: Option<WidgetRef>,
    /// Its label.
    pub label: String,
    /// Whether it can be selected.
    pub enabled: bool,
}

impl NavigationSuiteDestination {
    /// Creates a destination with an icon and a label.
    pub fn new<K>(
        icon: impl IntoWidget<K>,
        label: impl Into<String>,
    ) -> NavigationSuiteDestination {
        NavigationSuiteDestination {
            icon: icon.into_widget(),
            selected_icon: None,
            label: label.into(),
            enabled: true,
        }
    }

    /// The icon shown when the destination is selected.
    pub fn selected_icon<K>(
        mut self,
        selected_icon: impl IntoWidget<K>,
    ) -> NavigationSuiteDestination {
        self.selected_icon = Some(selected_icon.into_widget());
        self
    }

    /// Whether the destination can be selected. Defaults to `true`.
    pub fn enabled(mut self, enabled: bool) -> NavigationSuiteDestination {
        self.enabled = enabled;
        self
    }
}

/// Content beside a bar, rail or drawer built from one list of destinations.
pub struct NavigationSuiteScaffold {
    /// The scaffold's key.
    pub key: Option<KeyRef>,
    /// The destinations, in order.
    pub destinations: Vec<NavigationSuiteDestination>,
    /// The selected destination.
    pub selected_index: usize,
    /// Called with a selected destination's index.
    pub on_destination_selected: Option<ValueChanged<usize>>,
    /// What to show the destinations in; `None` follows the window's size class.
    pub navigation_suite_type: Option<NavigationSuiteType>,
    /// The rail's leading widget, the drawer's header, or a button above the bar's end.
    pub primary_action: Option<WidgetRef>,
    /// The background of each navigation widget.
    pub navigation_suite_colors: NavigationSuiteColors,
    /// The background behind the navigation widget and the content; `None` is the scheme's
    /// surface, Compose's `colorScheme.background`.
    pub container_color: Option<Color>,
    /// The content beside the navigation widget.
    pub content: WidgetRef,
}

impl NavigationSuiteScaffold {
    /// Creates a suite around `content`.
    pub fn new<K>(
        destinations: Vec<NavigationSuiteDestination>,
        content: impl IntoWidget<K>,
    ) -> NavigationSuiteScaffold {
        NavigationSuiteScaffold {
            key: None,
            destinations,
            selected_index: 0,
            on_destination_selected: None,
            navigation_suite_type: None,
            primary_action: None,
            navigation_suite_colors: NavigationSuiteColors::default(),
            container_color: None,
            content: content.into_widget(),
        }
    }

    /// Sets the selected destination.
    pub fn selected_index(mut self, selected_index: usize) -> NavigationSuiteScaffold {
        self.selected_index = selected_index;
        self
    }

    /// Sets the callback for a selected destination.
    pub fn on_destination_selected(
        mut self,
        on_destination_selected: ValueChanged<usize>,
    ) -> NavigationSuiteScaffold {
        self.on_destination_selected = Some(on_destination_selected);
        self
    }

    /// Sets what to show the destinations in.
    pub fn navigation_suite_type(
        mut self,
        navigation_suite_type: NavigationSuiteType,
    ) -> NavigationSuiteScaffold {
        self.navigation_suite_type = Some(navigation_suite_type);
        self
    }

    /// Sets the rail's leading widget, the drawer's header, or the button above the bar's end.
    pub fn primary_action<K>(
        mut self,
        primary_action: impl IntoWidget<K>,
    ) -> NavigationSuiteScaffold {
        self.primary_action = Some(primary_action.into_widget());
        self
    }

    /// Sets the background of each navigation widget.
    pub fn navigation_suite_colors(
        mut self,
        navigation_suite_colors: NavigationSuiteColors,
    ) -> NavigationSuiteScaffold {
        self.navigation_suite_colors = navigation_suite_colors;
        self
    }

    /// Sets the background behind the navigation widget and the content.
    pub fn container_color(mut self, container_color: Color) -> NavigationSuiteScaffold {
        self.container_color = Some(container_color);
        self
    }

    /// Sets the scaffold's key.
    pub fn key(mut self, key: KeyRef) -> NavigationSuiteScaffold {
        self.key = Some(key);
        self
    }

    fn bar(&self) -> WidgetRef {
        let destinations = self
            .destinations
            .iter()
            .map(|destination| {
                let mut item =
                    NavigationDestination::new(destination.icon.clone(), destination.label.clone())
                        .enabled(destination.enabled);
                if let Some(selected_icon) = &destination.selected_icon {
                    item = item.selected_icon(selected_icon.clone());
                }
                item.into_widget()
            })
            .collect::<Vec<_>>();
        let mut bar = NavigationBar::new(destinations).selected_index(self.selected_index);
        if let Some(color) = self.navigation_suite_colors.navigation_bar_container_color {
            bar = bar.background_color(color);
        }
        if let Some(on_selected) = &self.on_destination_selected {
            let on_selected = on_selected.clone();
            bar = bar.on_destination_selected(move |app, index| on_selected(app, index));
        }
        bar.into_widget()
    }

    fn rail(&self, extended: bool) -> WidgetRef {
        let destinations = self
            .destinations
            .iter()
            .map(|destination| {
                let selected_icon = destination
                    .selected_icon
                    .clone()
                    .unwrap_or_else(|| destination.icon.clone());
                NavigationRailDestination::new(
                    destination.icon.clone(),
                    Text::new(destination.label.clone()),
                )
                .selected_icon(selected_icon)
                .disabled(!destination.enabled)
            })
            .collect::<Vec<_>>();
        let mut rail = NavigationRail::new(destinations, Some(self.selected_index))
            .label_type(NavigationRailLabelType::All);
        if extended {
            rail = rail
                .extended(true)
                .label_type(NavigationRailLabelType::None)
                .min_extended_width(256.0);
        }
        if let Some(color) = self.navigation_suite_colors.navigation_rail_container_color {
            rail = rail.background_color(color);
        }
        if let Some(primary_action) = &self.primary_action {
            rail = rail.leading(primary_action.clone());
        }
        if let Some(on_selected) = &self.on_destination_selected {
            let on_selected = on_selected.clone();
            rail = rail.on_destination_selected(move |app, index| on_selected(app, index));
        }
        rail.into_widget()
    }

    /// Compose's `PermanentDrawerSheet`: the drawer with square corners and no elevation.
    fn drawer(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let destinations = self
            .destinations
            .iter()
            .map(|destination| {
                let mut item = NavigationDrawerDestination::new(
                    destination.icon.clone(),
                    Text::new(destination.label.clone()),
                )
                .enabled(destination.enabled);
                if let Some(selected_icon) = &destination.selected_icon {
                    item = item.selected_icon(selected_icon.clone());
                }
                item.into_widget()
            })
            .collect::<Vec<_>>();
        let mut drawer = NavigationDrawer::new(destinations)
            .selected_index(Some(self.selected_index as i32))
            .elevation(0.0);
        if let Some(color) = self
            .navigation_suite_colors
            .navigation_drawer_container_color
        {
            drawer = drawer.background_color(color);
        }
        if let Some(primary_action) = &self.primary_action {
            drawer = drawer.header(primary_action.clone());
        }
        if let Some(on_selected) = &self.on_destination_selected {
            let on_selected = on_selected.clone();
            drawer = drawer.on_destination_selected(Rc::new(move |app, index| {
                on_selected(app, index as usize);
            }));
        }
        let sheet = DrawerTheme::of(app, context).shape(RoundedRectangleBorder::default());
        DrawerTheme::new(sheet, SizedBox::new().width(360.0).child(drawer)).into_widget()
    }

    /// The content with the primary action floated above its bottom end, as over a bar.
    fn content_with_primary_action(
        &self,
        app: &mut App,
        context: BuildContext,
        content: WidgetRef,
    ) -> WidgetRef {
        let Some(primary_action) = &self.primary_action else {
            return content;
        };
        let text_direction = Directionality::of(app, context);
        Stack::new()
            .children([
                Positioned::fill(content).into_widget(),
                Positioned::directional(
                    text_direction,
                    None,
                    Some(PRIMARY_ACTION_CONTENT_PADDING),
                    primary_action.clone(),
                )
                .bottom(PRIMARY_ACTION_CONTENT_PADDING)
                .into_widget(),
            ])
            .into_widget()
    }

    fn content_without_padding(
        &self,
        app: &mut App,
        context: BuildContext,
        suite_type: NavigationSuiteType,
    ) -> WidgetRef {
        let remove_start = matches!(
            suite_type,
            NavigationSuiteType::NavigationRail
                | NavigationSuiteType::ExtendedNavigationRail
                | NavigationSuiteType::NavigationDrawer
        );
        let remove_bottom = suite_type == NavigationSuiteType::NavigationBar;
        if !remove_start && !remove_bottom {
            return self.content.clone();
        }
        let rtl = Directionality::of(app, context) == TextDirection::Rtl;
        let data = MediaQuery::of(app, context).remove_padding(
            remove_start && !rtl,
            false,
            remove_start && rtl,
            remove_bottom,
        );
        MediaQuery::new(data, self.content.clone()).into_widget()
    }
}

impl std::fmt::Debug for NavigationSuiteScaffold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationSuiteScaffold")
            .field("key", &self.key)
            .field("selected_index", &self.selected_index)
            .field("navigation_suite_type", &self.navigation_suite_type)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for NavigationSuiteScaffold {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let suite_type = self.navigation_suite_type.unwrap_or_else(|| {
            NavigationSuiteType::from_window_size_class(WindowSizeClass::of(app, context))
        });
        let content = self.content_without_padding(app, context, suite_type);
        let container_color = self
            .container_color
            .unwrap_or_else(|| Theme::of(app, context).color_scheme.surface);
        let suite = match suite_type {
            NavigationSuiteType::None => content,
            NavigationSuiteType::NavigationBar => {
                let content = self.content_with_primary_action(app, context, content);
                Column::new()
                    .children([Expanded::new(content).into_widget(), self.bar()])
                    .into_widget()
            }
            NavigationSuiteType::NavigationRail => Row::new()
                .children([self.rail(false), Expanded::new(content).into_widget()])
                .into_widget(),
            NavigationSuiteType::ExtendedNavigationRail => Row::new()
                .children([self.rail(true), Expanded::new(content).into_widget()])
                .into_widget(),
            NavigationSuiteType::NavigationDrawer => Row::new()
                .children([
                    self.drawer(app, context),
                    Expanded::new(content).into_widget(),
                ])
                .into_widget(),
        };
        // Compose's `Surface(color = containerColor)` around the suite and its content.
        Material::new()
            .color(container_color)
            .child(suite)
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use waft_embedder::Size;
    use waft_foundation::{App, ValueKey};
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{MediaQueryData, Text};

    use super::*;
    use crate::compose::WindowHeightSizeClass;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn size_class(width: WindowWidthSizeClass, height: WindowHeightSizeClass) -> WindowSizeClass {
        WindowSizeClass { width, height }
    }

    fn destination(label: &str) -> NavigationSuiteDestination {
        NavigationSuiteDestination::new(Text::new(format!("{label} icon")), label)
    }

    fn suite() -> NavigationSuiteScaffold {
        NavigationSuiteScaffold::new(
            vec![destination("A"), destination("B")],
            Text::new("content"),
        )
    }

    fn pump(tester: &mut WidgetTester, width: f64, height: f64, suite: NavigationSuiteScaffold) {
        tester.set_surface_size(Some(Size::new(width, height)));
        let theme = ThemeData::new(&tester.cell.borrow());
        let home = MediaQuery::new(MediaQueryData::new().size(Size::new(width, height)), suite);
        tester.pump_widget(material_app(theme, home.into_widget()));
    }

    #[test]
    fn the_suite_type_follows_the_window() {
        assert_eq!(
            NavigationSuiteType::from_window_size_class(size_class(
                WindowWidthSizeClass::Compact,
                WindowHeightSizeClass::Expanded
            )),
            NavigationSuiteType::NavigationBar
        );
        assert_eq!(
            NavigationSuiteType::from_window_size_class(size_class(
                WindowWidthSizeClass::Expanded,
                WindowHeightSizeClass::Compact
            )),
            NavigationSuiteType::NavigationBar
        );
        assert_eq!(
            NavigationSuiteType::from_window_size_class(size_class(
                WindowWidthSizeClass::Medium,
                WindowHeightSizeClass::Expanded
            )),
            NavigationSuiteType::NavigationRail
        );
        assert_eq!(
            NavigationSuiteType::from_window_size_class(size_class(
                WindowWidthSizeClass::Expanded,
                WindowHeightSizeClass::Expanded
            )),
            NavigationSuiteType::NavigationRail
        );
        assert_eq!(
            NavigationSuiteType::from_window_size_class(size_class(
                WindowWidthSizeClass::Large,
                WindowHeightSizeClass::Expanded
            )),
            NavigationSuiteType::ExtendedNavigationRail
        );
        assert_eq!(
            NavigationSuiteType::from_window_size_class(size_class(
                WindowWidthSizeClass::ExtraLarge,
                WindowHeightSizeClass::Expanded
            )),
            NavigationSuiteType::ExtendedNavigationRail
        );
    }

    #[test]
    fn a_compact_window_shows_a_navigation_bar() {
        test_widgets(|tester| {
            pump(tester, 400.0, 700.0, suite());
            assert_eq!(find::by_type::<NavigationBar>().evaluate(tester).len(), 1);
            assert_eq!(find::by_type::<NavigationRail>().evaluate(tester).len(), 0);
            assert!(!find::text("A").evaluate(tester).is_empty());
        });
    }

    #[test]
    fn a_short_window_keeps_the_bar() {
        test_widgets(|tester| {
            pump(tester, 800.0, 400.0, suite());
            assert_eq!(find::by_type::<NavigationBar>().evaluate(tester).len(), 1);
            assert_eq!(find::by_type::<NavigationRail>().evaluate(tester).len(), 0);
        });
    }

    #[test]
    fn a_medium_window_shows_a_rail_with_labels() {
        test_widgets(|tester| {
            pump(tester, 800.0, 700.0, suite());
            assert_eq!(find::by_type::<NavigationRail>().evaluate(tester).len(), 1);
            let (extended, labels) = tester.widget(
                find::by_type::<NavigationRail>(),
                |rail: &NavigationRail| (rail.extended, rail.label_type),
            );
            assert!(!extended);
            assert_eq!(labels, Some(NavigationRailLabelType::All));
        });
    }

    #[test]
    fn a_large_window_shows_the_extended_rail() {
        test_widgets(|tester| {
            pump(tester, 1300.0, 700.0, suite());
            let (extended, width, labels) = tester.widget(
                find::by_type::<NavigationRail>(),
                |rail: &NavigationRail| (rail.extended, rail.min_extended_width, rail.label_type),
            );
            assert!(extended);
            assert_eq!(width, Some(256.0));
            assert_eq!(labels, Some(NavigationRailLabelType::None));
        });
    }

    #[test]
    fn an_app_can_choose_the_drawer() {
        test_widgets(|tester| {
            pump(
                tester,
                1300.0,
                700.0,
                suite().navigation_suite_type(NavigationSuiteType::NavigationDrawer),
            );
            assert_eq!(
                find::by_type::<NavigationDrawer>().evaluate(tester).len(),
                1
            );
            assert_eq!(find::by_type::<NavigationRail>().evaluate(tester).len(), 0);
        });
    }

    #[test]
    fn the_suite_sits_on_its_container_color_and_the_rail_takes_its_own() {
        test_widgets(|tester| {
            let container = Color::from_argb(255, 1, 2, 3);
            let rail = Color::from_argb(0, 0, 0, 0);
            pump(
                tester,
                800.0,
                700.0,
                suite()
                    .container_color(container)
                    .navigation_suite_colors(NavigationSuiteColors {
                        navigation_rail_container_color: Some(rail),
                        ..NavigationSuiteColors::default()
                    }),
            );
            let suite_color = tester.widget(
                find::ancestor(
                    find::by_type::<NavigationRail>(),
                    find::by_type::<Material>(),
                )
                .last(),
                |material: &Material| material.color,
            );
            assert_eq!(suite_color, Some(container));
            let rail_color = tester.widget(
                find::by_type::<NavigationRail>(),
                |navigation_rail: &NavigationRail| navigation_rail.background_color,
            );
            assert_eq!(rail_color, Some(rail));
        });
    }

    #[test]
    fn the_container_defaults_to_the_scheme_s_surface() {
        test_widgets(|tester| {
            pump(tester, 400.0, 700.0, suite());
            let surface = ThemeData::new(&tester.cell.borrow()).color_scheme.surface;
            let suite_color = tester.widget(
                find::ancestor(find::text("content"), find::by_type::<Material>()).last(),
                |material: &Material| material.color,
            );
            assert_eq!(suite_color, Some(surface));
        });
    }

    #[test]
    fn selecting_a_destination_reports_its_index() {
        test_widgets(|tester| {
            let selected = Rc::new(Cell::new(None));
            let callback = Rc::clone(&selected);
            pump(
                tester,
                800.0,
                700.0,
                suite().on_destination_selected(Rc::new(move |_app: &mut App, index: usize| {
                    callback.set(Some(index));
                })),
            );
            tester.tap(find::text("B"));
            assert_eq!(selected.get(), Some(1));
        });
    }

    #[test]
    fn the_primary_action_leads_the_rail() {
        test_widgets(|tester| {
            pump(
                tester,
                800.0,
                700.0,
                suite().primary_action(Text::new("compose")),
            );
            let leading = tester.widget(
                find::by_type::<NavigationRail>(),
                |rail: &NavigationRail| rail.leading.is_some(),
            );
            assert!(leading);
            assert!(!find::text("compose").evaluate(tester).is_empty());
        });
    }

    #[test]
    fn the_primary_action_floats_above_the_bar_s_end_in_either_direction() {
        test_widgets(|tester| {
            for (direction, expected_left) in
                [(TextDirection::Ltr, 334.0), (TextDirection::Rtl, 16.0)]
            {
                let action = SizedBox::new()
                    .key(ValueKey::new("action").into())
                    .width(50.0)
                    .height(50.0);
                let suite = Directionality::new(direction, suite().primary_action(action));
                tester.set_surface_size(Some(Size::new(400.0, 700.0)));
                let theme = ThemeData::new(&tester.cell.borrow());
                let home =
                    MediaQuery::new(MediaQueryData::new().size(Size::new(400.0, 700.0)), suite);
                tester.pump_widget(material_app(theme, home.into_widget()));

                let action = tester.get_rect(find::by_key(ValueKey::new("action")));
                let bar = tester.get_rect(find::by_type::<NavigationBar>());
                assert_eq!(action.left, expected_left, "{direction:?}");
                assert_eq!(action.bottom, bar.top - 16.0, "{direction:?}");
            }
        });
    }
}
