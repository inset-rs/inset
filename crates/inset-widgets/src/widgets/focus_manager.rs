//! Flutter counterpart: `widgets/focus_manager.dart`.
//!
//! The focus tree: [`FocusNode`] and [`FocusScopeNode`] (one arena struct per Dart leaf,
//! joined by the type-erased handle [`AnyFocusNode`]), the [`FocusAttachment`] a host widget uses to
//! keep a node in the tree, and the [`FocusManager`] the `BuildOwner` holds.

use std::fmt;
use std::rc::Rc;

use inset_embedder::{Offset, PointerDeviceKind, Rect, Size, TargetPlatform};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Listener};
use inset_gestures::{GestureBinding, PointerEvent, PointerRoute};
use inset_rendering::RendererBinding;
use inset_scheduler::{SchedulerBinding, SchedulerPhase};
use inset_services::{HardwareKeyboard, KeyEvent, KeyEventCallback};

use crate::binding::WidgetsBinding;
use crate::framework::BuildContext;
use crate::widgets::focus_traversal::{FocusTraversalGroup, TraversalDirection};

/// An enum that describes how to handle a key event handled by a
/// [`FocusOnKeyEventCallback`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEventResult {
    /// The key event has been handled, and the event should not be propagated to other key
    /// event handlers.
    Handled,

    /// The key event has not been handled, and the event should continue to be propagated to
    /// other key event handlers, even non-Flutter ones.
    Ignored,

    /// The key event has not been handled, but the key event should not be propagated to
    /// other key event handlers.
    ///
    /// It will be returned to the platform embedding to be propagated to text fields and
    /// non-Flutter key event handlers on the platform.
    SkipRemainingHandlers,
}

/// Combine the results returned by multiple [`FocusOnKeyEventCallback`]s.
///
/// If any callback returns [`KeyEventResult::Handled`], the node considers the message
/// handled; otherwise, if any callback returns [`KeyEventResult::SkipRemainingHandlers`], the
/// node skips the remaining handlers without preventing the platform to handle; otherwise the
/// node is ignored.
pub fn combine_key_event_results(
    results: impl IntoIterator<Item = KeyEventResult>,
) -> KeyEventResult {
    let mut has_skip_remaining_handlers = false;
    for result in results {
        match result {
            KeyEventResult::Handled => return KeyEventResult::Handled,
            KeyEventResult::SkipRemainingHandlers => has_skip_remaining_handlers = true,
            KeyEventResult::Ignored => {}
        }
    }
    if has_skip_remaining_handlers {
        KeyEventResult::SkipRemainingHandlers
    } else {
        KeyEventResult::Ignored
    }
}

/// Signature of a callback used by `Focus::on_key_event` and `FocusScope::on_key_event` to
/// receive key events.
///
/// The `node` is the node that received the event.
///
/// Returns a [`KeyEventResult`] that describes how, and whether, the key event was handled.
pub type FocusOnKeyEventCallback = Rc<dyn Fn(&mut App, AnyFocusNode, &KeyEvent) -> KeyEventResult>;

/// Signature of a callback used by [`FocusManager::add_early_key_event_handler`] and
/// [`FocusManager::add_late_key_event_handler`].
///
/// The `event` parameter is a [`KeyEvent`] that is being sent to the callback to be handled.
///
/// The [`KeyEventResult`] return value indicates whether or not the event will continue to be
/// propagated. If the value returned is [`KeyEventResult::Handled`] or
/// [`KeyEventResult::SkipRemainingHandlers`], then the event will not continue to be
/// propagated.
pub type OnKeyEventCallback = Rc<dyn Fn(&mut App, &KeyEvent) -> KeyEventResult>;

/// Signature of a closure registered with [`FocusManager::add_highlight_mode_listener`].
pub type HighlightModeListener = Rc<dyn Fn(&mut App, FocusHighlightMode)>;

/// Represents a pending autofocus request.
#[derive(Clone, Copy)]
struct Autofocus {
    scope: Handle<FocusScopeNode>,
    autofocus_node: AnyFocusNode,
}

impl Autofocus {
    /// Applies the autofocus request, if the node is still attached to the original scope and
    /// the scope has no focused child.
    ///
    /// The widget tree is responsible for calling reparent/detach on attached nodes to keep
    /// their parent/manager information up-to-date, so here we can safely check if the
    /// scope/node involved in each autofocus request is still attached, and discard the ones
    /// which are no longer attached to the original manager.
    fn apply_if_valid(self, app: &mut App, manager: Handle<FocusManager>) {
        let scope = self.scope.as_node();
        let should_apply = (scope.parent(app).is_some()
            || self.scope == app.get(manager).root_scope)
            && scope.manager(app) == Some(manager)
            && self.scope.focused_child(app).is_none()
            && self.autofocus_node.ancestors(app).contains(&scope);
        if should_apply {
            self.autofocus_node.do_request_focus(app, true);
        }
    }
}

/// An attachment point for a [`FocusNode`].
///
/// Using a [`FocusAttachment`] is rarely needed, unless building something akin to the
/// `Focus` or `FocusScope` widgets from scratch.
///
/// Once created, a [`FocusNode`] must be attached to the widget tree by its _host_
/// `StatefulWidget` via a [`FocusAttachment`] object. [`FocusAttachment`]s are owned by the
/// `StatefulWidget` that hosts a [`FocusNode`] or [`FocusScopeNode`]. There can be multiple
/// [`FocusAttachment`]s for each [`FocusNode`], but the node will only ever be attached to one
/// of them at a time.
///
/// This attachment is created by calling [`AnyFocusNode::attach`], usually from the host
/// widget's `State::init_state` method. If the widget is updated to have a different focus
/// node, then the new node needs to be attached in `State::did_update_widget`, after calling
/// [`detach`](Self::detach) on the previous [`FocusAttachment`]. Once detached, the attachment
/// is defunct and will no longer make changes to the [`FocusNode`] through
/// [`reparent`](Self::reparent).
///
/// Without these attachment points, it would be possible for a focus node to simultaneously be
/// attached to more than one part of the widget tree during the build stage.
pub struct FocusAttachment {
    /// The focus node that this attachment manages an attachment for. The node may not yet
    /// have a parent, or may have been detached from this attachment, so don't count on this
    /// node being in a usable state.
    node: AnyFocusNode,
}

impl FocusAttachment {
    /// A private constructor, because [`FocusAttachment`]s are only to be created by
    /// [`AnyFocusNode::attach`].
    fn new(app: &mut App, node: AnyFocusNode) -> Handle<FocusAttachment> {
        app.create(FocusAttachment { node })
    }

    /// Returns true if the associated node is attached to this attachment.
    ///
    /// It is possible to be attached to the widget tree, but not be placed in the focus tree
    /// (i.e. to not have a parent yet in the focus tree).
    pub fn is_attached(self: Handle<Self>, app: &App) -> bool {
        let node = app.get(self).node;
        node.data(app).attachment == Some(self)
    }

    /// Detaches the [`FocusNode`] this attachment point is associated with from the focus
    /// tree, and disconnects it from this attachment point.
    ///
    /// Calling [`AnyFocusNode::dispose`] will also automatically detach the node.
    pub fn detach(self: Handle<Self>, app: &mut App) {
        let node = app.get(self).node;
        if self.is_attached(app) {
            let marked_for_focus = node
                .manager(app)
                .map(|manager| app.get(manager).marked_for_focus);
            if node.has_primary_focus(app) || marked_for_focus == Some(Some(node)) {
                node.unfocus(app, UnfocusDisposition::PreviouslyFocusedChild);
            }
            // This node is no longer in the tree, so shouldn't send notifications anymore.
            if let Some(manager) = node.manager(app) {
                manager.mark_detached(app, node);
            }
            if let Some(parent) = node.data(app).parent {
                parent.remove_child(app, node, true);
            }
            node.data_mut(app).attachment = None;
            debug_assert!(
                !node.has_primary_focus(app),
                "Node {node:?} still has primary focus while being detached."
            );
            debug_assert!(
                node.manager(app)
                    .is_none_or(|manager| app.get(manager).marked_for_focus != Some(node)),
                "Node {node:?} still marked for focus while being detached."
            );
        }
        debug_assert!(!self.is_attached(app));
    }

    /// Ensures that the [`FocusNode`] attached at this attachment point has the proper parent
    /// node, changing it if necessary.
    ///
    /// If given, ensures that the given `parent` node is the parent of the node that is
    /// attached at this attachment point, changing it if necessary. However, it is usually not
    /// necessary to supply an explicit parent, since [`reparent`](Self::reparent) will use
    /// `Focus::maybe_of` to determine the correct parent node for the context given in
    /// [`AnyFocusNode::attach`].
    ///
    /// If [`is_attached`](Self::is_attached) is false, then calling this method does nothing.
    ///
    /// Should be called whenever the associated widget is rebuilt in order to maintain the
    /// focus hierarchy.
    ///
    /// A `StatefulWidget` that hosts a [`FocusNode`] should call this method on the node it
    /// hosts during its `State::build` or `State::did_change_dependencies` methods in case the
    /// widget is moved from one location in the tree to another location that has a different
    /// `FocusScope` or context.
    ///
    /// The optional `parent` argument must be supplied when not using `Focus` and `FocusScope`
    /// widgets to build the focus tree, or if there is a need to supply the parent explicitly
    /// (which are both uncommon).
    pub fn reparent(self: Handle<Self>, app: &mut App, parent: Option<AnyFocusNode>) {
        if self.is_attached(app) {
            let node = app.get(self).node;
            let context = node.data(app).context;
            debug_assert!(context.is_some());
            let context = context.expect("an attached node has a context");
            let parent = parent
                .or_else(|| crate::widgets::focus_scope::Focus::maybe_of(app, context, true, true))
                .or_else(|| {
                    context
                        .owner(app)
                        .map(|owner| owner.focus_manager(app))
                        .map(|manager| app.get(manager).root_scope.as_node())
                })
                .expect("the element's build owner has a focus manager");
            parent.reparent_child(app, node);
        }
    }
}

/// Describe what should happen after [`AnyFocusNode::unfocus`] is called.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnfocusDisposition {
    /// Focus the nearest focusable enclosing scope of this node, but do not descend to locate
    /// the leaf [`FocusScopeNode::focused_child`] the way
    /// [`PreviouslyFocusedChild`](Self::PreviouslyFocusedChild) does.
    ///
    /// Focusing the scope in this way clears the [`FocusScopeNode::focused_child`] history for
    /// the enclosing scope when it receives focus. Because of this, calling a traversal method
    /// like `FocusNode.nextFocus` after unfocusing will cause the `FocusTraversalPolicy` to
    /// pick the node it thinks should be first in the scope.
    ///
    /// This is the default disposition for [`AnyFocusNode::unfocus`].
    #[default]
    Scope,

    /// Focus the previously focused child of the nearest focusable enclosing scope of this
    /// node.
    ///
    /// If there is no previously focused child, then this is equivalent to using the
    /// [`Scope`](Self::Scope) disposition.
    ///
    /// Unfocusing with this disposition will cause [`AnyFocusNode::unfocus`] to walk up the
    /// tree to the nearest focusable enclosing scope, then start to walk down the tree, looking
    /// for a focused child at its [`FocusScopeNode::focused_child`].
    ///
    /// If the [`FocusScopeNode::focused_child`] is a scope, then look for its
    /// [`FocusScopeNode::focused_child`], and so on, finding the leaf
    /// [`FocusScopeNode::focused_child`] that is not a scope, or, failing that, a leaf scope
    /// that has no focused child.
    PreviouslyFocusedChild,
}

// ---------------------------------------------------------------------------------------------
// FocusNode

/// The fields Dart's `FocusNode` declares; every focus node leaf carries this bag under the
/// field `focus_node`.
pub struct FocusNodeData {
    skip_traversal: bool,
    can_request_focus: bool,
    descendants_are_focusable: bool,
    descendants_are_traversable: bool,
    context: Option<BuildContext>,
    on_key_event: Option<FocusOnKeyEventCallback>,
    manager: Option<Handle<FocusManager>>,
    ancestors: Option<Vec<AnyFocusNode>>,
    descendants: Option<Vec<AnyFocusNode>>,
    has_keyboard_token: bool,
    parent: Option<AnyFocusNode>,
    children: Vec<AnyFocusNode>,
    debug_label: Option<String>,
    attachment: Option<Handle<FocusAttachment>>,
    enclosing_scope: Option<Handle<FocusScopeNode>>,
    request_focus_when_reparented: bool,
}

impl FocusNodeData {
    /// The bag of a freshly created node: Dart's constructor defaults.
    pub fn new() -> FocusNodeData {
        FocusNodeData {
            skip_traversal: false,
            can_request_focus: true,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
            context: None,
            on_key_event: None,
            manager: None,
            ancestors: None,
            descendants: None,
            has_keyboard_token: false,
            parent: None,
            children: Vec::new(),
            debug_label: None,
            attachment: None,
            enclosing_scope: None,
            request_focus_when_reparented: false,
        }
    }
}

impl Default for FocusNodeData {
    fn default() -> FocusNodeData {
        FocusNodeData::new()
    }
}

/// The accessors [`FocusNodeLeaf`] asks for, for a struct whose bag is the field `focus_node`.
#[macro_export]
macro_rules! focus_node_accessors {
    () => {
        fn focus_node_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::FocusNodeData {
            &app.get(self).focus_node
        }

        fn focus_node_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::FocusNodeData {
            &mut app.get_mut(self).focus_node
        }
    };
}

