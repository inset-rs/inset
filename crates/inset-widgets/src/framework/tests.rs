//! Framework tests on the shared test root.

use std::cell::Cell;
use std::rc::Rc;

use inset_embedder::Size;
use inset_foundation::{App, AppCell, Entity, Handle};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::{
    AnyRenderObject, BoxConstraints, RenderBox, RenderConstrainedBox, RenderHandle, RenderPadding,
};

use super::*;
use crate::test_harness::{Harness, padding_under_root};

// ---- widgets under test ----

/// `Padding` in miniature.
#[derive(Debug)]
struct Padding {
    key: Option<KeyRef>,
    padding: f64,
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for Padding {
    type RenderObject = RenderPadding;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderPadding::new(app, EdgeInsetsGeometry::all(self.padding), None, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderPadding>,
    ) {
        render_object.set_padding(app, EdgeInsetsGeometry::all(self.padding));
    }
}

impl SingleChildRenderObjectWidget for Padding {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// `SizedBox` in miniature.
#[derive(Debug)]
struct Sized {
    size: Size,
}

impl RenderObjectWidget for Sized {
    type RenderObject = RenderConstrainedBox;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderConstrainedBox::new(app, BoxConstraints::tight(self.size), None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderConstrainedBox>,
    ) {
        render_object.set_additional_constraints(app, BoxConstraints::tight(self.size));
    }
}

impl LeafRenderObjectWidget for Sized {}

/// A stateless widget that counts its builds.
#[derive(Debug)]
struct Labelled {
    builds: Rc<Cell<u32>>,
    padding: f64,
}

impl StatelessWidget for Labelled {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.builds.set(self.builds.get() + 1);
        Padding {
            key: None,
            padding: self.padding,
            child: Some(
                Sized {
                    size: Size::new(10.0, 10.0),
                }
                .into_widget(),
            ),
        }
        .into_widget()
    }
}

/// A stateful counter whose state decides the padding.
#[derive(Debug)]
struct Counter {
    key: Option<KeyRef>,
    initial: f64,
}

impl StatefulWidget for Counter {
    type State = CounterState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CounterState {
        CounterState {
            state: StateData::new(),
            padding: 0.0,
            builds: 0,
            updates: 0,
            disposed: false,
        }
    }
}

struct CounterState {
    state: StateData<Counter>,
    padding: f64,
    builds: u32,
    updates: u32,
    disposed: bool,
}

impl State for CounterState {
    type Widget = Counter;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let initial = self.widget(app).initial;
        app.get_mut(self).padding = initial;
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &Counter) {
        app.get_mut(self).updates += 1;
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        app.get_mut(self).builds += 1;
        Padding {
            key: None,
            padding: app.get(self).padding,
            child: Some(
                Sized {
                    size: Size::new(10.0, 10.0),
                }
                .into_widget(),
            ),
        }
        .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).disposed = true;
    }
}

/// An inherited widget carrying a number.
#[derive(Debug)]
struct Theme {
    value: f64,
    child: WidgetRef,
}

impl InheritedWidget for Theme {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &Theme) -> bool {
        self.value != old_widget.value
    }
}

/// Reads the theme and pads by it.
#[derive(Debug)]
struct Themed {
    builds: Rc<Cell<u32>>,
}

impl StatelessWidget for Themed {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        self.builds.set(self.builds.get() + 1);
        let value = context
            .depend_on_inherited_widget_of_exact_type::<Theme>(app)
            .map_or(0.0, |theme| theme.value);
        Padding {
            key: None,
            padding: value,
            child: Some(
                Sized {
                    size: Size::new(10.0, 10.0),
                }
                .into_widget(),
            ),
        }
        .into_widget()
    }
}

