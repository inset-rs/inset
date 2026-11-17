//! Flutter counterpart: `widgets/scroll_delegate.dart`.
//!
//! The delegates that supply children to a lazy sliver. `NullableIndexedWidgetBuilder` is
//! Flutter's `framework.dart` typedef; the framework module does not export one, so it is
//! defined here next to its callers.
//!
//! `TwoDimensionalChildDelegate` and its subclasses wait with `two_dimensional_viewport.dart`;
//! see `PORTING.md`.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::hash::Hasher;
use std::rc::Rc;

use waft_foundation::{App, Handle, Key, Listener, ValueKey};

use waft_rendering::{AnySelectable, SelectionRegistrar, SelectionRegistrarRef};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::automatic_keep_alive::{
    AutomaticKeepAlive, AutomaticKeepAliveClientMixin, AutomaticKeepAliveClientMixinData,
};
use crate::widgets::selection_container::{SelectionContainer, SelectionRegistrarScope};
use crate::widgets::basic::{IndexedSemantics, KeyedSubtree, RepaintBoundary};

/// Signature for a function that creates a widget for a given index, e.g., in a list, but may
/// return `None`.
///
/// Used by [`SliverChildBuilderDelegate::builder`] and `ListView.builder`. If a `builder`
/// returns `None`, the sliver will stop building children.
///
/// Flutter's `Widget? Function(BuildContext context, int index)` from `framework.dart`;
/// receives [`App`] because a Rust callback cannot capture what it mutates.
pub type NullableIndexedWidgetBuilder =
    Rc<dyn Fn(&mut App, BuildContext, i32) -> Option<WidgetRef>>;

/// A callback which produces a semantic index given a widget and the local index.
///
/// Return `None` to prevent a widget from receiving an index.
///
/// A semantic index is used to tag child semantic nodes for accessibility announcements in
/// scroll view.
///
/// See also:
///
///  * `CustomScrollView`, for an explanation of scroll semantics.
///  * [`SliverChildBuilderDelegate`], for an explanation of how this is used to generate
///    indexes.
pub type SemanticIndexCallback = Rc<dyn Fn(&WidgetRef, i32) -> Option<i32>>;

/// Flutter's `_kDefaultSemanticIndexCallback`.
fn default_semantic_index_callback() -> SemanticIndexCallback {
    Rc::new(|_widget, local_index| Some(local_index))
}