/// A concrete focus node: Dart's `FocusNode` and its one subclass [`FocusScopeNode`].
///
/// The trait holds the members a superclass body dispatches to the leaf ([`nearest_scope`],
/// [`descendants_are_focusable`], [`traversal_children`], [`traversal_descendants`],
/// [`do_request_focus`], and Dart's `node is FocusScopeNode` as [`as_scope`]), plus the
/// [`as_node`] type-erased handle and one-line forwarders for the shared members, which live
/// on [`AnyFocusNode`] because a node reaches its parent and children through that handle.
///
/// The defaults are `FocusNode`'s own bodies; [`FocusScopeNode`] overrides them and calls
/// `FocusNode::…` where Dart writes `super.…`.
///
/// [`nearest_scope`]: Self::nearest_scope
/// [`descendants_are_focusable`]: Self::descendants_are_focusable
/// [`traversal_children`]: Self::traversal_children
/// [`traversal_descendants`]: Self::traversal_descendants
/// [`do_request_focus`]: Self::do_request_focus
/// [`as_scope`]: Self::as_scope
/// [`as_node`]: Self::as_node
pub trait FocusNodeLeaf: ChangeNotifier + Sized + 'static {
    /// Dart's `FocusNode` fields, held under the field `focus_node`
    /// ([`focus_node_accessors!`](crate::focus_node_accessors)).
    fn focus_node_data(self: Handle<Self>, app: &App) -> &FocusNodeData;

    /// See [`focus_node_data`](Self::focus_node_data).
    fn focus_node_data_mut(self: Handle<Self>, app: &mut App) -> &mut FocusNodeData;

    /// This node as the erased [`AnyFocusNode`] — what to pass where a Dart API takes a
    /// `FocusNode`.
    ///
    /// The object is untouched; this mints an erased second handle to it, so concrete members
    /// stay reachable through the typed one.
    fn as_node(self: Handle<Self>) -> AnyFocusNode {
        AnyFocusNode {
            id: self.id(),
            vtable: const { &FocusNodeVTable::of::<Self>() },
        }
    }

    /// Dart's `node is FocusScopeNode`.
    fn as_scope(self: Handle<Self>) -> Option<Handle<FocusScopeNode>> {
        let _ = self;
        None
    }

    /// Returns the nearest enclosing scope node above this node, including this node, if it's
    /// a scope.
    fn nearest_scope(self: Handle<Self>, app: &mut App) -> Option<Handle<FocusScopeNode>> {
        FocusNode::nearest_scope(self.as_node(), app)
    }

    /// If false, will disable focus for all of this node's descendants.
    fn descendants_are_focusable(self: Handle<Self>, app: &App) -> bool {
        FocusNode::descendants_are_focusable(self.as_node(), app)
    }

    /// An iterator over the children that are allowed to be traversed by the
    /// `FocusTraversalPolicy`.
    fn traversal_children(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        FocusNode::traversal_children(self.as_node(), app)
    }

    /// Returns all descendants which do not have the `skip_traversal` and do have the
    /// `can_request_focus` flag set.
    fn traversal_descendants(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        FocusNode::traversal_descendants(self.as_node(), app)
    }

    /// Requests the primary focus for this node; overridden by [`FocusScopeNode`].
    fn do_request_focus(self: Handle<Self>, app: &mut App, find_first_focus: bool) {
        FocusNode::do_request_focus(self.as_node(), app, find_first_focus);
    }

    // ---- one-line forwarders to the type-erased handle ----

    /// See [`AnyFocusNode::skip_traversal`].
    fn skip_traversal(self: Handle<Self>, app: &mut App) -> bool {
        self.as_node().skip_traversal(app)
    }

    /// See [`AnyFocusNode::set_skip_traversal`].
    fn set_skip_traversal(self: Handle<Self>, app: &mut App, value: bool) {
        self.as_node().set_skip_traversal(app, value);
    }

    /// See [`AnyFocusNode::can_request_focus`].
    fn can_request_focus(self: Handle<Self>, app: &mut App) -> bool {
        self.as_node().can_request_focus(app)
    }

    /// See [`AnyFocusNode::set_can_request_focus`].
    fn set_can_request_focus(self: Handle<Self>, app: &mut App, value: bool) {
        self.as_node().set_can_request_focus(app, value);
    }

    /// See [`AnyFocusNode::set_descendants_are_focusable`].
    fn set_descendants_are_focusable(self: Handle<Self>, app: &mut App, value: bool) {
        self.as_node().set_descendants_are_focusable(app, value);
    }

    /// See [`AnyFocusNode::descendants_are_traversable`].
    fn descendants_are_traversable(self: Handle<Self>, app: &App) -> bool {
        self.as_node().descendants_are_traversable(app)
    }

    /// See [`AnyFocusNode::set_descendants_are_traversable`].
    fn set_descendants_are_traversable(self: Handle<Self>, app: &mut App, value: bool) {
        self.as_node().set_descendants_are_traversable(app, value);
    }

    /// See [`AnyFocusNode::context`].
    fn context(self: Handle<Self>, app: &App) -> Option<BuildContext> {
        self.as_node().context(app)
    }

    /// See [`AnyFocusNode::on_key_event`].
    fn on_key_event(self: Handle<Self>, app: &App) -> Option<FocusOnKeyEventCallback> {
        self.as_node().on_key_event(app)
    }

    /// See [`AnyFocusNode::set_on_key_event`].
    fn set_on_key_event(self: Handle<Self>, app: &mut App, value: Option<FocusOnKeyEventCallback>) {
        self.as_node().set_on_key_event(app, value);
    }

    /// See [`AnyFocusNode::debug_label`].
    fn debug_label(self: Handle<Self>, app: &App) -> Option<String> {
        self.as_node().debug_label(app)
    }

    /// See [`AnyFocusNode::set_debug_label`].
    fn set_debug_label(self: Handle<Self>, app: &mut App, value: Option<String>) {
        self.as_node().set_debug_label(app, value);
    }

    /// See [`AnyFocusNode::parent`].
    fn parent(self: Handle<Self>, app: &App) -> Option<AnyFocusNode> {
        self.as_node().parent(app)
    }

    /// See [`AnyFocusNode::children`].
    fn children(self: Handle<Self>, app: &App) -> Vec<AnyFocusNode> {
        self.as_node().children(app)
    }

    /// See [`AnyFocusNode::descendants`].
    fn descendants(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        self.as_node().descendants(app)
    }

    /// See [`AnyFocusNode::ancestors`].
    fn ancestors(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        self.as_node().ancestors(app)
    }

    /// See [`AnyFocusNode::has_focus`].
    fn has_focus(self: Handle<Self>, app: &mut App) -> bool {
        self.as_node().has_focus(app)
    }

    /// See [`AnyFocusNode::has_primary_focus`].
    fn has_primary_focus(self: Handle<Self>, app: &App) -> bool {
        self.as_node().has_primary_focus(app)
    }

    /// See [`AnyFocusNode::highlight_mode`].
    fn highlight_mode(self: Handle<Self>, app: &mut App) -> FocusHighlightMode {
        self.as_node().highlight_mode(app)
    }

    /// See [`AnyFocusNode::enclosing_scope`].
    fn enclosing_scope(self: Handle<Self>, app: &mut App) -> Option<Handle<FocusScopeNode>> {
        self.as_node().enclosing_scope(app)
    }

    /// See [`AnyFocusNode::size`].
    fn size(self: Handle<Self>, app: &App) -> Size {
        self.as_node().size(app)
    }

    /// See [`AnyFocusNode::offset`].
    fn offset(self: Handle<Self>, app: &App) -> Offset {
        self.as_node().offset(app)
    }

    /// See [`AnyFocusNode::rect`].
    fn rect(self: Handle<Self>, app: &App) -> Rect {
        self.as_node().rect(app)
    }

    /// See [`AnyFocusNode::unfocus`].
    fn unfocus(self: Handle<Self>, app: &mut App, disposition: UnfocusDisposition) {
        self.as_node().unfocus(app, disposition);
    }

    /// See [`AnyFocusNode::consume_keyboard_token`].
    fn consume_keyboard_token(self: Handle<Self>, app: &mut App) -> bool {
        self.as_node().consume_keyboard_token(app)
    }

    /// See [`AnyFocusNode::attach`].
    fn attach(
        self: Handle<Self>,
        app: &mut App,
        context: Option<BuildContext>,
        on_key_event: Option<FocusOnKeyEventCallback>,
    ) -> Handle<FocusAttachment> {
        self.as_node().attach(app, context, on_key_event)
    }

    /// See [`AnyFocusNode::dispose`].
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.as_node().dispose(app);
    }

    /// See [`AnyFocusNode::request_focus`].
    fn request_focus(self: Handle<Self>, app: &mut App, node: Option<AnyFocusNode>) {
        self.as_node().request_focus(app, node);
    }
}

/// The vtable of an erased [`AnyFocusNode`]: one `&'static` table per leaf type, built by
/// [`FocusNodeVTable::of`].
///
/// It lives outside the arena — `&'static` — because a virtual hands `&mut App` onward.
struct FocusNodeVTable {
    type_name: fn() -> &'static str,
    data: fn(&App, HandleId) -> &FocusNodeData,
    data_mut: fn(&mut App, HandleId) -> &mut FocusNodeData,
    as_scope: fn(HandleId) -> Option<Handle<FocusScopeNode>>,
    nearest_scope: fn(&mut App, HandleId) -> Option<Handle<FocusScopeNode>>,
    descendants_are_focusable: fn(&App, HandleId) -> bool,
    traversal_children: fn(&mut App, HandleId) -> Vec<AnyFocusNode>,
    traversal_descendants: fn(&mut App, HandleId) -> Vec<AnyFocusNode>,
    do_request_focus: fn(&mut App, HandleId, bool),
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    notify_listeners: fn(&mut App, HandleId),
    dispose_notifier: fn(&mut App, HandleId),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

impl FocusNodeVTable {
    /// The table for one leaf type.
    const fn of<L: FocusNodeLeaf>() -> FocusNodeVTable {
        FocusNodeVTable {
            type_name: std::any::type_name::<L>,
            data: |app, id| L::focus_node_data(resolve(id), app),
            data_mut: |app, id| L::focus_node_data_mut(resolve(id), app),
            as_scope: |id| L::as_scope(resolve(id)),
            nearest_scope: |app, id| L::nearest_scope(resolve(id), app),
            descendants_are_focusable: |app, id| L::descendants_are_focusable(resolve(id), app),
            traversal_children: |app, id| L::traversal_children(resolve(id), app),
            traversal_descendants: |app, id| L::traversal_descendants(resolve(id), app),
            do_request_focus: |app, id, find_first_focus| {
                L::do_request_focus(resolve(id), app, find_first_focus)
            },
            add_listener: |app, id, listener| {
                app.get_mut(resolve::<L>(id))
                    .change_notifier_data_mut()
                    .add_listener(listener)
            },
            remove_listener: |app, id, listener| {
                app.get_mut(resolve::<L>(id))
                    .change_notifier_data_mut()
                    .remove_listener(listener)
            },
            notify_listeners: |app, id| resolve::<L>(id).notify_listeners(app),
            dispose_notifier: |app, id| {
                app.get_mut(resolve::<L>(id))
                    .change_notifier_data_mut()
                    .dispose()
            },
        }
    }
}

/// Erased `FocusNode`: one identity and a static vtable, the fat pointer rustc cannot build
/// for an arena id. No lease.
///
/// This is what a field or parameter Dart types as `FocusNode` becomes; the leaf stays in the
/// [`App`] under its own type, and [`downcast`](Self::downcast) gets the typed handle back.
/// The shared `FocusNode` members live here because a node reaches its parent, children, and
/// the primary focus through this handle.
///
/// Equality is Dart's `==` on an object reference: two handles are equal exactly when they
/// address the same node.
#[derive(Clone, Copy)]
pub struct AnyFocusNode {
    id: HandleId,
    vtable: &'static FocusNodeVTable,
}

impl PartialEq for AnyFocusNode {
    fn eq(&self, other: &AnyFocusNode) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyFocusNode {}

impl std::hash::Hash for AnyFocusNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for AnyFocusNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyFocusNode {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `node as T`: the typed handle when this node is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// Dart's `node is FocusScopeNode`.
    pub fn as_scope(self) -> Option<Handle<FocusScopeNode>> {
        (self.vtable.as_scope)(self.id)
    }

    /// Dart's `FocusNode` fields.
    fn data(self, app: &App) -> &FocusNodeData {
        (self.vtable.data)(app, self.id)
    }

    fn data_mut(self, app: &mut App) -> &mut FocusNodeData {
        (self.vtable.data_mut)(app, self.id)
    }

    pub(crate) fn manager(self, app: &App) -> Option<Handle<FocusManager>> {
        self.data(app).manager
    }

    /// If true, tells the focus traversal policy to skip over this node for purposes of the
    /// traversal algorithm.
    ///
    /// This may be used to place nodes in the focus tree that may be focused, but not
    /// traversed, allowing them to receive key events as part of the focus chain, but not be
    /// traversed to via focus traversal.
    ///
    /// This is different from [`can_request_focus`](Self::can_request_focus) because it only
    /// implies that the node can't be reached via traversal, not that it can't be focused. It
    /// may still be focused explicitly.
    pub fn skip_traversal(self, app: &mut App) -> bool {
        if self.data(app).skip_traversal {
            return true;
        }
        for ancestor in self.ancestors(app) {
            if !ancestor.descendants_are_traversable(app) {
                return true;
            }
        }
        false
    }

    /// See [`skip_traversal`](Self::skip_traversal).
    pub fn set_skip_traversal(self, app: &mut App, value: bool) {
        if value != self.data(app).skip_traversal {
            self.data_mut(app).skip_traversal = value;
            if let Some(manager) = self.manager(app) {
                manager.mark_properties_changed(app, self);
            }
        }
    }

