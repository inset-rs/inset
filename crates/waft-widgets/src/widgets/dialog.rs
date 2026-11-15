//! Flutter counterpart: `widgets/dialog.dart`.

use std::rc::Rc;

use waft_foundation::{App, CompleterFuture};

use crate::framework::BuildContext;
use crate::widgets::basic::WidgetBuilder;
use crate::widgets::debug::debug_check_has_widgets_localizations;
use crate::widgets::navigator::{AnyRoute, Navigator, Route, RouteResult, RouteSettingsRef};
use crate::widgets::routes::{RawDialogRoute, RoutePageBuilder};

/// Signature for a function that creates a [`Route`] for a dialog.
///
/// Used by [`show_raw_dialog`] to allow customization of the route.
pub type RawDialogRouteBuilder = Rc<dyn Fn(&mut App, BuildContext, WidgetBuilder) -> AnyRoute>;

/// Displays a dialog above the current contents of the app.
///
/// This function allows for customization of the dialog route through `route_builder`. If
/// `route_builder` is not provided, a [`RawDialogRoute`] is used.
///
/// The `context` argument is used to look up the [`Navigator`] for the dialog. It is only used
/// when the method is called. Its corresponding widget can be safely removed from the tree
/// before the dialog is closed.
///
/// The `use_root_navigator` argument is used to determine whether to push the dialog to the
/// [`Navigator`] furthest from or nearest to the given `context`. By default,
/// `use_root_navigator` is `true` and the dialog route created by this method is pushed to the
/// root navigator.
///
/// The `route_settings` argument is passed to the route created by this method when no
/// `route_builder` is given.
///
/// Returns a future that resolves to the value (if any) that was passed to [`Navigator::pop`]
/// when the dialog was closed.
pub fn show_raw_dialog(
    app: &mut App,
    context: BuildContext,
    builder: WidgetBuilder,
    route_builder: Option<RawDialogRouteBuilder>,
    use_root_navigator: bool,
    route_settings: Option<RouteSettingsRef>,
    fullscreen_dialog: bool,
) -> CompleterFuture<RouteResult> {
    debug_assert!(debug_check_has_widgets_localizations(app, context));

    let navigator = Navigator::of(app, context, use_root_navigator);

    let route = match route_builder {
        Some(route_builder) => route_builder(app, context, builder),
        None => {
            let page_builder: RoutePageBuilder =
                Rc::new(move |app, context, _animation, _secondary_animation| {
                    builder(app, context)
                });
            let route =
                RawDialogRoute::new(app, page_builder).fullscreen_dialog(app, fullscreen_dialog);
            if let Some(route_settings) = route_settings {
                route.settings(app, route_settings);
            }
            route.as_route()
        }
    };

    navigator.push(app, route)
}

#[cfg(test)]
mod tests {
    //! Flutter has no `widgets/dialog_test.dart`; these pin what `show_raw_dialog` hands the
    //! navigator.

    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use waft_embedder::{Locale, TargetPlatform, TextDirection};
    use waft_foundation::{AppCell, Handle};
    use waft_test::{TestPlatform, TestView};

    use super::*;
    use crate::binding::run_app;
    use crate::framework::{GlobalKey, IntoWidget, WidgetRef};
    use crate::test_harness::pump_every_view;
    use crate::widgets::basic::{Directionality, SizedBox};
    use crate::widgets::localizations::{DefaultWidgetsLocalizations, Localizations};
    use crate::widgets::navigator::{NavigatorState, RouteSettings};
    use crate::widgets::pages::PageRouteBuilder;
    use crate::widgets::routes::AnyModalRoute;