/// A delegate that supplies children for slivers.
///
/// Many slivers lazily construct their box children to avoid creating more children than are
/// visible through the `Viewport`. Rather than receiving their children as an explicit
/// [`Vec`], they receive their children using a [`SliverChildDelegate`].
///
/// It's uncommon to implement [`SliverChildDelegate`]. Instead, consider using one of the
/// existing implementors that provide adaptors to builder callbacks or explicit child lists.
///
/// ## Child elements' lifecycle
///
/// ### Creation
///
/// While laying out the list, visible children's elements, states and render objects will be
/// created lazily based on existing widgets (such as in the case of
/// [`SliverChildListDelegate`]) or lazily provided ones (such as in the case of
/// [`SliverChildBuilderDelegate`]).
///
/// ### Destruction
///
/// When a child is scrolled out of view, the associated element subtree, states and render
/// objects are destroyed. A new child at the same position in the sliver will be lazily
/// recreated along with new elements, states and render objects when it is scrolled back.
///
/// ### Destruction mitigation
///
/// In order to preserve state as child elements are scrolled in and out of view, the following
/// options are possible:
///
///  * Moving the ownership of non-trivial UI-state-driving business logic out of the sliver
///    child subtree. For instance, if a list contains posts with their number of upvotes coming
///    from a cached network response, store the list of posts and upvote number in a data model
///    outside the list. Let the sliver child UI subtree be easily recreate-able from the
///    source-of-truth model object. Use `StatefulWidget`s in the child widget subtree to store
///    instantaneous UI state only.
///
///  * Letting `KeepAlive` be the root widget of the sliver child widget subtree that needs to
///    be preserved. The `KeepAlive` widget marks the child subtree's top render object child
///    for keepalive. When the associated top render object is scrolled out of view, the sliver
///    keeps the child's render object (and by extension, its associated elements and states) in
///    a cache list instead of destroying them. When scrolled back into view, the render object
///    is repainted as-is (if it wasn't marked dirty in the interim).
///
/// ## Using more than one delegate in a `Viewport`
///
/// If multiple delegates are used in a single scroll view, the first child of each delegate
/// will always be laid out, even if it extends beyond the currently viewable area. This is
/// because at least one child is required in order to
/// [`estimate_max_scroll_offset`](Self::estimate_max_scroll_offset) for the whole scroll view,
/// as it uses the currently built children to estimate the remaining children's extent.
///
/// See also:
///
///  * [`SliverChildBuilderDelegate`], which is a delegate that uses a builder callback to
///    construct the children.
///  * [`SliverChildListDelegate`], which is a delegate that has an explicit list of children.
pub trait SliverChildDelegate: Debug + 'static {
    /// Returns the child with the given index.
    ///
    /// Should return `None` if asked to build a widget with a greater index than exists. If
    /// this returns `None`, [`estimated_child_count`](Self::estimated_child_count) must
    /// subsequently return a precise non-`None` value (which is then used to implement
    /// `RenderSliverBoxChildManager::child_count`).
    ///
    /// Implementors typically wrap their children in `AutomaticKeepAlive`, `IndexedSemantics`,
    /// and [`RepaintBoundary`] widgets.
    ///
    /// The values returned by this method are cached. To indicate that the widgets have
    /// changed, a new delegate must be provided, and the new delegate's
    /// [`should_rebuild`](Self::should_rebuild) method must return true.
    fn build(&self, app: &mut App, context: BuildContext, index: i32) -> Option<WidgetRef>;

    /// Returns an estimate of the number of children this delegate will build.
    ///
    /// Used to estimate the maximum scroll offset if
    /// [`estimate_max_scroll_offset`](Self::estimate_max_scroll_offset) returns `None`.
    ///
    /// Return `None` if there are an unbounded number of children or if it would be too
    /// difficult to estimate the number of children.
    ///
    /// This must return a precise number once [`build`](Self::build) has returned `None`, as it
    /// is used to implement `RenderSliverBoxChildManager::child_count`.
    fn estimated_child_count(&self) -> Option<i32> {
        None
    }

    /// Returns an estimate of the max scroll extent for all the children.
    ///
    /// Implementors should override this function if they have additional information about
    /// their max scroll extent.
    ///
    /// The default implementation returns `None`, which causes the caller to extrapolate the
    /// max scroll offset from the given parameters.
    fn estimate_max_scroll_offset(
        &self,
        first_index: i32,
        last_index: i32,
        leading_scroll_offset: f64,
        trailing_scroll_offset: f64,
    ) -> Option<f64> {
        let _ = (
            first_index,
            last_index,
            leading_scroll_offset,
            trailing_scroll_offset,
        );
        None
    }

    /// Called at the end of layout to indicate that layout is now complete.
    ///
    /// The `first_index` argument is the index of the first child that was included in the
    /// current layout. The `last_index` argument is the index of the last child that was
    /// included in the current layout.
    ///
    /// Useful for implementors that wish to track which children are included in the underlying
    /// render tree.
    fn did_finish_layout(&self, first_index: i32, last_index: i32) {
        let _ = (first_index, last_index);
    }

    /// Called whenever a new instance of the child delegate class is provided to the sliver.
    ///
    /// If the new instance represents different information than the old instance, then the
    /// method should return true, otherwise it should return false.
    ///
    /// If the method returns false, then the [`build`](Self::build) call might be optimized
    /// away.
    ///
    /// Dart's `covariant SliverChildDelegate oldDelegate`: the caller has already compared
    /// [`delegate_type`](Self::delegate_type), so the argument downcasts through
    /// [`as_any`](Self::as_any).
    fn should_rebuild(&self, old_delegate: &dyn SliverChildDelegate) -> bool;

    /// Find index of child element with associated key.
    ///
    /// This will be called during `perform_rebuild` in `SliverMultiBoxAdaptorElement` to check
    /// if a child has moved to a different position. It should return the index of the child
    /// element with associated key, `None` if not found.
    ///
    /// If not provided, a child widget may not map to its existing `RenderObject` when the
    /// order of children returned from the children builder changes. This may result in
    /// state-loss.
    fn find_index_by_key(&self, key: &KeyRef) -> Option<i32> {
        let _ = key;
        None
    }

    /// The concrete delegate, for Dart's `oldDelegate as Foo`.
    fn as_any(&self) -> &dyn Any;

    /// The concrete delegate's type, Dart's `runtimeType`.
    fn delegate_type(&self) -> TypeId {
        self.as_any().type_id()
    }

    /// Add additional information to the given description for use by `toString`.
    fn debug_fill_description(&self, description: &mut Vec<String>) {
        if let Some(children) = self.estimated_child_count() {
            description.push(format!("estimated child count: {children}"));
        }
    }
}

