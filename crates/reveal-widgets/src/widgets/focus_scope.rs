//! Flutter counterpart: `widgets/focus_scope.dart`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use reveal_foundation::{App, Handle, Listener, ValueChanged};

use crate::framework::{
    AnyElement, BuildContext, ComponentElement, ComponentElementData, Element, ElementData,
    ElementLifecycle, IntoWidget, KeyRef, ProxyElement, Slot, State, StateData, StatefulWidget,
    Widget, WidgetKind, WidgetRef, component_element_overrides, downcast_widget,
};
use crate::widgets::focus_manager::{
    AnyFocusNode, FocusAttachment, FocusNode, FocusNodeLeaf, FocusOnKeyEventCallback,
    FocusScopeNode,
};

/// A widget that manages a [`FocusNode`] to allow keyboard focus to be given to this widget and
/// its descendants.
///
/// When the focus is gained or lost, [`on_focus_change`](Self::on_focus_change) is called.
///
/// For key events, [`on_key_event`](Self::on_key_event) is called if
/// [`AnyFocusNode::has_focus`] is true for this widget's [`focus_node`](Self::focus_node),
/// unless a focused descendant's `on_key_event` callback returned
/// `KeyEventResult::Handled` when called.
///
/// This widget does not provide any visual indication that the focus has changed. Any desired
/// visual changes should be made when [`on_focus_change`](Self::on_focus_change) is called.
///
/// To access the [`FocusNode`] of the nearest ancestor [`Focus`] widget and establish a
/// relationship that will rebuild the widget when the focus changes, use the [`Focus::of`] and
/// [`FocusScope::of`] static methods.
///
/// Managing a [`FocusNode`] means managing its lifecycle, listening for changes in focus, and
/// re-parenting it when needed to keep the focus hierarchy in sync with the widget hierarchy.
/// This widget does all of those things for you.
///
/// If the [`Focus::new`] constructor is used, then this widget will manage any given
/// [`focus_node`](Self::focus_node) by overwriting the appropriate values of the node with the
/// values of this widget whenever the [`Focus`] widget is updated.
///
/// If [`Focus::with_external_focus_node`] is used instead, then the values returned by
/// [`on_key_event`](Self::on_key_event), [`skip_traversal`](Self::skip_traversal),
/// [`can_request_focus`](Self::can_request_focus), and
/// [`descendants_are_focusable`](Self::descendants_are_focusable) will be the values in the
/// external focus node, and the external focus node's values will not be overwritten when the
/// widget is updated.
///
/// To collect a sub-tree of nodes into an exclusive group that restricts focus traversal to the
/// group, use a [`FocusScope`].
pub struct Focus {
    /// See [`Widget::key`].
    pub key: Option<KeyRef>,

    /// The optional parent node to use when reparenting the
    /// [`focus_node`](Self::focus_node) for this [`Focus`] widget.
    ///
    /// If [`parent_node`](Self::parent_node) is `None`, then [`Focus::maybe_of`] is used to
    /// find the parent in the widget tree, which is typically what is desired, since it is
    /// easier to reason about the focus tree if it mirrors the shape of the widget tree.
    ///
    /// Set this property if the focus tree needs to have a different shape than the widget
    /// tree. This is typically in cases where a dialog is in an `Overlay` (or another part of
    /// the widget tree), and focus should behave as if the widgets in the overlay are
    /// descendants of the given [`parent_node`](Self::parent_node) for purposes of focus.
    ///
    /// Defaults to `None`.
    pub parent_node: Option<AnyFocusNode>,

    /// The child widget of this [`Focus`].
    pub child: WidgetRef,

    /// An optional focus node to use as the focus node for this widget.
    ///
    /// If one is not supplied, then one will be automatically allocated, owned, and managed by
    /// this widget. The widget will be focusable even if a
    /// [`focus_node`](Self::focus_node) is not supplied. If supplied, the given node will be
    /// _hosted_ by this widget, but not owned. See [`FocusNode`] for more information on what
    /// being hosted and/or owned implies.
    ///
    /// A non-null [`focus_node`](Self::focus_node) must be supplied if using the
    /// [`Focus::with_external_focus_node`] constructor.
    pub focus_node: Option<AnyFocusNode>,

    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    ///
    /// Ideally, there is only one widget with autofocus set in each [`FocusScope`]. If there is
    /// more than one widget with autofocus set, then the first one added to the tree will get
    /// focus.
    ///
    /// Defaults to false.
    pub autofocus: bool,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// Include semantics information in this widget.
    ///
    /// Defaults to true. Nothing reads it yet: the `Semantics` wrapper waits with
    /// accessibility.
    pub include_semantics: bool,

    /// Dart's `Focus._usingExternalFocus`: whether the widget's focus node attributes should
    /// have priority when the widget is updated.
    using_external_focus: bool,

    /// Dart's `Focus._onKeyEvent`.
    on_key_event: Option<FocusOnKeyEventCallback>,

    /// Dart's `Focus._canRequestFocus`.
    can_request_focus: Option<bool>,

    /// Dart's `Focus._skipTraversal`.
    skip_traversal: Option<bool>,

    /// Dart's `Focus._descendantsAreFocusable`.
    descendants_are_focusable: Option<bool>,

    /// Dart's `Focus._descendantsAreTraversable`.
    descendants_are_traversable: Option<bool>,

    /// Dart's `Focus._debugLabel`.
    debug_label: Option<String>,
}

impl Focus {
    /// Creates a widget that manages a [`FocusNode`].
    pub fn new<K>(child: impl IntoWidget<K>) -> Focus {
        Focus {
            key: None,
            parent_node: None,
            child: child.into_widget(),
            focus_node: None,
            autofocus: false,
            on_focus_change: None,
            include_semantics: true,
            using_external_focus: false,
            on_key_event: None,
            can_request_focus: None,
            skip_traversal: None,
            descendants_are_focusable: None,
            descendants_are_traversable: None,
            debug_label: None,
        }
    }

    /// Creates a [`Focus`] widget that uses the given `focus_node` as the source of truth for
    /// attributes on the node, rather than the attributes of this widget.
    pub fn with_external_focus_node<K>(
        child: impl IntoWidget<K>,
        focus_node: AnyFocusNode,
    ) -> Focus {
        Focus {
            focus_node: Some(focus_node),
            using_external_focus: true,
            ..Focus::new(child)
        }
    }