    /// If true, this focus node may request the primary focus.
    ///
    /// Defaults to true. Set to false if you want this node to do nothing when
    /// [`request_focus`](Self::request_focus) is called on it.
    ///
    /// If set to false on a [`FocusScopeNode`], will cause all of the children of the scope
    /// node to not be focusable.
    ///
    /// If set to false on a [`FocusNode`], it will not affect the focusability of children of
    /// the node.
    ///
    /// The [`has_focus`](Self::has_focus) member can still return true if this node is the
    /// ancestor of a node with primary focus.
    ///
    /// This is different than [`skip_traversal`](Self::skip_traversal) because
    /// [`skip_traversal`](Self::skip_traversal) still allows the node to be focused, just not
    /// traversed to via the `FocusTraversalPolicy`.
    ///
    /// Setting [`can_request_focus`](Self::can_request_focus) to false implies that the node
    /// will also be skipped for traversal purposes.
    pub fn can_request_focus(self, app: &mut App) -> bool {
        if !self.data(app).can_request_focus {
            return false;
        }
        let ancestors = self.ancestors(app);
        ancestors
            .into_iter()
            .all(|ancestor| AnyFocusNode::allow_descendants_to_be_focused(ancestor, app))
    }

    fn allow_descendants_to_be_focused(ancestor: AnyFocusNode, app: &App) -> bool {
        ancestor.descendants_are_focusable(app)
    }

    /// See [`can_request_focus`](Self::can_request_focus).
    pub fn set_can_request_focus(self, app: &mut App, value: bool) {
        if value != self.data(app).can_request_focus {
            // Have to set this first before unfocusing, since it checks this to cull
            // unfocusable, previously-focused children.
            self.data_mut(app).can_request_focus = value;
            if self.has_focus(app) && !value {
                self.unfocus(app, UnfocusDisposition::PreviouslyFocusedChild);
            }
            if let Some(manager) = self.manager(app) {
                manager.mark_properties_changed(app, self);
            }
        }
    }

    /// If false, will disable focus for all of this node's descendants.
    ///
    /// Defaults to true. Does not affect focusability of this node: for that, use
    /// [`can_request_focus`](Self::can_request_focus).
    ///
    /// If any descendants are focused when this is set to false, they will be unfocused. When
    /// [`descendants_are_focusable`](Self::descendants_are_focusable) is set to true again,
    /// they will not be refocused, although they will be able to accept focus again.
    ///
    /// Does not affect the value of [`can_request_focus`](Self::can_request_focus) on the
    /// descendants.
    ///
    /// If a descendant node loses focus when this value is changed, the focus will move to the
    /// scope enclosing this node.
    pub fn descendants_are_focusable(self, app: &App) -> bool {
        (self.vtable.descendants_are_focusable)(app, self.id)
    }

    /// See [`descendants_are_focusable`](Self::descendants_are_focusable).
    pub fn set_descendants_are_focusable(self, app: &mut App, value: bool) {
        if value == self.data(app).descendants_are_focusable {
            return;
        }
        // Set descendants_are_focusable before unfocusing, so the scope won't try and focus
        // any of the children here again if it is false.
        self.data_mut(app).descendants_are_focusable = value;
        if !value && self.has_focus(app) {
            self.unfocus(app, UnfocusDisposition::PreviouslyFocusedChild);
        }
        if let Some(manager) = self.manager(app) {
            manager.mark_properties_changed(app, self);
        }
    }

    /// If false, tells the focus traversal policy to skip over for all of this node's
    /// descendants for purposes of the traversal algorithm.
    ///
    /// Defaults to true. Does not affect the focus traversal of this node: for that, use
    /// [`skip_traversal`](Self::skip_traversal).
    ///
    /// Does not affect the value of [`skip_traversal`](Self::skip_traversal) on the
    /// descendants. Does not affect focusability of the descendants.
    pub fn descendants_are_traversable(self, app: &App) -> bool {
        self.data(app).descendants_are_traversable
    }

    /// See [`descendants_are_traversable`](Self::descendants_are_traversable).
    pub fn set_descendants_are_traversable(self, app: &mut App, value: bool) {
        if value != self.data(app).descendants_are_traversable {
            self.data_mut(app).descendants_are_traversable = value;
            if let Some(manager) = self.manager(app) {
                manager.mark_properties_changed(app, self);
            }
        }
    }

    /// The context that was supplied to [`attach`](Self::attach).
    ///
    /// This is typically the context for the widget that is being focused, as it is used to
    /// determine the bounds of the widget.
    pub fn context(self, app: &App) -> Option<BuildContext> {
        self.data(app).context
    }

    /// Called if this focus node receives a key event while focused (i.e. when
    /// [`has_focus`](Self::has_focus) returns true).
    ///
    /// The [`FocusManager`] receives key events from `HardwareKeyboard` and will pass them to
    /// the focused nodes. It starts with the node with the primary focus, and will call the
    /// `on_key_event` callback for that node. If the callback returns
    /// [`KeyEventResult::Ignored`], indicating that it did not handle the event, the
    /// [`FocusManager`] will move to the parent of that node and call its `on_key_event`. If
    /// that `on_key_event` returns [`KeyEventResult::Handled`], then it will stop propagating
    /// the event. If it reaches the root [`FocusScopeNode`], [`FocusManager::root_scope`], the
    /// event is discarded.
    pub fn on_key_event(self, app: &App) -> Option<FocusOnKeyEventCallback> {
        self.data(app).on_key_event.clone()
    }

    /// See [`on_key_event`](Self::on_key_event).
    pub fn set_on_key_event(self, app: &mut App, value: Option<FocusOnKeyEventCallback>) {
        self.data_mut(app).on_key_event = value;
    }

    /// Returns the parent node for this object.
    ///
    /// All nodes except for the root [`FocusScopeNode`] ([`FocusManager::root_scope`]) will be
    /// given a parent when they are added to the focus tree, which is done using
    /// [`FocusAttachment::reparent`].
    pub fn parent(self, app: &App) -> Option<AnyFocusNode> {
        self.data(app).parent
    }

    /// An iterator over the children of this node.
    pub fn children(self, app: &App) -> Vec<AnyFocusNode> {
        self.data(app).children.clone()
    }

    /// An iterator over the children that are allowed to be traversed by the
    /// `FocusTraversalPolicy`.
    ///
    /// Returns the list of focusable, traversable children of this node, regardless of those
    /// settings on this focus node. Will return an empty iterable if
    /// [`descendants_are_focusable`](Self::descendants_are_focusable) is false.
    pub fn traversal_children(self, app: &mut App) -> Vec<AnyFocusNode> {
        (self.vtable.traversal_children)(app, self.id)
    }

    /// A debug label that is used for diagnostic output.
    pub fn debug_label(self, app: &App) -> Option<String> {
        self.data(app).debug_label.clone()
    }

    /// See [`debug_label`](Self::debug_label).
    pub fn set_debug_label(self, app: &mut App, value: Option<String>) {
        if cfg!(debug_assertions) {
            // Only set the value in debug builds.
            self.data_mut(app).debug_label = value;
        }
    }

    /// The hierarchy of children below this one, in depth-first order.
    pub fn descendants(self, app: &mut App) -> Vec<AnyFocusNode> {
        if self.data(app).descendants.is_none() {
            let mut result = Vec::new();
            for child in self.data(app).children.clone() {
                result.extend(child.descendants(app));
                result.push(child);
            }
            self.data_mut(app).descendants = Some(result);
        }
        self.data(app)
            .descendants
            .clone()
            .expect("just filled the cache")
    }

    /// Returns all descendants which do not have the
    /// [`skip_traversal`](Self::skip_traversal) and do have the
    /// [`can_request_focus`](Self::can_request_focus) flag set.
    pub fn traversal_descendants(self, app: &mut App) -> Vec<AnyFocusNode> {
        (self.vtable.traversal_descendants)(app, self.id)
    }

    /// The ancestors of this node.
    ///
    /// Iterates the ancestors of this node starting at the parent and iterating over
    /// successively more remote ancestors of this node, ending at the root [`FocusScopeNode`]
    /// ([`FocusManager::root_scope`]).
    pub fn ancestors(self, app: &mut App) -> Vec<AnyFocusNode> {
        if self.data(app).ancestors.is_none() {
            let mut result = Vec::new();
            let mut parent = self.data(app).parent;
            while let Some(node) = parent {
                result.push(node);
                parent = node.data(app).parent;
            }
            self.data_mut(app).ancestors = Some(result);
        }
        self.data(app)
            .ancestors
            .clone()
            .expect("just filled the cache")
    }

    /// Whether this node has input focus.
    ///
    /// A [`FocusNode`] has focus when it is an ancestor of a node that returns true from
    /// [`has_primary_focus`](Self::has_primary_focus), or it has the primary focus itself.
    ///
    /// The [`has_focus`](Self::has_focus) accessor is different from
    /// [`has_primary_focus`](Self::has_primary_focus) in that
    /// [`has_focus`](Self::has_focus) is true if the node is anywhere in the focus chain, but
    /// for [`has_primary_focus`](Self::has_primary_focus) the node must be at the end of the
    /// chain to return true.
    pub fn has_focus(self, app: &mut App) -> bool {
        if self.has_primary_focus(app) {
            return true;
        }
        let primary_focus = self
            .manager(app)
            .and_then(|manager| app.get(manager).primary_focus);
        primary_focus.is_some_and(|primary| primary.ancestors(app).contains(&self))
    }

    /// Returns true if this node currently has the application-wide input focus.
    ///
    /// A [`FocusNode`] has the primary focus when the node is focused in its nearest ancestor
    /// [`FocusScopeNode`] and [`has_focus`](Self::has_focus) is true for all its ancestor
    /// nodes, but none of its descendants.
    pub fn has_primary_focus(self, app: &App) -> bool {
        self.manager(app)
            .is_some_and(|manager| app.get(manager).primary_focus == Some(self))
    }

    /// Returns the [`FocusHighlightMode`] that is currently in effect for this node.
    pub fn highlight_mode(self, app: &mut App) -> FocusHighlightMode {
        FocusManager::instance(app).highlight_mode(app)
    }

    /// Returns the nearest enclosing scope node above this node, including this node, if it's
    /// a scope.
    ///
    /// Returns `None` if no scope is found.
    ///
    /// Use [`enclosing_scope`](Self::enclosing_scope) to look for scopes above this node.
    pub fn nearest_scope(self, app: &mut App) -> Option<Handle<FocusScopeNode>> {
        (self.vtable.nearest_scope)(app, self.id)
    }

    fn clear_enclosing_scope_cache(self, app: &mut App) {
        let Some(cached_scope) = self.data(app).enclosing_scope else {
            return;
        };
        self.data_mut(app).enclosing_scope = None;
        for child in self.data(app).children.clone() {
            if child.data(app).enclosing_scope == Some(cached_scope) {
                child.clear_enclosing_scope_cache(app);
            }
        }
    }

    /// Returns the nearest enclosing scope node above this node, or `None` if the node has not
    /// yet been added to the focus tree.
    ///
    /// If this node is itself a scope, this will only return ancestors of this scope.
    ///
    /// Use [`nearest_scope`](Self::nearest_scope) to start at this node instead of above it.
    pub fn enclosing_scope(self, app: &mut App) -> Option<Handle<FocusScopeNode>> {
        if let Some(cached) = self.data(app).enclosing_scope {
            return Some(cached);
        }
        let parent = self.data(app).parent;
        // Point access: the parent is another slot, so resolve it before writing ours.
        let computed = parent.and_then(|parent| parent.nearest_scope(app));
        self.data_mut(app).enclosing_scope = computed;
        computed
    }

    /// Returns the size of the attached widget's `RenderObject`, in logical units.
    ///
    /// Size is the size of the transformed widget in global coordinates.
    pub fn size(self, app: &App) -> Size {
        self.rect(app).size()
    }

    /// Returns the global offset to the upper left corner of the attached widget's
    /// `RenderObject`, in logical units.
    ///
    /// Offset is the offset of the transformed widget in global coordinates.
    pub fn offset(self, app: &App) -> Offset {
        let object = self.render_box(app);
        object.local_to_global(app, Offset::ZERO, None)
    }

    /// Returns the global rectangle of the attached widget's `RenderObject`, in logical units.
    ///
    /// Rect is the rectangle of the transformed widget in global coordinates.
    pub fn rect(self, app: &App) -> Rect {
        let object = self.render_box(app);
        let top_left = object.local_to_global(app, Offset::ZERO, None);
        let bottom_right =
            object.local_to_global(app, object.size(app).bottom_right(Offset::ZERO), None);
        Rect::from_ltrb(
            top_left.dx(),
            top_left.dy(),
            bottom_right.dx(),
            bottom_right.dy(),
        )
    }

    /// Dart reads `context!.findRenderObject()!` and its `semanticBounds`; a focus node hosts a
    /// box, whose semantic bounds are `Offset.zero & size`.
    fn render_box(self, app: &App) -> inset_rendering::AnyRenderBox {
        let context = self.data(app).context.expect(
            "Tried to get the bounds of a focus node that didn't have its context set yet.\n\
             The context needs to be set before trying to evaluate traversal policies. \
             Setting the context is typically done with the attach method.",
        );
        context
            .find_render_object(app)
            .expect("the focus node's context has a render object")
            .as_box()
            .expect("the focus node's render object is a RenderBox")
    }

