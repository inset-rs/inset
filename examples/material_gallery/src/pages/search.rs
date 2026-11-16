//! Search: a `SearchBar` on its own, a `SearchAnchor.bar` whose view lists suggestions, and a
//! button that opens `showSearch` with a `SearchDelegate`.
//!
//! A caption under each says what the last interaction chose.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use waft_foundation::{App, CompleterFuture, Handle, Listener};
use waft_material::{
    FilledButton, Icons, ListTile, SearchAnchor, SearchBar, SearchController, SearchDelegate,
    SearchDelegateData, show_search,
};
use waft_widgets::{
    BuildContext, Icon, IntoWidget, ListView, State, StateData, StatefulWidget, Text,
    TextEditingControllerLeaf as _, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

const FRUITS: [&str; 6] = ["Apple", "Apricot", "Banana", "Blueberry", "Cherry", "Grape"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Search.title(), SearchDemo.into_widget()).into_widget()
}

/// What each search chose lives here, in the smallest widget that renders it.
#[derive(Debug)]
pub struct SearchDemo;

/// Dart would spell this `_SearchDemoState`.
pub struct SearchDemoState {
    state: StateData<SearchDemo>,
    bar_text: String,
    anchor_choice: Option<String>,
    delegate_result: Option<String>,
}

impl StatefulWidget for SearchDemo {
    type State = SearchDemoState;

    fn create_state(&self) -> SearchDemoState {
        SearchDemoState {
            state: StateData::new(),
            bar_text: String::new(),
            anchor_choice: None,
            delegate_result: None,
        }
    }
}

impl State for SearchDemoState {
    type Widget = SearchDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (bar_text, anchor_choice, delegate_result) = {
            let state = app.get(self);
            (
                state.bar_text.clone(),
                state.anchor_choice.clone(),
                state.delegate_result.clone(),
            )
        };
        ListView::new()
            .children(vec![
                section_header(app, context, "Search bar"),
                caption(app, context, format!("Typed: {bar_text}")),
                showcase(vec![
                    SearchBar::new()
                        .hint_text("Search fruit")
                        .leading(Icon::new(Some(Icons::search())))
                        .on_changed(move |app, text| {
                            self.set_state(app, |state| state.bar_text = text);
                        })
                        .into_widget(),
                ]),
                section_header(app, context, "Search anchor"),
                caption(
                    app,
                    context,
                    format!("Chose: {}", anchor_choice.as_deref().unwrap_or("\u{2014}")),
                ),
                showcase(vec![anchor_with_bar(self)]),
                section_header(app, context, "Search delegate"),
                caption(
                    app,
                    context,
                    format!(
                        "Result: {}",
                        delegate_result.as_deref().unwrap_or("\u{2014}")
                    ),
                ),
                showcase(vec![delegate_button(self, context)]),
            ])
            .into_widget()
    }
}

/// A search bar whose tap opens a view of the fruits that match what was typed.
fn anchor_with_bar(state: Handle<SearchDemoState>) -> WidgetRef {
    SearchAnchor::bar(move |app, context, controller: Handle<SearchController>| {
        let query = controller.text_value(app).to_lowercase();
        let suggestions = FRUITS
            .iter()
            .filter(|fruit| fruit.to_lowercase().contains(&query))
            .map(|fruit| {
                let fruit = *fruit;
                ListTile::new()
                    .title(Text::new(fruit))
                    .on_tap(Listener::new(move |app: &mut App| {
                        controller.close_view(app, Some(fruit.to_owned()));
                        state.set_state(app, |demo| demo.anchor_choice = Some(fruit.to_owned()));
                    }))
                    .into_widget()
            })
            .collect();
        let _ = context;
        CompleterFuture::ready(suggestions)
    })
    .bar_hint_text("Pick a fruit")
    .into_search_anchor()
    .into_widget()
}

/// A button that opens `show_search` and writes what the search page closed with.
fn delegate_button(state: Handle<SearchDemoState>, context: BuildContext) -> WidgetRef {
    FilledButton::tonal(
        Some(Listener::new(move |app: &mut App| {
            let delegate = FruitDelegate::new(app);
            let result = show_search(app, context, delegate, None, false, true);
            result.then(app, move |app, route_result| {
                let chosen = route_result
                    .as_ref()
                    .and_then(|value| value.downcast_ref::<String>().cloned());
                state.set_state(app, |demo| demo.delegate_result = chosen);
            });
        })),
        Some(Text::new("Open search")),
    )
    .into_widget()
}

/// A delegate whose suggestions are the fruits matching the query and whose results are the
/// query's one chosen fruit.
struct FruitDelegate {
    data: SearchDelegateData,
    me: RefCell<Weak<FruitDelegate>>,
}

impl FruitDelegate {
    fn new(app: &mut App) -> Rc<FruitDelegate> {
        let delegate = Rc::new(FruitDelegate {
            data: SearchDelegateData::new(app),
            me: RefCell::new(Weak::new()),
        });
        *delegate.me.borrow_mut() = Rc::downgrade(&delegate);
        delegate
    }

    fn this(&self) -> Rc<FruitDelegate> {
        self.me.borrow().upgrade().expect("the delegate is alive")
    }
}

impl SearchDelegate for FruitDelegate {
    fn search_delegate_data(&self) -> &SearchDelegateData {
        &self.data
    }

    fn build_leading(&self, app: &mut App, context: BuildContext) -> Option<WidgetRef> {
        let delegate = self.this();
        let _ = app;
        Some(
            waft_material::IconButton::new(
                Some(Listener::new(move |app: &mut App| {
                    delegate.close(app, context, None);
                })),
                Icon::new(Some(Icons::arrow_back())),
            )
            .tooltip("Back")
            .into_widget(),
        )
    }

    fn build_actions(&self, _app: &mut App, _context: BuildContext) -> Option<Vec<WidgetRef>> {
        Some(Vec::new())
    }

    fn build_suggestions(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let query = self.query(app).to_lowercase();
        let delegate = self.this();
        let tiles: Vec<WidgetRef> = FRUITS
            .iter()
            .filter(|fruit| fruit.to_lowercase().contains(&query))
            .map(|fruit| {
                let fruit = *fruit;
                let delegate = Rc::clone(&delegate);
                ListTile::new()
                    .leading(Icon::new(Some(Icons::history())))
                    .title(Text::new(fruit))
                    .on_tap(Listener::new(move |app: &mut App| {
                        delegate.set_query(app, fruit.to_owned());
                        delegate.show_results(app, context);
                    }))
                    .into_widget()
            })
            .collect();
        ListView::new().children(tiles).into_widget()
    }

    fn build_results(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let query = self.query(app);
        let delegate = self.this();
        ListView::new()
            .children(vec![
                ListTile::new()
                    .title(Text::new(format!("Result for {query}")))
                    .on_tap(Listener::new(move |app: &mut App| {
                        let query = delegate.query(app);
                        delegate.close(app, context, Some(Rc::new(query)));
                    }))
                    .into_widget(),
            ])
            .into_widget()
    }
}