    /// Dart `Focus(key:)`.
    pub fn key(mut self, key: KeyRef) -> Focus {
        self.key = Some(key);
        self
    }

    /// Dart `Focus(parentNode:)`.
    pub fn parent_node(mut self, parent_node: AnyFocusNode) -> Focus {
        self.parent_node = Some(parent_node);
        self
    }

    /// Dart `Focus(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> Focus {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `Focus(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> Focus {
        self.autofocus = autofocus;
        self
    }

    /// Dart `Focus(onFocusChange:)`.
    pub fn on_focus_change(mut self, on_focus_change: impl Fn(&mut App, bool) + 'static) -> Focus {
        self.on_focus_change = Some(Rc::new(on_focus_change));
        self
    }

    /// Dart `Focus(onKeyEvent:)`.
    pub fn on_key_event(mut self, on_key_event: FocusOnKeyEventCallback) -> Focus {
        self.on_key_event = Some(on_key_event);
        self
    }

    /// Dart `Focus(canRequestFocus:)`.
    pub fn can_request_focus(mut self, can_request_focus: bool) -> Focus {
        self.can_request_focus = Some(can_request_focus);
        self
    }

    /// Dart `Focus(skipTraversal:)`.
    pub fn skip_traversal(mut self, skip_traversal: bool) -> Focus {
        self.skip_traversal = Some(skip_traversal);
        self
    }

    /// Dart `Focus(descendantsAreFocusable:)`.
    pub fn descendants_are_focusable(mut self, descendants_are_focusable: bool) -> Focus {
        self.descendants_are_focusable = Some(descendants_are_focusable);
        self
    }

    /// Dart `Focus(descendantsAreTraversable:)`.
    pub fn descendants_are_traversable(mut self, descendants_are_traversable: bool) -> Focus {
        self.descendants_are_traversable = Some(descendants_are_traversable);
        self
    }

    /// Dart `Focus(includeSemantics:)`.
    pub fn include_semantics(mut self, include_semantics: bool) -> Focus {
        self.include_semantics = include_semantics;
        self
    }

    /// Dart `Focus(debugLabel:)`.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> Focus {
        self.debug_label = Some(debug_label.into());
        self
    }

    // ---- the getters `_FocusState` reads; a `withExternalFocusNode` widget leaves every
    // private field unset, so the node is already the source of truth ----

    /// A handler for keys that are pressed when this object or one of its children has focus.
    ///
    /// Key events are first given to the [`FocusNode`] that has primary focus, and if its
    /// `on_key_event` returns `KeyEventResult::Ignored`, then they are given to each ancestor
    /// node up the focus hierarchy in turn. If an event reaches the root of the hierarchy, it
    /// is discarded.
    pub fn get_on_key_event(&self, app: &App) -> Option<FocusOnKeyEventCallback> {
        self.on_key_event
            .clone()
            .or_else(|| self.focus_node.and_then(|node| node.on_key_event(app)))
    }

    /// If true, this widget may request the primary focus.
    ///
    /// Defaults to true. Set to false if you want the [`FocusNode`] this widget manages to do
    /// nothing when [`AnyFocusNode::request_focus`] is called on it. Does not affect the
    /// children of this node, and [`AnyFocusNode::has_focus`] can still return true if this
    /// node is the ancestor of the primary focus.
    pub fn get_can_request_focus(&self, app: &mut App) -> bool {
        let (explicit, node) = (self.can_request_focus, self.focus_node);
        Focus::resolve_can_request_focus(explicit, node, app)
    }

    /// The body of [`get_can_request_focus`](Self::get_can_request_focus), over copies of the
    /// two fields it reads: reading the node needs `&mut App`, which a borrow of the widget
    /// out of the arena would conflict with.
    fn resolve_can_request_focus(
        explicit: Option<bool>,
        node: Option<AnyFocusNode>,
        app: &mut App,
    ) -> bool {
        explicit
            .or_else(|| node.map(|node| node.can_request_focus(app)))
            .unwrap_or(true)
    }

    /// Sets the [`AnyFocusNode::skip_traversal`] flag on the focus node so that it won't be
    /// visited by the `FocusTraversalPolicy`.
    pub fn get_skip_traversal(&self, app: &mut App) -> bool {
        let (explicit, node) = (self.skip_traversal, self.focus_node);
        Focus::resolve_skip_traversal(explicit, node, app)
    }

    /// The body of [`get_skip_traversal`](Self::get_skip_traversal); see
    /// [`resolve_can_request_focus`](Self::resolve_can_request_focus).
    fn resolve_skip_traversal(
        explicit: Option<bool>,
        node: Option<AnyFocusNode>,
        app: &mut App,
    ) -> bool {
        explicit
            .or_else(|| node.map(|node| node.skip_traversal(app)))
            .unwrap_or(false)
    }

    /// If false, will make this widget's descendants unfocusable.
    ///
    /// Defaults to true. Does not affect focusability of this node (just its descendants): for
    /// that, use [`AnyFocusNode::can_request_focus`].
    pub fn get_descendants_are_focusable(&self, app: &App) -> bool {
        self.descendants_are_focusable
            .or_else(|| {
                self.focus_node
                    .map(|node| node.descendants_are_focusable(app))
            })
            .unwrap_or(true)
    }

    /// If false, will make this widget's descendants untraversable.
    ///
    /// Defaults to true. Does not affect traversability of this node (just its descendants):
    /// for that, use [`AnyFocusNode::skip_traversal`].
    pub fn get_descendants_are_traversable(&self, app: &App) -> bool {
        self.descendants_are_traversable
            .or_else(|| {
                self.focus_node
                    .map(|node| node.descendants_are_traversable(app))
            })
            .unwrap_or(true)
    }

    /// A debug label for this widget.
    pub fn get_debug_label(&self, app: &App) -> Option<String> {
        self.debug_label
            .clone()
            .or_else(|| self.focus_node.and_then(|node| node.debug_label(app)))
    }

    /// Returns the focus node of the [`Focus`] that most tightly encloses the given
    /// `BuildContext`.
    ///
    /// If no [`Focus`] node is found before reaching the nearest [`FocusScope`] widget, or
    /// there is no [`Focus`] widget in the context, then this method panics.
    ///
    /// If `create_dependency` is true, calling this function creates a dependency that will
    /// rebuild the given context when the focus node gains or loses focus.
    pub fn of(
        app: &mut App,
        context: BuildContext,
        scope_ok: bool,
        create_dependency: bool,
    ) -> AnyFocusNode {
        let node = Focus::maybe_of(app, context, scope_ok, create_dependency);
        let node = node.expect(
            "Focus.of() was called with a context that does not contain a Focus widget.\n\
             No Focus widget ancestor could be found starting from the context that was passed \
             to Focus.of(). This can happen because you are using a widget that looks for a \
             Focus ancestor, and do not have a Focus widget descendant in the nearest \
             FocusScope.",
        );
        debug_assert!(
            scope_ok || node.as_scope().is_none(),
            "Focus.of() was called with a context that does not contain a Focus between the \
             given context and the nearest FocusScope widget."
        );
        node
    }

    /// Returns the focus node of the [`Focus`] that most tightly encloses the given
    /// `BuildContext`.
    ///
    /// If no [`Focus`] node is found before reaching the nearest [`FocusScope`] widget, or
    /// there is no [`Focus`] widget in scope, then this method returns `None`.
    ///
    /// If `create_dependency` is true, calling this function creates a dependency that will
    /// rebuild the given context when the focus node gains or loses focus.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
        scope_ok: bool,
        create_dependency: bool,
    ) -> Option<AnyFocusNode> {
        let node = if create_dependency {
            context
                .depend_on_inherited_widget_of_exact_type::<FocusInheritedScope>(app)
                .map(|scope| scope.node)
        } else {
            context
                .get_inherited_widget_of_exact_type::<FocusInheritedScope>(app)
                .map(|scope| scope.node)
        }?;
        match node.as_scope() {
            Some(_) if !scope_ok => None,
            _ => Some(node),
        }
    }

    /// Returns true if the nearest enclosing [`Focus`] widget's node is focused.
    ///
    /// A convenience method to allow build methods to write `Focus::is_at(app, context)` to get
    /// whether or not the nearest [`Focus`] above them in the widget hierarchy currently has
    /// the input focus.
    ///
    /// Returns false if no [`Focus`] widget is found before reaching the nearest
    /// [`FocusScope`], or if the root of the focus tree is reached without finding a [`Focus`]
    /// widget.
    ///
    /// Calling this function creates a dependency that will rebuild the given context when the
    /// focus changes.
    pub fn is_at(app: &mut App, context: BuildContext) -> bool {
        match Focus::maybe_of(app, context, false, true) {
            Some(node) => node.has_focus(app),
            None => false,
        }
    }
}

