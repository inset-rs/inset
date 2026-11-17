//! A Navigator whose pages follow an app's back stack and a three-pane scene strategy.

use std::fmt;
use std::rc::Rc;

use waft_foundation::{App, ValueKey};
use waft_widgets::{
    BuildContext, IntoWidget, KeyRef, Navigator, PageRef, StatelessWidget, WidgetRef,
};

use crate::compose::{ThreePaneScaffoldScene, ThreePaneSceneStrategy};
use crate::page::MaterialPage;

/// Builds one page for `back_stack[index]`.
pub type ThreePaneEntryPageBuilder = Rc<dyn Fn(&mut App, BuildContext, usize) -> WidgetRef>;

/// Builds the one page that holds a scene's [`ThreePaneScaffold`](crate::ThreePaneScaffold).
pub type ThreePaneScenePageBuilder =
    Rc<dyn Fn(&mut App, BuildContext, &ThreePaneScaffoldScene) -> WidgetRef>;

/// Called with the length the back stack should have after a page is removed.
pub type ThreePaneTruncateCallback = Rc<dyn Fn(&mut App, usize)>;

/// A Navigator whose pages follow an app's back stack and a three-pane scene strategy.
///
/// One page is shown for each back-stack entry before the scene, and one page holds the scene
/// itself. With no scene every entry has its own page, so a phone's detail is an ordinary pushed
/// page. A page's key and name are the first back-stack index it shows, so a resize that splits
/// a scene keeps the list's page.
pub struct ThreePaneNavigator<T: PartialEq + 'static> {
    /// The navigator's key.
    pub key: Option<KeyRef>,
    /// Which trailing entries share a page.
    pub strategy: ThreePaneSceneStrategy,
    /// The app's back stack; `None` for an entry that is not a pane.
    pub back_stack: Vec<Option<crate::compose::ThreePaneScaffoldDestinationItem<T>>>,
    entry_page_builder: ThreePaneEntryPageBuilder,
    scene_page_builder: ThreePaneScenePageBuilder,
    on_truncate: Option<ThreePaneTruncateCallback>,
}

impl<T: PartialEq + 'static> ThreePaneNavigator<T> {
    /// Creates a navigator over `back_stack`.
    pub fn new(
        strategy: ThreePaneSceneStrategy,
        back_stack: Vec<Option<crate::compose::ThreePaneScaffoldDestinationItem<T>>>,
    ) -> ThreePaneNavigator<T> {
        ThreePaneNavigator {
            key: None,
            strategy,
            back_stack,
            entry_page_builder: Rc::new(|_app, _context, _index| {
                unreachable!("a ThreePaneNavigator needs an entry page builder")
            }),
            scene_page_builder: Rc::new(|_app, _context, _scene| {
                unreachable!("a ThreePaneNavigator needs a scene page builder")
            }),
            on_truncate: None,
        }
    }

    /// Sets the scaffold's key.
    pub fn key(mut self, key: KeyRef) -> ThreePaneNavigator<T> {
        self.key = Some(key);
        self
    }

    /// Builds the page for `back_stack[index]` when it is a page of its own.
    pub fn entry_page(
        mut self,
        builder: impl Fn(&mut App, BuildContext, usize) -> WidgetRef + 'static,
    ) -> ThreePaneNavigator<T> {
        self.entry_page_builder = Rc::new(builder);
        self
    }

    /// Builds the one page that holds a scene.
    pub fn scene_page(
        mut self,
        builder: impl Fn(&mut App, BuildContext, &ThreePaneScaffoldScene) -> WidgetRef + 'static,
    ) -> ThreePaneNavigator<T> {
        self.scene_page_builder = Rc::new(builder);
        self
    }

    /// Called with the length the back stack should have after a page is removed.
    pub fn on_truncate(
        mut self,
        on_truncate: impl Fn(&mut App, usize) + 'static,
    ) -> ThreePaneNavigator<T> {
        self.on_truncate = Some(Rc::new(on_truncate));
        self
    }

    fn build_entry_page(&self, app: &mut App, context: BuildContext, index: usize) -> PageRef {
        let child = (self.entry_page_builder)(app, context, index);
        Rc::new(
            MaterialPage::new(child)
                .key(ValueKey::new(index).into())
                .name(index.to_string()),
        )
    }

    fn build_scene_page(
        &self,
        app: &mut App,
        context: BuildContext,
        scene: &ThreePaneScaffoldScene,
    ) -> PageRef {
        let child = (self.scene_page_builder)(app, context, scene);
        let start = scene.entries.start;
        let len_after_back = scene.back_stack_len_after_back;
        let on_truncate = self.on_truncate.clone();
        let mut page = MaterialPage::new(child)
            .key(ValueKey::new(start).into())
            .name(start.to_string())
            .can_pop(scene.previous_value.is_none());
        if scene.previous_value.is_some() {
            page = page.on_pop_invoked(Rc::new(move |app, did_pop, _result| {
                if !did_pop && let Some(on_truncate) = &on_truncate {
                    on_truncate(app, len_after_back);
                }
            }));
        }
        Rc::new(page)
    }
}