    /// Removes the focus on this node by moving the primary focus to another node.
    ///
    /// This method removes focus from a node that has the primary focus, cancels any
    /// outstanding requests to focus it, while setting the primary focus to another node
    /// according to the `disposition`.
    ///
    /// It is safe to call regardless of whether this node has ever requested focus or not. If
    /// this node doesn't have focus or primary focus, nothing happens.
    ///
    /// If `disposition` is [`UnfocusDisposition::Scope`], then the previously focused node
    /// history of the enclosing scope will be cleared, and the primary focus will be moved to
    /// the nearest enclosing scope ancestor that is enabled for focus, ignoring the
    /// [`FocusScopeNode::focused_child`] for that scope.
    ///
    /// If `disposition` is [`UnfocusDisposition::PreviouslyFocusedChild`], then this node will
    /// be removed from the previously focused list in the
    /// [`enclosing_scope`](Self::enclosing_scope), and the focus will be moved to the
    /// previously focused node of the [`enclosing_scope`](Self::enclosing_scope), which (if it
    /// is a scope itself), will find its focused child, etc., until a leaf focus node is found.
    /// If there is no previously focused child, then the scope itself will receive focus, as if
    /// [`UnfocusDisposition::Scope`] were specified.
    pub fn unfocus(self, app: &mut App, disposition: UnfocusDisposition) {
        let marked_for_focus = self
            .manager(app)
            .map(|manager| app.get(manager).marked_for_focus);
        if !self.has_focus(app) && marked_for_focus != Some(Some(self)) {
            return;
        }
        // If the scope is null, then this is either the root node, or a node that is not yet
        // in the tree, neither of which do anything when unfocused.
        let Some(mut scope) = self.enclosing_scope(app) else {
            return;
        };
        match disposition {
            UnfocusDisposition::Scope => {
                // If it can't request focus, then don't modify its focused children.
                if scope.can_request_focus(app) {
                    // Clearing the focused children here prevents re-focusing the node that we
                    // just unfocused if we immediately hit "next" after unfocusing, and also
                    // prevents choosing to refocus the next-to-last focused child if unfocus is
                    // called more than once.
                    app.get_mut(scope).scope.focused_children.clear();
                }
                while !scope.can_request_focus(app) {
                    scope = scope
                        .enclosing_scope(app)
                        .or_else(|| self.manager(app).map(|manager| app.get(manager).root_scope))
                        .expect("a node with a scope has a manager");
                }
                scope.do_request_focus(app, false);
            }
            UnfocusDisposition::PreviouslyFocusedChild => {
                // Select the most recent focused child from the nearest focusable scope and
                // focus that. If there isn't one, focus the scope itself.
                if scope.can_request_focus(app) {
                    remove_focused_child(app, scope, self);
                }
                while !scope.can_request_focus(app) {
                    if let Some(enclosing) = scope.enclosing_scope(app) {
                        remove_focused_child(app, enclosing, scope.as_node());
                    }
                    scope = scope
                        .enclosing_scope(app)
                        .or_else(|| self.manager(app).map(|manager| app.get(manager).root_scope))
                        .expect("a node with a scope has a manager");
                }
                scope.do_request_focus(app, true);
            }
        }
    }

    /// Removes the keyboard token from this focus node if it has one.
    ///
    /// This mechanism helps distinguish between an input control gaining focus by default and
    /// gaining focus as a result of an explicit user action.
    ///
    /// When a focus node requests the focus (either via
    /// [`request_focus`](Self::request_focus) or [`FocusScopeNode::autofocus`]), the focus node
    /// receives a keyboard token if it does not already have one. Later, when the focus node
    /// becomes focused, the widget that manages the text input connection should show the
    /// keyboard only if it successfully consumes the keyboard token from the focus node.
    ///
    /// Returns true if this method successfully consumes the keyboard token.
    pub fn consume_keyboard_token(self, app: &mut App) -> bool {
        if !self.data(app).has_keyboard_token {
            return false;
        }
        self.data_mut(app).has_keyboard_token = false;
        true
    }

    /// Marks the node as being the next to be focused, meaning that it will become the primary
    /// focus and notify listeners of a focus change the next time focus is resolved by the
    /// manager. If something else calls `mark_next_focus` before then, then that node will
    /// become the next focus instead of the previous one.
    fn mark_next_focus(self, app: &mut App, new_focus: AnyFocusNode) {
        if let Some(manager) = self.manager(app) {
            // If we have a manager, then let it handle the focus change.
            manager.mark_next_focus(app, self);
            return;
        }
        // If we don't have a manager, then change the focus locally.
        new_focus.set_as_focused_child_for_scope(app);
        new_focus.notify(app);
        if new_focus != self {
            self.notify(app);
        }
    }

    /// Removes the given [`AnyFocusNode`] and its children as a child of this node.
    fn remove_child(self, app: &mut App, node: AnyFocusNode, remove_scope_focus: bool) {
        debug_assert!(
            self.data(app).children.contains(&node),
            "Tried to remove a node that wasn't a child."
        );
        debug_assert!(node.data(app).parent == Some(self));
        debug_assert!(node.manager(app) == self.manager(app));

        if remove_scope_focus && let Some(node_scope) = node.enclosing_scope(app) {
            remove_focused_child(app, node_scope, node);
            for descendant in node.descendants(app) {
                if descendant.enclosing_scope(app) == Some(node_scope) {
                    remove_focused_child(app, node_scope, descendant);
                }
            }
        }

        node.data_mut(app).parent = None;
        node.clear_enclosing_scope_cache(app);
        self.data_mut(app).children.retain(|child| *child != node);
        for ancestor in self.ancestors(app) {
            ancestor.data_mut(app).descendants = None;
        }
        self.data_mut(app).descendants = None;
        debug_assert!(match self.manager(app) {
            None => true,
            Some(manager) => {
                let root = app.get(manager).root_scope.as_node();
                !root.descendants(app).contains(&node)
            }
        });
    }

    fn update_manager(self, app: &mut App, manager: Option<Handle<FocusManager>>) {
        self.data_mut(app).manager = manager;
        for descendant in self.descendants(app) {
            let data = descendant.data_mut(app);
            data.manager = manager;
            data.ancestors = None;
        }
    }

    /// Used by [`FocusAttachment::reparent`] to perform the actual parenting operation.
    fn reparent_child(self, app: &mut App, child: AnyFocusNode) {
        debug_assert!(
            child != self,
            "Tried to make a child into a parent of itself."
        );
        if child.data(app).parent == Some(self) {
            debug_assert!(
                self.data(app).children.contains(&child),
                "Found a node that says it's a child, but doesn't appear in the child list."
            );
            // The child is already a child of this parent.
            return;
        }
        debug_assert!(match self.manager(app) {
            None => true,
            Some(manager) => child != app.get(manager).root_scope.as_node(),
        });
        debug_assert!(
            !self.ancestors(app).contains(&child),
            "The supplied child is already an ancestor of this node. Loops are not allowed."
        );
        let old_scope = child.enclosing_scope(app);
        let had_focus = child.has_focus(app);
        if let Some(parent) = child.data(app).parent {
            let nearest_scope = self.nearest_scope(app);
            parent.remove_child(app, child, old_scope != nearest_scope);
        }
        self.data_mut(app).children.push(child);
        let child_data = child.data_mut(app);
        child_data.parent = Some(self);
        child_data.ancestors = None;
        let manager = self.manager(app);
        child.update_manager(app, manager);
        for ancestor in child.ancestors(app) {
            ancestor.data_mut(app).descendants = None;
        }
        if had_focus {
            // Update the focus chain for the current focus without changing it.
            let primary_focus = self
                .manager(app)
                .and_then(|manager| app.get(manager).primary_focus);
            if let Some(primary_focus) = primary_focus {
                primary_focus.set_as_focused_child_for_scope(app);
            }
        }
        if let Some(old_scope) = old_scope
            && let Some(context) = child.data(app).context
            && child.enclosing_scope(app) != Some(old_scope)
            && let Some(policy) = FocusTraversalGroup::maybe_of(app, context)
        {
            policy.changed_scope(app, Some(child), Some(old_scope));
        }
        if child.data(app).request_focus_when_reparented {
            child.do_request_focus(app, true);
            child.data_mut(app).request_focus_when_reparented = false;
        }
    }

    /// Called by the _host_ `StatefulWidget` to attach a [`FocusNode`] to the widget tree.
    ///
    /// In order to attach a [`FocusNode`] to the widget tree, call
    /// [`attach`](Self::attach), typically from the `StatefulWidget`'s `State::init_state`
    /// method.
    ///
    /// If the focus node in the host widget is swapped out, the new node will need to be
    /// attached. [`FocusAttachment::detach`] should be called on the old node, and then
    /// [`attach`](Self::attach) called on the new node. This typically happens in the
    /// `State::did_update_widget` method.
    pub fn attach(
        self,
        app: &mut App,
        context: Option<BuildContext>,
        on_key_event: Option<FocusOnKeyEventCallback>,
    ) -> Handle<FocusAttachment> {
        self.data_mut(app).context = context;
        if on_key_event.is_some() {
            self.data_mut(app).on_key_event = on_key_event;
        }
        let attachment = FocusAttachment::new(app, self);
        self.data_mut(app).attachment = Some(attachment);
        attachment
    }

    /// Discards any resources used by this node.
    ///
    /// Detaching also unfocuses and cleans up the manager's data structures.
    pub fn dispose(self, app: &mut App) {
        if let Some(attachment) = self.data(app).attachment {
            attachment.detach(app);
        }
        (self.vtable.dispose_notifier)(app, self.id);
    }

    fn notify(self, app: &mut App) {
        if self.data(app).parent.is_none() {
            // no longer part of the tree, so don't notify.
            return;
        }
        if self.has_primary_focus(app) {
            self.set_as_focused_child_for_scope(app);
        }
        (self.vtable.notify_listeners)(app, self.id);
    }

    /// Requests the primary focus for this node, or for a supplied `node`, which will also give
    /// focus to its [`ancestors`](Self::ancestors).
    ///
    /// If called without a node, request focus for this node. If the node hasn't been added to
    /// the focus tree yet, then defer the focus request until it is, allowing newly created
    /// widgets to request focus as soon as they are added.
    ///
    /// If the given `node` is not yet a part of the focus tree, then this method will add the
    /// `node` as a child of this node before requesting focus.
    ///
    /// If the given `node` is a [`FocusScopeNode`] and that focus scope node has a non-null
    /// [`FocusScopeNode::focused_child`], then request the focus for the focused child. This
    /// process is recursive and continues until it encounters either a focus scope node with a
    /// null focused child or an ordinary (non-scope) [`FocusNode`] is found.
    ///
    /// The node is notified that it has received the primary focus in a microtask, so
    /// notification may lag the request by up to one frame.
    pub fn request_focus(self, app: &mut App, node: Option<AnyFocusNode>) {
        if let Some(node) = node {
            if node.data(app).parent.is_none() {
                self.reparent_child(app, node);
            }
            debug_assert!(
                node.ancestors(app).contains(&self),
                "Focus was requested for a node that is not a descendant of the scope from \
                 which it was requested."
            );
            node.do_request_focus(app, true);
            return;
        }
        self.do_request_focus(app, true);
    }

    /// Dispatches to the leaf; [`FocusScopeNode`] overrides this.
    fn do_request_focus(self, app: &mut App, find_first_focus: bool) {
        (self.vtable.do_request_focus)(app, self.id, find_first_focus);
    }

    /// Sets this node as the [`FocusScopeNode::focused_child`] of the enclosing scope.
    ///
    /// Sets this node as the focused child for the enclosing scope, and that scope as the
    /// focused child for the scope above it, etc., until it reaches the root node. It doesn't
    /// change the primary focus, it just changes what node would be focused if the enclosing
    /// scope receives focus, and keeps track of previously focused children in that scope, so
    /// that if the focused child in that scope is removed, the previous focus returns.
    fn set_as_focused_child_for_scope(self, app: &mut App) {
        let mut scope_focus = self;
        for ancestor in self.ancestors(app) {
            let Some(ancestor) = ancestor.as_scope() else {
                continue;
            };
            debug_assert!(
                scope_focus != ancestor.as_node(),
                "Somehow made a loop by setting focusedChild to its scope."
            );
            // Remove it anywhere in the focused child history.
            remove_focused_child(app, ancestor, scope_focus);
            // Add it to the end of the list, which is also the top of the queue: the end of the
            // list represents the currently focused child.
            app.get_mut(ancestor)
                .scope
                .focused_children
                .push(scope_focus);
            scope_focus = ancestor.as_node();
        }
    }

    /// Request to move the focus to the next focus node, by calling the
    /// [`FocusTraversalPolicy::next`] method.
    ///
    /// Returns true if it successfully found a node and requested focus.
    ///
    /// [`FocusTraversalPolicy::next`]: crate::FocusTraversalPolicy::next
    pub fn next_focus(self, app: &mut App) -> bool {
        let context = self
            .data(app)
            .context
            .expect("a node in the tree has a context");
        FocusTraversalGroup::of(app, context).next(app, self)
    }

    /// Request to move the focus to the previous focus node, by calling the
    /// [`FocusTraversalPolicy::previous`] method.
    ///
    /// Returns true if it successfully found a node and requested focus.
    ///
    /// [`FocusTraversalPolicy::previous`]: crate::FocusTraversalPolicy::previous
    pub fn previous_focus(self, app: &mut App) -> bool {
        let context = self
            .data(app)
            .context
            .expect("a node in the tree has a context");
        FocusTraversalGroup::of(app, context).previous(app, self)
    }

    /// Request to move the focus to the nearest focus node in the given direction, by calling
    /// the [`FocusTraversalPolicy::in_direction`] method.
    ///
    /// Returns true if it successfully found a node and requested focus.
    ///
    /// [`FocusTraversalPolicy::in_direction`]: crate::FocusTraversalPolicy::in_direction
    pub fn focus_in_direction(self, app: &mut App, direction: TraversalDirection) -> bool {
        let context = self
            .data(app)
            .context
            .expect("a node in the tree has a context");
        FocusTraversalGroup::of(app, context).in_direction(app, self, direction)
    }