/// A shared [`SliverChildDelegate`] (Dart's `SliverChildDelegate` reference).
pub type SliverChildDelegateRef = Rc<dyn SliverChildDelegate>;

/// Dart's `identical(a, b)` on two delegates: `SliverChildDelegate` does not override `==`.
pub fn same_delegate(a: &SliverChildDelegateRef, b: &SliverChildDelegateRef) -> bool {
    Rc::ptr_eq(a, b)
}

/// Flutter's `_SaltedValueKey`: a `ValueKey<Key>` that cannot collide with a key the child
/// itself carries.
#[derive(Clone, Debug)]
struct SaltedValueKey(ValueKey<KeyRef>);

impl SaltedValueKey {
    fn new(value: KeyRef) -> SaltedValueKey {
        SaltedValueKey(ValueKey::new(value))
    }
}

impl Key for SaltedValueKey {
    fn eq_key(&self, other: &dyn Key) -> bool {
        (other as &dyn Any)
            .downcast_ref::<SaltedValueKey>()
            .is_some_and(|other| other.0.value == self.0.value)
    }

    fn hash_key(&self, mut state: &mut dyn Hasher) {
        use std::hash::Hash;
        TypeId::of::<SaltedValueKey>().hash(&mut state);
        self.0.value.hash(&mut state);
    }
}

/// Dart's `key is _SaltedValueKey ? key.value : key`.
fn unsalt(key: &KeyRef) -> KeyRef {
    match (key.key() as &dyn Any).downcast_ref::<SaltedValueKey>() {
        Some(salted) => salted.0.value.clone(),
        None => key.clone(),
    }
}

/// Keeps alive the children whose selectables have a selection.
///
/// `_SelectionKeepAlive` in Dart: a registrar for one sliver child that stays between the parent
/// registrar and the selectables below it, so a selection off screen keeps its child alive.
struct SelectionKeepAlive {
    key: Option<KeyRef>,
    child: WidgetRef,
}

impl SelectionKeepAlive {
    fn new<K>(child: impl IntoWidget<K>) -> SelectionKeepAlive {
        SelectionKeepAlive {
            key: None,
            child: child.into_widget(),
        }
    }
}

impl Debug for SelectionKeepAlive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectionKeepAlive").finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectionKeepAlive {
    type State = SelectionKeepAliveState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SelectionKeepAliveState {
        SelectionKeepAliveState {
            state: StateData::new(),
            keep_alive: AutomaticKeepAliveClientMixinData::new(),
            selectables_with_selections: None,
            selectable_attachments: None,
            registrar: None,
            want_keep_alive: false,
        }
    }
}

struct SelectionKeepAliveState {
    state: StateData<SelectionKeepAlive>,
    keep_alive: AutomaticKeepAliveClientMixinData,
    selectables_with_selections: Option<HashSet<AnySelectable>>,
    selectable_attachments: Option<HashMap<AnySelectable, Listener>>,
    registrar: Option<SelectionRegistrarRef>,
    want_keep_alive: bool,
}

impl SelectionKeepAliveState {
    fn registrar_handle(self: Handle<Self>, app: &App) -> Option<waft_rendering::AnySelectionRegistrar> {
        app.get(self).registrar.as_ref().map(|registrar| registrar.handle())
    }

    fn listens_to(self: Handle<Self>, selectable: AnySelectable) -> Listener {
        Listener::new(move |app| {
            let add = selectable.value(app).has_selection();
            SelectionKeepAliveState::update_selectables_with_selections(
                self, app, selectable, add,
            );
        })
    }

    fn update_selectables_with_selections(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        add: bool,
    ) {
        let has_selection = selectable.value(app).has_selection();
        {
            let this = app.get_mut(self);
            if add {
                debug_assert!(has_selection);
                this.selectables_with_selections
                    .get_or_insert_with(HashSet::new)
                    .insert(selectable);
            } else if let Some(selectables) = &mut this.selectables_with_selections {
                selectables.remove(&selectable);
            }
        }
        let want = app
            .get(self)
            .selectables_with_selections
            .as_ref()
            .is_some_and(|selectables| !selectables.is_empty());
        self.set_want_keep_alive(app, want);
    }

