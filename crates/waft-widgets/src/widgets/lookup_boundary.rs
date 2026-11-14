//! Flutter counterpart: `widgets/lookup_boundary.dart`.

use std::any::{Any, TypeId};
use std::rc::Rc;

use waft_foundation::{App, Handle};
use waft_rendering::{RenderHandle, RenderObject};

use crate::framework::{
    AnyElement, BuildContext, InheritedWidget, IntoWidget, KeyRef, State, WidgetRef,
    downcast_widget,
};

/// A lookup boundary controls what entities are visible to descendants of the boundary via
/// the static lookup methods provided by the boundary.
///
/// The static lookup methods of the boundary mirror the lookup methods by the same name
/// exposed on [`BuildContext`] and they can be used as direct replacements. Unlike the
/// methods on [`BuildContext`], these methods do not find any ancestor entities of the
/// closest [`LookupBoundary`] surrounding the provided [`BuildContext`]. The root of the tree
/// is an implicit lookup boundary.
///
/// A [`LookupBoundary`] only affects the behavior of the static lookup methods defined on
/// the boundary. It does not affect the behavior of the lookup methods defined on
/// [`BuildContext`].
///
/// A [`LookupBoundary`] is rarely instantiated directly. They are inserted at locations of
/// the widget tree where the render tree diverges from the element tree, which is rather
/// uncommon. Such anomalies are created by `RenderObjectElement`s that don't attach their
/// render object to the closest ancestor render object, e.g. because they bootstrap a
/// separate stand-alone render tree. This behavior breaks the assumption some widgets have
/// about the structure of the render tree: These widgets may try to reach out to an ancestor
/// widget, assuming that their associated render objects are also ancestors, which due to
/// the anomaly may not be the case. At the point where the divergence in the two trees is
/// introduced, a [`LookupBoundary`] can be used to hide that ancestor from the querying
/// widget.
///
/// As an example, `Material.of` relies on lookup boundaries to hide the `Material` widget
/// from certain descendant button widget. Buttons reach out to their `Material` ancestor to
/// draw ink splashes on its associated render object. This only produces the desired effect
/// if the button render object is a descendant of the `Material` render object. If the
/// element tree and the render tree are not in sync due to anomalies described above, this
/// may not be the case. To avoid incorrect visuals, the `Material` relies on lookup
/// boundaries to hide itself from descendants in subtrees with such anomalies. Those
/// subtrees are expected to introduce their own `Material` widget that buttons there can
/// utilize without crossing a lookup boundary.
#[derive(Debug)]
pub struct LookupBoundary {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl LookupBoundary {
    /// Creates a [`LookupBoundary`].
    ///
    /// A none-`None` `child` widget must be provided.
    pub fn new<K>(child: impl IntoWidget<K>) -> LookupBoundary {
        LookupBoundary {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `LookupBoundary(key:)`.
    pub fn key(mut self, key: KeyRef) -> LookupBoundary {
        self.key = Some(key);
        self
    }

    /// Obtains the nearest widget of the given type `W` within the current [`LookupBoundary`]
    /// of `context`, which must be the type of a concrete [`InheritedWidget`] subclass, and
    /// registers the provided build `context` with that widget such that when that widget
    /// changes (or a new widget of that type is introduced, or the widget goes away), the
    /// build context is rebuilt so that it can obtain new values from that widget.
    ///
    /// This method behaves exactly like
    /// [`BuildContext::depend_on_inherited_widget_of_exact_type`], except it only considers
    /// [`InheritedWidget`]s of the specified type `W` between the provided [`BuildContext`]
    /// and its closest [`LookupBoundary`] ancestor. [`InheritedWidget`]s past that
    /// [`LookupBoundary`] are invisible to this method. The root of the tree is treated as an
    /// implicit lookup boundary.
    pub fn depend_on_inherited_widget_of_exact_type<W: InheritedWidget>(
        app: &mut App,
        context: BuildContext,
    ) -> Option<&W> {
        LookupBoundary::depend_on_inherited_widget_of_exact_type_with_aspect::<W>(
            app, context, None,
        )
    }

    /// [`depend_on_inherited_widget_of_exact_type`](Self::depend_on_inherited_widget_of_exact_type)
    /// with Dart's `aspect` argument, for `InheritedModel`.
    pub fn depend_on_inherited_widget_of_exact_type_with_aspect<W: InheritedWidget>(
        app: &mut App,
        context: BuildContext,
        aspect: Option<Rc<dyn Any>>,
    ) -> Option<&W> {
        // The following call makes sure that context depends on something so
        // Element.didChangeDependencies is called when context moves in the tree
        // even when requested dependency remains unfulfilled (i.e. null is
        // returned).
        context.depend_on_inherited_widget_of_exact_type::<LookupBoundary>(app);
        let candidate =
            LookupBoundary::get_element_for_inherited_widget_of_exact_type::<W>(app, context)?;
        context.depend_on_inherited_element(app, candidate, aspect);
        downcast_widget::<W>(&**candidate.widget(app))
    }

    /// Obtains the nearest widget of the given type `W` within the current [`LookupBoundary`]
    /// of `context`, which must be the type of a concrete [`InheritedWidget`] subclass, and
    /// returns `None` if no such widget exists.
    ///
    /// This method behaves exactly like
    /// [`BuildContext::get_inherited_widget_of_exact_type`], except it only considers
    /// [`InheritedWidget`]s of the specified type `W` between the provided [`BuildContext`]
    /// and its closest [`LookupBoundary`] ancestor. [`InheritedWidget`]s past that
    /// [`LookupBoundary`] are invisible to this method. The root of the tree is treated as an
    /// implicit lookup boundary.
    pub fn get_inherited_widget_of_exact_type<W: InheritedWidget>(
        app: &App,
        context: BuildContext,
    ) -> Option<&W> {
        let candidate =
            LookupBoundary::get_element_for_inherited_widget_of_exact_type::<W>(app, context)?;
        downcast_widget::<W>(&**candidate.widget(app))
    }

    /// Obtains the element corresponding to the nearest widget of the given type `W` within
    /// the current [`LookupBoundary`] of `context`.
    ///
    /// `W` must be the type of a concrete [`InheritedWidget`] subclass. Returns `None` if no
    /// such element is found.
    ///
    /// This method behaves exactly like
    /// [`BuildContext::get_element_for_inherited_widget_of_exact_type`], except it only
    /// considers [`InheritedWidget`]s of the specified type `W` between the provided
    /// [`BuildContext`] and its closest [`LookupBoundary`] ancestor. [`InheritedWidget`]s
    /// past that [`LookupBoundary`] are invisible to this method. The root of the tree is
    /// treated as an implicit lookup boundary.
    pub fn get_element_for_inherited_widget_of_exact_type<W: InheritedWidget>(
        app: &App,
        context: BuildContext,
    ) -> Option<AnyElement> {
        let candidate = context.get_element_for_inherited_widget_of_exact_type::<W>(app)?;
        let boundary =
            context.get_element_for_inherited_widget_of_exact_type::<LookupBoundary>(app);
        if boundary.is_some_and(|boundary| boundary.depth(app) > candidate.depth(app)) {
            return None;
        }
        Some(candidate)
    }

    /// Returns the nearest ancestor widget of the given type `W` within the current
    /// [`LookupBoundary`] of `context`.
    ///
    /// `W` must be the type of a concrete widget subclass.
    ///
    /// This method behaves exactly like [`BuildContext::find_ancestor_widget_of_exact_type`],
    /// except it only considers widgets of the specified type `W` between the provided
    /// [`BuildContext`] and its closest [`LookupBoundary`] ancestor. Widgets past that
    /// [`LookupBoundary`] are invisible to this method. The root of the tree is treated as an
    /// implicit lookup boundary.
    pub fn find_ancestor_widget_of_exact_type<W: 'static>(
        app: &App,
        context: BuildContext,
    ) -> Option<&W> {
        let mut target = None;
        context.visit_ancestor_elements(app, &mut |ancestor| {
            if ancestor.widget(app).widget_type() == TypeId::of::<W>() {
                target = Some(ancestor);
                return false;
            }
            !is_lookup_boundary(app, ancestor)
        });
        target.and_then(|target| downcast_widget::<W>(&**target.widget(app)))
    }

    /// Returns the [`State`] object of the nearest ancestor `StatefulWidget` widget within
    /// the current [`LookupBoundary`] of `context` that is an instance of the given type `S`.
    ///
    /// This method behaves exactly like [`BuildContext::find_ancestor_state_of_type`], except
    /// it only considers [`State`] objects of the specified type `S` between the provided
    /// [`BuildContext`] and its closest [`LookupBoundary`] ancestor. [`State`] objects past
    /// that [`LookupBoundary`] are invisible to this method. The root of the tree is treated
    /// as an implicit lookup boundary.
    pub fn find_ancestor_state_of_type<S: State>(
        app: &App,
        context: BuildContext,
    ) -> Option<Handle<S>> {
        let mut target = None;
        context.visit_ancestor_elements(app, &mut |ancestor| {
            if let Some(state) = ancestor.state_handle::<S>(app) {
                target = Some(state);
                return false;
            }
            !is_lookup_boundary(app, ancestor)
        });
        target
    }

    /// Returns the [`State`] object of the furthest ancestor `StatefulWidget` widget within
    /// the current [`LookupBoundary`] of `context` that is an instance of the given type `S`.
    ///
    /// This method behaves exactly like [`BuildContext::find_root_ancestor_state_of_type`],
    /// except it considers the closest [`LookupBoundary`] ancestor of `context` to be the
    /// root. [`State`] objects past that [`LookupBoundary`] are invisible to this method. The
    /// root of the tree is treated as an implicit lookup boundary.
    pub fn find_root_ancestor_state_of_type<S: State>(
        app: &App,
        context: BuildContext,
    ) -> Option<Handle<S>> {
        let mut target = None;
        context.visit_ancestor_elements(app, &mut |ancestor| {
            if let Some(state) = ancestor.state_handle::<S>(app) {
                target = Some(state);
            }
            !is_lookup_boundary(app, ancestor)
        });
        target
    }

    /// Returns the `RenderObject` object of the nearest ancestor `RenderObjectWidget` widget
    /// within the current [`LookupBoundary`] of `context` that is an instance of the given
    /// type `R`.
    ///
    /// This method behaves exactly like
    /// [`BuildContext::find_ancestor_render_object_of_type`], except it only considers
    /// `RenderObject`s of the specified type `R` between the provided [`BuildContext`] and
    /// its closest [`LookupBoundary`] ancestor. `RenderObject`s past that [`LookupBoundary`]
    /// are invisible to this method. The root of the tree is treated as an implicit lookup
    /// boundary.
    pub fn find_ancestor_render_object_of_type<R: RenderObject>(
        app: &App,
        context: BuildContext,
    ) -> Option<RenderHandle<R>> {
        let mut target = None;
        context.visit_ancestor_elements(app, &mut |ancestor| {
            if ancestor.is_render_object_element()
                && let Some(render_object) = ancestor.render_object(app)
                && let Some(typed) = render_object.downcast::<R>(app)
            {
                target = Some(typed);
                return false;
            }
            !is_lookup_boundary(app, ancestor)
        });
        target
    }

    /// Walks the ancestor chain, starting with the parent of the build context's widget,
    /// invoking the argument for each ancestor until a [`LookupBoundary`] or the root is
    /// reached.
    ///
    /// This method behaves exactly like [`BuildContext::visit_ancestor_elements`], except it
    /// only walks the tree up to the closest [`LookupBoundary`] ancestor of the provided
    /// context. The root of the tree is treated as an implicit lookup boundary.
    pub fn visit_ancestor_elements(
        app: &App,
        context: BuildContext,
        visitor: &mut dyn FnMut(AnyElement) -> bool,
    ) {
        context.visit_ancestor_elements(app, &mut |ancestor| {
            visitor(ancestor) && !is_lookup_boundary(app, ancestor)
        });
    }

    /// Walks the non-[`LookupBoundary`] child [`Element`](crate::Element)s of the provided
    /// `context`.
    ///
    /// This method behaves exactly like [`BuildContext::visit_child_elements`], except it
    /// only visits children that are not a [`LookupBoundary`].
    pub fn visit_child_elements(
        app: &App,
        context: BuildContext,
        visitor: &mut dyn FnMut(AnyElement),
    ) {
        context.visit_child_elements(app, &mut |child| {
            if !is_lookup_boundary(app, child) {
                visitor(child);
            }
        });
    }

    /// Returns true if a [`LookupBoundary`] is hiding the nearest widget of the provided type
    /// `W` from the provided [`BuildContext`].
    ///
    /// This method throws when asserts are disabled.
    pub fn debug_is_hiding_ancestor_widget_of_exact_type<W: 'static>(
        app: &App,
        context: BuildContext,
    ) -> bool {
        debug_is_hiding_ancestor(app, context, |ancestor| {
            ancestor.widget(app).widget_type() == TypeId::of::<W>()
        })
    }