impl fmt::Debug for Focus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Focus")
            .field("debugLabel", &self.debug_label)
            .field("autofocus", &self.autofocus)
            .field("focusNode", &self.focus_node)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Focus {
    type State = FocusState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> FocusState {
        FocusState {
            state: StateData::new(),
            focus: FocusStateData::new(),
        }
    }
}

/// Dart's private members of `_FocusState`; both [`FocusState`] and [`FocusScopeState`] carry
/// this bag under the field `focus`.
pub struct FocusStateData {
    internal_node: Option<AnyFocusNode>,
    had_primary_focus: bool,
    could_request_focus: bool,
    descendants_were_focusable: bool,
    descendants_were_traversable: bool,
    did_autofocus: bool,
    focus_attachment: Option<Handle<FocusAttachment>>,
}

impl FocusStateData {
    /// A state that has not run `init_state` yet.
    pub fn new() -> FocusStateData {
        FocusStateData {
            internal_node: None,
            had_primary_focus: false,
            could_request_focus: false,
            descendants_were_focusable: false,
            descendants_were_traversable: false,
            did_autofocus: false,
            focus_attachment: None,
        }
    }
}

impl Default for FocusStateData {
    fn default() -> FocusStateData {
        FocusStateData::new()
    }
}

/// The shared bodies of Dart's `_FocusState`, which `_FocusScopeState` extends.
///
/// [`create_node`](Self::create_node) is the only virtual; the rest are Dart's `_FocusState`
/// members, and each state's `State` hooks call them where Dart's inherited body would run.
pub trait FocusStateLeaf: State + Sized + 'static {
    /// Dart's `_FocusState` fields, held under the field `focus`.
    fn focus_state_data(self: Handle<Self>, app: &App) -> &FocusStateData;

    /// See [`focus_state_data`](Self::focus_state_data).
    fn focus_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut FocusStateData;

    /// The `Focus` fields of this state's widget: the widget itself for [`Focus`], the
    /// superclass bag for [`FocusScope`].
    fn focus_widget(self: Handle<Self>, app: &App) -> &Focus;

    /// Dart's `_FocusState._createNode`.
    fn create_node(self: Handle<Self>, app: &mut App) -> AnyFocusNode;

    /// Dart's `widget.canRequestFocus`.
    fn widget_can_request_focus(self: Handle<Self>, app: &mut App) -> bool {
        let widget = self.focus_widget(app);
        let (explicit, node) = (widget.can_request_focus, widget.focus_node);
        Focus::resolve_can_request_focus(explicit, node, app)
    }

    /// Dart's `widget.skipTraversal`.
    fn widget_skip_traversal(self: Handle<Self>, app: &mut App) -> bool {
        let widget = self.focus_widget(app);
        let (explicit, node) = (widget.skip_traversal, widget.focus_node);
        Focus::resolve_skip_traversal(explicit, node, app)
    }

    /// Dart's `_FocusState.focusNode`.
    fn focus_node(self: Handle<Self>, app: &mut App) -> AnyFocusNode {
        if let Some(node) = self.focus_widget(app).focus_node {
            return node;
        }
        if let Some(node) = self.focus_state_data(app).internal_node {
            return node;
        }
        let node = self.create_node(app);
        self.focus_state_data_mut(app).internal_node = Some(node);
        node
    }

    /// Dart's `_FocusState._initNode`.
    fn init_node(self: Handle<Self>, app: &mut App) {
        let focus_node = self.focus_node(app);
        if !self.focus_widget(app).using_external_focus {
            let descendants_are_focusable =
                self.focus_widget(app).get_descendants_are_focusable(app);
            focus_node.set_descendants_are_focusable(app, descendants_are_focusable);
            let descendants_are_traversable =
                self.focus_widget(app).get_descendants_are_traversable(app);
            focus_node.set_descendants_are_traversable(app, descendants_are_traversable);
            let skip_traversal = self.widget_skip_traversal(app);
            focus_node.set_skip_traversal(app, skip_traversal);
            if let Some(can_request_focus) = self.focus_widget(app).can_request_focus {
                focus_node.set_can_request_focus(app, can_request_focus);
            }
        }
        let could_request_focus = focus_node.can_request_focus(app);
        let descendants_were_focusable = focus_node.descendants_are_focusable(app);
        let descendants_were_traversable = focus_node.descendants_are_traversable(app);
        let had_primary_focus = focus_node.has_primary_focus(app);
        let data = self.focus_state_data_mut(app);
        data.could_request_focus = could_request_focus;
        data.descendants_were_focusable = descendants_were_focusable;
        data.descendants_were_traversable = descendants_were_traversable;
        data.had_primary_focus = had_primary_focus;
        let context = self.context(app);
        let on_key_event = self.focus_widget(app).get_on_key_event(app);
        let attachment = focus_node.attach(app, Some(context), on_key_event);
        self.focus_state_data_mut(app).focus_attachment = Some(attachment);

        // Add listener even if the internal node existed before, since it should not be
        // listening now if we're re-using a previous one because it should have already
        // removed its listener.
        focus_node.add_listener(
            app,
            Listener::handle_method(self, Self::handle_focus_changed),
        );
    }

    /// Dart's `_FocusState.dispose`.
    fn dispose_focus(self: Handle<Self>, app: &mut App) {
        // Regardless of the node owner, we need to remove it from the tree and stop listening
        // to it.
        let focus_node = self.focus_node(app);
        focus_node.remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_focus_changed),
        );
        self.focus_state_data(app)
            .focus_attachment
            .expect("attached in init_state")
            .detach(app);

        // Don't manage the lifetime of external nodes given to the widget, just the internal
        // node.
        if let Some(internal_node) = self.focus_state_data(app).internal_node {
            internal_node.dispose(app);
        }
    }

    /// Dart's `_FocusState.didChangeDependencies`.
    fn did_change_dependencies_focus(self: Handle<Self>, app: &mut App) {
        if let Some(attachment) = self.focus_state_data(app).focus_attachment {
            attachment.reparent(app, None);
        }
        self.handle_autofocus(app);
    }

    /// Dart's `_FocusState._handleAutofocus`.
    fn handle_autofocus(self: Handle<Self>, app: &mut App) {
        if !self.focus_state_data(app).did_autofocus && self.focus_widget(app).autofocus {
            let context = self.context(app);
            let scope = FocusScope::of(app, context, true);
            let focus_node = self.focus_node(app);
            scope.autofocus(app, focus_node);
            self.focus_state_data_mut(app).did_autofocus = true;
        }
    }

    /// Dart's `_FocusState.deactivate`.
    ///
    /// The focus node's location in the tree is no longer valid here. But we can't unfocus or
    /// remove the node from the tree because if the widget is moved to a different part of the
    /// tree (via global key) it should retain its focus state. That's why we temporarily park
    /// it on the root focus node (via reparent) until it either gets moved to a different part
    /// of the tree (via `did_change_dependencies`) or until it is disposed.
    fn deactivate_focus(self: Handle<Self>, app: &mut App) {
        if let Some(attachment) = self.focus_state_data(app).focus_attachment {
            attachment.reparent(app, None);
        }
        self.focus_state_data_mut(app).did_autofocus = false;
    }

    /// Dart's `_FocusState.didUpdateWidget`.
    fn did_update_widget_focus(self: Handle<Self>, app: &mut App, old_widget: &Focus) {
        if cfg!(debug_assertions) {
            // Only update the debug label in debug builds.
            let widget_label = self.focus_widget(app).debug_label.clone();
            if old_widget.focus_node == self.focus_widget(app).focus_node
                && !self.focus_widget(app).using_external_focus
                && old_widget.debug_label != widget_label
            {
                let focus_node = self.focus_node(app);
                let label = self.focus_widget(app).get_debug_label(app);
                focus_node.set_debug_label(app, label);
            }
        }

        if old_widget.focus_node == self.focus_widget(app).focus_node {
            if !self.focus_widget(app).using_external_focus {
                let focus_node = self.focus_node(app);
                let on_key_event = self.focus_widget(app).get_on_key_event(app);
                let same_handler = match (&on_key_event, &focus_node.on_key_event(app)) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                    _ => false,
                };
                if !same_handler {
                    focus_node.set_on_key_event(app, on_key_event);
                }
                let skip_traversal = self.widget_skip_traversal(app);
                focus_node.set_skip_traversal(app, skip_traversal);
                if let Some(can_request_focus) = self.focus_widget(app).can_request_focus {
                    focus_node.set_can_request_focus(app, can_request_focus);
                }
                let descendants_are_focusable =
                    self.focus_widget(app).get_descendants_are_focusable(app);
                focus_node.set_descendants_are_focusable(app, descendants_are_focusable);
                let descendants_are_traversable =
                    self.focus_widget(app).get_descendants_are_traversable(app);
                focus_node.set_descendants_are_traversable(app, descendants_are_traversable);
            }
        } else {
            self.focus_state_data(app)
                .focus_attachment
                .expect("attached in init_state")
                .detach(app);
            if let Some(old_node) = old_widget.focus_node {
                old_node.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_focus_changed),
                );
            }
            self.init_node(app);
        }

        if old_widget.autofocus != self.focus_widget(app).autofocus {
            self.handle_autofocus(app);
        }
    }

    /// Dart's `_FocusState._handleFocusChanged`.
    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        let focus_node = self.focus_node(app);
        let has_primary_focus = focus_node.has_primary_focus(app);
        let can_request_focus = focus_node.can_request_focus(app);
        let descendants_are_focusable = focus_node.descendants_are_focusable(app);
        let descendants_are_traversable = focus_node.descendants_are_traversable(app);
        if let Some(on_focus_change) = self.focus_widget(app).on_focus_change.clone() {
            let has_focus = focus_node.has_focus(app);
            on_focus_change(app, has_focus);
        }
        // Check the cached states that matter here, and call set_state if they have changed.
        if self.focus_state_data(app).had_primary_focus != has_primary_focus {
            self.set_state(app, |state| {
                state.focus_state_data_field().had_primary_focus = has_primary_focus;
            });
        }
        if self.focus_state_data(app).could_request_focus != can_request_focus {
            self.set_state(app, |state| {
                state.focus_state_data_field().could_request_focus = can_request_focus;
            });
        }
        if self.focus_state_data(app).descendants_were_focusable != descendants_are_focusable {
            self.set_state(app, |state| {
                state.focus_state_data_field().descendants_were_focusable =
                    descendants_are_focusable;
            });
        }
        if self.focus_state_data(app).descendants_were_traversable != descendants_are_traversable {
            self.set_state(app, |state| {
                state.focus_state_data_field().descendants_were_traversable =
                    descendants_are_traversable;
            });
        }
    }

    /// The bag, reached from `&mut Self` inside a `set_state` callback.
    fn focus_state_data_field(&mut self) -> &mut FocusStateData;
}