    fn set_want_keep_alive(self: Handle<Self>, app: &mut App, value: bool) {
        if app.get(self).want_keep_alive != value {
            app.get_mut(self).want_keep_alive = value;
            self.update_keep_alive(app);
        }
    }
}


impl State for SelectionKeepAliveState {
    type Widget = SelectionKeepAlive;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AutomaticKeepAliveClientMixin::init_state(self, app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let new_registrar = SelectionContainer::maybe_of(app, self.context(app));
        let current = self.registrar_handle(app);
        let new_handle = new_registrar.as_ref().map(|registrar| registrar.handle());
        if current == new_handle {
            return;
        }
        let attachments: Vec<AnySelectable> = app
            .get(self)
            .selectable_attachments
            .as_ref()
            .map(|attachments| attachments.keys().copied().collect())
            .unwrap_or_default();
        if let Some(registrar) = current {
            for selectable in &attachments {
                registrar.remove(app, *selectable);
            }
        }
        app.get_mut(self).registrar = new_registrar;
        if let Some(registrar) = self.registrar_handle(app) {
            for selectable in &attachments {
                registrar.add(app, *selectable);
            }
        }
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        AutomaticKeepAliveClientMixin::deactivate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let attachments = app.get_mut(self).selectable_attachments.take();
        if let Some(attachments) = attachments {
            let registrar = self.registrar_handle(app);
            for (selectable, listener) in attachments {
                if let Some(registrar) = registrar {
                    registrar.remove(app, selectable);
                }
                selectable.remove_listener(app, &listener);
            }
        }
        app.get_mut(self).selectables_with_selections = None;
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let _ = AutomaticKeepAliveClientMixin::build(self, app);
        if self.registrar_handle(app).is_none() {
            let child = self.widget(app).child.clone();
            return child;
        }
        let child = self.widget(app).child.clone();
        SelectionRegistrarScope::new(
            self.as_selection_registrar().keep(app),
            child,
        )
        .into_widget()
    }
}

impl AutomaticKeepAliveClientMixin for SelectionKeepAliveState {
    fn automatic_keep_alive_client_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AutomaticKeepAliveClientMixinData {
        &app.get(self).keep_alive
    }

    fn automatic_keep_alive_client_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AutomaticKeepAliveClientMixinData {
        &mut app.get_mut(self).keep_alive
    }

    fn want_keep_alive(self: Handle<Self>, app: &App) -> bool {
        app.get(self).want_keep_alive
    }
}

impl SelectionRegistrar for SelectionKeepAliveState {
    fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        let attachment = self.listens_to(selectable);
        selectable.add_listener(app, attachment.clone());
        app.get_mut(self)
            .selectable_attachments
            .get_or_insert_with(HashMap::new)
            .insert(selectable, attachment);
        self.registrar_handle(app)
            .expect("a sliver child with selectables is under a selection container")
            .add(app, selectable);
        if selectable.value(app).has_selection() {
            self.update_selectables_with_selections(app, selectable, true);
        }
    }

    fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        let Some(attachments) = app.get_mut(self).selectable_attachments.as_mut() else {
            return;
        };
        let attachment = attachments
            .remove(&selectable)
            .expect("the selectable was attached");
        selectable.remove_listener(app, &attachment);
        self.registrar_handle(app)
            .expect("a sliver child with selectables is under a selection container")
            .remove(app, selectable);
        self.update_selectables_with_selections(app, selectable, false);
    }
}

/// Wraps a built child the way both delegates do: a repaint boundary, an [`IndexedSemantics`],
/// an [`AutomaticKeepAlive`], then a [`KeyedSubtree`] carrying the salted key.
///
/// Flutter puts a [`SelectionKeepAlive`] inside the `AutomaticKeepAlive`.
fn wrap_child(
    child: WidgetRef,
    index: i32,
    add_automatic_keep_alives: bool,
    add_repaint_boundaries: bool,
    add_semantic_indexes: bool,
    semantic_index_callback: &SemanticIndexCallback,
    semantic_index_offset: i32,
) -> WidgetRef {
    let key = child
        .key()
        .map(|key| KeyRef::from(SaltedValueKey::new(key.clone())));
    let mut child = if add_repaint_boundaries {
        RepaintBoundary::new().child(child).into_widget()
    } else {
        child
    };
    if add_semantic_indexes && let Some(semantic_index) = semantic_index_callback(&child, index) {
        child = IndexedSemantics::new(semantic_index + semantic_index_offset)
            .child(child)
            .into_widget();
    }
    if add_automatic_keep_alives {
        child = AutomaticKeepAlive::new(SelectionKeepAlive::new(child)).into_widget();
    }
    let subtree = KeyedSubtree::new(child);
    match key {
        Some(key) => subtree.key(key).into_widget(),
        None => subtree.into_widget(),
    }
}