    /// Returns true if a [`LookupBoundary`] is hiding the nearest `StatefulWidget` with a
    /// [`State`] of the provided type `S` from the provided [`BuildContext`].
    ///
    /// This method throws when asserts are disabled.
    pub fn debug_is_hiding_ancestor_state_of_type<S: State>(
        app: &App,
        context: BuildContext,
    ) -> bool {
        debug_is_hiding_ancestor(app, context, |ancestor| {
            ancestor.state_handle::<S>(app).is_some()
        })
    }

    /// Returns true if a [`LookupBoundary`] is hiding the nearest `RenderObjectWidget` with a
    /// `RenderObject` of the provided type `R` from the provided [`BuildContext`].
    ///
    /// This method throws when asserts are disabled.
    pub fn debug_is_hiding_ancestor_render_object_of_type<R: RenderObject>(
        app: &App,
        context: BuildContext,
    ) -> bool {
        debug_is_hiding_ancestor(app, context, |ancestor| {
            ancestor.is_render_object_element()
                && ancestor
                    .render_object(app)
                    .is_some_and(|render_object| render_object.downcast::<R>(app).is_some())
        })
    }
}

/// Dart's `ancestor.widget.runtimeType == LookupBoundary`.
fn is_lookup_boundary(app: &App, element: AnyElement) -> bool {
    element.widget(app).widget_type() == TypeId::of::<LookupBoundary>()
}

/// The walk the three `debugIsHidingAncestor…` methods share: whether the first ancestor
/// `matches` accepts lies past a [`LookupBoundary`].
fn debug_is_hiding_ancestor(
    app: &App,
    context: BuildContext,
    matches: impl Fn(AnyElement) -> bool,
) -> bool {
    if !cfg!(debug_assertions) {
        panic!("LookupBoundary's debugIsHiding methods are only available in debug mode");
    }
    let mut hidden_by_boundary = false;
    let mut ancestor_found = false;
    context.visit_ancestor_elements(app, &mut |ancestor| {
        if matches(ancestor) {
            ancestor_found = true;
            return false;
        }
        hidden_by_boundary = hidden_by_boundary || is_lookup_boundary(app, ancestor);
        true
    });
    ancestor_found & hidden_by_boundary
}

impl InheritedWidget for LookupBoundary {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, _old_widget: &LookupBoundary) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_foundation::AppCell;
    use waft_rendering::RenderPositionedBox;

