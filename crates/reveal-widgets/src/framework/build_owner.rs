//! `BuildScope`, `BuildOwner`, and the inactive-element list.

use std::collections::{HashMap, HashSet};

use reveal_foundation::{App, Handle, Listener};

use super::element::{AnyElement, ElementLifecycle};
use super::widget::GlobalKeyId;
use crate::widgets::focus_manager::FocusManager;

/// A class that determines the scope of a `BuildOwner::build_scope` operation.
///
/// The `BuildOwner::build_scope` method rebuilds all dirty `Element`s who share the same
/// [`BuildScope`] with its `context` argument, and skips those with a different
/// `Element::build_scope`.
///
/// `Element`s by default have the same [`BuildScope`] as their parents. Special `Element`s
/// may override `Element::build_scope` to create an isolated build scope for its descendants.
/// The `LayoutBuilder` widget, for example, establishes its own [`BuildScope`] such that no
/// descendant `Element`s may rebuild prematurely until the incoming constraints are known.
pub struct BuildScope {
    build_scheduled: bool,
    building: bool,
    /// An optional `VoidCallback` that will be called when `Element`s in this [`BuildScope`]
    /// are marked as dirty for the first time.
    ///
    /// This callback usually signifies that the `BuildOwner::build_scope` method must be
    /// called at a later time in this frame to rebuild dirty elements in this
    /// [`BuildScope`]. It will **not** be called if this scope is actively being built by
    /// `BuildOwner::build_scope`, since the [`BuildScope`] will be clean when
    /// `BuildOwner::build_scope` returns.
    schedule_rebuild: Option<Listener>,
    /// Whether `dirty_elements` need to be sorted again as a result of more elements
    /// becoming dirty during the build.
    ///
    /// This is necessary to preserve the sort order defined by `Element::sort`.
    ///
    /// This field is set to `None` when `BuildOwner::build_scope` is not actively rebuilding
    /// the widget tree.
    dirty_elements_needs_resorting: Option<bool>,
    dirty_elements: Vec<AnyElement>,
}

impl BuildScope {
    /// Creates a [`BuildScope`] with an optional `schedule_rebuild` callback.
    pub fn new(app: &mut App, schedule_rebuild: Option<Listener>) -> Handle<BuildScope> {
        app.create(BuildScope {
            build_scheduled: false,
            building: false,
            schedule_rebuild,
            dirty_elements_needs_resorting: None,
            dirty_elements: Vec::new(),
        })
    }

    /// The elements marked dirty in this scope, in scheduling order.
    pub fn dirty_elements(self: Handle<Self>, app: &App) -> &[AnyElement] {
        &app.get(self).dirty_elements
    }

    pub(crate) fn schedule_build_for(self: Handle<Self>, app: &mut App, element: AnyElement) {
        debug_assert!(element.build_scope(app) == self);
        if !element.in_dirty_list(app) {
            app.get_mut(self).dirty_elements.push(element);
            element.data_mut(app).in_dirty_list = true;
        }
        let scope = app.get(self);
        if !scope.build_scheduled && !scope.building {
            app.get_mut(self).build_scheduled = true;
            if let Some(schedule_rebuild) = app.get(self).schedule_rebuild.clone() {
                schedule_rebuild.call(app);
            }
        }
        if app.get(self).dirty_elements_needs_resorting.is_some() {
            app.get_mut(self).dirty_elements_needs_resorting = Some(true);
        }
    }

    fn try_rebuild(self: Handle<Self>, app: &mut App, element: AnyElement) {
        debug_assert!(element.in_dirty_list(app));
        debug_assert!(element.build_scope(app) == self);
        element.rebuild(app, false);
    }

    fn debug_assert_element_in_scope(
        app: &App,
        element: AnyElement,
        debug_build_root: AnyElement,
    ) -> bool {
        let is_in_scope =
            element.debug_is_descendant_of(app, debug_build_root) || !element.debug_is_active(app);
        assert!(
            is_in_scope,
            "Tried to build dirty widget in the wrong build scope. A widget which was marked \
             as dirty and is still active was scheduled to be built, but the current build \
             scope unexpectedly does not contain that widget."
        );
        true
    }