/// Called to find the new index of a child based on its `key` in case of reordering.
///
/// If the child with the `key` is no longer present, `None` is returned.
///
/// Used by [`SliverChildBuilderDelegate::find_child_index_callback`].
pub type ChildIndexGetter = Rc<dyn Fn(&KeyRef) -> Option<i32>>;

/// A delegate that supplies children for slivers using a builder callback.
///
/// Many slivers lazily construct their box children to avoid creating more children than are
/// visible through the `Viewport`. This delegate provides children using a
/// [`NullableIndexedWidgetBuilder`] callback, so that the children do not even have to be built
/// until they are displayed.
///
/// The widgets returned from the builder callback are automatically wrapped in
/// `AutomaticKeepAlive` widgets if [`add_automatic_keep_alives`](Self::add_automatic_keep_alives)
/// is true (the default) and in [`RepaintBoundary`] widgets if
/// [`add_repaint_boundaries`](Self::add_repaint_boundaries) is true (also the default).
///
/// ## Accessibility
///
/// `CustomScrollView` requires that its semantic children are annotated using
/// `IndexedSemantics`. This is done by default in the delegate with the
/// [`add_semantic_indexes`](Self::add_semantic_indexes) parameter set to true.
///
/// If multiple delegates are used in a single scroll view, then the indexes will not be correct
/// by default. The [`semantic_index_offset`](Self::semantic_index_offset) can be used to offset
/// the semantic indexes of each delegate so that the indexes are monotonically increasing. For
/// example, if a scroll view contains two delegates where the first has 10 children
/// contributing semantics, then the second delegate should offset its children by 10.
///
/// In certain cases, only a subset of child widgets should be annotated with a semantic index.
/// For example, in `ListView.separated` the separators do not have an index associated with
/// them. This is done by providing a
/// [`semantic_index_callback`](Self::semantic_index_callback) which returns `None` for
/// separators indexes and rounds the non-separator indexes down by half.
///
/// See also:
///
///  * [`SliverChildListDelegate`], which is a delegate that has an explicit list of children.
#[derive(Clone)]
pub struct SliverChildBuilderDelegate {
    /// Called to build children for the sliver.
    ///
    /// Will be called only for indices greater than or equal to zero and less than
    /// [`child_count`](Self::child_count) (if it is non-`None`).
    ///
    /// Should return `None` if asked to build a widget with a greater index than exists.
    ///
    /// May result in an infinite loop or run out of memory if
    /// [`child_count`](Self::child_count) is `None` and the builder always provides a zero-size
    /// widget (such as `SizedBox::shrink()`). If possible, provide children with non-zero size,
    /// return `None` from the builder, or set a [`child_count`](Self::child_count).
    ///
    /// The delegate wraps the children returned by this builder in [`RepaintBoundary`] widgets.
    pub builder: NullableIndexedWidgetBuilder,

    /// The total number of children this delegate can provide.
    ///
    /// If `None`, the number of children is determined by the least index for which
    /// [`builder`](Self::builder) returns `None`.
    ///
    /// May result in an infinite loop or run out of memory if it is `None` and the builder
    /// always provides a zero-size widget. If possible, provide children with non-zero size,
    /// return `None` from the builder, or set a child count.
    pub child_count: Option<i32>,

    /// Whether to wrap each child in an `AutomaticKeepAlive`.
    ///
    /// Typically, lazily laid out children are wrapped in `AutomaticKeepAlive` widgets so that
    /// the children can use `KeepAliveNotification`s to preserve their state when they would
    /// otherwise be garbage collected off-screen.
    ///
    /// This feature (and [`add_repaint_boundaries`](Self::add_repaint_boundaries)) must be
    /// disabled if the children are going to manually maintain their `KeepAlive` state. It may
    /// also be more efficient to disable this feature if it is known ahead of time that none of
    /// the children will ever try to keep themselves alive.
    ///
    /// Defaults to true.
    pub add_automatic_keep_alives: bool,