    use super::*;
    use crate::framework::{StateData, StatefulWidget};
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, Center, SizedBox};

    /// `lookup_boundary_test.dart`'s `MyInheritedWidget`.
    #[derive(Debug)]
    struct MyInheritedWidget {
        value: i32,
        child: WidgetRef,
    }

    impl InheritedWidget for MyInheritedWidget {
        fn child(&self) -> &WidgetRef {
            &self.child
        }

        fn update_should_notify(&self, old_widget: &MyInheritedWidget) -> bool {
            self.value != old_widget.value
        }
    }

    fn inherited<K>(value: i32, child: impl IntoWidget<K>) -> WidgetRef {
        MyInheritedWidget {
            value,
            child: child.into_widget(),
        }
        .into_widget()
    }

    /// `lookup_boundary_test.dart`'s `MyStatefulContainer`.
    #[derive(Debug)]
    struct MyStatefulContainer {
        value: i32,
        child: WidgetRef,
    }

    impl StatefulWidget for MyStatefulContainer {
        type State = MyStatefulContainerState;

        fn create_state(&self) -> MyStatefulContainerState {
            MyStatefulContainerState {
                state: StateData::new(),
            }
        }
    }

    struct MyStatefulContainerState {
        state: StateData<MyStatefulContainer>,
    }

    impl State for MyStatefulContainerState {
        type Widget = MyStatefulContainer;
        crate::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            self.widget(app).child.clone()
        }
    }

    fn stateful<K>(value: i32, child: impl IntoWidget<K>) -> WidgetRef {
        MyStatefulContainer {
            value,
            child: child.into_widget(),
        }
        .into_widget()
    }

    /// A builder that hands its context to `seen` on every build.
    fn probe(seen: &Rc<Cell<Option<BuildContext>>>, builds: &Rc<Cell<u32>>) -> WidgetRef {
        let (seen, builds) = (Rc::clone(seen), Rc::clone(builds));
        Builder::new(move |_app, context| {
            seen.set(Some(context));
            builds.set(builds.get() + 1);
            SizedBox::expand().into_widget()
        })
        .into_widget()
    }

    fn mount(app: &mut App, tree: WidgetRef) -> BuildContext {
        let seen: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let builds = Rc::new(Cell::new(0));
        let harness = Harness::mount(app, replace_probe(tree, &seen, &builds));
        harness.pump(app);
        seen.get().expect("the probe built")
    }

    thread_local! {
        static PROBE: RefCell<Option<WidgetRef>> = const { RefCell::new(None) };
    }

    /// The tree builders below put [`probe_slot`] where the probe goes; this fills it.
    fn replace_probe(
        tree: WidgetRef,
        seen: &Rc<Cell<Option<BuildContext>>>,
        builds: &Rc<Cell<u32>>,
    ) -> WidgetRef {
        PROBE.with(|probe_widget| *probe_widget.borrow_mut() = Some(probe(seen, builds)));
        tree
    }

    /// A builder that builds whatever probe [`replace_probe`] installed.
    fn probe_slot() -> WidgetRef {
        Builder::new(|_app, _context| {
            PROBE.with(|probe_widget| probe_widget.borrow().clone().expect("a probe"))
        })
        .into_widget()
    }

    fn value_of(widget: Option<&MyInheritedWidget>) -> Option<i32> {
        widget.map(|widget| widget.value)
    }

    /// "LookupBoundary.dependOnInheritedWidgetOfExactType": respects the boundary, ignores
    /// an ancestor boundary, and finds the widget before the boundary.
    #[test]
    fn depend_on_inherited_widget_stops_at_the_closest_boundary() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();

        let context = mount(&mut app, inherited(2, LookupBoundary::new(probe_slot())));
        assert_eq!(
            value_of(
                context.depend_on_inherited_widget_of_exact_type::<MyInheritedWidget>(&mut app)
            ),
            Some(2),
            "the context's own lookup goes through the boundary"
        );
        assert_eq!(
            value_of(LookupBoundary::depend_on_inherited_widget_of_exact_type::<
                MyInheritedWidget,
            >(&mut app, context)),
            None
        );

        let context = mount(
            &mut app,
            LookupBoundary::new(inherited(2, probe_slot())).into_widget(),
        );
        assert_eq!(
            value_of(LookupBoundary::depend_on_inherited_widget_of_exact_type::<
                MyInheritedWidget,
            >(&mut app, context)),
            Some(2),
            "a boundary above the widget does not hide it"
        );

        let context = mount(
            &mut app,
            inherited(2, LookupBoundary::new(inherited(1, probe_slot()))),
        );
        assert_eq!(
            value_of(LookupBoundary::depend_on_inherited_widget_of_exact_type::<
                MyInheritedWidget,
            >(&mut app, context)),
            Some(1)
        );
        assert_eq!(
            value_of(LookupBoundary::get_inherited_widget_of_exact_type::<
                MyInheritedWidget,
            >(&app, context)),
            Some(1)
        );
    }

    /// `lookup_boundary_test.dart`'s `DidChangeDependencySpy`: counts its state's
    /// `did_change_dependencies` and runs the callback in it.
    struct DidChangeDependencySpy {
        key: Option<KeyRef>,
        count: Rc<Cell<u32>>,
        on_did_change_dependencies: OnDidChangeDependencies,
    }

    /// Dart's `OnDidChangeDependencies`.
    type OnDidChangeDependencies = Rc<dyn Fn(&mut App, BuildContext)>;

    impl std::fmt::Debug for DidChangeDependencySpy {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("DidChangeDependencySpy")
        }
    }

    impl StatefulWidget for DidChangeDependencySpy {
        type State = DidChangeDependencySpyState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> DidChangeDependencySpyState {
            DidChangeDependencySpyState {
                state: StateData::new(),
            }
        }
    }

    struct DidChangeDependencySpyState {
        state: StateData<DidChangeDependencySpy>,
    }

    impl State for DidChangeDependencySpyState {
        type Widget = DidChangeDependencySpy;
        crate::state_accessors!();

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            let widget = self.widget(app);
            widget.count.set(widget.count.get() + 1);
            let callback = Rc::clone(&widget.on_did_change_dependencies);
            let context = self.context(app);
            callback(app, context);
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::shrink().into_widget()
        }
    }

    /// A spy keyed by `key`, so that a change of the tree around it moves its element.
    fn spy(
        key: &crate::framework::LabeledGlobalKey,
        count: &Rc<Cell<u32>>,
        on_did_change_dependencies: impl Fn(&mut App, BuildContext) + 'static,
    ) -> WidgetRef {
        DidChangeDependencySpy {
            key: Some(Rc::new(key.clone())),
            count: Rc::clone(count),
            on_did_change_dependencies: Rc::new(on_did_change_dependencies),
        }
        .into_widget()
    }

    fn wrapped(child: WidgetRef) -> WidgetRef {
        SizedBox::new().child(child).into_widget()
    }

    /// A spy whose dependencies callback records `LookupBoundary::depend_on…`'s value.
    fn depending_spy(
        key: &crate::framework::LabeledGlobalKey,
        count: &Rc<Cell<u32>>,
        seen: &Rc<Cell<Option<i32>>>,
    ) -> WidgetRef {
        let seen = Rc::clone(seen);
        spy(key, count, move |app, context| {
            seen.set(value_of(
                LookupBoundary::depend_on_inherited_widget_of_exact_type::<MyInheritedWidget>(
                    app, context,
                ),
            ));
        })
    }

    fn pump_tree(app: &mut App, harness: &Harness, tree: WidgetRef) {
        harness.set_child(app, tree);
        harness.pump(app);
    }

    /// "LookupBoundary.dependOnInheritedWidgetOfExactType creates dependency".
    #[test]
    fn depend_on_inherited_widget_creates_a_dependency() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let key = <dyn crate::framework::GlobalKey>::new();
        let (count, seen) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)));
        let harness = Harness::mount(&mut app, inherited(1, depending_spy(&key, &count, &seen)));
        harness.pump(&mut app);
        assert_eq!((seen.get(), count.get()), (Some(1), 1));

        pump_tree(
            &mut app,
            &harness,
            inherited(2, depending_spy(&key, &count, &seen)),
        );
        assert_eq!((seen.get(), count.get()), (Some(2), 2));
    }

    /// "causes didChangeDependencies to be called on move even if dependency was not fulfilled
    /// due to boundary".
    #[test]
    fn a_move_reaches_did_change_dependencies_though_the_boundary_hid_the_dependency() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let key = <dyn crate::framework::GlobalKey>::new();
        let (count, seen) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)));
        let tree = |value: i32| {
            inherited(
                value,
                LookupBoundary::new(depending_spy(&key, &count, &seen)),
            )
        };
        let harness = Harness::mount(&mut app, tree(1));
        harness.pump(&mut app);
        assert_eq!((seen.get(), count.get()), (None, 1));

        // Value of inherited widget changes, but there should be no dependency due to boundary.
        pump_tree(&mut app, &harness, tree(2));
        assert_eq!((seen.get(), count.get()), (None, 1));

        // Widget is moved, didChangeDependencies is called, but dependency is still not found
        // due to boundary.
        pump_tree(&mut app, &harness, wrapped(tree(2)));
        assert_eq!((seen.get(), count.get()), (None, 2));

        pump_tree(
            &mut app,
            &harness,
            wrapped(inherited(
                2,
                LookupBoundary::new(inherited(4, depending_spy(&key, &count, &seen))),
            )),
        );
        assert_eq!((seen.get(), count.get()), (Some(4), 3));
    }

    /// "causes didChangeDependencies to be called on move even if dependency was non-existent".
    #[test]
    fn a_move_reaches_did_change_dependencies_though_nothing_was_found() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let key = <dyn crate::framework::GlobalKey>::new();
        let (count, seen) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)));
        let harness = Harness::mount(&mut app, depending_spy(&key, &count, &seen));
        harness.pump(&mut app);
        assert_eq!((seen.get(), count.get()), (None, 1));

        // Widget moved, didChangeDependencies must be called.
        pump_tree(
            &mut app,
            &harness,
            wrapped(depending_spy(&key, &count, &seen)),
        );
        assert_eq!((seen.get(), count.get()), (None, 2));

        pump_tree(
            &mut app,
            &harness,
            inherited(6, wrapped(depending_spy(&key, &count, &seen))),
        );
        assert_eq!((seen.get(), count.get()), (Some(6), 3));
    }

    /// A spy whose dependencies callback only looks an element up.
    fn looking_spy(key: &crate::framework::LabeledGlobalKey, count: &Rc<Cell<u32>>) -> WidgetRef {
        spy(key, count, |app, context| {
            LookupBoundary::get_element_for_inherited_widget_of_exact_type::<MyInheritedWidget>(
                app, context,
            );
        })
    }

    /// "LookupBoundary.getElementForInheritedWidgetOfExactType does not cause
    /// didChangeDependencies to be called on move when found" and "… when nothing was found".
    #[test]
    fn get_element_for_inherited_widget_does_not_bring_did_change_dependencies_on_a_move() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let key = <dyn crate::framework::GlobalKey>::new();
        let count = Rc::new(Cell::new(0));
        let tree = |value: i32| inherited(value, LookupBoundary::new(looking_spy(&key, &count)));
        let harness = Harness::mount(&mut app, tree(1));
        harness.pump(&mut app);
        assert_eq!(count.get(), 1);
        pump_tree(&mut app, &harness, tree(2));
        assert_eq!(count.get(), 1);
        pump_tree(&mut app, &harness, wrapped(tree(2)));
        assert_eq!(count.get(), 1);
        pump_tree(
            &mut app,
            &harness,
            wrapped(inherited(
                2,
                LookupBoundary::new(inherited(4, looking_spy(&key, &count))),
            )),
        );
        assert_eq!(count.get(), 1);

        let key = <dyn crate::framework::GlobalKey>::new();
        let count = Rc::new(Cell::new(0));
        let harness = Harness::mount(&mut app, looking_spy(&key, &count));
        harness.pump(&mut app);
        assert_eq!(count.get(), 1);
        pump_tree(&mut app, &harness, wrapped(looking_spy(&key, &count)));
        assert_eq!(count.get(), 1);
        pump_tree(
            &mut app,
            &harness,
            inherited(6, wrapped(looking_spy(&key, &count))),
        );
        assert_eq!(count.get(), 1);
    }

    /// "LookupBoundary.findAncestorWidgetOfExactType / findAncestorStateOfType /
    /// findRootAncestorStateOfType / findAncestorRenderObjectOfType does not establish a
    /// dependency".
    #[test]
    fn the_ancestor_finds_establish_no_dependency() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let key = <dyn crate::framework::GlobalKey>::new();
        let count = Rc::new(Cell::new(0));
        let found = Rc::new(Cell::new((false, false, false, false)));
        let tree = || {
            let found = Rc::clone(&found);
            LookupBoundary::new(stateful(
                1,
                Center::new().child(spy(&key, &count, move |app, context| {
                    found.set(
                            (
                                LookupBoundary::find_ancestor_widget_of_exact_type::<
                                    MyStatefulContainer,
                                >(app, context)
                                .is_some(),
                                LookupBoundary::find_ancestor_state_of_type::<
                                    MyStatefulContainerState,
                                >(app, context)
                                .is_some(),
                                LookupBoundary::find_root_ancestor_state_of_type::<
                                    MyStatefulContainerState,
                                >(app, context)
                                .is_some(),
                                LookupBoundary::find_ancestor_render_object_of_type::<
                                    RenderPositionedBox,
                                >(app, context)
                                .is_some(),
                            ),
                        );
                })),
            ))
            .into_widget()
        };
        let harness = Harness::mount(&mut app, tree());
        harness.pump(&mut app);
        assert_eq!(found.get(), (true, true, true, true));
        assert_eq!(count.get(), 1);

        // Changes tree structure, triggers global key move of the spy.
        pump_tree(&mut app, &harness, wrapped(tree()));
        assert_eq!(count.get(), 1);
    }

    /// "LookupBoundary.getElementForInheritedWidgetOfExactType does not create dependency".
    #[test]
    fn get_element_for_inherited_widget_creates_no_dependency() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let context = mount(&mut app, inherited(2, probe_slot()));
        let element = LookupBoundary::get_element_for_inherited_widget_of_exact_type::<
            MyInheritedWidget,
        >(&app, context)
        .expect("found");
        assert!(!context.does_depend_on_inherited_element(&app, element));
    }

    /// "LookupBoundary.findAncestorWidgetOfExactType", "findAncestorStateOfType",
    /// "findRootAncestorStateOfType": respect the boundary and find the right one before it.
    #[test]
    fn the_ancestor_finds_stop_at_the_closest_boundary() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();

        let context = mount(
            &mut app,
            stateful(
                3,
                inherited(
                    2,
                    LookupBoundary::new(stateful(1, stateful(0, probe_slot()))),
                ),
            ),
        );
        assert!(
            LookupBoundary::find_ancestor_widget_of_exact_type::<MyInheritedWidget>(&app, context)
                .is_none()
        );
        assert!(
            context
                .find_ancestor_widget_of_exact_type::<MyInheritedWidget>(&app)
                .is_some()
        );
        let nearest =
            LookupBoundary::find_ancestor_state_of_type::<MyStatefulContainerState>(&app, context)
                .expect("a state inside the boundary");
        assert_eq!(nearest.widget(&app).value, 0);
        let root = LookupBoundary::find_root_ancestor_state_of_type::<MyStatefulContainerState>(
            &app, context,
        )
        .expect("a state inside the boundary");
        assert_eq!(
            root.widget(&app).value,
            1,
            "the furthest one inside the boundary"
        );

        let context = mount(&mut app, stateful(3, LookupBoundary::new(probe_slot())));
        assert!(
            LookupBoundary::find_ancestor_state_of_type::<MyStatefulContainerState>(&app, context)
                .is_none()
        );
        assert!(
            LookupBoundary::find_root_ancestor_state_of_type::<MyStatefulContainerState>(
                &app, context
            )
            .is_none()
        );
    }

    /// "LookupBoundary.findAncestorRenderObjectOfType respects boundary".
    #[test]
    fn find_ancestor_render_object_stops_at_the_closest_boundary() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let context = mount(
            &mut app,
            Center::new()
                .child(LookupBoundary::new(probe_slot()))
                .into_widget(),
        );
        assert!(
            LookupBoundary::find_ancestor_render_object_of_type::<RenderPositionedBox>(
                &app, context
            )
            .is_none()
        );
        assert!(
            context
                .find_ancestor_render_object_of_type::<RenderPositionedBox>(&app)
                .is_some()
        );

        let context = mount(
            &mut app,
            LookupBoundary::new(Center::new().child(probe_slot())).into_widget(),
        );
        assert!(
            LookupBoundary::find_ancestor_render_object_of_type::<RenderPositionedBox>(
                &app, context
            )
            .is_some()
        );
    }

    /// "LookupBoundary.visitAncestorElements respects boundary": the walk ends at the boundary,
    /// which it visits.
    #[test]
    fn visit_ancestor_elements_ends_at_the_boundary() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let context = mount(
            &mut app,
            inherited(2, LookupBoundary::new(inherited(1, probe_slot()))),
        );
        let mut through = 0;
        context.visit_ancestor_elements(&app, &mut |ancestor| {
            if ancestor.widget(&app).widget_type() == TypeId::of::<MyInheritedWidget>() {
                through += 1;
            }
            true
        });
        let mut within = 0;
        let mut visited_boundary = false;
        LookupBoundary::visit_ancestor_elements(&app, context, &mut |ancestor| {
            if ancestor.widget(&app).widget_type() == TypeId::of::<MyInheritedWidget>() {
                within += 1;
            }
            visited_boundary |= is_lookup_boundary(&app, ancestor);
            true
        });
        assert_eq!(through, 2);
        assert_eq!(within, 1);
        assert!(visited_boundary, "the boundary itself is visited");
    }

    /// "LookupBoundary.visitChildElements respects boundary".
    #[test]
    fn visit_child_elements_skips_a_boundary_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let parent: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let tree = {
            let parent = Rc::clone(&parent);
            inherited(
                1,
                Builder::new(move |_app, context| {
                    parent.set(Some(context));
                    LookupBoundary::new(SizedBox::expand()).into_widget()
                }),
            )
        };
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);
        let parent = parent.get().expect("built");
        let mut all = 0;
        parent.visit_child_elements(&app, &mut |_| all += 1);
        let mut within = 0;
        LookupBoundary::visit_child_elements(&app, parent, &mut |_| within += 1);
        assert_eq!((all, within), (1, 0));
    }

    /// "LookupBoundary.debugIsHidingAncestorWidgetOfExactType", "…StateOfType" and
    /// "…RenderObjectOfType": hiding only when the entity is past a boundary.
    #[test]
    fn debug_is_hiding_reports_an_entity_past_a_boundary() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();

        let context = mount(
            &mut app,
            Center::new()
                .child(stateful(0, inherited(1, LookupBoundary::new(probe_slot()))))
                .into_widget(),
        );
        assert!(
            LookupBoundary::debug_is_hiding_ancestor_widget_of_exact_type::<MyInheritedWidget>(
                &app, context
            )
        );
        assert!(LookupBoundary::debug_is_hiding_ancestor_state_of_type::<
            MyStatefulContainerState,
        >(&app, context));
        assert!(
            LookupBoundary::debug_is_hiding_ancestor_render_object_of_type::<RenderPositionedBox>(
                &app, context
            )
        );

        let context = mount(
            &mut app,
            LookupBoundary::new(Center::new().child(stateful(0, inherited(1, probe_slot()))))
                .into_widget(),
        );
        assert!(
            !LookupBoundary::debug_is_hiding_ancestor_widget_of_exact_type::<MyInheritedWidget>(
                &app, context
            )
        );
        assert!(!LookupBoundary::debug_is_hiding_ancestor_state_of_type::<
            MyStatefulContainerState,
        >(&app, context));
        assert!(
            !LookupBoundary::debug_is_hiding_ancestor_render_object_of_type::<RenderPositionedBox>(
                &app, context
            )
        );

        let context = mount(&mut app, probe_slot());
        assert!(
            !LookupBoundary::debug_is_hiding_ancestor_widget_of_exact_type::<MyInheritedWidget>(
                &app, context
            )
        );
    }
}
