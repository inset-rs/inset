//! A Navigator whose pages follow an app's back stack and a three-pane scene strategy.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use waft_foundation::{App, Handle, ValueKey};
use waft_widgets::{
    BuildContext, GlobalKey, IntoWidget, KeyRef, Navigator, NavigatorPopHandler, NavigatorState,
    PageRef, RouteEntry, RouteTransitionRecord, State, StateData, StatefulWidget,
    TransitionDelegate, WidgetRef,
};

use crate::compose::{
    ThreePaneScaffoldDestinationItem, ThreePaneScaffoldScene, ThreePaneSceneStrategy,
};
use crate::page::MaterialPage;

/// Builds the page of its own for `back_stack[index]`.
pub type ThreePaneEntryPageBuilder = Rc<dyn Fn(&mut App, BuildContext, usize) -> WidgetRef>;

/// Builds the one page that holds a scene's [`ThreePaneScaffold`](crate::ThreePaneScaffold).
pub type ThreePaneScenePageBuilder =
    Rc<dyn Fn(&mut App, BuildContext, &ThreePaneScaffoldScene) -> WidgetRef>;

/// Called with the length the back stack has after back, so the app truncates its stack.
pub type ThreePaneBackStackTruncated = Rc<dyn Fn(&mut App, usize)>;

/// A [`Navigator`] whose pages follow an app's back stack, grouped by a [`ThreePaneSceneStrategy`].
///
/// Each entry before the scene is a page of its own, and one page holds the scene. With no scene
/// every entry is its own page, so a phone's detail is an ordinary pushed page with the
/// platform's back gestures. A page's key and name are the first back-stack index it shows, so a
/// resize that splits a scene keeps the list's page; pages a resize adds or removes come and go
/// without a transition.
///
/// The back stack stays the app's: every back, whether it leaves a page or only changes the
/// panes of the scene, reaches the app as the length its stack should be truncated to. The
/// navigator is nested in the app's, so it takes the system back first, through a
/// [`NavigatorPopHandler`], while it has a page to pop.
pub struct ThreePaneNavigator<T: PartialEq + 'static> {
    /// The widget's key.
    pub key: Option<KeyRef>,
    /// The inner navigator's key; `None` uses one of the widget's own.
    pub navigator_key: Option<GlobalKey>,
    /// Which trailing entries share a page.
    pub strategy: ThreePaneSceneStrategy,
    /// The app's back stack; `None` for an entry that is not a pane.
    pub back_stack: Vec<Option<ThreePaneScaffoldDestinationItem<T>>>,
    /// Builds the page of its own for an entry.
    pub entry_page: ThreePaneEntryPageBuilder,
    /// Builds the page that holds the scene.
    pub scene_page: ThreePaneScenePageBuilder,
    /// Called with the length the back stack has after back.
    pub on_back_stack_truncated: ThreePaneBackStackTruncated,
}

impl<T: PartialEq + 'static> ThreePaneNavigator<T> {
    /// Creates a navigator over `back_stack`: `entry_page` builds an entry's own page,
    /// `scene_page` the page of the scene, and `on_back_stack_truncated` receives every back.
    pub fn new(
        strategy: ThreePaneSceneStrategy,
        back_stack: Vec<Option<ThreePaneScaffoldDestinationItem<T>>>,
        entry_page: impl Fn(&mut App, BuildContext, usize) -> WidgetRef + 'static,
        scene_page: impl Fn(&mut App, BuildContext, &ThreePaneScaffoldScene) -> WidgetRef + 'static,
        on_back_stack_truncated: impl Fn(&mut App, usize) + 'static,
    ) -> ThreePaneNavigator<T> {
        ThreePaneNavigator {
            key: None,
            navigator_key: None,
            strategy,
            back_stack,
            entry_page: Rc::new(entry_page),
            scene_page: Rc::new(scene_page),
            on_back_stack_truncated: Rc::new(on_back_stack_truncated),
        }
    }

    /// Sets the widget's key.
    pub fn key(mut self, key: KeyRef) -> ThreePaneNavigator<T> {
        self.key = Some(key);
        self
    }

    /// Sets the inner navigator's key.
    pub fn navigator_key(mut self, navigator_key: GlobalKey) -> ThreePaneNavigator<T> {
        self.navigator_key = Some(navigator_key);
        self
    }

    fn pages(&self, app: &mut App, context: BuildContext) -> Vec<PageRef> {
        match self.strategy.calculate_scene(&self.back_stack) {
            None => (0..self.back_stack.len())
                .map(|index| self.build_entry_page(app, context, index))
                .collect(),
            Some(scene) => {
                let mut pages: Vec<PageRef> = (0..scene.entries.start)
                    .map(|index| self.build_entry_page(app, context, index))
                    .collect();
                pages.push(self.build_scene_page(app, context, &scene));
                pages
            }
        }
    }

    fn build_entry_page(&self, app: &mut App, context: BuildContext, index: usize) -> PageRef {
        let child = (self.entry_page)(app, context, index);
        Rc::new(page_at(index, child))
    }

    /// The scene's page pops only when back leaves the scene; back inside it truncates the stack.
    fn build_scene_page(
        &self,
        app: &mut App,
        context: BuildContext,
        scene: &ThreePaneScaffoldScene,
    ) -> PageRef {
        let child = (self.scene_page)(app, context, scene);
        let page = page_at(scene.entries.start, child).can_pop(scene.previous_value.is_none());
        if scene.previous_value.is_none() {
            return Rc::new(page);
        }
        let len_after_back = scene.back_stack_len_after_back;
        let on_back_stack_truncated = Rc::clone(&self.on_back_stack_truncated);
        Rc::new(page.on_pop_invoked(Rc::new(move |app, did_pop, _result| {
            if !did_pop {
                on_back_stack_truncated(app, len_after_back);
            }
        })))
    }
}