/// Dart's `_FocusState`.
pub struct FocusState {
    state: StateData<Focus>,
    focus: FocusStateData,
}

impl FocusStateLeaf for FocusState {
    fn focus_state_data(self: Handle<Self>, app: &App) -> &FocusStateData {
        &app.get(self).focus
    }

    fn focus_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut FocusStateData {
        &mut app.get_mut(self).focus
    }

    fn focus_state_data_field(&mut self) -> &mut FocusStateData {
        &mut self.focus
    }

    fn focus_widget(self: Handle<Self>, app: &App) -> &Focus {
        self.widget(app)
    }

    fn create_node(self: Handle<Self>, app: &mut App) -> AnyFocusNode {
        let node = FocusNode::new(app);
        let label = self.widget(app).get_debug_label(app);
        node.set_debug_label(app, label);
        let can_request_focus = self.widget_can_request_focus(app);
        node.set_can_request_focus(app, can_request_focus);
        let descendants_are_focusable = self.widget(app).get_descendants_are_focusable(app);
        node.set_descendants_are_focusable(app, descendants_are_focusable);
        let descendants_are_traversable = self.widget(app).get_descendants_are_traversable(app);
        node.set_descendants_are_traversable(app, descendants_are_traversable);
        let skip_traversal = self.widget_skip_traversal(app);
        node.set_skip_traversal(app, skip_traversal);
        node.as_node()
    }
}