    /// Rebuilds every dirty element of this scope, in depth order, resorting when a rebuild
    /// dirties more.
    pub(crate) fn flush_dirty_elements(
        self: Handle<Self>,
        app: &mut App,
        debug_build_root: AnyElement,
    ) {
        debug_assert!(
            app.get(self).dirty_elements_needs_resorting.is_none(),
            "_flushDirtyElements must be non-reentrant"
        );
        sort_dirty_elements(app, self);
        app.get_mut(self).dirty_elements_needs_resorting = Some(false);
        let mut index = 0;
        while index < app.get(self).dirty_elements.len() {
            let element = app.get(self).dirty_elements[index];
            if element.build_scope(app) == self {
                debug_assert!(BuildScope::debug_assert_element_in_scope(
                    app,
                    element,
                    debug_build_root
                ));
                self.try_rebuild(app, element);
            }
            index = self.dirty_element_index_after(app, index);
        }
        if cfg!(debug_assertions) {
            let missed: Vec<AnyElement> = app
                .get(self)
                .dirty_elements
                .iter()
                .copied()
                .filter(|element| {
                    element.debug_is_active(app)
                        && element.dirty(app)
                        && element.build_scope(app) == self
                })
                .collect();
            assert!(
                missed.is_empty(),
                "buildScope missed some dirty elements. This probably indicates that the \
                 dirty list should have been resorted but was not: {missed:?}"
            );
        }
        let dirty_elements = std::mem::take(&mut app.get_mut(self).dirty_elements);
        for element in dirty_elements {
            if element.build_scope(app) == self {
                element.data_mut(app).in_dirty_list = false;
            }
        }
        let scope = app.get_mut(self);
        scope.dirty_elements_needs_resorting = None;
        scope.build_scheduled = false;
    }

    fn dirty_element_index_after(self: Handle<Self>, app: &mut App, index: usize) -> usize {
        if app.get(self).dirty_elements_needs_resorting != Some(true) {
            return index + 1;
        }
        let mut index = index + 1;
        sort_dirty_elements(app, self);
        app.get_mut(self).dirty_elements_needs_resorting = Some(false);
        while index > 0 && app.get(self).dirty_elements[index - 1].dirty(app) {
            index -= 1;
        }
        if cfg!(debug_assertions) {
            for i in (0..index).rev() {
                let element = app.get(self).dirty_elements[i];
                debug_assert!(
                    !element.dirty(app) || element.lifecycle(app) != ElementLifecycle::Active
                );
            }
        }
        index
    }

    pub(crate) fn set_building(self: Handle<Self>, app: &mut App, building: bool) {
        app.get_mut(self).building = building;
    }
}

/// Dart's `_dirtyElements.sort(Element._sort)`: by depth, dirty last within a depth.
fn sort_dirty_elements(app: &mut App, scope: Handle<BuildScope>) {
    let mut keyed: Vec<(u32, bool, AnyElement)> = app
        .get(scope)
        .dirty_elements
        .iter()
        .map(|element| (element.depth(app), element.dirty(app), *element))
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    app.get_mut(scope).dirty_elements = keyed.into_iter().map(|(_, _, element)| element).collect();
}

/// The elements removed from the tree during this frame, waiting to be reactivated by a
/// global key or unmounted by `BuildOwner::finalize_tree`.
#[derive(Default)]
struct InactiveElements {
    locked: bool,
    elements: HashSet<AnyElement>,
}

/// The callback `BuildOwner::build_scope` runs before building the dirty elements.
pub type BuildScopeCallback = Box<dyn FnOnce(&mut App)>;

/// Manager class for the widgets framework.
///
/// This class tracks which widgets need rebuilding, and handles other tasks that apply to
/// widget trees as a whole, such as managing the inactive element list for the tree and
/// triggering the "reassemble" command when necessary during hot reload when debugging.
///
/// The main build owner is typically owned by the `WidgetsBinding`, and is driven from the
/// operating system along with the rest of the build/layout/paint pipeline.
///
/// Additional build owners can be built to manage off-screen widget trees.
///
/// To assign a build owner to a tree, use the `RootElementMixin::assign_owner` method on the
/// root element of the widget tree.
pub struct BuildOwner {
    /// Called on each build pass when the first buildable element is marked dirty.
    on_build_scheduled: Option<Listener>,
    focus_manager: Handle<FocusManager>,
    inactive_elements: InactiveElements,
    scheduled_flush_dirty_elements: bool,
    global_key_registry: HashMap<GlobalKeyId, AnyElement>,
    debug_state_lock_level: i32,
    debug_building: bool,
    debug_current_build_target: Option<AnyElement>,
}