    /// Register a closure to be called when this node notifies its listeners.
    pub fn add_listener(self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener);
    }

    /// Remove a previously registered closure from the list of closures this node notifies.
    pub fn remove_listener(self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener);
    }
}

/// Dart's `scope._focusedChildren.remove(node)`.
fn remove_focused_child(app: &mut App, scope: Handle<FocusScopeNode>, node: AnyFocusNode) {
    app.get_mut(scope)
        .scope
        .focused_children
        .retain(|child| *child != node);
}

/// An object that can be used by a stateful widget to obtain the keyboard focus and to handle
/// keyboard events.
///
/// _Please see the `Focus` and `FocusScope` widgets, which are utility widgets that manage
/// their own [`FocusNode`]s and [`FocusScopeNode`]s, respectively. If they aren't appropriate,
/// [`FocusNode`]s can be managed directly, but doing this is rare._
///
/// [`FocusNode`]s are persistent objects that form a _focus tree_ that is a representation of
/// the widgets in the hierarchy that are interested in focus. A focus node might need to be
/// created if it is passed in from an ancestor of a `Focus` widget to control the focus of the
/// children from the ancestor, or a widget might need to host one if the widget subsystem is
/// not being used, or if the `Focus` and `FocusScope` widgets provide insufficient control.
///
/// [`FocusNode`]s are organized into _scopes_ (see [`FocusScopeNode`]), which form sub-trees of
/// nodes that restrict traversal to a group of nodes. Within a scope, the most recent nodes to
/// have focus are remembered, and if a node is focused and then unfocused, the previous node
/// receives focus again.
///
/// The focus node hierarchy can be traversed using the [`AnyFocusNode::parent`],
/// [`AnyFocusNode::children`], [`AnyFocusNode::ancestors`] and [`AnyFocusNode::descendants`]
/// accessors.
///
/// [`FocusNode`]s are `ChangeNotifier`s, so a listener can be registered to receive a
/// notification when the focus changes. Listeners will also be notified when
/// [`AnyFocusNode::skip_traversal`], [`AnyFocusNode::can_request_focus`],
/// [`AnyFocusNode::descendants_are_focusable`], and
/// [`AnyFocusNode::descendants_are_traversable`] properties are updated.
///
/// ## Lifecycle
///
/// There are several actors involved in the lifecycle of a [`FocusNode`] / [`FocusScopeNode`].
/// They are created and disposed by their _owner_, attached, detached, and re-parented using a
/// [`FocusAttachment`] by their _host_ (which must be owned by the `State` of a
/// `StatefulWidget`), and they are managed by the [`FocusManager`].
///
/// Dart's optional constructor arguments (`debugLabel`, `onKeyEvent`, `skipTraversal`,
/// `canRequestFocus`, `descendantsAreFocusable`, `descendantsAreTraversable`) are the node's
/// setters; each of them is a public setter in Dart too, and none of them notifies before the
/// node joins a tree.
pub struct FocusNode {
    change_notifier: ChangeNotifierData,
    focus_node: FocusNodeData,
}

impl FocusNode {
    /// Creates a focus node.
    pub fn new(app: &mut App) -> Handle<FocusNode> {
        app.create(FocusNode {
            change_notifier: ChangeNotifierData::new(),
            focus_node: FocusNodeData::new(),
        })
    }

    // ---- the bodies a subclass reaches with `super` ----

    /// `FocusNode.nearestScope`.
    fn nearest_scope(this: AnyFocusNode, app: &mut App) -> Option<Handle<FocusScopeNode>> {
        this.enclosing_scope(app)
    }

    /// `FocusNode.descendantsAreFocusable`.
    fn descendants_are_focusable(this: AnyFocusNode, app: &App) -> bool {
        this.data(app).descendants_are_focusable
    }

    /// `FocusNode.traversalChildren`.
    fn traversal_children(this: AnyFocusNode, app: &mut App) -> Vec<AnyFocusNode> {
        if !this.descendants_are_focusable(app) {
            return Vec::new();
        }
        this.children(app)
            .into_iter()
            .filter(|node| !node.skip_traversal(app) && node.can_request_focus(app))
            .collect()
    }

    /// `FocusNode.traversalDescendants`.
    fn traversal_descendants(this: AnyFocusNode, app: &mut App) -> Vec<AnyFocusNode> {
        if !this.descendants_are_focusable(app) {
            return Vec::new();
        }
        this.descendants(app)
            .into_iter()
            .filter(|node| !node.skip_traversal(app) && node.can_request_focus(app))
            .collect()
    }

    /// `FocusNode._doRequestFocus`.
    fn do_request_focus(this: AnyFocusNode, app: &mut App, _find_first_focus: bool) {
        if !this.can_request_focus(app) {
            return;
        }
        // If the node isn't part of the tree, then we just defer the focus request until the
        // next time it is reparented, so that it's possible to focus newly added widgets.
        if this.data(app).parent.is_none() {
            this.data_mut(app).request_focus_when_reparented = true;
            return;
        }
        this.set_as_focused_child_for_scope(app);
        let marked_for_focus = this
            .manager(app)
            .and_then(|manager| app.get(manager).marked_for_focus);
        if this.has_primary_focus(app)
            && (marked_for_focus.is_none() || marked_for_focus == Some(this))
        {
            return;
        }
        this.data_mut(app).has_keyboard_token = true;
        this.mark_next_focus(app, this);
    }
}

impl ChangeNotifier for FocusNode {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl FocusNodeLeaf for FocusNode {
    crate::focus_node_accessors!();
}

// ---------------------------------------------------------------------------------------------
// FocusScopeNode

/// The fields Dart's `FocusScopeNode` adds to [`FocusNodeData`].
pub struct FocusScopeNodeData {
    /// Controls the transfer of focus beyond the first and the last items of a
    /// [`FocusScopeNode`].
    ///
    /// Changing this field value has no immediate effect on the UI. Instead, next time focus
    /// traversal takes place `FocusTraversalPolicy` will read this value and apply the new
    /// behavior.
    pub traversal_edge_behavior: TraversalEdgeBehavior,

    /// Controls the directional transfer of focus when the focus is on the first or last item.
    ///
    /// Changing this field value has no immediate effect on the UI. Instead, next time focus
    /// traversal takes place `FocusTraversalPolicy` will read this value and apply the new
    /// behavior.
    pub directional_traversal_edge_behavior: TraversalEdgeBehavior,

    /// A stack of the children that have been set as the `focused_child`, most recent last
    /// (which is the top of the stack).
    focused_children: Vec<AnyFocusNode>,
}

/// A subclass of [`FocusNode`] that acts as a scope for its descendants, maintaining
/// information about which descendant is currently or was last focused.
///
/// _Please see the `FocusScope` and `Focus` widgets, which are utility widgets that manage
/// their own [`FocusScopeNode`]s and [`FocusNode`]s, respectively. If they aren't appropriate,
/// [`FocusScopeNode`]s can be managed directly._
///
/// [`FocusScopeNode`] organizes [`FocusNode`]s into _scopes_. Scopes form sub-trees of nodes
/// that can be traversed as a group. Within a scope, the most recent nodes to have focus are
/// remembered, and if a node is focused and then removed, the original node receives focus
/// again.
///
/// From a [`FocusScopeNode`], calling [`set_first_focus`](Self::set_first_focus) sets the given
/// focus scope as the [`focused_child`](Self::focused_child) of this node, adopting if it isn't
/// already part of the focus tree.
pub struct FocusScopeNode {
    change_notifier: ChangeNotifierData,
    focus_node: FocusNodeData,
    scope: FocusScopeNodeData,
}

impl FocusScopeNode {
    /// Creates a [`FocusScopeNode`].
    ///
    /// Dart's optional constructor arguments are the setters; `descendantsAreFocusable` is
    /// always true on a scope, which is [`FocusNodeData`]'s default.
    pub fn new(app: &mut App) -> Handle<FocusScopeNode> {
        app.create(FocusScopeNode {
            change_notifier: ChangeNotifierData::new(),
            focus_node: FocusNodeData::new(),
            scope: FocusScopeNodeData {
                traversal_edge_behavior: TraversalEdgeBehavior::ClosedLoop,
                directional_traversal_edge_behavior: TraversalEdgeBehavior::Stop,
                focused_children: Vec::new(),
            },
        })
    }

    /// See [`FocusScopeNodeData::traversal_edge_behavior`].
    pub fn traversal_edge_behavior(self: Handle<Self>, app: &App) -> TraversalEdgeBehavior {
        app.get(self).scope.traversal_edge_behavior
    }

    /// See [`FocusScopeNodeData::traversal_edge_behavior`].
    pub fn set_traversal_edge_behavior(
        self: Handle<Self>,
        app: &mut App,
        value: TraversalEdgeBehavior,
    ) {
        app.get_mut(self).scope.traversal_edge_behavior = value;
    }

    /// See [`FocusScopeNodeData::directional_traversal_edge_behavior`].
    pub fn directional_traversal_edge_behavior(
        self: Handle<Self>,
        app: &App,
    ) -> TraversalEdgeBehavior {
        app.get(self).scope.directional_traversal_edge_behavior
    }

    /// See [`FocusScopeNodeData::directional_traversal_edge_behavior`].
    pub fn set_directional_traversal_edge_behavior(
        self: Handle<Self>,
        app: &mut App,
        value: TraversalEdgeBehavior,
    ) {
        app.get_mut(self).scope.directional_traversal_edge_behavior = value;
    }

    /// Returns true if this scope is the focused child of its parent scope.
    pub fn is_first_focus(self: Handle<Self>, app: &mut App) -> bool {
        let enclosing = self
            .enclosing_scope(app)
            .expect("a scope that is asked for isFirstFocus has an enclosing scope");
        enclosing.focused_child(app) == Some(self.as_node())
    }

    /// Returns the child of this node that should receive focus if this scope node receives
    /// focus.
    ///
    /// If [`has_focus`](FocusNodeLeaf::has_focus) is true, then this points to the child of
    /// this node that is currently focused.
    ///
    /// Returns `None` if there is no currently focused child.
    pub fn focused_child(self: Handle<Self>, app: &mut App) -> Option<AnyFocusNode> {
        let last = app.get(self).scope.focused_children.last().copied();
        debug_assert!(match last {
            None => true,
            Some(last) => last.enclosing_scope(app) == Some(self),
        });
        last
    }

    /// Make the given `scope` the active child scope for this scope.
    ///
    /// If the given `scope` is not yet a part of the focus tree, then add it to the tree as a
    /// child of this scope. If it is already part of the focus tree, the given scope must be a
    /// descendant of this scope.
    pub fn set_first_focus(self: Handle<Self>, app: &mut App, scope: Handle<FocusScopeNode>) {
        debug_assert!(scope != self, "Unexpected self-reference in setFirstFocus.");
        if scope.parent(app).is_none() {
            self.as_node().reparent_child(app, scope.as_node());
        }
        debug_assert!(
            scope.ancestors(app).contains(&self.as_node()),
            "A FocusScopeNode must be a child of this scope to set it as first focus."
        );
        if self.has_focus(app) {
            scope.do_request_focus(app, true);
        } else {
            scope.as_node().set_as_focused_child_for_scope(app);
        }
    }

    /// If this scope lacks a focus, request that the given node become the focus.
    ///
    /// If the given node is not yet part of the focus tree, then add it as a child of this
    /// node.
    ///
    /// Useful for widgets that wish to grab the focus if no other widget already has the focus.
    ///
    /// The node is notified that it has received the primary focus in a microtask, so
    /// notification may lag the request by up to one frame.
    pub fn autofocus(self: Handle<Self>, app: &mut App, node: AnyFocusNode) {
        // Attach the node to the tree first, so in `apply_focus_change` if the node is detached
        // we don't add it back to the tree.
        if node.parent(app).is_none() {
            self.as_node().reparent_child(app, node);
        }
        debug_assert!(self.as_node().manager(app).is_some());
        if let Some(manager) = self.as_node().manager(app) {
            app.get_mut(manager).pending_autofocuses.push(Autofocus {
                scope: self,
                autofocus_node: node,
            });
            manager.mark_needs_update(app);
        }
    }

    /// Requests that the scope itself receive focus, without trying to find a descendant that
    /// should receive focus.
    ///
    /// This is used only if you want to park the focus on a scope itself.
    pub fn request_scope_focus(self: Handle<Self>, app: &mut App) {
        self.do_request_focus(app, false);
    }
}

impl ChangeNotifier for FocusScopeNode {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl FocusNodeLeaf for FocusScopeNode {
    crate::focus_node_accessors!();

    fn as_scope(self: Handle<Self>) -> Option<Handle<FocusScopeNode>> {
        Some(self)
    }

    fn nearest_scope(self: Handle<Self>, _app: &mut App) -> Option<Handle<FocusScopeNode>> {
        Some(self)
    }

    fn descendants_are_focusable(self: Handle<Self>, app: &App) -> bool {
        app.get(self).focus_node.can_request_focus
            && FocusNode::descendants_are_focusable(self.as_node(), app)
    }

    /// Will return an empty iterable if this scope node is not focusable, or if
    /// `descendants_are_focusable` is false.
    fn traversal_children(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        if !self.can_request_focus(app) {
            return Vec::new();
        }
        FocusNode::traversal_children(self.as_node(), app)
    }

    /// Will return an empty iterable if this scope node is not focusable, or if
    /// `descendants_are_focusable` is false.
    fn traversal_descendants(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        if !self.can_request_focus(app) {
            return Vec::new();
        }
        FocusNode::traversal_descendants(self.as_node(), app)
    }