    /// Whether to wrap each child in a [`RepaintBoundary`].
    ///
    /// Typically, children in a scrolling container are wrapped in repaint boundaries so that
    /// they do not need to be repainted as the list scrolls. If the children are easy to
    /// repaint (e.g., solid color blocks or a short snippet of text), it might be more
    /// efficient to not add a repaint boundary and instead always repaint the children during
    /// scrolling.
    ///
    /// Defaults to true.
    pub add_repaint_boundaries: bool,

    /// Whether to wrap each child in an `IndexedSemantics`.
    ///
    /// Typically, children in a scrolling container must be annotated with a semantic index in
    /// order to generate the correct accessibility announcements. This should only be set to
    /// false if the indexes have already been provided by an `IndexedSemantics` widget.
    ///
    /// Defaults to true.
    pub add_semantic_indexes: bool,

    /// An initial offset to add to the semantic indexes generated by this widget.
    ///
    /// Defaults to zero.
    pub semantic_index_offset: i32,

    /// A [`SemanticIndexCallback`] which is used when
    /// [`add_semantic_indexes`](Self::add_semantic_indexes) is true.
    ///
    /// Defaults to providing an index for each widget.
    pub semantic_index_callback: SemanticIndexCallback,

    /// Called to find the new index of a child based on its key in case of reordering.
    ///
    /// If not provided, a child widget may not map to its existing `RenderObject` when the
    /// order of children returned from the children builder changes. This may result in
    /// state-loss.
    ///
    /// This callback should take an input `Key`, and it should return the index of the child
    /// element with that associated key, or `None` if not found.
    pub find_child_index_callback: Option<ChildIndexGetter>,
}

impl SliverChildBuilderDelegate {
    /// Creates a delegate that supplies children for slivers using the given builder callback.
    ///
    /// If the order in which the builder returns children ever changes, consider providing a
    /// [`find_child_index_callback`](Self::find_child_index_callback). This allows the delegate
    /// to find the new index for a child that was previously located at a different index to
    /// attach the existing state to the widget at its new location.
    pub fn new(
        builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
    ) -> SliverChildBuilderDelegate {
        SliverChildBuilderDelegate::from_builder(Rc::new(builder))
    }

    /// Creates the delegate from a shared builder, which the `builder` constructors of the
    /// sliver widgets pass on.
    pub fn from_builder(builder: NullableIndexedWidgetBuilder) -> SliverChildBuilderDelegate {
        SliverChildBuilderDelegate {
            builder,
            child_count: None,
            add_automatic_keep_alives: true,
            add_repaint_boundaries: true,
            add_semantic_indexes: true,
            semantic_index_offset: 0,
            semantic_index_callback: default_semantic_index_callback(),
            find_child_index_callback: None,
        }
    }

    /// Dart `SliverChildBuilderDelegate(childCount:)`.
    pub fn child_count(mut self, child_count: i32) -> SliverChildBuilderDelegate {
        self.child_count = Some(child_count);
        self
    }

    /// Dart `SliverChildBuilderDelegate(addAutomaticKeepAlives:)`.
    pub fn add_automatic_keep_alives(mut self, value: bool) -> SliverChildBuilderDelegate {
        self.add_automatic_keep_alives = value;
        self
    }

    /// Dart `SliverChildBuilderDelegate(addRepaintBoundaries:)`.
    pub fn add_repaint_boundaries(mut self, value: bool) -> SliverChildBuilderDelegate {
        self.add_repaint_boundaries = value;
        self
    }

    /// Dart `SliverChildBuilderDelegate(addSemanticIndexes:)`.
    pub fn add_semantic_indexes(mut self, value: bool) -> SliverChildBuilderDelegate {
        self.add_semantic_indexes = value;
        self
    }

    /// Dart `SliverChildBuilderDelegate(semanticIndexOffset:)`.
    pub fn semantic_index_offset(mut self, value: i32) -> SliverChildBuilderDelegate {
        self.semantic_index_offset = value;
        self
    }

    /// Dart `SliverChildBuilderDelegate(semanticIndexCallback:)`.
    pub fn semantic_index_callback(
        mut self,
        value: SemanticIndexCallback,
    ) -> SliverChildBuilderDelegate {
        self.semantic_index_callback = value;
        self
    }

    /// Dart `SliverChildBuilderDelegate(findChildIndexCallback:)`.
    pub fn find_child_index_callback(
        mut self,
        value: ChildIndexGetter,
    ) -> SliverChildBuilderDelegate {
        self.find_child_index_callback = Some(value);
        self
    }