impl State for FocusState {
    type Widget = Focus;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.init_node(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_focus(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_focus(app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        self.deactivate_focus(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Focus) {
        self.did_update_widget_focus(app, old_widget);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let parent_node = self.widget(app).parent_node;
        self.focus_state_data(app)
            .focus_attachment
            .expect("attached in init_state")
            .reparent(app, parent_node);
        let child = self.widget(app).child.clone();
        // Dart wraps the child in `Semantics` when `includeSemantics` is set, and in a
        // `_DebugFocusBorder` when `debugPaintFocusBoxes` is set; both wait.
        let node = self.focus_node(app);
        FocusInheritedScope { node, child }.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// FocusScope

/// A [`FocusScope`] is similar to a [`Focus`], but also serves as a scope for its descendants,
/// restricting focus traversal to the scoped controls.
///
/// For example a new [`FocusScope`] is created automatically when a route is pushed, keeping
/// the focus traversal from moving to a control in a previous route.
///
/// Like [`Focus`], [`FocusScope`] provides an `on_focus_change` as a way to be notified when
/// the focus is given to or removed from this widget.
///
/// Managing a [`FocusScopeNode`] means managing its lifecycle, listening for changes in focus,
/// and re-parenting it when needed to keep the focus hierarchy in sync with the widget
/// hierarchy. This widget does all of those things for you.
///
/// [`FocusScopeNode`]s remember the last [`FocusNode`] that was focused within their
/// descendants.
pub struct FocusScope {
    /// Dart's `class FocusScope extends Focus`: the superclass's fields.
    focus: Focus,
}

impl FocusScope {
    /// Creates a widget that manages a [`FocusScopeNode`].
    pub fn new<K>(child: impl IntoWidget<K>) -> FocusScope {
        FocusScope {
            focus: Focus::new(child),
        }
    }

    /// Creates a [`FocusScope`] widget that uses the given `focus_scope_node` as the source of
    /// truth for attributes on the node, rather than the attributes of this widget.
    pub fn with_external_focus_node<K>(
        child: impl IntoWidget<K>,
        focus_scope_node: Handle<FocusScopeNode>,
    ) -> FocusScope {
        FocusScope {
            focus: Focus::with_external_focus_node(child, focus_scope_node.as_node()),
        }
    }

    /// Dart `FocusScope(node:)`.
    pub fn node(mut self, node: Handle<FocusScopeNode>) -> FocusScope {
        self.focus.focus_node = Some(node.as_node());
        self
    }

    /// Dart `FocusScope(key:)`.
    pub fn key(mut self, key: KeyRef) -> FocusScope {
        self.focus = self.focus.key(key);
        self
    }

    /// Dart `FocusScope(parentNode:)`.
    pub fn parent_node(mut self, parent_node: AnyFocusNode) -> FocusScope {
        self.focus = self.focus.parent_node(parent_node);
        self
    }

    /// Dart `FocusScope(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> FocusScope {
        self.focus = self.focus.autofocus(autofocus);
        self
    }

    /// Dart `FocusScope(onFocusChange:)`.
    pub fn on_focus_change(
        mut self,
        on_focus_change: impl Fn(&mut App, bool) + 'static,
    ) -> FocusScope {
        self.focus = self.focus.on_focus_change(on_focus_change);
        self
    }

    /// Dart `FocusScope(onKeyEvent:)`.
    pub fn on_key_event(mut self, on_key_event: FocusOnKeyEventCallback) -> FocusScope {
        self.focus = self.focus.on_key_event(on_key_event);
        self
    }

    /// Dart `FocusScope(canRequestFocus:)`.
    pub fn can_request_focus(mut self, can_request_focus: bool) -> FocusScope {
        self.focus = self.focus.can_request_focus(can_request_focus);
        self
    }

    /// Dart `FocusScope(skipTraversal:)`.
    pub fn skip_traversal(mut self, skip_traversal: bool) -> FocusScope {
        self.focus = self.focus.skip_traversal(skip_traversal);
        self
    }

    /// Dart `FocusScope(descendantsAreFocusable:)`.
    pub fn descendants_are_focusable(mut self, descendants_are_focusable: bool) -> FocusScope {
        self.focus = self
            .focus
            .descendants_are_focusable(descendants_are_focusable);
        self
    }

    /// Dart `FocusScope(descendantsAreTraversable:)`.
    pub fn descendants_are_traversable(mut self, descendants_are_traversable: bool) -> FocusScope {
        self.focus = self
            .focus
            .descendants_are_traversable(descendants_are_traversable);
        self
    }

    /// Dart `FocusScope(includeSemantics:)`.
    pub fn include_semantics(mut self, include_semantics: bool) -> FocusScope {
        self.focus = self.focus.include_semantics(include_semantics);
        self
    }

    /// Dart `FocusScope(debugLabel:)`.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> FocusScope {
        self.focus = self.focus.debug_label(debug_label);
        self
    }

    /// The `Focus` fields this widget inherits.
    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// Returns the [`AnyFocusNode::nearest_scope`] of the [`Focus`] or [`FocusScope`] that most
    /// tightly encloses the given `context`.
    ///
    /// If this node doesn't have a [`Focus`] or [`FocusScope`] widget ancestor, then the
    /// [`FocusManager`](crate::FocusManager)'s root scope is returned.
    pub fn of(
        app: &mut App,
        context: BuildContext,
        create_dependency: bool,
    ) -> Handle<FocusScopeNode> {
        Focus::maybe_of(app, context, true, create_dependency)
            .and_then(|node| node.nearest_scope(app))
            .unwrap_or_else(|| {
                let owner = context
                    .owner(app)
                    .expect("a mounted element has a build owner");
                let manager = owner.focus_manager(app);
                manager.root_scope(app)
            })
    }
}

impl fmt::Debug for FocusScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FocusScope")
            .field("focus", &self.focus)
            .finish()
    }
}

impl StatefulWidget for FocusScope {
    type State = FocusScopeState;