    fn do_request_focus(self: Handle<Self>, app: &mut App, find_first_focus: bool) {
        // It is possible that a previously focused child is no longer focusable, so clean out
        // the list if so.
        loop {
            let last = app.get(self).scope.focused_children.last().copied();
            let Some(last) = last else { break };
            if last.can_request_focus(app) && last.enclosing_scope(app).is_some() {
                break;
            }
            app.get_mut(self).scope.focused_children.pop();
        }

        let focused_child = self.focused_child(app);
        // If find_first_focus is false, then the request is to make this scope the focus
        // instead of looking for the ultimate first focus for this scope and its descendants.
        let Some(focused_child) = focused_child.filter(|_| find_first_focus) else {
            if self.can_request_focus(app) {
                self.as_node().set_as_focused_child_for_scope(app);
                self.as_node().mark_next_focus(app, self.as_node());
            }
            return;
        };

        focused_child.do_request_focus(app, true);
    }
}

// ---------------------------------------------------------------------------------------------
// The highlight mode

/// An enum to describe which kind of focus highlight behavior to use when displaying focus
/// information.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusHighlightMode {
    /// Touch interfaces will not show the focus highlight except for controls which bring up
    /// the soft keyboard.
    ///
    /// If a device that uses a traditional mouse and keyboard has a touch screen attached, it
    /// can also enter `touch` mode if the user is using the touch screen.
    Touch,

    /// Traditional interfaces (keyboard and mouse) will show the currently focused control via
    /// a focus highlight of some sort.
    ///
    /// If a touch device (like a mobile phone) has a keyboard and/or mouse attached, it also
    /// can enter `traditional` mode if the user is using these input devices.
    Traditional,
}

/// An enum to describe how the current value of [`FocusManager::highlight_mode`] is determined.
/// The strategy is set on [`FocusManager::set_highlight_strategy`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusHighlightStrategy {
    /// Automatic switches between the various highlight modes based on the last kind of input
    /// that was received. This is the default.
    #[default]
    Automatic,

    /// [`FocusManager::highlight_mode`] always returns [`FocusHighlightMode::Touch`].
    AlwaysTouch,

    /// [`FocusManager::highlight_mode`] always returns [`FocusHighlightMode::Traditional`].
    AlwaysTraditional,
}

// ---------------------------------------------------------------------------------------------
// FocusManager

/// Manages the focus tree.
///
/// The focus tree is a separate, sparser, tree from the widget tree that maintains the
/// hierarchical relationship between focusable widgets in the widget tree.
///
/// The focus manager is responsible for tracking which [`FocusNode`] has the primary input
/// focus (the [`primary_focus`](Self::primary_focus)), holding the [`FocusScopeNode`] that is
/// the root of the focus tree (the [`root_scope`](Self::root_scope)), and what the current
/// [`highlight_mode`](Self::highlight_mode) is. It also distributes [`KeyEvent`]s to the nodes
/// in the focus tree.
///
/// The singleton [`FocusManager`] instance is held by the `WidgetsBinding` as
/// `WidgetsBinding::focus_manager`, and can be conveniently accessed using the
/// [`FocusManager::instance`] accessor.
///
/// To find the [`FocusNode`] for a given `BuildContext`, use `Focus::of`. To find the
/// [`FocusScopeNode`] for a given `BuildContext`, use `FocusScope::of`.
///
/// If you would like notification whenever the [`primary_focus`](Self::primary_focus) changes,
/// register a listener with `add_listener`.
///
/// The [`highlight_mode`](Self::highlight_mode) describes how focus highlights should be
/// displayed on components in the UI. The highlight mode changes are notified separately via
/// [`add_highlight_mode_listener`](Self::add_highlight_mode_listener) and removed with
/// [`remove_highlight_mode_listener`](Self::remove_highlight_mode_listener).
pub struct FocusManager {
    change_notifier: ChangeNotifierData,
    highlight_manager: Handle<HighlightModeManager>,
    /// The root [`FocusScopeNode`] in the focus tree.
    root_scope: Handle<FocusScopeNode>,
    primary_focus: Option<AnyFocusNode>,
    /// The set of nodes that need to notify their listeners of changes at the next update.
    dirty_nodes: Vec<AnyFocusNode>,
    /// The node that has requested to have the primary focus, but hasn't been given it yet.
    marked_for_focus: Option<AnyFocusNode>,
    /// The list of autofocus requests made since the last `apply_focus_change` call.
    pending_autofocuses: Vec<Autofocus>,
    /// True indicates that there is an update pending.
    have_scheduled_update: bool,
}

impl FocusManager {
    /// Creates an object that manages the focus tree.
    ///
    /// This constructor is rarely called directly. To access the [`FocusManager`], consider
    /// using the [`instance`](Self::instance) accessor instead (which gets it from the
    /// `WidgetsBinding` singleton).
    ///
    /// This newly constructed focus manager does not have the necessary event handlers
    /// registered to allow it to manage focus. To register those event handlers, callers must
    /// call [`register_global_handlers`](Self::register_global_handlers).
    pub fn new(app: &mut App) -> Handle<FocusManager> {
        let root_scope = FocusScopeNode::new(app);
        root_scope.set_debug_label(app, Some("Root Focus Scope".to_string()));
        let highlight_manager = HighlightModeManager::new(app);
        // Dart also subscribes to the application lifecycle here; that waits (see the crate's
        // PORTING.md).
        let this = app.create(FocusManager {
            change_notifier: ChangeNotifierData::new(),
            highlight_manager,
            root_scope,
            primary_focus: None,
            dirty_nodes: Vec::new(),
            marked_for_focus: None,
            pending_autofocuses: Vec::new(),
            have_scheduled_update: false,
        });
        root_scope.focus_node_data_mut(app).manager = Some(this);
        this
    }

    /// Registers global input event handlers that are needed to manage focus.
    ///
    /// This calls `HardwareKeyboard::add_handler` on the shared instance of `HardwareKeyboard`
    /// and adds a route to the global entry in the gesture routing table. As such, only one
    /// [`FocusManager`] instance should register its global handlers.
    ///
    /// When this focus manager is no longer needed, calling [`dispose`](Self::dispose) on it
    /// will unregister these handlers.
    pub fn register_global_handlers(self: Handle<Self>, app: &mut App) {
        app.get(self)
            .highlight_manager
            .register_global_handlers(app);
    }

    /// Discards any resources used by this object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).highlight_manager.dispose(app);
        app.get(self).root_scope.dispose(app);
        app.get_mut(self).change_notifier.dispose();
    }

    /// Provides convenient access to the current [`FocusManager`] singleton from the
    /// `WidgetsBinding` instance.
    pub fn instance(app: &mut App) -> Handle<FocusManager> {
        WidgetsBinding::instance(app).focus_manager(app)
    }

    /// The strategy by which [`highlight_mode`](Self::highlight_mode) is determined.
    ///
    /// If set to [`FocusHighlightStrategy::Automatic`], then the highlight mode will change
    /// depending upon the interaction mode used last.
    ///
    /// Defaults to [`FocusHighlightStrategy::Automatic`].
    pub fn highlight_strategy(self: Handle<Self>, app: &App) -> FocusHighlightStrategy {
        app.get(app.get(self).highlight_manager).strategy
    }

    /// See [`highlight_strategy`](Self::highlight_strategy).
    pub fn set_highlight_strategy(
        self: Handle<Self>,
        app: &mut App,
        value: FocusHighlightStrategy,
    ) {
        let highlight_manager = app.get(self).highlight_manager;
        if app.get(highlight_manager).strategy == value {
            return;
        }
        highlight_manager.set_strategy(app, value);
    }

    /// Indicates the current interaction mode for focus highlights.
    ///
    /// The value returned depends upon the
    /// [`highlight_strategy`](Self::highlight_strategy) used, and possibly the most recent
    /// interaction mode that the user used.
    ///
    /// If [`highlight_mode`](Self::highlight_mode) returns [`FocusHighlightMode::Touch`], then
    /// widgets should not draw their focus highlight unless they perform text entry.
    ///
    /// If it returns [`FocusHighlightMode::Traditional`], then widgets should draw their focus
    /// highlight whenever they are focused.
    pub fn highlight_mode(self: Handle<Self>, app: &mut App) -> FocusHighlightMode {
        app.get(self).highlight_manager.highlight_mode(app)
    }

    /// Register a closure to be called when the [`FocusManager`] notifies its listeners that
    /// the value of [`highlight_mode`](Self::highlight_mode) has changed.
    pub fn add_highlight_mode_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: HighlightModeListener,
    ) {
        app.get(self).highlight_manager.add_listener(app, listener);
    }

    /// Remove a previously registered closure from the list of closures that the
    /// [`FocusManager`] notifies.
    pub fn remove_highlight_mode_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &HighlightModeListener,
    ) {
        app.get(self)
            .highlight_manager
            .remove_listener(app, listener);
    }

    /// Adds a key event handler to a set of handlers that are called before any key event
    /// handlers in the focus tree are called.
    ///
    /// All of the handlers in the set will be called for every key event the [`FocusManager`]
    /// receives. If any one of the handlers returns [`KeyEventResult::Handled`] or
    /// [`KeyEventResult::SkipRemainingHandlers`], then none of the handlers in the focus tree
    /// will be called.
    pub fn add_early_key_event_handler(
        self: Handle<Self>,
        app: &mut App,
        handler: OnKeyEventCallback,
    ) {
        app.get(self)
            .highlight_manager
            .add_early_key_event_handler(app, handler);
    }

    /// Removes a key handler added by calling
    /// [`add_early_key_event_handler`](Self::add_early_key_event_handler).
    pub fn remove_early_key_event_handler(
        self: Handle<Self>,
        app: &mut App,
        handler: &OnKeyEventCallback,
    ) {
        app.get(self)
            .highlight_manager
            .remove_early_key_event_handler(app, handler);
    }

    /// Adds a key event handler to a set of handlers that are called if none of the key event
    /// handlers in the focus tree handle the event.
    ///
    /// If the event reaches the root of the focus tree without being handled, then all of the
    /// handlers in the set will be called. If any of them returns [`KeyEventResult::Handled`]
    /// or [`KeyEventResult::SkipRemainingHandlers`], then event propagation to the platform
    /// will be stopped.
    pub fn add_late_key_event_handler(
        self: Handle<Self>,
        app: &mut App,
        handler: OnKeyEventCallback,
    ) {
        app.get(self)
            .highlight_manager
            .add_late_key_event_handler(app, handler);
    }

    /// Removes a key handler added by calling
    /// [`add_late_key_event_handler`](Self::add_late_key_event_handler).
    pub fn remove_late_key_event_handler(
        self: Handle<Self>,
        app: &mut App,
        handler: &OnKeyEventCallback,
    ) {
        app.get(self)
            .highlight_manager
            .remove_late_key_event_handler(app, handler);
    }

    /// The root [`FocusScopeNode`] in the focus tree.
    ///
    /// This field is rarely used directly. To find the nearest [`FocusScopeNode`] for a given
    /// [`FocusNode`], call [`AnyFocusNode::nearest_scope`].
    pub fn root_scope(self: Handle<Self>, app: &App) -> Handle<FocusScopeNode> {
        app.get(self).root_scope
    }

    /// The node that currently has the primary focus.
    pub fn primary_focus(self: Handle<Self>, app: &App) -> Option<AnyFocusNode> {
        app.get(self).primary_focus
    }

    fn mark_detached(self: Handle<Self>, app: &mut App, node: AnyFocusNode) {
        // The node has been removed from the tree, so it no longer needs to be notified of
        // changes.
        let manager = app.get_mut(self);
        if manager.primary_focus == Some(node) {
            manager.primary_focus = None;
        }
        manager.dirty_nodes.retain(|dirty| *dirty != node);
    }

    fn mark_properties_changed(self: Handle<Self>, app: &mut App, node: AnyFocusNode) {
        self.mark_needs_update(app);
        add_dirty_node(app, self, node);
    }

    fn mark_next_focus(self: Handle<Self>, app: &mut App, node: AnyFocusNode) {
        if app.get(self).primary_focus == Some(node) {
            // The caller asked for the current focus to be the next focus, so just pretend that
            // didn't happen.
            app.get_mut(self).marked_for_focus = None;
        } else {
            app.get_mut(self).marked_for_focus = Some(node);
            self.mark_needs_update(app);
        }
    }

    /// Request that an update be scheduled, optionally requesting focus for the given newFocus
    /// node.
    fn mark_needs_update(self: Handle<Self>, app: &mut App) {
        if app.get(self).have_scheduled_update {
            return;
        }
        app.get_mut(self).have_scheduled_update = true;
        app.schedule_microtask(Listener::handle_method(
            self,
            FocusManager::apply_focus_changes_if_needed,
        ));
    }

    /// Applies any pending focus changes and notifies listeners that the focus has changed.
    ///
    /// Must not be called during the build phase. This method is meant to be called in a
    /// post-frame callback or microtask when the pending focus changes need to be resolved
    /// before something else occurs.
    ///
    /// It can't be called during the build phase because not all listeners are safe to be
    /// called with an update during a build.
    ///
    /// Typically, this is called automatically by the [`FocusManager`], but sometimes it is
    /// necessary to ensure that no focus changes are pending before executing an action.
    ///
    /// It is safe to call this if no focus changes are pending.
    pub fn apply_focus_changes_if_needed(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks,
            "applyFocusChangesIfNeeded() should not be called during the build phase."
        );

        app.get_mut(self).have_scheduled_update = false;
        let previous_focus = app.get(self).primary_focus;

        for autofocus in std::mem::take(&mut app.get_mut(self).pending_autofocuses) {
            autofocus.apply_if_valid(app, self);
        }

        if app.get(self).primary_focus.is_none() && app.get(self).marked_for_focus.is_none() {
            // If we don't have any current focus, and nobody has asked to focus yet, then
            // revert to the root scope.
            let root_scope = app.get(self).root_scope;
            app.get_mut(self).marked_for_focus = Some(root_scope.as_node());
        }
        // A node has requested to be the next focus, and isn't already the primary focus.
        let marked_for_focus = app.get(self).marked_for_focus;
        if let Some(marked_for_focus) = marked_for_focus
            && Some(marked_for_focus) != app.get(self).primary_focus
        {
            let previous_path = match previous_focus {
                Some(previous) => previous.ancestors(app),
                None => Vec::new(),
            };
            let next_path = marked_for_focus.ancestors(app);
            // Notify nodes that are newly focused.
            for node in next_path
                .iter()
                .filter(|node| !previous_path.contains(node))
            {
                add_dirty_node(app, self, *node);
            }
            // Notify nodes that are no longer focused.
            for node in previous_path
                .iter()
                .filter(|node| !next_path.contains(node))
            {
                add_dirty_node(app, self, *node);
            }

            let manager = app.get_mut(self);
            manager.primary_focus = Some(marked_for_focus);
            manager.marked_for_focus = None;
        }
        debug_assert!(app.get(self).marked_for_focus.is_none());
        let primary_focus = app.get(self).primary_focus;
        if previous_focus != primary_focus {
            if let Some(previous_focus) = previous_focus {
                add_dirty_node(app, self, previous_focus);
            }
            if let Some(primary_focus) = primary_focus {
                add_dirty_node(app, self, primary_focus);
            }
        }
        for node in std::mem::take(&mut app.get_mut(self).dirty_nodes) {
            node.notify(app);
        }
        if previous_focus != primary_focus {
            self.notify_listeners(app);
        }
    }
}