#[test]
fn mounting_builds_the_render_tree_and_lays_it_out() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let builds = Rc::new(Cell::new(0));
    let harness = Harness::mount(
        &mut app,
        Labelled {
            builds: Rc::clone(&builds),
            padding: 4.0,
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    assert_eq!(builds.get(), 1);
    let padding = padding_under_root(&harness, &app);
    assert_eq!(padding.padding(&app), EdgeInsetsGeometry::all(4.0));
    assert_eq!(
        padding.size(&app),
        Size::new(18.0, 18.0),
        "the sized box plus 4 on each side"
    );
    assert_eq!(harness.owner.global_key_count(&app), 0);
}

#[test]
fn a_new_widget_of_the_same_type_updates_the_render_object_in_place() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let builds = Rc::new(Cell::new(0));
    let harness = Harness::mount(
        &mut app,
        Labelled {
            builds: Rc::clone(&builds),
            padding: 4.0,
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    let before = padding_under_root(&harness, &app);

    harness.set_child(
        &mut app,
        Labelled {
            builds: Rc::clone(&builds),
            padding: 6.0,
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    let after = padding_under_root(&harness, &app);
    assert_eq!(after, before, "the same RenderPadding is reconfigured");
    assert_eq!(after.padding(&app), EdgeInsetsGeometry::all(6.0));
    assert_eq!(builds.get(), 2);
}

#[test]
fn set_state_rebuilds_only_after_the_owner_flushes() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let harness = Harness::mount(
        &mut app,
        Counter {
            key: None,
            initial: 2.0,
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    let state = harness.root.as_element().children(&app)[0]
        .state_handle::<CounterState>(&app)
        .expect("the counter's state");
    assert_eq!(app.get(state).builds, 1);
    assert_eq!(
        padding_under_root(&harness, &app).padding(&app),
        EdgeInsetsGeometry::all(2.0)
    );

    state.set_state(&mut app, |state| state.padding = 5.0);
    assert!(state.context(&app).dirty(&app));
    assert_eq!(app.get(state).builds, 1, "not rebuilt until the frame");
    harness.pump(&mut app);
    assert_eq!(app.get(state).builds, 2);
    assert_eq!(
        padding_under_root(&harness, &app).padding(&app),
        EdgeInsetsGeometry::all(5.0)
    );
}

#[test]
fn a_new_stateful_widget_keeps_its_state_and_reports_the_update() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let harness = Harness::mount(
        &mut app,
        Counter {
            key: None,
            initial: 2.0,
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    let state = harness.root.as_element().children(&app)[0]
        .state_handle::<CounterState>(&app)
        .unwrap();
    harness.set_child(
        &mut app,
        Counter {
            key: None,
            initial: 9.0,
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    let same_state = harness.root.as_element().children(&app)[0]
        .state_handle::<CounterState>(&app)
        .unwrap();
    assert_eq!(same_state, state);
    assert_eq!(app.get(state).updates, 1);
    assert_eq!(app.get(state).padding, 2.0, "init_state does not run again");
    assert_eq!(app.get(state).builds, 2);
}

#[test]
fn a_different_widget_type_disposes_the_old_subtree_at_finalize() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let harness = Harness::mount(
        &mut app,
        Counter {
            key: None,
            initial: 2.0,
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    let old_element = harness.root.as_element().children(&app)[0];
    let state = old_element.state_handle::<CounterState>(&app).unwrap();
    let old_padding = padding_under_root(&harness, &app);

    harness.set_child(
        &mut app,
        Sized {
            size: Size::new(5.0, 5.0),
        }
        .into_widget(),
    );
    assert!(harness.owner.debug_inactive_contains(&app, old_element));
    assert_eq!(old_element.lifecycle(&app), ElementLifecycle::Inactive);
    assert!(!app.get(state).disposed, "dispose waits for finalize_tree");
    harness.pump(&mut app);
    assert_eq!(old_element.lifecycle(&app), ElementLifecycle::Defunct);
    assert!(
        !app.contains(state),
        "the state slot is freed after dispose"
    );
    assert!(
        !app.contains(old_padding.as_object().id()),
        "the render object is disposed"
    );
    assert!(
        harness
            .render_root(&app)
            .child(&app)
            .unwrap()
            .as_object()
            .downcast::<RenderConstrainedBox>(&app)
            .is_some()
    );
}

#[test]
fn inherited_widgets_notify_their_dependents_only_when_they_should() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let builds = Rc::new(Cell::new(0));
    // One child instance, reused like a `const` widget, so only the theme decides rebuilds.
    let child: WidgetRef = Themed {
        builds: Rc::clone(&builds),
    }
    .into_widget();
    let themed = |value: f64| {
        Theme {
            value,
            child: child.clone(),
        }
        .into_widget()
    };
    let harness = Harness::mount(&mut app, themed(3.0));
    harness.pump(&mut app);
    assert_eq!(builds.get(), 1);
    assert_eq!(
        padding_under_root(&harness, &app).padding(&app),
        EdgeInsetsGeometry::all(3.0)
    );

    harness.set_child(&mut app, themed(3.0));
    harness.pump(&mut app);
    assert_eq!(builds.get(), 1, "an equal theme does not notify");

    harness.set_child(&mut app, themed(7.0));
    harness.pump(&mut app);
    assert_eq!(builds.get(), 2, "a changed theme rebuilds the dependent");
    assert_eq!(
        padding_under_root(&harness, &app).padding(&app),
        EdgeInsetsGeometry::all(7.0)
    );
}

#[test]
fn a_global_key_moves_an_element_with_its_state() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let key: KeyRef = Rc::new(GlobalKey::new());
    let global = downcast_key(&key);
    let counter = |key: &KeyRef| {
        Counter {
            key: Some(key.clone()),
            initial: 1.0,
        }
        .into_widget()
    };
    let harness = Harness::mount(
        &mut app,
        Padding {
            key: None,
            padding: 1.0,
            child: Some(counter(&key)),
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    assert_eq!(harness.owner.global_key_count(&app), 1);
    let registered = harness
        .owner
        .global_key_element(&app, global.identity())
        .unwrap();
    let state = registered.state_handle::<CounterState>(&app).unwrap();
    state.set_state(&mut app, |state| state.padding = 8.0);
    harness.pump(&mut app);

    // Move the keyed counter under a different parent.
    harness.set_child(
        &mut app,
        Padding {
            key: None,
            padding: 2.0,
            child: Some(
                Padding {
                    key: None,
                    padding: 3.0,
                    child: Some(counter(&key)),
                }
                .into_widget(),
            ),
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    let moved = harness
        .owner
        .global_key_element(&app, global.identity())
        .unwrap();
    assert_eq!(moved, registered, "the same element was reparented");
    assert_eq!(moved.depth(&app), registered.depth(&app));
    assert_eq!(app.get(state).padding, 8.0, "its state survived the move");
    assert!(!app.get(state).disposed);
    assert_eq!(harness.owner.global_key_count(&app), 1);
}

fn downcast_key(key: &KeyRef) -> &GlobalKey {
    ((&**key) as &dyn std::any::Any)
        .downcast_ref::<GlobalKey>()
        .expect("a global key")
}

struct Store {
    count: i32,
}

#[derive(Debug)]
struct StoreView {
    key: Option<KeyRef>,
    store: Entity<Store>,
    builds: Rc<Cell<u32>>,
    last: Rc<Cell<i32>>,
}

impl StatelessWidget for StoreView {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        self.builds.set(self.builds.get() + 1);
        self.last.set(self.store.read(app).count);
        Sized {
            size: Size::new(10.0, 10.0),
        }
        .into_widget()
    }
}

#[test]
fn a_build_that_reads_an_entity_rebuilds_when_it_notifies() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let store = app.new_entity(|_cx| Store { count: 1 });
    let builds = Rc::new(Cell::new(0));
    let last = Rc::new(Cell::new(0));
    let harness = Harness::mount(
        &mut app,
        StoreView {
            key: None,
            store: store.clone(),
            builds: Rc::clone(&builds),
            last: Rc::clone(&last),
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    assert_eq!(builds.get(), 1);
    assert_eq!(last.get(), 1);

    store.update(&mut app, |store, cx| {
        store.count = 4;
        cx.notify();
    });
    harness.pump(&mut app);
    assert_eq!(builds.get(), 2);
    assert_eq!(last.get(), 4);
}

#[derive(Debug)]
struct BlindView {
    builds: Rc<Cell<u32>>,
}

impl StatelessWidget for BlindView {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.builds.set(self.builds.get() + 1);
        Sized {
            size: Size::new(10.0, 10.0),
        }
        .into_widget()
    }
}

#[test]
fn a_read_outside_build_does_not_subscribe() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let store = app.new_entity(|_cx| Store { count: 1 });
    let builds = Rc::new(Cell::new(0));
    let harness = Harness::mount(
        &mut app,
        BlindView {
            builds: Rc::clone(&builds),
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    assert_eq!(builds.get(), 1);

    let _ = store.read(&app).count;
    store.update(&mut app, |_store, cx| cx.notify());
    harness.pump(&mut app);
    assert_eq!(builds.get(), 1);
}

#[test]
fn an_entity_notify_while_inactive_rebuilds_on_activate() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let store = app.new_entity(|_cx| Store { count: 1 });
    let builds = Rc::new(Cell::new(0));
    let last = Rc::new(Cell::new(0));
    let key: KeyRef = Rc::new(GlobalKey::new());
    let view = || {
        StoreView {
            key: Some(key.clone()),
            store: store.clone(),
            builds: Rc::clone(&builds),
            last: Rc::clone(&last),
        }
        .into_widget()
    };
    let harness = Harness::mount(
        &mut app,
        Padding {
            key: None,
            padding: 1.0,
            child: Some(view()),
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    assert_eq!(builds.get(), 1);

    // Park on the inactive list; do not pump, or finalize_tree unmounts it.
    harness.set_child(
        &mut app,
        Sized {
            size: Size::new(10.0, 10.0),
        }
        .into_widget(),
    );
    store.update(&mut app, |store, cx| {
        store.count = 4;
        cx.notify();
    });

    harness.set_child(
        &mut app,
        Padding {
            key: None,
            padding: 1.0,
            child: Some(view()),
        }
        .into_widget(),
    );
    harness.pump(&mut app);
    assert_eq!(builds.get(), 2);
    assert_eq!(last.get(), 4);
}