    fn key(&self) -> Option<&KeyRef> {
        self.focus.key.as_ref()
    }

    fn create_state(&self) -> FocusScopeState {
        FocusScopeState {
            state: StateData::new(),
            focus: FocusStateData::new(),
        }
    }
}

/// Dart's `_FocusScopeState`.
pub struct FocusScopeState {
    state: StateData<FocusScope>,
    focus: FocusStateData,
}

impl FocusStateLeaf for FocusScopeState {
    fn focus_state_data(self: Handle<Self>, app: &App) -> &FocusStateData {
        &app.get(self).focus
    }

    fn focus_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut FocusStateData {
        &mut app.get_mut(self).focus
    }

    fn focus_state_data_field(&mut self) -> &mut FocusStateData {
        &mut self.focus
    }

    fn focus_widget(self: Handle<Self>, app: &App) -> &Focus {
        &self.widget(app).focus
    }

    fn create_node(self: Handle<Self>, app: &mut App) -> AnyFocusNode {
        let node = FocusScopeNode::new(app);
        let label = self.focus_widget(app).get_debug_label(app);
        node.set_debug_label(app, label);
        let can_request_focus = self.widget_can_request_focus(app);
        node.set_can_request_focus(app, can_request_focus);
        let skip_traversal = self.widget_skip_traversal(app);
        node.set_skip_traversal(app, skip_traversal);
        node.as_node()
    }
}

impl State for FocusScopeState {
    type Widget = FocusScope;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.init_node(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_focus(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_focus(app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        self.deactivate_focus(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &FocusScope) {
        self.did_update_widget_focus(app, &old_widget.focus);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let parent_node = self.widget(app).focus.parent_node;
        self.focus_state_data(app)
            .focus_attachment
            .expect("attached in init_state")
            .reparent(app, parent_node);
        let child = self.widget(app).focus.child.clone();
        let node = self.focus_node(app);
        // Dart wraps the result in `Semantics(explicitChildNodes: true)` when
        // `includeSemantics` is set; accessibility waits.
        FocusInheritedScope { node, child }.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _FocusInheritedScope

/// The `InheritedWidget` for [`Focus`] and [`FocusScope`].
///
/// Dart's `_FocusInheritedScope extends InheritedNotifier<FocusNode>`; the notifier behavior
/// lives in [`FocusInheritedScopeElement`] until `inherited_notifier.dart` is ported.
struct FocusInheritedScope {
    /// The [`AnyFocusNode`] to which to listen. Whenever it sends change notifications, the
    /// dependents of this widget are triggered.
    node: AnyFocusNode,
    child: WidgetRef,
}

impl FocusInheritedScope {
    /// The tree node: an `InheritedWidget` with its own element, outside the `IntoWidget`
    /// kinds.
    fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }
}

impl fmt::Debug for FocusInheritedScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_FocusInheritedScope")
            .field("node", &self.node)
            .finish_non_exhaustive()
    }
}

/// Dart's `InheritedNotifier.updateShouldNotify`; the trait also names the type
/// `Focus::maybe_of` looks up with `depend_on_inherited_widget_of_exact_type`.
impl crate::framework::InheritedWidget for FocusInheritedScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &FocusInheritedScope) -> bool {
        old_widget.node != self.node
    }
}

impl Widget for FocusInheritedScope {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        FocusInheritedScopeElement::create(app, this)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<FocusInheritedScope>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

/// Dart's `_InheritedNotifierElement<FocusNode>`, specialised to [`FocusInheritedScope`]: an
/// `InheritedElement` that marks itself dirty whenever the node notifies, and notifies its
/// dependents on the next build.
struct FocusInheritedScopeElement {
    element: ElementData,
    component: ComponentElementData,
    dependents: HashMap<AnyElement, Option<Rc<dyn Any>>>,
    dirty: bool,
}

impl FocusInheritedScopeElement {
    fn create(app: &mut App, widget: WidgetRef) -> AnyElement {
        let node = FocusInheritedScopeElement::widget_of(&widget).node;
        let this = app.create(FocusInheritedScopeElement {
            element: ElementData::new(widget),
            component: ComponentElementData::default(),
            dependents: HashMap::new(),
            dirty: false,
        });
        node.add_listener(
            app,
            Listener::handle_method(this, FocusInheritedScopeElement::handle_update),
        );
        this.as_element()
    }