/// Dart's `_dirtyNodes.add(node)`: a set that keeps insertion order.
fn add_dirty_node(app: &mut App, manager: Handle<FocusManager>, node: AnyFocusNode) {
    let dirty_nodes = &mut app.get_mut(manager).dirty_nodes;
    if !dirty_nodes.contains(&node) {
        dirty_nodes.push(node);
    }
}

impl ChangeNotifier for FocusManager {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

// ---------------------------------------------------------------------------------------------
// The highlight mode manager

/// A class to detect and manage the highlight mode transitions. An instance of this is owned by
/// the [`FocusManager`].
///
/// This doesn't hold a `ChangeNotifierData` because the callback passes the updated value, and
/// `ChangeNotifier` requires using a plain [`Listener`].
struct HighlightModeManager {
    /// If `None`, no interactions have occurred yet and the default highlight mode for the
    /// current platform applies.
    last_interaction_requires_traditional_highlights: Option<bool>,
    highlight_mode: Option<FocusHighlightMode>,
    strategy: FocusHighlightStrategy,
    /// The list of callbacks for early key handling.
    early_key_event_handlers: Vec<OnKeyEventCallback>,
    /// The list of callbacks for late key handling.
    late_key_event_handlers: Vec<OnKeyEventCallback>,
    /// The list of listeners for `highlight_mode` state changes.
    listeners: Vec<HighlightModeListener>,
    /// The handler `register_global_handlers` gave `HardwareKeyboard`, kept so that
    /// `dispose` can hand the same `Rc` back.
    key_event_handler: Option<KeyEventCallback>,
}

impl HighlightModeManager {
    fn new(app: &mut App) -> Handle<HighlightModeManager> {
        app.create(HighlightModeManager {
            last_interaction_requires_traditional_highlights: None,
            highlight_mode: None,
            strategy: FocusHighlightStrategy::default(),
            early_key_event_handlers: Vec::new(),
            late_key_event_handlers: Vec::new(),
            listeners: Vec::new(),
            key_event_handler: None,
        })
    }

    fn highlight_mode(self: Handle<Self>, app: &mut App) -> FocusHighlightMode {
        match app.get(self).highlight_mode {
            Some(mode) => mode,
            None => HighlightModeManager::default_mode_for_platform(app),
        }
    }

    fn set_strategy(self: Handle<Self>, app: &mut App, value: FocusHighlightStrategy) {
        if app.get(self).strategy == value {
            return;
        }
        app.get_mut(self).strategy = value;
        self.update_mode(app);
    }

    fn add_early_key_event_handler(
        self: Handle<Self>,
        app: &mut App,
        callback: OnKeyEventCallback,
    ) {
        app.get_mut(self).early_key_event_handlers.push(callback);
    }

    fn remove_early_key_event_handler(
        self: Handle<Self>,
        app: &mut App,
        callback: &OnKeyEventCallback,
    ) {
        app.get_mut(self)
            .early_key_event_handlers
            .retain(|handler| !Rc::ptr_eq(handler, callback));
    }

    fn add_late_key_event_handler(self: Handle<Self>, app: &mut App, callback: OnKeyEventCallback) {
        app.get_mut(self).late_key_event_handlers.push(callback);
    }

    fn remove_late_key_event_handler(
        self: Handle<Self>,
        app: &mut App,
        callback: &OnKeyEventCallback,
    ) {
        app.get_mut(self)
            .late_key_event_handlers
            .retain(|handler| !Rc::ptr_eq(handler, callback));
    }

    fn add_listener(self: Handle<Self>, app: &mut App, listener: HighlightModeListener) {
        app.get_mut(self).listeners.push(listener);
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &HighlightModeListener) {
        app.get_mut(self)
            .listeners
            .retain(|registered| !Rc::ptr_eq(registered, listener));
    }

    fn register_global_handlers(self: Handle<Self>, app: &mut App) {
        let handler: KeyEventCallback =
            Rc::new(move |app: &mut App, event: &KeyEvent| self.handle_key_event(app, event));
        app.get_mut(self).key_event_handler = Some(Rc::clone(&handler));
        HardwareKeyboard::instance(app).add_handler(app, handler);
        let router = GestureBinding::instance(app).pointer_router(app);
        router.add_global_route(
            app,
            PointerRoute::handle_method(self, HighlightModeManager::handle_pointer_event),
            None,
        );
        // Dart also listens for semantics actions here; accessibility waits.
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(handler) = app.get_mut(self).key_event_handler.take() {
            let router = GestureBinding::instance(app).pointer_router(app);
            router.remove_global_route(
                app,
                &PointerRoute::handle_method(self, HighlightModeManager::handle_pointer_event),
            );
            HardwareKeyboard::instance(app).remove_handler(app, &handler);
        }
        app.get_mut(self).listeners = Vec::new();
    }

    fn notify_listeners(self: Handle<Self>, app: &mut App) {
        if app.get(self).listeners.is_empty() {
            return;
        }
        let highlight_mode = self.highlight_mode(app);
        // Make a copy to prevent problems if the list is modified during iteration.
        let local_listeners = app.get(self).listeners.clone();
        for listener in local_listeners {
            let still_registered = app
                .get(self)
                .listeners
                .iter()
                .any(|registered| Rc::ptr_eq(registered, &listener));
            if still_registered {
                listener(app, highlight_mode);
            }
        }
    }

    fn handle_pointer_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        match event.kind() {
            PointerDeviceKind::Touch
            | PointerDeviceKind::Stylus
            | PointerDeviceKind::InvertedStylus => {
                if app
                    .get(self)
                    .last_interaction_requires_traditional_highlights
                    != Some(true)
                {
                    app.get_mut(self)
                        .last_interaction_requires_traditional_highlights = Some(true);
                    self.update_mode(app);
                }
            }
            PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad | PointerDeviceKind::Unknown => {
            }
        }
    }

    /// Dart's `handleKeyMessage`: the handler `HardwareKeyboard` calls for every key event.
    fn handle_key_event(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> bool {
        // Update highlight_mode first, since things responding to the keys might look at the
        // highlight mode, and it should be accurate.
        if app
            .get(self)
            .last_interaction_requires_traditional_highlights
            != Some(false)
        {
            app.get_mut(self)
                .last_interaction_requires_traditional_highlights = Some(false);
            self.update_mode(app);
        }

        if FocusManager::instance(app).primary_focus(app).is_none() {
            return false;
        }

        let mut handled = false;
        // Check to see if any of the early handlers handle the key. If so, then return early.
        if !app.get(self).early_key_event_handlers.is_empty() {
            // Make a copy to prevent problems if the list is modified during iteration.
            let callbacks = app.get(self).early_key_event_handlers.clone();
            let results: Vec<KeyEventResult> = callbacks
                .into_iter()
                .map(|callback| callback(app, event))
                .collect();
            match combine_key_event_results(results) {
                KeyEventResult::Ignored => {}
                KeyEventResult::Handled => handled = true,
                KeyEventResult::SkipRemainingHandlers => handled = false,
            }
        }
        if handled {
            return true;
        }

        // Walk the current focus from the leaf to the root, calling each node's on_key_event on
        // the way up, and if one responds that they handled it or want to stop propagation,
        // stop.
        let primary_focus = FocusManager::instance(app)
            .primary_focus(app)
            .expect("checked above");
        let mut nodes = vec![primary_focus];
        nodes.extend(primary_focus.ancestors(app));
        for node in nodes {
            let results: Vec<KeyEventResult> = match node.on_key_event(app) {
                Some(on_key_event) => vec![on_key_event(app, node, event)],
                None => Vec::new(),
            };
            let result = combine_key_event_results(results);
            match result {
                KeyEventResult::Ignored => continue,
                KeyEventResult::Handled => handled = true,
                KeyEventResult::SkipRemainingHandlers => handled = false,
            }
            // Only KeyEventResult::Ignored will continue the loop. All other options will stop
            // the event propagation.
            debug_assert!(result != KeyEventResult::Ignored);
            break;
        }

        // Check to see if any late key event handlers want to handle the event.
        if !handled && !app.get(self).late_key_event_handlers.is_empty() {
            // Make a copy to prevent problems if the list is modified during iteration.
            let callbacks = app.get(self).late_key_event_handlers.clone();
            let results: Vec<KeyEventResult> = callbacks
                .into_iter()
                .map(|callback| callback(app, event))
                .collect();
            match combine_key_event_results(results) {
                KeyEventResult::Ignored => {}
                KeyEventResult::Handled => handled = true,
                KeyEventResult::SkipRemainingHandlers => handled = false,
            }
        }
        handled
    }

    /// Update function to be called whenever the state relating to `highlight_mode` changes.
    fn update_mode(self: Handle<Self>, app: &mut App) {
        let new_mode = match app.get(self).strategy {
            FocusHighlightStrategy::Automatic => {
                match app
                    .get(self)
                    .last_interaction_requires_traditional_highlights
                {
                    // If we don't have any information about the last interaction yet, then just
                    // rely on the default value for the platform, which will be determined based
                    // on the target platform if `highlight_mode` is not set.
                    None => return,
                    Some(true) => FocusHighlightMode::Touch,
                    Some(false) => FocusHighlightMode::Traditional,
                }
            }
            FocusHighlightStrategy::AlwaysTouch => FocusHighlightMode::Touch,
            FocusHighlightStrategy::AlwaysTraditional => FocusHighlightMode::Traditional,
        };
        // We can't just compare new_mode with `highlight_mode` here, since `highlight_mode`
        // could be unset, so we want to compare with the return value for the getter, since
        // that's what clients will be looking at.
        let old_mode = self.highlight_mode(app);
        app.get_mut(self).highlight_mode = Some(new_mode);
        if self.highlight_mode(app) != old_mode {
            self.notify_listeners(app);
        }
    }

    fn default_mode_for_platform(app: &mut App) -> FocusHighlightMode {
        // Assume that if we're on one of the mobile platforms, and there's no mouse connected,
        // that the initial interaction will be touch-based, and that it's traditional mouse and
        // keyboard on all other platforms.
        //
        // This only affects the initial value: the ongoing value is updated to a known correct
        // value as soon as any pointer/keyboard events are received.
        match app.platform().target_platform() {
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => {
                let mouse_tracker = RendererBinding::instance(app).mouse_tracker(app);
                if mouse_tracker.mouse_is_connected(app) {
                    FocusHighlightMode::Traditional
                } else {
                    FocusHighlightMode::Touch
                }
            }
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                FocusHighlightMode::Traditional
            }
        }
    }
}

/// Provides convenient access to the current [`FocusManager::primary_focus`] from the
/// `WidgetsBinding` instance.
pub fn primary_focus(app: &mut App) -> Option<AnyFocusNode> {
    let manager = WidgetsBinding::instance(app).focus_manager(app);
    manager.primary_focus(app)
}

// ---------------------------------------------------------------------------------------------
// TraversalEdgeBehavior

/// Controls the transfer of focus beyond the first and the last items of a
/// [`FocusScopeNode`].
///
/// This enumeration only controls the traversal behavior performed by
/// `FocusTraversalPolicy`. Other methods of focus transfer, such as direct calls to
/// [`AnyFocusNode::request_focus`] and [`AnyFocusNode::unfocus`], are not affected by this
/// enumeration.
///
/// Dart declares this in `focus_traversal.dart`; it lives here, where [`FocusScopeNode`] reads
/// it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TraversalEdgeBehavior {
    /// Keeps the focus among the items of the focus scope.
    ///
    /// Transfer focus to the edge node in the opposite direction of [`FocusScopeNode`] as the
    /// edge node continues to move, thus forming a closed loop of focusable items.
    #[default]
    ClosedLoop,

    /// Allows the focus to leave the `FlutterView`.
    ///
    /// Requesting next focus after the last focusable item or previous to the first item will
    /// unfocus any focused nodes.
    LeaveFlutterView,

    /// Allows focus to traverse up to parent scope.
    ///
    /// When reaching the edge of the current scope, requesting the next focus will look up to
    /// the parent scope of the current scope and focus the focus node next to the current
    /// scope.
    ///
    /// If there is no parent scope above the current scope, fallback to
    /// [`ClosedLoop`](Self::ClosedLoop) behavior.
    ParentScope,

    /// Stops the focus traversal at the edge of the focus scope.
    ///
    /// Keeps the focus in its current position when it reaches the edge of a focus scope.
    Stop,
}