impl<T: PartialEq + 'static> fmt::Debug for ThreePaneNavigator<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreePaneNavigator")
            .field("key", &self.key)
            .field("strategy", &self.strategy)
            .finish_non_exhaustive()
    }
}

impl<T: PartialEq + 'static> StatelessWidget for ThreePaneNavigator<T> {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let scene = self.strategy.calculate_scene(&self.back_stack);
        let mut pages: Vec<PageRef> = Vec::new();
        match &scene {
            None => {
                for index in 0..self.back_stack.len() {
                    pages.push(self.build_entry_page(app, context, index));
                }
            }
            Some(scene) => {
                for index in 0..scene.entries.start {
                    pages.push(self.build_entry_page(app, context, index));
                }
                pages.push(self.build_scene_page(app, context, scene));
            }
        }

        let on_truncate = self.on_truncate.clone();
        let mut navigator = Navigator::new().pages(pages);
        if let Some(key) = &self.key {
            navigator = navigator.key(key.clone());
        }
        navigator
            .on_did_remove_page(move |app, page| {
                if let Some(on_truncate) = &on_truncate
                    && let Some(index) = page.name().and_then(|name| name.parse::<usize>().ok())
                {
                    on_truncate(app, index);
                }
            })
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use waft_embedder::{Size, TextDirection};
    use waft_foundation::Handle;
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{
        Directionality, GlobalKey, MediaQuery, MediaQueryData, NavigatorState, StateSetter,
        StatefulBuilder, Text,
    };

    use super::*;
    use crate::compose::{
        ListDetailPaneScaffoldRole, Pane, PaneScaffoldDirective, ThreePaneScaffold,
        ThreePaneScaffoldDestinationItem, WindowSizeClass,
    };

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

    fn host(
        stack: Stack,
        navigator_key: GlobalKey,
    ) -> impl Fn(&mut App, BuildContext, StateSetter) -> WidgetRef {
        move |app: &mut App, context: BuildContext, set_state: StateSetter| {
            let size_class = WindowSizeClass::of(app, context);
            let strategy = ThreePaneSceneStrategy::list_detail(
                PaneScaffoldDirective::from_window_size_class(size_class),
            );
            let stack_for_pages = Rc::clone(&stack);
            let stack_for_truncate = Rc::clone(&stack);
            ThreePaneNavigator::new(strategy.clone(), stack.borrow().clone())
                .key(navigator_key.clone().into())
                .entry_page(move |_app, _context, index| entry_widget(&stack_for_pages, index))
                .scene_page(move |_app, _context, scene| scene_widget(&strategy, scene))
                .on_truncate(move |app, len| {
                    stack_for_truncate.borrow_mut().truncate(len);
                    set_state(app, &mut |_app| {});
                })
                .into_widget()
        }
    }

    fn pump(tester: &mut WidgetTester, size: Size, stack: Stack, key: &GlobalKey) {
        tester.pump_widget(
            MediaQuery::new(
                MediaQueryData::new().size(size),
                Directionality::new(
                    TextDirection::Ltr,
                    StatefulBuilder::new({
                        let stack = Rc::clone(&stack);
                        let key = key.clone();
                        host(stack, key)
                    }),
                ),
            )
            .into_widget(),
        );
    }

    fn shown(tester: &WidgetTester, text: &str) -> bool {
        !find::text(text).evaluate(tester).is_empty()
    }

    fn navigator(tester: &WidgetTester, key: &GlobalKey) -> Handle<NavigatorState> {
        key.current_state::<NavigatorState>(&tester.cell.borrow())
            .expect("the navigator is mounted")
    }

    #[test]
    fn a_phone_shows_the_detail_as_its_own_page_and_back_pops_it() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1)]));
            let key = GlobalKey::new();
            pump(tester, Size::new(400.0, 700.0), Rc::clone(&stack), &key);
            assert!(shown(tester, "entry detail"));
            assert!(!shown(tester, "entry list"));

            let navigator = navigator(tester, &key);
            tester.update(|app| {
                navigator.maybe_pop(app, None);
            });
            tester.pump_and_settle();
            assert_eq!(stack.borrow().len(), 2);
            assert!(shown(tester, "entry list"));
        });
    }

    #[test]
    fn a_wide_window_shows_the_list_and_detail_in_one_page() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1)]));
            let key = GlobalKey::new();
            pump(tester, Size::new(1000.0, 700.0), Rc::clone(&stack), &key);
            assert!(shown(tester, "list pane"));
            assert!(shown(tester, "detail pane"));
            assert_eq!(
                find::by_type::<ThreePaneScaffold>().evaluate(tester).len(),
                1
            );
            assert!(!shown(tester, "entry detail"));
        });
    }

    #[test]
    fn back_on_a_wide_scene_page_leaves_the_inbox() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1)]));
            let key = GlobalKey::new();
            pump(tester, Size::new(1000.0, 700.0), Rc::clone(&stack), &key);
            let navigator = navigator(tester, &key);
            tester.update(|app| {
                navigator.maybe_pop(app, None);
            });
            tester.pump_and_settle();
            assert_eq!(stack.borrow().len(), 1);
            assert!(shown(tester, "entry home"));
        });
    }

    #[test]
    fn back_that_only_changes_the_panes_keeps_the_scene_page() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1), extra()]));
            let key = GlobalKey::new();
            pump(tester, Size::new(1000.0, 700.0), Rc::clone(&stack), &key);
            assert!(shown(tester, "extra pane"));

            let navigator = navigator(tester, &key);
            tester.update(|app| {
                navigator.maybe_pop(app, None);
            });
            tester.pump_and_settle();
            assert_eq!(stack.borrow().len(), 3);
            assert_eq!(
                find::by_type::<ThreePaneScaffold>().evaluate(tester).len(),
                1
            );
            assert!(!shown(tester, "extra pane"));
            assert!(shown(tester, "list pane"));
        });
    }

    #[test]
    fn resizing_across_the_breakpoint_regroups_the_pages() {
        test_widgets(|tester| {
            let stack: Stack = Rc::new(RefCell::new(vec![home(), list(), detail(1)]));
            let key = GlobalKey::new();
            pump(tester, Size::new(400.0, 700.0), Rc::clone(&stack), &key);
            assert_eq!(
                find::by_type::<ThreePaneScaffold>().evaluate(tester).len(),
                0
            );
            assert!(shown(tester, "entry detail"));

            pump(tester, Size::new(1000.0, 700.0), Rc::clone(&stack), &key);
            tester.pump_and_settle();
            assert_eq!(
                find::by_type::<ThreePaneScaffold>().evaluate(tester).len(),
                1
            );
            assert!(shown(tester, "list pane"));
            assert!(shown(tester, "detail pane"));

            pump(tester, Size::new(400.0, 700.0), Rc::clone(&stack), &key);
            tester.pump_and_settle();
            assert_eq!(
                find::by_type::<ThreePaneScaffold>().evaluate(tester).len(),
                0
            );
            assert!(shown(tester, "entry detail"));
        });
    }

    #[test]
    fn a_scene_that_shows_one_pane_is_left_to_pages() {
        let strategy = ThreePaneSceneStrategy::list_detail(
            PaneScaffoldDirective::from_window_size_class(WindowSizeClass {
                width: crate::compose::WindowWidthSizeClass::Compact,
                height: crate::compose::WindowHeightSizeClass::Expanded,
            }),
        );
        let stack = vec![list(), detail(1)];
        assert_eq!(strategy.calculate_scene(&stack), None);
    }
}