    fn widget_of(widget: &WidgetRef) -> &FocusInheritedScope {
        downcast_widget::<FocusInheritedScope>(&**widget)
            .expect("a FocusInheritedScopeElement holds its FocusInheritedScope")
    }

    fn widget(self: Handle<Self>, app: &App) -> &FocusInheritedScope {
        FocusInheritedScopeElement::widget_of(self.as_element().widget(app))
    }

    fn handle_update(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).dirty = true;
        self.as_element().mark_needs_build(app);
    }

    /// `InheritedElement.notifyClients`, plus Dart's `_dirty` reset.
    fn notify_clients(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        let dependents: Vec<AnyElement> = app.get(self).dependents.keys().copied().collect();
        for dependent in dependents {
            let _ = &old_widget;
            dependent.did_change_dependencies(app);
        }
        app.get_mut(self).dirty = false;
    }
}

impl ComponentElement for FocusInheritedScopeElement {
    fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
        &app.get(self).component
    }

    fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
        &mut app.get_mut(self).component
    }

    fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
        if app.get(self).dirty {
            let widget = self.as_element().widget(app).clone();
            FocusInheritedScopeElement::notify_clients(self, app, widget);
        }
        self.proxied_child(app)
    }
}

impl ProxyElement for FocusInheritedScopeElement {
    fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
        self.widget(app).child.clone()
    }

    fn updated(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        // `InheritedNotifier.updateShouldNotify`: the notifier changed.
        let old_node = FocusInheritedScopeElement::widget_of(&old_widget).node;
        if old_node != self.widget(app).node {
            FocusInheritedScopeElement::notify_clients(self, app, old_widget);
        }
    }

    fn notify_clients(self: Handle<Self>, app: &mut App, old_widget: WidgetRef) {
        FocusInheritedScopeElement::notify_clients(self, app, old_widget);
    }
}

impl Element for FocusInheritedScopeElement {
    crate::element_accessors!();
    component_element_overrides!();

    const IS_INHERITED_ELEMENT: bool = true;

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        ComponentElement::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_node = self.widget(app).node;
        let new_node = FocusInheritedScopeElement::widget_of(&new_widget).node;
        if old_node != new_node {
            old_node.remove_listener(
                app,
                &Listener::handle_method(self, FocusInheritedScopeElement::handle_update),
            );
            new_node.add_listener(
                app,
                Listener::handle_method(self, FocusInheritedScopeElement::handle_update),
            );
        }
        ProxyElement::update(self, app, new_widget);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        ComponentElement::perform_rebuild(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        let node = self.widget(app).node;
        node.remove_listener(
            app,
            &Listener::handle_method(self, FocusInheritedScopeElement::handle_update),
        );
        crate::framework::ElementBase::unmount(self, app);
    }

    fn update_inheritance(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Active);
        let incoming = self
            .as_element()
            .parent(app)
            .and_then(|parent| parent.inherited_elements(app));
        let mut widgets: HashMap<TypeId, AnyElement> = incoming
            .map(|incoming| (*incoming).clone())
            .unwrap_or_default();
        widgets.insert(TypeId::of::<FocusInheritedScope>(), self.as_element());
        self.as_element()
            .set_inherited_elements(app, Some(Rc::new(widgets)));
    }

    fn debug_deactivated(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).dependents.is_empty());
        debug_assert!(self.as_element().lifecycle(app) == ElementLifecycle::Inactive);
    }

    fn update_dependencies(
        self: Handle<Self>,
        app: &mut App,
        dependent: AnyElement,
        _aspect: Option<Rc<dyn Any>>,
    ) {
        app.get_mut(self).dependents.insert(dependent, None);
    }

    fn remove_dependent(self: Handle<Self>, app: &mut App, dependent: AnyElement) {
        app.get_mut(self).dependents.remove(&dependent);
    }
}

// ---------------------------------------------------------------------------------------------
// ExcludeFocus

/// A widget that controls whether or not the descendants of this widget are focusable.
///
/// Does not affect the value of [`Focus::can_request_focus`] on the descendants.
#[derive(Debug)]
pub struct ExcludeFocus {
    /// See [`Widget::key`].
    pub key: Option<KeyRef>,

    /// If true, will make this widget's descendants unfocusable.
    ///
    /// Defaults to true.
    ///
    /// If any descendants are focused when this is set to true, they will be unfocused. When
    /// [`excluding`](Self::excluding) is set to false again, they will not be refocused,
    /// although they will be able to accept focus again.
    ///
    /// Does not affect the value of [`AnyFocusNode::can_request_focus`] on the descendants.
    pub excluding: bool,

    /// The child widget of this [`ExcludeFocus`].
    pub child: WidgetRef,
}

impl ExcludeFocus {
    /// Creates an [`ExcludeFocus`] widget.
    pub fn new<K>(child: impl IntoWidget<K>) -> ExcludeFocus {
        ExcludeFocus {
            key: None,
            excluding: true,
            child: child.into_widget(),
        }
    }

    /// Dart `ExcludeFocus(key:)`.
    pub fn key(mut self, key: KeyRef) -> ExcludeFocus {
        self.key = Some(key);
        self
    }

    /// Dart `ExcludeFocus(excluding:)`.
    pub fn excluding(mut self, excluding: bool) -> ExcludeFocus {
        self.excluding = excluding;
        self
    }
}

impl crate::framework::StatelessWidget for ExcludeFocus {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Focus::new(self.child.clone())
            .can_request_focus(false)
            .skip_traversal(true)
            .include_semantics(false)
            .descendants_are_focusable(!self.excluding)
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;
    use crate::framework::GlobalKey;
    use crate::widgets::basic::{Builder, Center, Padding, SizedBox};
    use crate::widgets::focus_manager::tests::{app_with_view, mount};
    use crate::widgets::focus_manager::{FocusManager, KeyEventResult};

    type Captured = Rc<Cell<Option<AnyFocusNode>>>;