/// A page for the entries from `first_entry`: its key and name are that index.
fn page_at(first_entry: usize, child: WidgetRef) -> MaterialPage {
    MaterialPage::new(child)
        .key(ValueKey::new(first_entry).into())
        .name(first_entry.to_string())
}

/// The first back-stack index `page` shows, from its name.
fn first_entry_of(page: &PageRef) -> Option<usize> {
    page.name().and_then(|name| name.parse().ok())
}

impl<T: PartialEq + 'static> fmt::Debug for ThreePaneNavigator<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreePaneNavigator")
            .field("key", &self.key)
            .field("strategy", &self.strategy)
            .field("back_stack_len", &self.back_stack.len())
            .finish_non_exhaustive()
    }
}

impl<T: PartialEq + 'static> StatefulWidget for ThreePaneNavigator<T> {
    type State = ThreePaneNavigatorState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ThreePaneNavigatorState<T> {
        ThreePaneNavigatorState {
            state: StateData::new(),
            own_navigator_key: GlobalKey::new(),
            last_strategy: None,
        }
    }
}

/// The navigator's key, and the strategy of the last build: when it changes, the window
/// regrouped the same entries.
pub struct ThreePaneNavigatorState<T: PartialEq + 'static> {
    state: StateData<ThreePaneNavigator<T>>,
    own_navigator_key: GlobalKey,
    last_strategy: Option<ThreePaneSceneStrategy>,
}

impl<T: PartialEq + 'static> State for ThreePaneNavigatorState<T> {
    type Widget = ThreePaneNavigator<T>;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget_ref = context.widget(app).clone();
        let widget = waft_widgets::downcast_widget::<ThreePaneNavigator<T>>(&*widget_ref)
            .expect("a ThreePaneNavigatorState builds its ThreePaneNavigator");
        let regrouped = app.get(self).last_strategy.as_ref() != Some(&widget.strategy);
        app.get_mut(self).last_strategy = Some(widget.strategy.clone());

        let pages = widget.pages(app, context);
        let on_back_stack_truncated = Rc::clone(&widget.on_back_stack_truncated);
        let mut navigator = Navigator::new()
            .pages(pages)
            .on_did_remove_page(move |app, page| {
                if let Some(first_entry) = first_entry_of(&page) {
                    on_back_stack_truncated(app, first_entry);
                }
            });
        if regrouped {
            navigator = navigator.transition_delegate(Rc::new(RegroupTransitionDelegate));
        }
        let navigator_key = widget
            .navigator_key
            .clone()
            .unwrap_or_else(|| app.get(self).own_navigator_key.clone());
        NavigatorPopHandler::new(navigator.key(navigator_key.clone().into()))
            .on_pop_with_result(move |app, _result| {
                if let Some(navigator) = navigator_key.current_state::<NavigatorState>(app) {
                    navigator.maybe_pop(app, None);
                }
            })
            .into_widget()
    }
}

/// Pages added or removed because the window regrouped the same back stack: they come and go
/// without a transition, as they are not navigation.
struct RegroupTransitionDelegate;

impl TransitionDelegate for RegroupTransitionDelegate {
    fn resolve(
        &self,
        app: &mut App,
        new_page_route_history: &[Handle<RouteEntry>],
        location_to_exiting_page_route: &HashMap<Option<Handle<RouteEntry>>, Handle<RouteEntry>>,
        page_route_to_pageless_routes: &HashMap<
            Option<Handle<RouteEntry>>,
            Vec<Handle<RouteEntry>>,
        >,
    ) -> Vec<Handle<RouteEntry>> {
        let mut results = Vec::new();
        complete_exiting_routes(
            app,
            &mut results,
            location_to_exiting_page_route,
            page_route_to_pageless_routes,
            None,
        );
        for page_route in new_page_route_history.iter().copied() {
            if page_route.is_waiting_for_entering_decision(app) {
                page_route.mark_for_add(app);
            }
            results.push(page_route);
            complete_exiting_routes(
                app,
                &mut results,
                location_to_exiting_page_route,
                page_route_to_pageless_routes,
                Some(page_route),
            );
        }
        results
    }
}