    fn app_with_view() -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::new(TestView::new(300.0, 200.0)));
        AppCell::with_platform(Rc::new(platform))
    }

    /// Frames until the routes' transitions have settled.
    fn settle(cell: &AppCell) {
        let mut at = Duration::ZERO;
        for _ in 0..40 {
            at += Duration::from_millis(20);
            cell.elapse(Duration::from_millis(20));
            pump_every_view(&mut cell.borrow_mut(), at);
            cell.checkpoint();
        }
    }

    /// A root navigator keyed `root` whose page is a nested navigator keyed `nested`, whose
    /// page saves its context into `page`.
    fn mount_nested(
        cell: &AppCell,
        root: &GlobalKey,
        nested: &GlobalKey,
        page: Rc<Cell<Option<BuildContext>>>,
    ) {
        let nested = nested.clone();
        let page_route = move |page: Rc<Cell<Option<BuildContext>>>| -> RoutePageBuilder {
            Rc::new(move |_app, context, _animation, _secondary| {
                page.set(Some(context));
                SizedBox::expand().into_widget()
            })
        };
        let root_navigator =
            Navigator::new()
                .key(root.clone().into())
                .on_generate_route(move |app, _settings| {
                    let nested = nested.clone();
                    let page = Rc::clone(&page);
                    let page_route = page_route;
                    let builder: RoutePageBuilder = Rc::new(move |_app, _context, _a, _s| {
                        let page = Rc::clone(&page);
                        let page_route = page_route;
                        Navigator::new()
                            .key(nested.clone().into())
                            .on_generate_route(move |app, _settings| {
                                Some(
                                    PageRouteBuilder::new(app, page_route(Rc::clone(&page)))
                                        .as_route(),
                                )
                            })
                            .into_widget()
                    });
                    Some(PageRouteBuilder::new(app, builder).as_route())
                });
        let tree: WidgetRef = Localizations::new(
            Locale::new("en"),
            vec![DefaultWidgetsLocalizations::delegate()],
        )
        .child(Directionality::new(TextDirection::Ltr, root_navigator))
        .into_widget();
        run_app(&mut cell.borrow_mut(), tree);
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
    }

    /// Whether `route` is a [`RawDialogRoute`] on the navigator keyed `key`.
    fn is_dialog_on(cell: &AppCell, route: AnyModalRoute, key: &GlobalKey) -> bool {
        let app = cell.borrow();
        let navigator = key
            .current_state::<NavigatorState>(&app)
            .expect("a mounted navigator");
        let route = route.as_route();
        route.downcast::<RawDialogRoute>(&app).is_some() && route.navigator(&app) == Some(navigator)
    }

    /// The modal route the dialog's content was built in, read where `builder` ran.
    fn dialog_content(seen: Rc<Cell<Option<AnyModalRoute>>>) -> WidgetBuilder {
        Rc::new(move |app, context| {
            seen.set(AnyModalRoute::of(app, context));
            SizedBox::new().width(10.0).height(10.0).into_widget()
        })
    }

    #[test]
    fn show_raw_dialog_pushes_a_raw_dialog_route_on_the_root_navigator_and_resolves_with_the_popped_value()
     {
        let cell = app_with_view();
        let (root, nested) = (GlobalKey::new(), GlobalKey::new());
        let page = Rc::new(Cell::new(None));
        mount_nested(&cell, &root, &nested, Rc::clone(&page));
        let context = page.get().expect("the nested page is built");

        let seen = Rc::new(Cell::new(None));
        let result: Rc<RefCell<Option<RouteResult>>> = Rc::default();
        {
            let mut app = cell.borrow_mut();
            let settings: RouteSettingsRef = RouteSettings::new().name("dialog".to_string()).into();
            let popped = show_raw_dialog(
                &mut app,
                context,
                dialog_content(Rc::clone(&seen)),
                None,
                true,
                Some(settings),
                true,
            );
            let out = Rc::clone(&result);
            popped.then(&mut app, move |_app, value| *out.borrow_mut() = Some(value));
        }
        settle(&cell);

        let route = seen.get().expect("the dialog's content was built");
        assert!(is_dialog_on(&cell, route, &root));
        {
            let app = cell.borrow();
            assert!(route.fullscreen_dialog(&app));
            assert_eq!(route.as_route().settings(&app).name(), Some("dialog"));
        }

        {
            let mut app = cell.borrow_mut();
            let navigator: Handle<NavigatorState> = root
                .current_state::<NavigatorState>(&app)
                .expect("a mounted navigator");
            navigator.pop(&mut app, Some(Rc::new(42_i32)));
        }
        settle(&cell);

        let value = result.borrow_mut().take().expect("the dialog was popped");
        assert_eq!(
            value.and_then(|value| value.downcast_ref::<i32>().copied()),
            Some(42)
        );
        assert!(!route.as_route().is_active(&cell.borrow()));
    }

    #[test]
    fn show_raw_dialog_pushes_on_the_nearest_navigator_without_the_root_navigator() {
        let cell = app_with_view();
        let (root, nested) = (GlobalKey::new(), GlobalKey::new());
        let page = Rc::new(Cell::new(None));
        mount_nested(&cell, &root, &nested, Rc::clone(&page));
        let context = page.get().expect("the nested page is built");

        let seen = Rc::new(Cell::new(None));
        {
            let mut app = cell.borrow_mut();
            drop(show_raw_dialog(
                &mut app,
                context,
                dialog_content(Rc::clone(&seen)),
                None,
                false,
                None,
                false,
            ));
        }
        settle(&cell);

        let route = seen.get().expect("the dialog's content was built");
        assert!(is_dialog_on(&cell, route, &nested));
        assert!(!is_dialog_on(&cell, route, &root));
    }

    #[test]
    fn show_raw_dialog_pushes_the_route_its_route_builder_makes() {
        let cell = app_with_view();
        let (root, nested) = (GlobalKey::new(), GlobalKey::new());
        let page = Rc::new(Cell::new(None));
        mount_nested(&cell, &root, &nested, Rc::clone(&page));
        let context = page.get().expect("the nested page is built");

        let handed: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let seen = Rc::new(Cell::new(None));
        {
            let mut app = cell.borrow_mut();
            let got = Rc::clone(&handed);
            let route_builder: RawDialogRouteBuilder = Rc::new(move |app, context, builder| {
                got.set(Some(context));
                let page_builder: RoutePageBuilder =
                    Rc::new(move |app, context, _animation, _secondary| builder(app, context));
                PageRouteBuilder::new(app, page_builder).as_route()
            });
            drop(show_raw_dialog(
                &mut app,
                context,
                dialog_content(Rc::clone(&seen)),
                Some(route_builder),
                true,
                None,
                false,
            ));
        }
        settle(&cell);

        assert_eq!(handed.get(), Some(context));
        let route = seen.get().expect("the dialog's content was built");
        let app = cell.borrow();
        assert!(
            route
                .as_route()
                .downcast::<PageRouteBuilder>(&app)
                .is_some()
        );
    }
}