    /// The delegate as the shared reference a sliver widget takes.
    pub fn into_delegate(self) -> SliverChildDelegateRef {
        Rc::new(self)
    }
}

impl SliverChildDelegate for SliverChildBuilderDelegate {
    fn find_index_by_key(&self, key: &KeyRef) -> Option<i32> {
        let find_child_index_callback = self.find_child_index_callback.as_ref()?;
        find_child_index_callback(&unsalt(key))
    }

    fn build(&self, app: &mut App, context: BuildContext, index: i32) -> Option<WidgetRef> {
        if index < 0 || self.child_count.is_some_and(|count| index >= count) {
            return None;
        }
        // A panic in the builder unwinds; there is no `ErrorWidget`.
        let child = (self.builder)(app, context, index)?;
        Some(wrap_child(
            child,
            index,
            self.add_automatic_keep_alives,
            self.add_repaint_boundaries,
            self.add_semantic_indexes,
            &self.semantic_index_callback,
            self.semantic_index_offset,
        ))
    }

    fn estimated_child_count(&self) -> Option<i32> {
        self.child_count
    }

    fn should_rebuild(&self, old_delegate: &dyn SliverChildDelegate) -> bool {
        let _ = old_delegate;
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for SliverChildBuilderDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        self.debug_fill_description(&mut description);
        write!(f, "SliverChildBuilderDelegate({})", description.join(", "))
    }
}

/// A delegate that supplies children for slivers using an explicit list.
///
/// Many slivers lazily construct their box children to avoid creating more children than are
/// visible through the `Viewport`. This delegate provides children using an explicit list, which
/// is convenient but reduces the benefit of building children lazily.
///
/// In general building all the widgets in advance is not efficient. It is better to create a
/// delegate that builds them on demand using [`SliverChildBuilderDelegate`] or by implementing
/// [`SliverChildDelegate`] directly.
///
/// This type is provided for the cases where either the list of children is known well in
/// advance, and therefore will not be built each time the delegate itself is created, or the
/// list is small, such that it's likely always visible (and thus there is nothing to be gained
/// by building it on demand). For example, the body of a dialog box might fit both of these
/// conditions.
///
/// The widgets in the given [`children`](Self::children) list are automatically wrapped in
/// `AutomaticKeepAlive` widgets if
/// [`add_automatic_keep_alives`](Self::add_automatic_keep_alives) is true (the default) and in
/// [`RepaintBoundary`] widgets if [`add_repaint_boundaries`](Self::add_repaint_boundaries) is
/// true (also the default).
///
/// ## Accessibility
///
/// `CustomScrollView` requires that its semantic children are annotated using
/// `IndexedSemantics`. This is done by default in the delegate with the
/// [`add_semantic_indexes`](Self::add_semantic_indexes) parameter set to true.
///
/// See also:
///
///  * [`SliverChildBuilderDelegate`], which is a delegate that uses a builder callback to
///    construct the children.
pub struct SliverChildListDelegate {
    /// Whether to wrap each child in an `AutomaticKeepAlive`.
    pub add_automatic_keep_alives: bool,

    /// Whether to wrap each child in a [`RepaintBoundary`].
    pub add_repaint_boundaries: bool,

    /// Whether to wrap each child in an `IndexedSemantics`.
    pub add_semantic_indexes: bool,

    /// An initial offset to add to the semantic indexes generated by this widget.
    pub semantic_index_offset: i32,

    /// A [`SemanticIndexCallback`] which is used when
    /// [`add_semantic_indexes`](Self::add_semantic_indexes) is true.
    pub semantic_index_callback: SemanticIndexCallback,

    /// The widgets to display.
    ///
    /// If this list is going to be mutated, it is usually wise to put a `Key` on each of the
    /// child widgets, so that the framework can match old configurations to new configurations
    /// and maintain the underlying render objects.
    ///
    /// A widget is immutable, so a new list must be provided whenever the children change.
    pub children: Vec<WidgetRef>,

    // A map to cache key to index lookup for children, and the index the lazy fill has reached.
    //
    // `None` for the constant instance (Dart's `SliverChildListDelegate.fixed`), whose children
    // never move.
    key_to_index: Option<RefCell<KeyToIndex>>,
}

/// The lazily filled key-to-index cache of a [`SliverChildListDelegate`].
///
/// Dart keeps the fill cursor under the map's `null` key; it is a field here.
#[derive(Default)]
struct KeyToIndex {
    map: HashMap<KeyRef, i32>,
    /// The index [`SliverChildListDelegate::find_child_index`] has scanned up to.
    cursor: usize,
}