/// Completes, without a transition, the exiting routes above `location` and their pageless
/// routes.
fn complete_exiting_routes(
    app: &mut App,
    results: &mut Vec<Handle<RouteEntry>>,
    location_to_exiting_page_route: &HashMap<Option<Handle<RouteEntry>>, Handle<RouteEntry>>,
    page_route_to_pageless_routes: &HashMap<Option<Handle<RouteEntry>>, Vec<Handle<RouteEntry>>>,
    location: Option<Handle<RouteEntry>>,
) {
    let Some(exiting_page_route) = location_to_exiting_page_route.get(&location).copied() else {
        return;
    };
    complete(app, exiting_page_route);
    for pageless_route in page_route_to_pageless_routes
        .get(&Some(exiting_page_route))
        .into_iter()
        .flatten()
    {
        complete(app, *pageless_route);
    }
    results.push(exiting_page_route);
    complete_exiting_routes(
        app,
        results,
        location_to_exiting_page_route,
        page_route_to_pageless_routes,
        Some(exiting_page_route),
    );
}

fn complete(app: &mut App, route: Handle<RouteEntry>) {
    if route.is_waiting_for_exiting_decision(app) {
        let result = route.route(app).current_result(app);
        route.mark_for_complete(app, result);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use waft_embedder::Size;
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{
        MediaQuery, MediaQueryData, StateSetter, StatefulBuilder, Text, WidgetsBinding,
    };

    use super::*;
    use crate::compose::{
        ListDetailPaneScaffoldRole, Pane, PaneScaffoldDirective, ThreePaneScaffold,
        ThreePaneScaffoldDestinationItem, WindowSizeClass,
    };
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    type Stack = Rc<RefCell<Vec<Option<ThreePaneScaffoldDestinationItem<i32>>>>>;

    fn list() -> Option<ThreePaneScaffoldDestinationItem<i32>> {
        Some(ThreePaneScaffoldDestinationItem::new(
            ListDetailPaneScaffoldRole::LIST,
            None,
        ))
    }

    fn detail(mail: i32) -> Option<ThreePaneScaffoldDestinationItem<i32>> {
        Some(ThreePaneScaffoldDestinationItem::new(
            ListDetailPaneScaffoldRole::DETAIL,
            Some(mail),
        ))
    }

    fn extra() -> Option<ThreePaneScaffoldDestinationItem<i32>> {
        Some(ThreePaneScaffoldDestinationItem::new(
            ListDetailPaneScaffoldRole::EXTRA,
            Some(9),
        ))
    }

    fn home() -> Option<ThreePaneScaffoldDestinationItem<i32>> {
        None
    }

    fn entry_widget(stack: &Stack, index: usize) -> WidgetRef {
        let label = match stack.borrow()[index].as_ref().map(|item| item.pane) {
            Some(ListDetailPaneScaffoldRole::LIST) => "entry list",
            Some(ListDetailPaneScaffoldRole::DETAIL) => "entry detail",
            Some(_) => "entry extra",
            None => "entry home",
        };
        Text::new(label).into_widget()
    }

    fn pane_widget(label: &'static str) -> Pane {
        Pane::new(Text::new(label))
    }

    fn scene_widget(
        strategy: &ThreePaneSceneStrategy,
        scene: &ThreePaneScaffoldScene,
    ) -> WidgetRef {
        let mut scaffold = ThreePaneScaffold::list_detail(
            strategy.directive.clone(),
            scene.value,
            pane_widget("list pane"),
            pane_widget("detail pane"),
        );
        if scene.entry_for(ListDetailPaneScaffoldRole::EXTRA).is_some() {
            scaffold = scaffold.tertiary_pane(pane_widget("extra pane"));
        }
        scaffold.into_widget()
    }

    fn host(stack: Stack) -> impl Fn(&mut App, BuildContext, StateSetter) -> WidgetRef {
        move |app: &mut App, context: BuildContext, set_state: StateSetter| {
            let size_class = WindowSizeClass::of(app, context);
            let strategy = ThreePaneSceneStrategy::list_detail(
                PaneScaffoldDirective::from_window_size_class(size_class),
            );
            let stack_for_pages = Rc::clone(&stack);
            let stack_for_truncate = Rc::clone(&stack);
            let scene_strategy = strategy.clone();
            ThreePaneNavigator::new(
                strategy,
                stack.borrow().clone(),
                move |_app, _context, index| entry_widget(&stack_for_pages, index),
                move |_app, _context, scene| scene_widget(&scene_strategy, scene),
                move |app, len| {
                    stack_for_truncate.borrow_mut().truncate(len);
                    set_state(app, &mut |_app| {});
                },
            )
            .into_widget()
        }
    }

    fn pump(tester: &mut WidgetTester, size: Size, stack: Stack) {
        tester.set_surface_size(Some(size));
        let theme = ThemeData::new(&tester.cell.borrow());
        let home = MediaQuery::new(
            MediaQueryData::new().size(size),
            StatefulBuilder::new(host(Rc::clone(&stack))),
        );
        tester.pump_widget(material_app(theme, home.into_widget()));
    }

    /// Pumps the first frame, and the one where the navigator's pop handler learns it can pop.
    fn pump_first(tester: &mut WidgetTester, size: Size, stack: Stack) {
        pump(tester, size, stack);
        tester.pump();
    }

    /// The system back, as the engine's `popRoute` sends it.
    fn system_back(tester: &mut WidgetTester) {
        {
            let mut app = tester.cell.borrow_mut();
            let view = app.platform().views().first().expect("a view").id();
            WidgetsBinding::instance(&mut app).handle_pop_route(&mut app, view);
            app.drain_microtasks();
        }
        tester.pump_and_settle();
    }

    fn shown(tester: &WidgetTester, text: &str) -> bool {
        !find::text(text).evaluate(tester).is_empty()
    }

    fn scaffolds(tester: &WidgetTester) -> usize {
        find::by_type::<ThreePaneScaffold>().evaluate(tester).len()
    }

    #[test]
    fn a_phone_shows_the_detail_as_its_own_page_and_back_pops_it() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list()]));
            pump_first(tester, Size::new(400.0, 700.0), Rc::clone(&stack));
            assert!(shown(tester, "entry list"));

            stack.borrow_mut().push(detail(1));
            pump(tester, Size::new(400.0, 700.0), Rc::clone(&stack));
            assert!(
                tester.pump_and_settle() > 1,
                "the detail is pushed with a transition"
            );
            assert!(shown(tester, "entry detail"));
            assert!(!shown(tester, "entry list"));
            assert_eq!(scaffolds(tester), 0);

            system_back(tester);
            assert_eq!(stack.borrow().len(), 2);
            assert!(shown(tester, "entry list"));
        });
    }

    #[test]
    fn a_wide_window_shows_the_list_and_detail_in_one_page_and_back_leaves_it() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1)]));
            pump_first(tester, Size::new(1000.0, 700.0), Rc::clone(&stack));
            assert!(shown(tester, "list pane"));
            assert!(shown(tester, "detail pane"));
            assert_eq!(scaffolds(tester), 1);
            assert!(!shown(tester, "entry detail"));

            system_back(tester);
            assert_eq!(stack.borrow().len(), 1);
            assert!(shown(tester, "entry home"));
        });
    }

    #[test]
    fn back_that_only_changes_the_panes_stays_in_the_scene_page() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1), extra()]));
            pump_first(tester, Size::new(1000.0, 700.0), Rc::clone(&stack));
            assert!(shown(tester, "extra pane"));

            system_back(tester);
            assert_eq!(stack.borrow().len(), 3);
            assert_eq!(scaffolds(tester), 1);
            assert!(!shown(tester, "extra pane"));
            assert!(shown(tester, "list pane"));
            assert!(shown(tester, "detail pane"));
        });
    }

    #[test]
    fn resizing_across_840_regroups_the_same_stack_without_a_transition() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1)]));
            pump_first(tester, Size::new(839.0, 700.0), Rc::clone(&stack));
            assert_eq!(scaffolds(tester), 0);
            assert!(shown(tester, "entry detail"));

            pump(tester, Size::new(840.0, 700.0), Rc::clone(&stack));
            assert_eq!(
                tester.pump_and_settle(),
                1,
                "the detail's page leaves at once"
            );
            assert_eq!(stack.borrow().len(), 3);
            assert_eq!(scaffolds(tester), 1);
            assert!(shown(tester, "list pane"));
            assert!(!shown(tester, "entry detail"));

            pump(tester, Size::new(839.0, 700.0), Rc::clone(&stack));
            assert_eq!(
                tester.pump_and_settle(),
                1,
                "the detail's page comes at once"
            );
            assert_eq!(stack.borrow().len(), 3);
            assert_eq!(scaffolds(tester), 0);
            assert!(shown(tester, "entry detail"));

            system_back(tester);
            assert_eq!(stack.borrow().len(), 2);
            assert!(shown(tester, "entry list"));
        });
    }
}