impl BuildOwner {
    /// Creates an object that manages widgets.
    ///
    /// This constructs a new [`FocusManager`] and registers its global input handlers via
    /// [`FocusManager::register_global_handlers`], which will modify static state. Callers
    /// wishing to avoid altering this state can explicitly pass a focus manager to
    /// [`with_focus_manager`](Self::with_focus_manager), Dart's `focusManager` argument.
    pub fn new(app: &mut App, on_build_scheduled: Option<Listener>) -> Handle<BuildOwner> {
        let focus_manager = FocusManager::new(app);
        focus_manager.register_global_handlers(app);
        BuildOwner::with_focus_manager(app, on_build_scheduled, focus_manager)
    }

    /// Creates an object that manages widgets, with the [`FocusManager`] Dart's `focusManager`
    /// argument supplies; its global input handlers are not registered.
    pub fn with_focus_manager(
        app: &mut App,
        on_build_scheduled: Option<Listener>,
        focus_manager: Handle<FocusManager>,
    ) -> Handle<BuildOwner> {
        app.create(BuildOwner {
            on_build_scheduled,
            focus_manager,
            inactive_elements: InactiveElements::default(),
            scheduled_flush_dirty_elements: false,
            global_key_registry: HashMap::new(),
            debug_state_lock_level: 0,
            debug_building: false,
            debug_current_build_target: None,
        })
    }

    /// The object in charge of the focus tree.
    ///
    /// Rarely used directly. Instead, consider using `FocusScope::of` to obtain the
    /// `FocusScopeNode` for a given `BuildContext`.
    pub fn focus_manager(self: Handle<Self>, app: &App) -> Handle<FocusManager> {
        app.get(self).focus_manager
    }

    /// Replaces the object in charge of the focus tree (Dart's mutable `focusManager` field).
    pub fn set_focus_manager(
        self: Handle<Self>,
        app: &mut App,
        focus_manager: Handle<FocusManager>,
    ) {
        app.get_mut(self).focus_manager = focus_manager;
    }

    /// Replaces the `on_build_scheduled` callback.
    pub fn set_on_build_scheduled(self: Handle<Self>, app: &mut App, callback: Option<Listener>) {
        app.get_mut(self).on_build_scheduled = callback;
    }

    /// Adds an element to the dirty elements list so that it will be rebuilt when
    /// `WidgetsBinding::draw_frame` calls [`build_scope`](Self::build_scope).
    pub fn schedule_build_for(self: Handle<Self>, app: &mut App, element: AnyElement) {
        debug_assert!(element.owner(app) == Some(self));
        debug_assert!(element.data(app).parent_build_scope.is_some());
        debug_assert!(
            element.dirty(app),
            "scheduleBuildFor() called for a widget that is not marked as dirty."
        );
        let build_scope = element.build_scope(app);
        debug_assert!(
            app.get(self).debug_building || !element.in_dirty_list(app),
            "BuildOwner.scheduleBuildFor() called on an Element that is already in the dirty \
             list."
        );
        if !app.get(self).scheduled_flush_dirty_elements
            && let Some(on_build_scheduled) = app.get(self).on_build_scheduled.clone()
        {
            app.get_mut(self).scheduled_flush_dirty_elements = true;
            on_build_scheduled.call(app);
        }
        build_scope.schedule_build_for(app, element);
    }

    pub(crate) fn debug_state_locked(self: Handle<Self>, app: &App) -> bool {
        app.get(self).debug_state_lock_level > 0
    }

    /// Whether this widget tree is in the build phase.
    ///
    /// Only valid when asserts are enabled.
    pub fn debug_building(self: Handle<Self>, app: &App) -> bool {
        app.get(self).debug_building
    }

    pub(crate) fn debug_current_build_target(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        app.get(self).debug_current_build_target
    }

    pub(crate) fn set_debug_current_build_target(
        self: Handle<Self>,
        app: &mut App,
        target: Option<AnyElement>,
    ) {
        app.get_mut(self).debug_current_build_target = target;
    }