#[cfg(test)]
pub(crate) mod tests {
    use inset_foundation::AppCell;
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use inset_embedder::{
        Picture, Platform, PlatformRef, View as EmbedderView, ViewId, ViewMetrics, ViewRef,
    };
    use inset_gestures::{PointerDownEvent, PointerEvent};
    use inset_scheduler::SchedulerBinding;
    use inset_services::{KeyDownEvent, LogicalKeyboardKey, PhysicalKeyboardKey};

    use super::*;
    use crate::binding::run_app;
    use crate::framework::{IntoWidget, WidgetRef};
    use crate::widgets::basic::{Builder, SizedBox};

    struct TestView;

    impl EmbedderView for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            ViewMetrics {
                physical_size: [800.0, 600.0],
                physical_constraints: inset_embedder::ViewConstraints::tight(800.0, 600.0),
                device_pixel_ratio: 2.0,
                ..ViewMetrics::default()
            }
        }

        fn present(&self, _picture: std::sync::Arc<Picture>) {}
    }

    struct TestPlatform {
        view: ViewRef,
        handles_text_editing_keys: bool,
    }

    impl Platform for TestPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::MacOS
        }

        fn handles_text_editing_keys(&self) -> bool {
            self.handles_text_editing_keys
        }

        fn request_frame(&self) {}

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            vec![Rc::clone(&self.view)]
        }

        fn view(&self, id: ViewId) -> Option<ViewRef> {
            (self.view.id() == id).then(|| Rc::clone(&self.view))
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            Some(Rc::clone(&self.view))
        }
    }

    /// An [`App`] with a single view, ready for [`run_app`].
    ///
    /// Its host reports plain key presses, so a text field keeps its own key bindings.
    pub(crate) fn app_with_view() -> Rc<AppCell> {
        app_with_view_of(false)
    }

    /// The same, over a host that turns editing keys into edits itself, as Flutter's macOS
    /// and iOS embedders do.
    pub(crate) fn app_with_view_whose_host_edits() -> Rc<AppCell> {
        app_with_view_of(true)
    }

    fn app_with_view_of(handles_text_editing_keys: bool) -> Rc<AppCell> {
        let platform: PlatformRef = Rc::new(TestPlatform {
            view: Rc::new(TestView),
            handles_text_editing_keys,
        });
        AppCell::with_platform(platform)
    }

    /// A frame: begin, draw, and the microtasks in between.
    pub(crate) fn pump_frame(app: &mut App) {
        SchedulerBinding::handle_begin_frame(app, Some(Duration::ZERO));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    /// Mounts `widget` through [`run_app`] and draws the first frame.
    pub(crate) fn mount(cell: &AppCell, widget: WidgetRef) {
        run_app(&mut cell.borrow_mut(), widget);
        cell.elapse(Duration::ZERO);
        pump_frame(&mut cell.borrow_mut());
    }

    /// A mounted app whose tree is one `Builder`, plus that builder's context.
    fn mounted_context() -> (Rc<AppCell>, BuildContext) {
        let cell = app_with_view();
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let sink = Rc::clone(&captured);
        mount(
            &cell,
            Builder::new(move |_app, context| {
                sink.set(Some(context));
                SizedBox::shrink().into_widget()
            })
            .into_widget(),
        );
        let context = captured.get().expect("the builder ran");
        (cell, context)
    }

    fn key_a_down() -> KeyEvent {
        KeyEvent::Down(KeyDownEvent::new(
            PhysicalKeyboardKey::KEY_A,
            LogicalKeyboardKey::KEY_A,
            Duration::ZERO,
        ))
    }

    /// Attaches `node` under `parent`, the way a host widget's `initState` would.
    fn attach_under(
        app: &mut App,
        node: AnyFocusNode,
        context: BuildContext,
        parent: AnyFocusNode,
    ) -> Handle<FocusAttachment> {
        let attachment = node.attach(app, Some(context), None);
        attachment.reparent(app, Some(parent));
        attachment
    }

    #[test]
    fn a_node_attached_under_a_scope_gains_primary_focus_on_request_focus() {
        let (cell, context) = mounted_context();
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let root = manager.root_scope(&app);
        let scope = FocusScopeNode::new(&mut app);
        attach_under(&mut app, scope.as_node(), context, root.as_node());
        let node = FocusNode::new(&mut app);
        attach_under(&mut app, node.as_node(), context, scope.as_node());

        node.request_focus(&mut app, None);
        // The node is notified in a microtask, so the manager still has the old focus.
        assert!(!node.has_primary_focus(&app));

        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(node.as_node()));
        assert!(node.has_primary_focus(&app));
        assert!(node.has_focus(&mut app));
        assert!(scope.has_focus(&mut app));
        assert!(!scope.has_primary_focus(&app));
        assert_eq!(scope.focused_child(&mut app), Some(node.as_node()));
    }

    #[test]
    fn unfocus_moves_the_focus_according_to_its_disposition() {
        let (cell, context) = mounted_context();
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let root = manager.root_scope(&app);
        let scope = FocusScopeNode::new(&mut app);
        attach_under(&mut app, scope.as_node(), context, root.as_node());
        let child1 = FocusNode::new(&mut app);
        attach_under(&mut app, child1.as_node(), context, scope.as_node());
        let child2 = FocusNode::new(&mut app);
        attach_under(&mut app, child2.as_node(), context, scope.as_node());

        child1.request_focus(&mut app, None);
        app.drain_microtasks();
        child2.request_focus(&mut app, None);
        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(child2.as_node()));

        // `previouslyFocusedChild` walks back down to the child focused before this one.
        child2.unfocus(&mut app, UnfocusDisposition::PreviouslyFocusedChild);
        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(child1.as_node()));

        // `scope` clears the history and parks the focus on the enclosing scope.
        child1.unfocus(&mut app, UnfocusDisposition::Scope);
        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(scope.as_node()));
        assert_eq!(scope.focused_child(&mut app), None);
    }

    #[test]
    fn set_first_focus_adopts_the_scope_and_focuses_it_with_its_parent() {
        let (cell, context) = mounted_context();
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let root = manager.root_scope(&app);
        let scope1 = FocusScopeNode::new(&mut app);
        attach_under(&mut app, scope1.as_node(), context, root.as_node());
        let scope2 = FocusScopeNode::new(&mut app);

        scope1.set_first_focus(&mut app, scope2);
        assert_eq!(scope2.parent(&app), Some(scope1.as_node()));
        assert_eq!(scope1.focused_child(&mut app), Some(scope2.as_node()));
        assert!(!scope2.has_focus(&mut app));

        scope1.request_focus(&mut app, None);
        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(scope2.as_node()));
    }

    #[test]
    fn a_node_that_cannot_request_focus_refuses_the_request() {
        let (cell, context) = mounted_context();
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let root = manager.root_scope(&app);
        let scope = FocusScopeNode::new(&mut app);
        attach_under(&mut app, scope.as_node(), context, root.as_node());
        let node = FocusNode::new(&mut app);
        attach_under(&mut app, node.as_node(), context, scope.as_node());
        node.set_can_request_focus(&mut app, false);

        node.request_focus(&mut app, None);
        app.drain_microtasks();
        assert!(!node.has_primary_focus(&app));
        // Nothing asked for the focus, so it reverted to the root scope.
        assert_eq!(manager.primary_focus(&app), Some(root.as_node()));
    }

    #[test]
    fn descendants_are_focusable_false_unfocuses_and_blocks_the_descendants() {
        let (cell, context) = mounted_context();
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let root = manager.root_scope(&app);
        let scope = FocusScopeNode::new(&mut app);
        attach_under(&mut app, scope.as_node(), context, root.as_node());
        let node = FocusNode::new(&mut app);
        attach_under(&mut app, node.as_node(), context, scope.as_node());

        node.request_focus(&mut app, None);
        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(node.as_node()));

        // Dart unfocuses the node the flag was set on, so the focus leaves the scope for the
        // nearest focusable enclosing scope: the root.
        scope.set_descendants_are_focusable(&mut app, false);
        app.drain_microtasks();
        assert!(!scope.descendants_are_focusable(&app));
        assert!(!node.can_request_focus(&mut app));
        assert_eq!(manager.primary_focus(&app), Some(root.as_node()));

        node.request_focus(&mut app, None);
        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(root.as_node()));
    }

    #[test]
    fn a_key_event_walks_from_the_primary_focus_to_the_root_until_one_handles_it() {
        let (cell, context) = mounted_context();
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let root = manager.root_scope(&app);
        let scope = FocusScopeNode::new(&mut app);
        attach_under(&mut app, scope.as_node(), context, root.as_node());
        let node = FocusNode::new(&mut app);
        attach_under(&mut app, node.as_node(), context, scope.as_node());
        node.request_focus(&mut app, None);
        app.drain_microtasks();

        let log: Rc<RefCell<Vec<&'static str>>> = Rc::default();
        let result: Rc<Cell<KeyEventResult>> = Rc::new(Cell::new(KeyEventResult::Ignored));
        {
            let log = Rc::clone(&log);
            let result = Rc::clone(&result);
            node.set_on_key_event(
                &mut app,
                Some(Rc::new(move |_app, _node, _event| {
                    log.borrow_mut().push("node");
                    result.get()
                })),
            );
        }
        {
            let log = Rc::clone(&log);
            scope.set_on_key_event(
                &mut app,
                Some(Rc::new(move |_app, _node, _event| {
                    log.borrow_mut().push("scope");
                    KeyEventResult::Ignored
                })),
            );
        }
        {
            let log = Rc::clone(&log);
            root.set_on_key_event(
                &mut app,
                Some(Rc::new(move |_app, _node, _event| {
                    log.borrow_mut().push("root");
                    KeyEventResult::Ignored
                })),
            );
        }

        let keyboard = HardwareKeyboard::instance(&mut app);
        assert!(!keyboard.handle_key_event(&mut app, &key_a_down()));
        assert_eq!(*log.borrow(), vec!["node", "scope", "root"]);

        log.borrow_mut().clear();
        result.set(KeyEventResult::Handled);
        assert!(keyboard.handle_key_event(&mut app, &key_a_down()));
        assert_eq!(*log.borrow(), vec!["node"]);
    }

    #[test]
    fn disposing_a_focused_node_moves_the_focus_to_its_scope() {
        let (cell, context) = mounted_context();
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let root = manager.root_scope(&app);
        let scope = FocusScopeNode::new(&mut app);
        attach_under(&mut app, scope.as_node(), context, root.as_node());
        let node = FocusNode::new(&mut app);
        attach_under(&mut app, node.as_node(), context, scope.as_node());
        node.request_focus(&mut app, None);
        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(node.as_node()));

        node.dispose(&mut app);
        assert_eq!(manager.primary_focus(&app), None);
        assert_eq!(node.parent(&app), None);

        app.drain_microtasks();
        assert_eq!(manager.primary_focus(&app), Some(scope.as_node()));
    }

    #[test]
    fn the_highlight_mode_follows_the_last_interaction() {
        let cell = app_with_view();
        mount(&cell, SizedBox::shrink().into_widget());
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        let seen: Rc<RefCell<Vec<FocusHighlightMode>>> = Rc::default();
        {
            let seen = Rc::clone(&seen);
            manager.add_highlight_mode_listener(
                &mut app,
                Rc::new(move |_app, mode| seen.borrow_mut().push(mode)),
            );
        }
        // macOS with no interaction yet.
        assert_eq!(
            manager.highlight_mode(&mut app),
            FocusHighlightMode::Traditional
        );

        let router = GestureBinding::instance(&mut app).pointer_router(&app);
        router.route(
            &mut app,
            PointerEvent::Down(PointerDownEvent {
                kind: PointerDeviceKind::Touch,
                ..PointerDownEvent::default()
            }),
        );
        assert_eq!(manager.highlight_mode(&mut app), FocusHighlightMode::Touch);

        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &key_a_down());
        assert_eq!(
            manager.highlight_mode(&mut app),
            FocusHighlightMode::Traditional
        );
        assert_eq!(
            *seen.borrow(),
            vec![FocusHighlightMode::Touch, FocusHighlightMode::Traditional]
        );
    }

    #[test]
    fn the_highlight_strategy_pins_the_mode() {
        let cell = app_with_view();
        mount(&cell, SizedBox::shrink().into_widget());
        let mut app = cell.borrow_mut();
        let manager = FocusManager::instance(&mut app);
        manager.set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTouch);
        assert_eq!(
            manager.highlight_strategy(&app),
            FocusHighlightStrategy::AlwaysTouch
        );
        assert_eq!(manager.highlight_mode(&mut app), FocusHighlightMode::Touch);

        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &key_a_down());
        assert_eq!(manager.highlight_mode(&mut app), FocusHighlightMode::Touch);
    }

    #[test]
    fn combine_key_event_results_prefers_handled_then_skip() {
        assert_eq!(
            combine_key_event_results([KeyEventResult::Ignored, KeyEventResult::Ignored]),
            KeyEventResult::Ignored
        );
        assert_eq!(
            combine_key_event_results([
                KeyEventResult::Ignored,
                KeyEventResult::SkipRemainingHandlers
            ]),
            KeyEventResult::SkipRemainingHandlers
        );
        assert_eq!(
            combine_key_event_results([
                KeyEventResult::SkipRemainingHandlers,
                KeyEventResult::Handled
            ]),
            KeyEventResult::Handled
        );
    }
}