    /// A `Builder` that records `Focus.of` at its own context.
    fn probe(captured: &Captured) -> WidgetRef {
        let captured = Rc::clone(captured);
        Builder::new(move |app, context| {
            captured.set(Focus::maybe_of(app, context, false, true));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    #[test]
    fn autofocus_gives_the_focus_widgets_node_the_primary_focus() {
        let mut app = app_with_view();
        let captured: Captured = Rc::default();
        mount(
            &mut app,
            Focus::new(probe(&captured)).autofocus(true).into_widget(),
        );
        let node = captured.get().expect("Focus.of found the node");
        assert!(node.has_primary_focus(&app));
        let manager = FocusManager::instance(&mut app);
        assert_eq!(manager.primary_focus(&app), Some(node));
    }

    #[test]
    fn focus_of_reports_has_focus_on_the_ancestor_and_primary_focus_on_the_leaf() {
        let mut app = app_with_view();
        let outer: Captured = Rc::default();
        let inner: Captured = Rc::default();
        let inner_sink = Rc::clone(&inner);
        let outer_sink = Rc::clone(&outer);
        mount(
            &mut app,
            Focus::new(Builder::new(move |app, context| {
                outer_sink.set(Focus::maybe_of(app, context, false, true));
                Focus::new(probe(&inner_sink)).autofocus(true).into_widget()
            }))
            .debug_label("outer")
            .into_widget(),
        );

        let outer = outer.get().expect("the outer Focus.of found nothing");
        let inner = inner.get().expect("the inner Focus.of found nothing");
        assert_ne!(outer, inner);
        assert_eq!(inner.parent(&app), Some(outer));
        assert!(inner.has_primary_focus(&app));
        assert!(inner.has_focus(&mut app));
        assert!(outer.has_focus(&mut app));
        assert!(!outer.has_primary_focus(&app));
        assert_eq!(outer.debug_label(&app).as_deref(), Some("outer"));
    }

    #[test]
    fn focus_scope_of_returns_the_enclosing_scope() {
        // With no FocusScope of its own, the context reaches the one the `View` installs, which
        // hangs off the root scope through the view's `FocusTraversalGroup`.
        let mut app = app_with_view();
        let scope_node = Rc::new(Cell::new(None));
        let sink = Rc::clone(&scope_node);
        mount(
            &mut app,
            Builder::new(move |app, context| {
                sink.set(Some(FocusScope::of(app, context, true)));
                SizedBox::shrink().into_widget()
            })
            .into_widget(),
        );
        let manager = FocusManager::instance(&mut app);
        let view_scope = scope_node.get().expect("FocusScope.of ran");
        assert_ne!(view_scope, manager.root_scope(&app));
        assert_eq!(view_scope.debug_label(&app).as_deref(), Some("View Scope"));
        assert_eq!(
            view_scope.enclosing_scope(&mut app),
            Some(manager.root_scope(&app))
        );

        // A FocusScope widget below the view scopes its own subtree instead.
        let mut app = app_with_view();
        let scope_node = Rc::new(Cell::new(None));
        let sink = Rc::clone(&scope_node);
        mount(
            &mut app,
            FocusScope::new(Builder::new(move |app, context| {
                sink.set(Some(FocusScope::of(app, context, true)));
                SizedBox::shrink().into_widget()
            }))
            .debug_label("scope")
            .into_widget(),
        );
        let manager = FocusManager::instance(&mut app);
        let scope = scope_node.get().expect("FocusScope.of ran");
        assert_ne!(scope, manager.root_scope(&app));
        assert_eq!(scope.debug_label(&app).as_deref(), Some("scope"));
        assert_eq!(
            scope
                .enclosing_scope(&mut app)
                .and_then(|scope| scope.debug_label(&app)),
            Some("View Scope".to_string())
        );
    }

    #[test]
    fn a_focus_widgets_on_key_event_receives_events_addressed_to_its_node() {
        use std::time::Duration;

        use reveal_services::{
            HardwareKeyboard, KeyDownEvent, KeyEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
        };

        let mut app = app_with_view();
        let seen: Rc<RefCell<Vec<&'static str>>> = Rc::default();
        let inner_seen = Rc::clone(&seen);
        let outer_seen = Rc::clone(&seen);
        mount(
            &mut app,
            Focus::new(
                Focus::new(SizedBox::shrink())
                    .autofocus(true)
                    .on_key_event(Rc::new(move |_app, _node, _event| {
                        inner_seen.borrow_mut().push("inner");
                        KeyEventResult::Ignored
                    })),
            )
            .on_key_event(Rc::new(move |_app, _node, _event| {
                outer_seen.borrow_mut().push("outer");
                KeyEventResult::Handled
            }))
            .into_widget(),
        );

        let event = KeyEvent::Down(KeyDownEvent::new(
            PhysicalKeyboardKey::KEY_A,
            LogicalKeyboardKey::KEY_A,
            Duration::ZERO,
        ));
        let handled = HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
        assert!(handled);
        assert_eq!(*seen.borrow(), vec!["inner", "outer"]);
    }

    #[test]
    fn moving_a_focus_widget_by_global_key_keeps_its_node_and_its_focus() {
        let mut app = app_with_view();
        let captured: Captured = Rc::default();
        let key: KeyRef = Rc::new(GlobalKey::new());
        mount(
            &mut app,
            Padding::new(reveal_painting::EdgeInsetsGeometry::all(4.0))
                .child(
                    Focus::new(probe(&captured))
                        .key(Rc::clone(&key))
                        .autofocus(true),
                )
                .into_widget(),
        );
        let node = captured.get().expect("Focus.of found the node");
        assert!(node.has_primary_focus(&app));

        mount(
            &mut app,
            Center::new()
                .child(Focus::new(probe(&captured)).key(Rc::clone(&key)))
                .into_widget(),
        );
        assert_eq!(captured.get(), Some(node));
        assert!(node.has_primary_focus(&app));
    }

    #[test]
    fn exclude_focus_blocks_the_descendants() {
        let mut app = app_with_view();
        let captured: Captured = Rc::default();
        mount(
            &mut app,
            ExcludeFocus::new(Focus::new(probe(&captured))).into_widget(),
        );
        let node = captured.get().expect("Focus.of found the node");
        assert!(!node.can_request_focus(&mut app));

        node.request_focus(&mut app, None);
        app.drain_microtasks();
        assert!(!node.has_primary_focus(&app));
    }
}