impl SliverChildListDelegate {
    /// Creates a delegate that supplies children for slivers using the given list.
    ///
    /// If the order of children never changes, consider using [`fixed`](Self::fixed).
    pub fn new(children: Vec<WidgetRef>) -> SliverChildListDelegate {
        SliverChildListDelegate {
            add_automatic_keep_alives: true,
            add_repaint_boundaries: true,
            add_semantic_indexes: true,
            semantic_index_offset: 0,
            semantic_index_callback: default_semantic_index_callback(),
            children,
            key_to_index: Some(RefCell::default()),
        }
    }

    /// Creates a constant version of the delegate that supplies children for slivers using the
    /// given list.
    ///
    /// If the order of the children will change, consider using [`new`](Self::new).
    pub fn fixed(children: Vec<WidgetRef>) -> SliverChildListDelegate {
        SliverChildListDelegate {
            key_to_index: None,
            ..SliverChildListDelegate::new(children)
        }
    }

    /// Dart `SliverChildListDelegate(addAutomaticKeepAlives:)`.
    pub fn add_automatic_keep_alives(mut self, value: bool) -> SliverChildListDelegate {
        self.add_automatic_keep_alives = value;
        self
    }

    /// Dart `SliverChildListDelegate(addRepaintBoundaries:)`.
    pub fn add_repaint_boundaries(mut self, value: bool) -> SliverChildListDelegate {
        self.add_repaint_boundaries = value;
        self
    }

    /// Dart `SliverChildListDelegate(addSemanticIndexes:)`.
    pub fn add_semantic_indexes(mut self, value: bool) -> SliverChildListDelegate {
        self.add_semantic_indexes = value;
        self
    }

    /// Dart `SliverChildListDelegate(semanticIndexOffset:)`.
    pub fn semantic_index_offset(mut self, value: i32) -> SliverChildListDelegate {
        self.semantic_index_offset = value;
        self
    }

    /// Dart `SliverChildListDelegate(semanticIndexCallback:)`.
    pub fn semantic_index_callback(
        mut self,
        value: SemanticIndexCallback,
    ) -> SliverChildListDelegate {
        self.semantic_index_callback = value;
        self
    }

    /// The delegate as the shared reference a sliver widget takes.
    pub fn into_delegate(self) -> SliverChildDelegateRef {
        Rc::new(self)
    }

    /// Flutter's `_findChildIndex`.
    fn find_child_index(&self, key: &KeyRef) -> Option<i32> {
        // Dart's `_isConstantInstance`.
        let cache = self.key_to_index.as_ref()?;
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.map.get(key) {
            return Some(*index);
        }
        // Lazily fill the cache.
        let mut index = cache.cursor;
        while index < self.children.len() {
            let child = &self.children[index];
            if let Some(child_key) = child.key() {
                cache.map.insert(child_key.clone(), index as i32);
                if child_key == key {
                    // Record current index for next function call.
                    cache.cursor = index + 1;
                    return Some(index as i32);
                }
            }
            index += 1;
        }
        cache.cursor = index;
        None
    }
}

impl SliverChildDelegate for SliverChildListDelegate {
    fn find_index_by_key(&self, key: &KeyRef) -> Option<i32> {
        self.find_child_index(&unsalt(key))
    }

    fn build(&self, _app: &mut App, _context: BuildContext, index: i32) -> Option<WidgetRef> {
        let slot = usize::try_from(index).ok()?;
        let child = self.children.get(slot)?.clone();
        Some(wrap_child(
            child,
            index,
            self.add_automatic_keep_alives,
            self.add_repaint_boundaries,
            self.add_semantic_indexes,
            &self.semantic_index_callback,
            self.semantic_index_offset,
        ))
    }

    fn estimated_child_count(&self) -> Option<i32> {
        Some(self.children.len() as i32)
    }

    fn should_rebuild(&self, old_delegate: &dyn SliverChildDelegate) -> bool {
        let old = old_delegate
            .as_any()
            .downcast_ref::<SliverChildListDelegate>()
            .expect("the caller compared the delegate types first");
        self.children.len() != old.children.len()
            || std::iter::zip(&self.children, &old.children).any(|(a, b)| !Rc::ptr_eq(a, b))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for SliverChildListDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        self.debug_fill_description(&mut description);
        write!(f, "SliverChildListDelegate({})", description.join(", "))
    }
}