    /// Establishes a scope in which calls to `State::set_state` are forbidden, and calls the
    /// given `callback`.
    ///
    /// This mechanism is used to ensure that, for instance, `State::dispose` does not call
    /// `State::set_state`.
    pub fn lock_state(self: Handle<Self>, app: &mut App, callback: impl FnOnce(&mut App)) {
        debug_assert!(app.get(self).debug_state_lock_level >= 0);
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_state_lock_level += 1;
        }
        callback(app);
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_state_lock_level -= 1;
        }
        debug_assert!(app.get(self).debug_state_lock_level >= 0);
    }

    /// Establishes a scope for updating the widget tree, and calls the given `callback`, if
    /// any. Then, builds all the elements that were marked as dirty using
    /// [`schedule_build_for`](Self::schedule_build_for), in depth order.
    ///
    /// This mechanism prevents build methods from transitively requiring other build methods
    /// to run, potentially causing infinite loops.
    ///
    /// The dirty list is processed after `callback` returns, building all the elements that
    /// were marked as dirty using [`schedule_build_for`](Self::schedule_build_for), in depth
    /// order. If elements are marked as dirty while this method is running, they must be
    /// deeper than the `context` node, and deeper than any previously-built node in this
    /// pass.
    ///
    /// To flush the current dirty list without performing any other work, this function can
    /// be called with no callback. This is what the framework does each frame, in
    /// `WidgetsBinding::draw_frame`.
    ///
    /// Only one [`build_scope`](Self::build_scope) can be active at a time.
    ///
    /// A [`build_scope`](Self::build_scope) implies a [`lock_state`](Self::lock_state) scope
    /// as well.
    pub fn build_scope(
        self: Handle<Self>,
        app: &mut App,
        context: AnyElement,
        callback: Option<BuildScopeCallback>,
    ) {
        let build_scope = context.build_scope(app);
        if callback.is_none() && app.get(build_scope).dirty_elements.is_empty() {
            return;
        }
        debug_assert!(app.get(self).debug_state_lock_level >= 0);
        debug_assert!(!app.get(self).debug_building);
        if cfg!(debug_assertions) {
            let owner = app.get_mut(self);
            owner.debug_state_lock_level += 1;
            owner.debug_building = true;
        }
        app.get_mut(self).scheduled_flush_dirty_elements = true;
        build_scope.set_building(app, true);
        if let Some(callback) = callback {
            debug_assert!(self.debug_state_locked(app));
            let debug_previous_build_target = self.debug_current_build_target(app);
            if cfg!(debug_assertions) {
                self.set_debug_current_build_target(app, Some(context));
            }
            callback(app);
            if cfg!(debug_assertions) {
                debug_assert!(self.debug_current_build_target(app) == Some(context));
                self.set_debug_current_build_target(app, debug_previous_build_target);
            }
        }
        build_scope.flush_dirty_elements(app, context);
        build_scope.set_building(app, false);
        app.get_mut(self).scheduled_flush_dirty_elements = false;
        debug_assert!(app.get(self).debug_building);
        if cfg!(debug_assertions) {
            let owner = app.get_mut(self);
            owner.debug_building = false;
            owner.debug_state_lock_level -= 1;
        }
        debug_assert!(app.get(self).debug_state_lock_level >= 0);
    }

    /// The number of `GlobalKey` instances that are currently associated with `Element`s
    /// that have been built by this build owner.
    pub fn global_key_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).global_key_registry.len()
    }

    pub(crate) fn register_global_key(
        self: Handle<Self>,
        app: &mut App,
        key: GlobalKeyId,
        element: AnyElement,
    ) {
        if cfg!(debug_assertions)
            && let Some(old_element) = app.get(self).global_key_registry.get(&key).copied()
        {
            debug_assert!(
                element.widget(app).widget_type() != old_element.widget(app).widget_type(),
                "Multiple widgets used the same GlobalKey: {key:?}"
            );
        }
        app.get_mut(self).global_key_registry.insert(key, element);
    }

    pub(crate) fn unregister_global_key(
        self: Handle<Self>,
        app: &mut App,
        key: GlobalKeyId,
        element: AnyElement,
    ) {
        if app.get(self).global_key_registry.get(&key) == Some(&element) {
            app.get_mut(self).global_key_registry.remove(&key);
        }
    }

    /// The element registered under a global key, if any.
    pub fn global_key_element(
        self: Handle<Self>,
        app: &App,
        key: GlobalKeyId,
    ) -> Option<AnyElement> {
        app.get(self).global_key_registry.get(&key).copied()
    }

    // ---- the inactive-element list (Dart's `_InactiveElements`) ----

    pub(crate) fn inactive_add(self: Handle<Self>, app: &mut App, element: AnyElement) {
        debug_assert!(!app.get(self).inactive_elements.locked);
        debug_assert!(!app.get(self).inactive_elements.elements.contains(&element));
        debug_assert!(element.parent(app).is_none());
        match element.lifecycle(app) {
            ElementLifecycle::Active => {
                deactivate_recursively(app, element);
                app.get_mut(self).inactive_elements.elements.insert(element);
            }
            ElementLifecycle::Inactive => {
                app.get_mut(self).inactive_elements.elements.insert(element);
            }
            state @ (ElementLifecycle::Initial
            | ElementLifecycle::Failed
            | ElementLifecycle::Defunct) => {
                panic!("{element:?} must not be deactivated when in {state:?} state.");
            }
        }
    }

    pub(crate) fn inactive_remove(self: Handle<Self>, app: &mut App, element: AnyElement) {
        debug_assert!(!app.get(self).inactive_elements.locked);
        debug_assert!(app.get(self).inactive_elements.elements.contains(&element));
        debug_assert!(element.parent(app).is_none());
        app.get_mut(self)
            .inactive_elements
            .elements
            .remove(&element);
        debug_assert!(element.lifecycle(app) == ElementLifecycle::Inactive);
    }

    /// Whether `element` is on the inactive list; for tests.
    pub fn debug_inactive_contains(self: Handle<Self>, app: &App, element: AnyElement) -> bool {
        app.get(self).inactive_elements.elements.contains(&element)
    }

    fn inactive_unmount_all(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).inactive_elements.locked = true;
        let mut elements: Vec<AnyElement> = app
            .get_mut(self)
            .inactive_elements
            .elements
            .drain()
            .collect();
        let mut keyed: Vec<(u32, bool, AnyElement)> = elements
            .drain(..)
            .map(|element| (element.depth(app), element.dirty(app), element))
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (_, _, element) in keyed.into_iter().rev() {
            unmount_recursively(app, element);
        }
        debug_assert!(app.get(self).inactive_elements.elements.is_empty());
        app.get_mut(self).inactive_elements.locked = false;
    }

    /// Complete the element build pass by unmounting any elements that are no longer active.
    ///
    /// This is called by `WidgetsBinding::draw_frame`.
    ///
    /// In debug mode, this also runs some sanity checks, for example checking for duplicate
    /// global keys.
    pub fn finalize_tree(self: Handle<Self>, app: &mut App) {
        self.lock_state(app, |app| self.inactive_unmount_all(app));
    }
}

fn unmount_recursively(app: &mut App, element: AnyElement) {
    debug_assert!(element.lifecycle(app) == ElementLifecycle::Inactive);
    for child in element.children(app) {
        debug_assert!(child.parent(app) == Some(element));
        unmount_recursively(app, child);
    }
    element.unmount(app);
    debug_assert!(element.lifecycle(app) == ElementLifecycle::Defunct);
}

fn deactivate_recursively(app: &mut App, element: AnyElement) {
    debug_assert!(element.lifecycle(app) == ElementLifecycle::Active);
    element.deactivate(app);
    for child in element.children(app) {
        deactivate_recursively(app, child);
    }
    if cfg!(debug_assertions) {
        element.debug_deactivated(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementBase, IntoWidget, SingleChildRenderObjectElement, SizedBox};
    use reveal_foundation::AppCell;

    /// A rebuild can leave clean and dirty siblings at the same depth before a resort.
    #[test]
    fn dirty_scope_resort_keeps_clean_siblings_before_pending_builds() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let widget = SizedBox::shrink().into_widget();
        let pending = widget.create_element(&mut app, widget.clone());
        let cleaned = widget.create_element(&mut app, widget.clone());
        let focus = FocusManager::new(&mut app);
        let owner = BuildOwner::with_focus_manager(&mut app, None, focus);
        for element in [pending, cleaned] {
            element.assign_owner(&mut app, owner);
            let handle = element
                .downcast::<SingleChildRenderObjectElement<SizedBox>>(&app)
                .unwrap();
            ElementBase::mount(handle, &mut app, None, None);
        }
        pending.set_dirty(&mut app, true);
        cleaned.set_dirty(&mut app, false);
        let scope = BuildScope::new(&mut app, None);
        app.get_mut(scope).dirty_elements = vec![pending, cleaned];
        app.get_mut(scope).dirty_elements_needs_resorting = Some(true);
        let next = scope.dirty_element_index_after(&mut app, 0);
        assert_eq!(app.get(scope).dirty_elements, [cleaned, pending]);
        assert_eq!(
            next, 1,
            "the resume index must point after the last clean sibling"
        );
        assert!(app.get(scope).dirty_elements[next].dirty(&app));
    }
}
