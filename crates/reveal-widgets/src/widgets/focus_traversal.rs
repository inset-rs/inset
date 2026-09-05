//! Flutter counterpart: `widgets/focus_traversal.dart`.
//!
//! The focus traversal policies: [`FocusTraversalPolicy`] and its three implementations,
//! [`FocusTraversalGroup`], which imposes one on a subtree, and the intents and actions that
//! move the focus.
//!
//! `TraversalEdgeBehavior` is declared in [`focus_manager`](crate::widgets::focus_manager),
//! where `FocusScopeNode` reads it.

use std::any::Any;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{Curve, Curves};
use reveal_embedder::{Offset, Rect, TextDirection};
use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, ListenableObject, Listener,
    ValueChanged,
};
use reveal_painting::Axis;

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    StatelessWidget, WidgetRef, downcast_widget,
};
use crate::widgets::actions::{Action, ActionData, Intent};
use crate::widgets::basic::Directionality;
use crate::widgets::focus_manager::{
    AnyFocusNode, FocusManager, FocusNodeData, FocusNodeLeaf, FocusScopeNode, KeyEventResult,
    TraversalEdgeBehavior, primary_focus,
};
use crate::widgets::focus_scope::Focus;
use crate::widgets::scroll_position::ScrollPositionAlignmentPolicy;
use crate::widgets::scrollable::Scrollable;

/// Returns the ancestor `count` levels above the given `context`.
///
/// [`BuildContext`] doesn't have a parent accessor, but it can be simulated with
/// [`visit_ancestor_elements`](crate::AnyElement::visit_ancestor_elements). This is needed
/// because `get_element_for_inherited_widget_of_exact_type` will return the context itself if
/// it happens to be of the correct type.
fn get_ancestor(app: &App, context: BuildContext, count: u32) -> Option<BuildContext> {
    let mut count = count;
    let mut target = None;
    context.visit_ancestor_elements(app, &mut |ancestor| {
        count -= 1;
        if count == 0 {
            target = Some(ancestor);
            return false;
        }
        true
    });
    target
}

/// Signature for the callback that's called when a traversal policy requests focus.
///
/// Dart's four named optional arguments (`alignmentPolicy`, `alignment`, `duration`, `curve`)
/// are positional here.
pub type TraversalRequestFocusCallback = Rc<
    dyn Fn(
        &mut App,
        AnyFocusNode,
        Option<ScrollPositionAlignmentPolicy>,
        Option<f64>,
        Option<Duration>,
        Option<Rc<dyn Curve>>,
    ),
>;

/// The default value for [`FocusTraversalPolicyData::request_focus_callback`].
///
/// Requests focus from `node` and ensures the node is visible by calling
/// [`Scrollable::ensure_visible`].
pub fn default_traversal_request_focus_callback(
    app: &mut App,
    node: AnyFocusNode,
    alignment_policy: Option<ScrollPositionAlignmentPolicy>,
    alignment: Option<f64>,
    duration: Option<Duration>,
    curve: Option<Rc<dyn Curve>>,
) {
    node.request_focus(app, None);
    let context = node.context(app).expect("a traversable node has a context");
    Scrollable::ensure_visible(
        app,
        context,
        alignment.unwrap_or(1.0),
        duration.unwrap_or(Duration::ZERO),
        curve.unwrap_or_else(Curves::ease),
        alignment_policy.unwrap_or(ScrollPositionAlignmentPolicy::Explicit),
    );
}

/// A direction along either the horizontal or vertical axes.
///
/// This is used by [`DirectionalFocusTraversalPolicyMixin`], and
/// [`AnyFocusNode::focus_in_direction`] to indicate which direction to look in for the next
/// focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TraversalDirection {
    /// Indicates a direction above the currently focused widget.
    Up,

    /// Indicates a direction to the right of the currently focused widget.
    ///
    /// This direction is unaffected by the [`Directionality`] of the current context.
    Right,

    /// Indicates a direction below the currently focused widget.
    Down,

    /// Indicates a direction to the left of the currently focused widget.
    ///
    /// This direction is unaffected by the [`Directionality`] of the current context.
    Left,
}

// ---------------------------------------------------------------------------------------------
// FocusTraversalPolicy

/// The fields Dart's `FocusTraversalPolicy` declares; every policy carries this bag under the
/// field `policy`.
pub struct FocusTraversalPolicyData {
    /// The callback used to move the focus from one focus node to another when traversing them
    /// using a keyboard. By default it requests focus on the next node.
    pub request_focus_callback: TraversalRequestFocusCallback,
}

impl FocusTraversalPolicyData {
    /// The bag of a freshly created policy: Dart's constructor defaults.
    pub fn new() -> FocusTraversalPolicyData {
        FocusTraversalPolicyData {
            request_focus_callback: Rc::new(default_traversal_request_focus_callback),
        }
    }
}

impl Default for FocusTraversalPolicyData {
    fn default() -> FocusTraversalPolicyData {
        FocusTraversalPolicyData::new()
    }
}

/// The accessors [`FocusTraversalPolicy`] asks for, for a struct whose bag is the field
/// `policy`.
#[macro_export]
macro_rules! focus_traversal_policy_accessors {
    () => {
        fn focus_traversal_policy_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::FocusTraversalPolicyData {
            &app.get(self).policy
        }

        fn focus_traversal_policy_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::FocusTraversalPolicyData {
            &mut app.get_mut(self).policy
        }
    };
}

/// The four members [`DirectionalFocusTraversalPolicyMixin`] overrides, forwarded from a
/// policy's `impl FocusTraversalPolicy` the way Dart's mixin linearization would.
#[macro_export]
macro_rules! directional_focus_traversal_policy_overrides {
    () => {
        fn invalidate_scope_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            node: ::reveal_foundation::Handle<$crate::FocusScopeNode>,
        ) {
            $crate::DirectionalFocusTraversalPolicyMixin::invalidate_scope_data(self, app, node);
        }

        fn changed_scope(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            node: Option<$crate::AnyFocusNode>,
            old_scope: Option<::reveal_foundation::Handle<$crate::FocusScopeNode>>,
        ) {
            $crate::DirectionalFocusTraversalPolicyMixin::changed_scope(self, app, node, old_scope);
        }

        fn find_first_focus_in_direction(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            current_node: $crate::AnyFocusNode,
            direction: $crate::TraversalDirection,
        ) -> Option<$crate::AnyFocusNode> {
            $crate::DirectionalFocusTraversalPolicyMixin::find_first_focus_in_direction(
                self,
                app,
                current_node,
                direction,
            )
        }

        fn in_direction(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            current_node: $crate::AnyFocusNode,
            direction: $crate::TraversalDirection,
        ) -> bool {
            $crate::DirectionalFocusTraversalPolicyMixin::in_direction(
                self,
                app,
                current_node,
                direction,
            )
        }
    };
}

/// Determines how focusable widgets are traversed within a [`FocusTraversalGroup`].
///
/// The focus traversal policy is what determines which widget is "next", "previous", or in a
/// direction from the widget associated with the currently focused `FocusNode` (usually a
/// `Focus` widget).
///
/// One of the pre-defined implementations may be used, or define a custom policy to create a
/// unique focus order.
///
/// When defining your own, implement [`sort_descendants`](Self::sort_descendants) to provide the
/// order in which you would like the descendants to be traversed.
///
/// See also:
///
///  * `FocusNode`, for a description of the focus system.
///  * [`FocusTraversalGroup`], a widget that groups together and imposes a traversal policy on
///    the `Focus` nodes below it in the widget hierarchy.
///  * [`WidgetOrderTraversalPolicy`], a policy that relies on the widget creation order to
///    describe the order of traversal.
///  * [`ReadingOrderTraversalPolicy`], a policy that describes the order as the natural "reading
///    order" for the current [`Directionality`].
///  * [`OrderedTraversalPolicy`], a policy that describes the order explicitly using
///    [`FocusTraversalOrder`] widgets.
///  * [`DirectionalFocusTraversalPolicyMixin`], a trait that implements focus traversal in a
///    direction.
pub trait FocusTraversalPolicy: Sized + 'static {
    /// Dart's `FocusTraversalPolicy` fields, held under the field `policy`
    /// ([`focus_traversal_policy_accessors!`](crate::focus_traversal_policy_accessors)).
    fn focus_traversal_policy_data(self: Handle<Self>, app: &App) -> &FocusTraversalPolicyData;

    /// See [`focus_traversal_policy_data`](Self::focus_traversal_policy_data).
    fn focus_traversal_policy_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut FocusTraversalPolicyData;

    /// This policy as the erased [`AnyFocusTraversalPolicy`] — what to pass where a Dart API
    /// takes a `FocusTraversalPolicy`.
    fn as_policy(self: Handle<Self>) -> AnyFocusTraversalPolicy {
        AnyFocusTraversalPolicy {
            id: self.id(),
            vtable: const { &FocusTraversalPolicyVTable::of::<Self>() },
        }
    }

    /// See [`FocusTraversalPolicyData::request_focus_callback`].
    fn request_focus_callback(self: Handle<Self>, app: &App) -> TraversalRequestFocusCallback {
        Rc::clone(&self.focus_traversal_policy_data(app).request_focus_callback)
    }

    /// See [`FocusTraversalPolicyData::request_focus_callback`].
    fn set_request_focus_callback(
        self: Handle<Self>,
        app: &mut App,
        value: TraversalRequestFocusCallback,
    ) {
        self.focus_traversal_policy_data_mut(app)
            .request_focus_callback = value;
    }

    /// Request focus on a focus node as a result of a tab traversal.
    ///
    /// If the `node` is a `FocusScopeNode`, this method will recursively find the next focus
    /// from its descendants until it finds a regular `FocusNode`.
    ///
    /// Returns true if this method focused a new focus node.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's `_requestTabTraversalFocus` takes these named arguments"
    )]
    fn request_tab_traversal_focus(
        self: Handle<Self>,
        app: &mut App,
        node: AnyFocusNode,
        alignment_policy: Option<ScrollPositionAlignmentPolicy>,
        alignment: Option<f64>,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
        forward: bool,
    ) -> bool {
        if let Some(scope) = node.as_scope() {
            if let Some(focused_child) = scope.focused_child(app) {
                // Can't stop here as the `focused_child` may be a focus scope node without a
                // first focus. The first focus will be picked in the next iteration.
                return self.request_tab_traversal_focus(
                    app,
                    focused_child,
                    alignment_policy,
                    alignment,
                    duration,
                    curve,
                    forward,
                );
            }
            let sorted_children = sort_all_descendants(app, scope, scope.as_node());
            if !sorted_children.is_empty() {
                let child = if forward {
                    sorted_children[0]
                } else {
                    sorted_children[sorted_children.len() - 1]
                };
                self.request_tab_traversal_focus(
                    app,
                    child,
                    alignment_policy,
                    alignment,
                    duration,
                    curve,
                    forward,
                );
                // Regardless of what `request_tab_traversal_focus` returns, a first focus has
                // been picked.
                return true;
            }
        }
        let node_had_primary_focus = node.has_primary_focus(app);
        let request_focus_callback = self.request_focus_callback(app);
        request_focus_callback(app, node, alignment_policy, alignment, duration, curve);
        !node_had_primary_focus
    }

    /// Returns the node that should receive focus if focus is traversing forwards, and there is
    /// no current focus.
    ///
    /// The node returned is the node that should receive focus if focus is traversing forwards
    /// (i.e. with [`next`](Self::next)), and there is no current focus in the nearest
    /// `FocusScopeNode` that `current_node` belongs to.
    ///
    /// If `ignore_current_focus` is false, this function returns the scope's focused child, if
    /// set, on the nearest scope of the `current_node`, otherwise, returns the first node from
    /// [`sort_descendants`](Self::sort_descendants), or the given `current_node` if there are no
    /// descendants.
    ///
    /// If `ignore_current_focus` is true, then the algorithm returns the first node from
    /// [`sort_descendants`](Self::sort_descendants), or the given `current_node` if there are no
    /// descendants.
    fn find_first_focus(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        ignore_current_focus: bool,
    ) -> Option<AnyFocusNode> {
        Some(self.find_initial_focus(app, current_node, false, ignore_current_focus))
    }

    /// Returns the node that should receive focus if focus is traversing backwards, and there
    /// is no current focus.
    fn find_last_focus(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        ignore_current_focus: bool,
    ) -> AnyFocusNode {
        self.find_initial_focus(app, current_node, true, ignore_current_focus)
    }

    /// Dart's `FocusTraversalPolicy._findInitialFocus`.
    fn find_initial_focus(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        from_end: bool,
        ignore_current_focus: bool,
    ) -> AnyFocusNode {
        let scope = current_node
            .nearest_scope(app)
            .expect("a node in the tree has a nearest scope");
        let mut candidate = scope.focused_child(app);
        if ignore_current_focus
            || (candidate.is_none() && !scope.as_node().descendants(app).is_empty())
        {
            let sorted: Vec<AnyFocusNode> = sort_all_descendants(app, scope, current_node)
                .into_iter()
                .filter(|node| can_request_traversal_focus(app, *node))
                .collect();
            candidate = if sorted.is_empty() {
                None
            } else if from_end {
                Some(sorted[sorted.len() - 1])
            } else {
                Some(sorted[0])
            };
        }

        // If we still didn't find any candidate, use the current node as a fallback.
        candidate.unwrap_or(current_node)
    }

    /// Returns the first node in the given `direction` that should receive focus if there is no
    /// current focus in the scope to which the `current_node` belongs.
    ///
    /// This is typically used by [`in_direction`](Self::in_direction) to determine which node to
    /// focus if it is called when no node is currently focused.
    fn find_first_focus_in_direction(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        direction: TraversalDirection,
    ) -> Option<AnyFocusNode>;

    /// Clears the data associated with the given `FocusScopeNode` for this object.
    ///
    /// This is used to indicate that the focus policy has changed its mode, and so any cached
    /// policy data should be invalidated. For example, changing the direction in which focus is
    /// moving, or changing from directional to next/previous navigation modes.
    ///
    /// The default implementation does nothing.
    fn invalidate_scope_data(self: Handle<Self>, app: &mut App, node: Handle<FocusScopeNode>) {
        let _ = (app, node);
    }

    /// This is called whenever the given `node` is re-parented into a new scope, so that the
    /// policy has a chance to update or invalidate any cached data that it maintains per scope
    /// about the node.
    ///
    /// The `old_scope` is the previous scope that this node belonged to, if any.
    ///
    /// The default implementation does nothing.
    fn changed_scope(
        self: Handle<Self>,
        app: &mut App,
        node: Option<AnyFocusNode>,
        old_scope: Option<Handle<FocusScopeNode>>,
    ) {
        let _ = (app, node, old_scope);
    }

    /// Focuses the next widget in the focus scope that contains the given `current_node`.
    ///
    /// This determines what the next node to receive focus should be by inspecting the node
    /// tree, and then calling [`AnyFocusNode::request_focus`] on the node that has been
    /// selected.
    ///
    /// Returns true if it successfully found a node and requested focus.
    fn next(self: Handle<Self>, app: &mut App, current_node: AnyFocusNode) -> bool {
        self.move_focus(app, current_node, true)
    }

    /// Focuses the previous widget in the focus scope that contains the given `current_node`.
    ///
    /// Returns true if it successfully found a node and requested focus.
    fn previous(self: Handle<Self>, app: &mut App, current_node: AnyFocusNode) -> bool {
        self.move_focus(app, current_node, false)
    }

    /// Focuses the next widget in the given `direction` in the focus scope that contains the
    /// given `current_node`.
    ///
    /// Returns true if it successfully found a node and requested focus.
    fn in_direction(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        direction: TraversalDirection,
    ) -> bool;

    /// Sorts the given `descendants` into focus order.
    ///
    /// Implementations override this to implement a different sort for [`next`](Self::next) and
    /// [`previous`](Self::previous) to use in their ordering. If the returned list omits a node
    /// that is a descendant of the given scope, then the user will be unable to use
    /// next/previous keyboard traversal to reach that node.
    ///
    /// The node used to initiate the traversal (the one passed to [`next`](Self::next) or
    /// [`previous`](Self::previous)) is passed as `current_node`.
    ///
    /// Having the current node in the list is what allows the algorithm to determine which nodes
    /// are adjacent to the current node. If the `current_node` is removed from the list, then
    /// the focus will be unchanged when [`next`](Self::next) or [`previous`](Self::previous) are
    /// called, and they will return false.
    ///
    /// This is not used for directional focus ([`in_direction`](Self::in_direction)), only for
    /// determining the focus order for [`next`](Self::next) and [`previous`](Self::previous).
    fn sort_descendants(
        self: Handle<Self>,
        app: &mut App,
        descendants: Vec<AnyFocusNode>,
        current_node: AnyFocusNode,
    ) -> Vec<AnyFocusNode>;

    /// Moves the focus to the next node in the `FocusScopeNode` nearest to the `current_node`
    /// argument, either in a forward or reverse direction, depending on the value of the
    /// `forward` argument.
    ///
    /// This function is called by [`next`](Self::next) and [`previous`](Self::previous).
    ///
    /// Uses [`find_first_focus`](Self::find_first_focus) / [`find_last_focus`](Self::find_last_focus)
    /// to find the first/last node if there is no focused child for the scope. If there is a
    /// focused child, then it calls [`sort_descendants`](Self::sort_descendants) to get a sorted
    /// list of descendants, and then finds the node after the current first focus of the scope
    /// if `forward` is true, and the node before it if `forward` is false.
    ///
    /// Returns true if a node requested focus.
    fn move_focus(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        forward: bool,
    ) -> bool {
        let nearest_scope = current_node
            .nearest_scope(app)
            .expect("a node in the tree has a nearest scope");
        self.invalidate_scope_data(app, nearest_scope);
        let mut focused_child = nearest_scope.focused_child(app);
        if focused_child.is_none() {
            let first_focus = if forward {
                self.find_first_focus(app, current_node, false)
            } else {
                Some(self.find_last_focus(app, current_node, false))
            };
            if let Some(first_focus) = first_focus {
                return self.request_tab_traversal_focus(
                    app,
                    first_focus,
                    Some(if forward {
                        ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
                    } else {
                        ScrollPositionAlignmentPolicy::KeepVisibleAtStart
                    }),
                    None,
                    None,
                    None,
                    forward,
                );
            }
        }
        let focused_child = *focused_child.get_or_insert(nearest_scope.as_node());
        let sorted_nodes = sort_all_descendants(app, nearest_scope, focused_child);
        debug_assert!(sorted_nodes.contains(&focused_child));

        if forward && Some(&focused_child) == sorted_nodes.last() {
            match nearest_scope.traversal_edge_behavior(app) {
                TraversalEdgeBehavior::LeaveFlutterView => {
                    focused_child.unfocus(app, Default::default());
                    return false;
                }
                TraversalEdgeBehavior::ParentScope => {
                    let parent_scope = nearest_scope.enclosing_scope(app);
                    let root_scope = FocusManager::instance(app).root_scope(app);
                    if let Some(parent_scope) = parent_scope
                        && parent_scope != root_scope
                    {
                        focused_child.unfocus(app, Default::default());
                        parent_scope.as_node().next_focus(app);
                        // Verify the focus really has changed.
                        return focused_child
                            .enclosing_scope(app)
                            .and_then(|scope| scope.focused_child(app))
                            != Some(focused_child);
                    }
                    // No valid parent scope. Fallback to closed loop behavior.
                    return self.request_tab_traversal_focus(
                        app,
                        sorted_nodes[0],
                        Some(ScrollPositionAlignmentPolicy::KeepVisibleAtEnd),
                        None,
                        None,
                        None,
                        forward,
                    );
                }
                TraversalEdgeBehavior::ClosedLoop => {
                    return self.request_tab_traversal_focus(
                        app,
                        sorted_nodes[0],
                        Some(ScrollPositionAlignmentPolicy::KeepVisibleAtEnd),
                        None,
                        None,
                        None,
                        forward,
                    );
                }
                TraversalEdgeBehavior::Stop => return false,
            }
        }
        if !forward && Some(&focused_child) == sorted_nodes.first() {
            let last = sorted_nodes[sorted_nodes.len() - 1];
            match nearest_scope.traversal_edge_behavior(app) {
                TraversalEdgeBehavior::LeaveFlutterView => {
                    focused_child.unfocus(app, Default::default());
                    return false;
                }
                TraversalEdgeBehavior::ParentScope => {
                    let parent_scope = nearest_scope.enclosing_scope(app);
                    let root_scope = FocusManager::instance(app).root_scope(app);
                    if let Some(parent_scope) = parent_scope
                        && parent_scope != root_scope
                    {
                        focused_child.unfocus(app, Default::default());
                        parent_scope.as_node().previous_focus(app);
                        // Verify the focus really has changed.
                        return focused_child
                            .enclosing_scope(app)
                            .and_then(|scope| scope.focused_child(app))
                            != Some(focused_child);
                    }
                    // No valid parent scope. Fallback to closed loop behavior.
                    return self.request_tab_traversal_focus(
                        app,
                        last,
                        Some(ScrollPositionAlignmentPolicy::KeepVisibleAtStart),
                        None,
                        None,
                        None,
                        forward,
                    );
                }
                TraversalEdgeBehavior::ClosedLoop => {
                    return self.request_tab_traversal_focus(
                        app,
                        last,
                        Some(ScrollPositionAlignmentPolicy::KeepVisibleAtStart),
                        None,
                        None,
                        None,
                        forward,
                    );
                }
                TraversalEdgeBehavior::Stop => return false,
            }
        }

        let maybe_flipped: Vec<AnyFocusNode> = if forward {
            sorted_nodes
        } else {
            sorted_nodes.into_iter().rev().collect()
        };
        let mut previous_node: Option<AnyFocusNode> = None;
        for node in maybe_flipped {
            if previous_node == Some(focused_child) {
                return self.request_tab_traversal_focus(
                    app,
                    node,
                    Some(if forward {
                        ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
                    } else {
                        ScrollPositionAlignmentPolicy::KeepVisibleAtStart
                    }),
                    None,
                    None,
                    None,
                    forward,
                );
            }
            previous_node = Some(node);
        }
        false
    }
}

/// Dart's `FocusTraversalPolicy._canRequestTraversalFocus`.
fn can_request_traversal_focus(app: &mut App, node: AnyFocusNode) -> bool {
    node.can_request_focus(app) && !node.skip_traversal(app)
}

/// Dart's `FocusTraversalPolicy._getDescendantsWithoutExpandingScope`.
fn get_descendants_without_expanding_scope(app: &App, node: AnyFocusNode) -> Vec<AnyFocusNode> {
    let mut result = Vec::new();
    for child in node.children(app) {
        result.push(child);
        if child.as_scope().is_none() {
            result.extend(get_descendants_without_expanding_scope(app, child));
        }
    }
    result
}

/// A structure to temporarily hold information about [`FocusTraversalGroup`]s when sorting their
/// contents.
struct FocusTraversalGroupInfo {
    policy: AnyFocusTraversalPolicy,
    members: Vec<AnyFocusNode>,
}

/// Dart's `FocusTraversalPolicy._findGroups`.
fn find_groups(
    app: &mut App,
    scope: Handle<FocusScopeNode>,
    scope_group_node: Option<Handle<FocusTraversalGroupNode>>,
    current_node: AnyFocusNode,
) -> HashMap<Option<AnyFocusNode>, FocusTraversalGroupInfo> {
    let default_policy = match scope_group_node {
        Some(group) => app.get(group).policy,
        None => ReadingOrderTraversalPolicy::new(app).as_policy(),
    };
    let mut groups: HashMap<Option<AnyFocusNode>, FocusTraversalGroupInfo> = HashMap::new();
    for node in get_descendants_without_expanding_scope(app, scope.as_node()) {
        let group_node = FocusTraversalGroup::get_group_node(app, node);
        // Group nodes need to be added to their parent's node, or to the `None` node if no
        // parent is found. This creates the hierarchy of group nodes and makes it so the entire
        // group is sorted along with the other members of the parent group.
        if Some(node) == group_node.map(|group| group.as_node()) {
            // To find the parent of the group node, we need to skip over the parent of the Focus
            // node added in `FocusTraversalGroupState::build`, and start looking with that
            // node's parent, since `get_group_node` will return the node it was called on if it
            // matches the type.
            let parent = node.parent(app).expect("a group node in the tree has one");
            let parent_group = FocusTraversalGroup::get_group_node(app, parent);
            let key = parent_group.map(|group| group.as_node());
            let info = groups
                .entry(key)
                .or_insert_with(|| FocusTraversalGroupInfo {
                    policy: match parent_group {
                        Some(group) => app.get(group).policy,
                        None => default_policy,
                    },
                    members: Vec::new(),
                });
            debug_assert!(!info.members.contains(&node));
            info.members.push(node);
            continue;
        }
        // Skip non-focusable and non-traversable nodes in the same way that
        // `FocusScopeNode::traversal_descendants` would.
        //
        // The current focused node needs to be in the group so that the caller can find the next
        // traversable node from the current focused node.
        if node == current_node || (node.can_request_focus(app) && !node.skip_traversal(app)) {
            let key = group_node.map(|group| group.as_node());
            let policy = match group_node {
                Some(group) => app.get(group).policy,
                None => default_policy,
            };
            let info = groups
                .entry(key)
                .or_insert_with(|| FocusTraversalGroupInfo {
                    policy,
                    members: Vec::new(),
                });
            debug_assert!(!info.members.contains(&node));
            info.members.push(node);
        }
    }
    groups
}

/// Sorts all descendants, taking into account the [`FocusTraversalGroup`] that they are each in,
/// and filtering out non-traversable/focusable nodes.
fn sort_all_descendants(
    app: &mut App,
    scope: Handle<FocusScopeNode>,
    current_node: AnyFocusNode,
) -> Vec<AnyFocusNode> {
    let scope_group_node = FocusTraversalGroup::get_group_node(app, scope.as_node());
    // Build the sorting data structure, separating descendants into groups.
    let mut groups = find_groups(app, scope, scope_group_node, current_node);

    // Sort the member lists using the individual policy sorts.
    let keys: Vec<Option<AnyFocusNode>> = groups.keys().copied().collect();
    for key in keys {
        let (policy, members) = {
            let info = groups.get(&key).expect("a key of this map");
            (info.policy, info.members.clone())
        };
        let sorted_members = policy.sort_descendants(app, members, current_node);
        let info = groups.get_mut(&key).expect("a key of this map");
        info.members = sorted_members;
    }

    // Traverse the group tree, adding the children of members in the order they appear in the
    // member lists.
    let mut sorted_descendants = Vec::new();
    let scope_group_key = scope_group_node.map(|group| group.as_node());
    if !groups.is_empty() && groups.contains_key(&scope_group_key) {
        visit_groups(&groups, scope_group_key, &mut sorted_descendants);
    }

    // Remove the FocusTraversalGroup nodes themselves, which aren't focusable. They were left in
    // above because they were needed to find their members during sorting.
    let mut kept = Vec::with_capacity(sorted_descendants.len());
    for node in sorted_descendants {
        if node == current_node || can_request_traversal_focus(app, node) {
            kept.push(node);
        }
    }
    kept
}

/// Dart's `visitGroups` closure inside `_sortAllDescendants`.
fn visit_groups(
    groups: &HashMap<Option<AnyFocusNode>, FocusTraversalGroupInfo>,
    key: Option<AnyFocusNode>,
    sorted_descendants: &mut Vec<AnyFocusNode>,
) {
    let Some(info) = groups.get(&key) else { return };
    for node in &info.members {
        if groups.contains_key(&Some(*node)) {
            // This is a policy group focus node. Replace it with the members of the
            // corresponding policy group.
            visit_groups(groups, Some(*node), sorted_descendants);
        } else {
            sorted_descendants.push(*node);
        }
    }
}

/// The vtable of an erased [`AnyFocusTraversalPolicy`]: one `&'static` table per leaf type.
struct FocusTraversalPolicyVTable {
    type_name: fn() -> &'static str,
    find_first_focus: fn(&mut App, HandleId, AnyFocusNode, bool) -> Option<AnyFocusNode>,
    find_last_focus: fn(&mut App, HandleId, AnyFocusNode, bool) -> AnyFocusNode,
    find_first_focus_in_direction:
        fn(&mut App, HandleId, AnyFocusNode, TraversalDirection) -> Option<AnyFocusNode>,
    invalidate_scope_data: fn(&mut App, HandleId, Handle<FocusScopeNode>),
    changed_scope: fn(&mut App, HandleId, Option<AnyFocusNode>, Option<Handle<FocusScopeNode>>),
    next: fn(&mut App, HandleId, AnyFocusNode) -> bool,
    previous: fn(&mut App, HandleId, AnyFocusNode) -> bool,
    in_direction: fn(&mut App, HandleId, AnyFocusNode, TraversalDirection) -> bool,
    sort_descendants: fn(&mut App, HandleId, Vec<AnyFocusNode>, AnyFocusNode) -> Vec<AnyFocusNode>,
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

impl FocusTraversalPolicyVTable {
    const fn of<L: FocusTraversalPolicy>() -> FocusTraversalPolicyVTable {
        FocusTraversalPolicyVTable {
            type_name: std::any::type_name::<L>,
            find_first_focus: |app, id, current_node, ignore| {
                L::find_first_focus(resolve(id), app, current_node, ignore)
            },
            find_last_focus: |app, id, current_node, ignore| {
                L::find_last_focus(resolve(id), app, current_node, ignore)
            },
            find_first_focus_in_direction: |app, id, current_node, direction| {
                L::find_first_focus_in_direction(resolve(id), app, current_node, direction)
            },
            invalidate_scope_data: |app, id, node| L::invalidate_scope_data(resolve(id), app, node),
            changed_scope: |app, id, node, old_scope| {
                L::changed_scope(resolve(id), app, node, old_scope)
            },
            next: |app, id, current_node| L::next(resolve(id), app, current_node),
            previous: |app, id, current_node| L::previous(resolve(id), app, current_node),
            in_direction: |app, id, current_node, direction| {
                L::in_direction(resolve(id), app, current_node, direction)
            },
            sort_descendants: |app, id, descendants, current_node| {
                L::sort_descendants(resolve(id), app, descendants, current_node)
            },
        }
    }
}

/// Erased `FocusTraversalPolicy`: one identity and a static vtable.
///
/// This is what a field or parameter Dart types as `FocusTraversalPolicy` becomes.
#[derive(Clone, Copy)]
pub struct AnyFocusTraversalPolicy {
    id: HandleId,
    vtable: &'static FocusTraversalPolicyVTable,
}

impl PartialEq for AnyFocusTraversalPolicy {
    fn eq(&self, other: &AnyFocusTraversalPolicy) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyFocusTraversalPolicy {}

impl fmt::Debug for AnyFocusTraversalPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyFocusTraversalPolicy {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `policy as T`: the typed handle when this policy is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// See [`FocusTraversalPolicy::find_first_focus`].
    pub fn find_first_focus(
        self,
        app: &mut App,
        current_node: AnyFocusNode,
        ignore_current_focus: bool,
    ) -> Option<AnyFocusNode> {
        (self.vtable.find_first_focus)(app, self.id, current_node, ignore_current_focus)
    }

    /// See [`FocusTraversalPolicy::find_last_focus`].
    pub fn find_last_focus(
        self,
        app: &mut App,
        current_node: AnyFocusNode,
        ignore_current_focus: bool,
    ) -> AnyFocusNode {
        (self.vtable.find_last_focus)(app, self.id, current_node, ignore_current_focus)
    }

    /// See [`FocusTraversalPolicy::find_first_focus_in_direction`].
    pub fn find_first_focus_in_direction(
        self,
        app: &mut App,
        current_node: AnyFocusNode,
        direction: TraversalDirection,
    ) -> Option<AnyFocusNode> {
        (self.vtable.find_first_focus_in_direction)(app, self.id, current_node, direction)
    }

    /// See [`FocusTraversalPolicy::invalidate_scope_data`].
    pub fn invalidate_scope_data(self, app: &mut App, node: Handle<FocusScopeNode>) {
        (self.vtable.invalidate_scope_data)(app, self.id, node);
    }

    /// See [`FocusTraversalPolicy::changed_scope`].
    pub fn changed_scope(
        self,
        app: &mut App,
        node: Option<AnyFocusNode>,
        old_scope: Option<Handle<FocusScopeNode>>,
    ) {
        (self.vtable.changed_scope)(app, self.id, node, old_scope);
    }

    /// See [`FocusTraversalPolicy::next`].
    pub fn next(self, app: &mut App, current_node: AnyFocusNode) -> bool {
        (self.vtable.next)(app, self.id, current_node)
    }

    /// See [`FocusTraversalPolicy::previous`].
    pub fn previous(self, app: &mut App, current_node: AnyFocusNode) -> bool {
        (self.vtable.previous)(app, self.id, current_node)
    }

    /// See [`FocusTraversalPolicy::in_direction`].
    pub fn in_direction(
        self,
        app: &mut App,
        current_node: AnyFocusNode,
        direction: TraversalDirection,
    ) -> bool {
        (self.vtable.in_direction)(app, self.id, current_node, direction)
    }

    /// See [`FocusTraversalPolicy::sort_descendants`].
    pub fn sort_descendants(
        self,
        app: &mut App,
        descendants: Vec<AnyFocusNode>,
        current_node: AnyFocusNode,
    ) -> Vec<AnyFocusNode> {
        (self.vtable.sort_descendants)(app, self.id, descendants, current_node)
    }
}

// ---------------------------------------------------------------------------------------------
// DirectionalFocusTraversalPolicyMixin

/// A policy data object for use by [`DirectionalFocusTraversalPolicyMixin`] so it can keep track
/// of the traversal history.
#[derive(Clone, Copy)]
struct DirectionalPolicyDataEntry {
    direction: TraversalDirection,
    node: AnyFocusNode,
}

/// A queue of entries that describe the path taken to the current node.
struct DirectionalPolicyData {
    history: Vec<DirectionalPolicyDataEntry>,
}

/// The fields Dart's `DirectionalFocusTraversalPolicyMixin` declares; a policy that mixes it in
/// carries this bag under the field `directional`.
pub struct DirectionalFocusTraversalPolicyMixinData {
    policy_data: HashMap<Handle<FocusScopeNode>, DirectionalPolicyData>,
}

impl DirectionalFocusTraversalPolicyMixinData {
    /// The bag of a freshly created policy.
    pub fn new() -> DirectionalFocusTraversalPolicyMixinData {
        DirectionalFocusTraversalPolicyMixinData {
            policy_data: HashMap::new(),
        }
    }
}

impl Default for DirectionalFocusTraversalPolicyMixinData {
    fn default() -> DirectionalFocusTraversalPolicyMixinData {
        DirectionalFocusTraversalPolicyMixinData::new()
    }
}

/// A trait that provides an implementation for finding a node in a particular direction.
///
/// This can be implemented by other [`FocusTraversalPolicy`] implementations that only want to
/// implement new next/previous policies.
///
/// Since hysteresis in the navigation order is undesirable, this implementation maintains a
/// stack of previous locations that have been visited on the policy data for the affected
/// `FocusScopeNode`. If the previous direction was the opposite of the current direction, then
/// this policy will request focus on the previously focused node. Changing to another direction
/// other than the current one or its opposite will clear the stack.
///
/// For instance, if the focus moves down, down, down, and then up, up, up, it will follow the
/// same path through the widgets in both directions. However, if it moves down, down, down,
/// left, right, and then up, up, up, it may not follow the same path on the way up as it did on
/// the way down, since changing the axis of motion resets the history.
///
/// This trait implements an algorithm that considers a band extending along the direction of
/// movement within the `FocusScope`, the width or height (depending on direction) of the
/// currently focused widget, and finds the closest widget in that band along the direction of
/// movement. If nothing is found in that band, then it picks the widget with an edge closest to
/// the band in the perpendicular direction. If two out-of-band widgets are the same distance
/// from the band, then it picks the one closest along the direction of movement. When reaching
/// the edge in the direction specified by the `FocusScope`, different behaviors are taken
/// according to `FocusScopeNode::directional_traversal_edge_behavior`.
pub trait DirectionalFocusTraversalPolicyMixin: FocusTraversalPolicy {
    /// Dart's `DirectionalFocusTraversalPolicyMixin` fields, held under the field `directional`.
    fn directional_data(self: Handle<Self>, app: &App)
    -> &DirectionalFocusTraversalPolicyMixinData;

    /// See [`directional_data`](Self::directional_data).
    fn directional_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut DirectionalFocusTraversalPolicyMixinData;

    /// Dart's `DirectionalFocusTraversalPolicyMixin.invalidateScopeData`.
    fn invalidate_scope_data(self: Handle<Self>, app: &mut App, node: Handle<FocusScopeNode>) {
        self.directional_data_mut(app).policy_data.remove(&node);
    }

    /// Dart's `DirectionalFocusTraversalPolicyMixin.changedScope`.
    fn changed_scope(
        self: Handle<Self>,
        app: &mut App,
        node: Option<AnyFocusNode>,
        old_scope: Option<Handle<FocusScopeNode>>,
    ) {
        if let Some(old_scope) = old_scope
            && let Some(data) = self
                .directional_data_mut(app)
                .policy_data
                .get_mut(&old_scope)
        {
            data.history.retain(|entry| Some(entry.node) != node);
        }
    }

    /// Dart's `DirectionalFocusTraversalPolicyMixin.findFirstFocusInDirection`.
    fn find_first_focus_in_direction(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        direction: TraversalDirection,
    ) -> Option<AnyFocusNode> {
        let nodes = current_node
            .nearest_scope(app)
            .expect("a node in the tree has a nearest scope")
            .traversal_descendants(app);
        let mut sorted = nodes;
        let (vertical, first) = match direction {
            // Start with the bottom-most node.
            TraversalDirection::Up => (true, false),
            // Start with the topmost node.
            TraversalDirection::Down => (true, true),
            // Start with the rightmost node.
            TraversalDirection::Left => (false, false),
            // Start with the leftmost node.
            TraversalDirection::Right => (false, true),
        };
        let rects = rects_of(app, &sorted);
        sorted.sort_by(|a, b| {
            let a = rects[a];
            let b = rects[b];
            if vertical {
                if first {
                    total_cmp(a.top, b.top)
                } else {
                    total_cmp(b.bottom, a.bottom)
                }
            } else if first {
                total_cmp(a.left, b.left)
            } else {
                total_cmp(b.right, a.right)
            }
        });
        sorted.first().copied()
    }

    /// Dart's `DirectionalFocusTraversalPolicyMixin._findNextFocusInDirection`.
    fn find_next_focus_in_direction(
        self: Handle<Self>,
        app: &mut App,
        focused_child: AnyFocusNode,
        traversal_descendants: Vec<AnyFocusNode>,
        direction: TraversalDirection,
        forward: bool,
    ) -> Option<AnyFocusNode> {
        let target = focused_child.rect(app);
        match direction {
            TraversalDirection::Down | TraversalDirection::Up => {
                let mut eligible_nodes = sort_and_filter_vertically(
                    app,
                    direction,
                    target,
                    &traversal_descendants,
                    forward,
                );
                if eligible_nodes.is_empty() {
                    return None;
                }
                filter_to_scrollable(app, focused_child, &mut eligible_nodes, Axis::Vertical);
                if direction == TraversalDirection::Up {
                    eligible_nodes.reverse();
                }
                // Find any nodes that intersect the band of the focused child.
                let band =
                    Rect::from_ltrb(target.left, f64::NEG_INFINITY, target.right, f64::INFINITY);
                let in_band = nodes_in_band(app, &eligible_nodes, band);
                if !in_band.is_empty() {
                    let sorted = sort_by_distance_prefer_vertical(app, target.center(), &in_band);
                    return if forward {
                        sorted.first().copied()
                    } else {
                        sorted.last().copied()
                    };
                }
                // Only out-of-band targets are eligible, so pick the one that is closest to the
                // center line horizontally, and if any are the same distance horizontally, pick
                // the closest one of those vertically.
                let sorted = sort_closest_edges_by_distance_prefer_horizontal(
                    app,
                    target.center(),
                    &eligible_nodes,
                );
                if forward {
                    sorted.first().copied()
                } else {
                    sorted.last().copied()
                }
            }
            TraversalDirection::Right | TraversalDirection::Left => {
                let mut eligible_nodes = sort_and_filter_horizontally(
                    app,
                    direction,
                    target,
                    &traversal_descendants,
                    forward,
                );
                if eligible_nodes.is_empty() {
                    return None;
                }
                filter_to_scrollable(app, focused_child, &mut eligible_nodes, Axis::Horizontal);
                if direction == TraversalDirection::Left {
                    eligible_nodes.reverse();
                }
                let band =
                    Rect::from_ltrb(f64::NEG_INFINITY, target.top, f64::INFINITY, target.bottom);
                let in_band = nodes_in_band(app, &eligible_nodes, band);
                if !in_band.is_empty() {
                    let sorted = sort_by_distance_prefer_horizontal(app, target.center(), &in_band);
                    return if forward {
                        sorted.first().copied()
                    } else {
                        sorted.last().copied()
                    };
                }
                // Only out-of-band targets are eligible, so pick the one that is closest to the
                // center line vertically, and if any are the same distance vertically, pick the
                // closest one of those horizontally.
                let sorted = sort_closest_edges_by_distance_prefer_vertical(
                    app,
                    target.center(),
                    &eligible_nodes,
                );
                if forward {
                    sorted.first().copied()
                } else {
                    sorted.last().copied()
                }
            }
        }
    }

    /// Updates the policy data to keep the previously visited node so that we can avoid
    /// hysteresis when we change directions in navigation.
    ///
    /// Returns true if focus was requested on a previous node.
    fn pop_policy_data_if_needed(
        self: Handle<Self>,
        app: &mut App,
        direction: TraversalDirection,
        nearest_scope: Handle<FocusScopeNode>,
        group_node: Option<Handle<FocusTraversalGroupNode>>,
    ) -> bool {
        let history_head = self
            .directional_data(app)
            .policy_data
            .get(&nearest_scope)
            .and_then(|data| data.history.first().copied());
        if let Some(head) = history_head
            && head.direction != direction
        {
            let last_node = self
                .directional_data(app)
                .policy_data
                .get(&nearest_scope)
                .and_then(|data| data.history.last().copied())
                .expect("a non-empty history has a last entry")
                .node;
            if last_node.parent(app).is_none() {
                // If a node has been removed from the tree, then we should stop referencing it
                // and reset the scope data so that we don't try and request focus on it.
                FocusTraversalPolicy::invalidate_scope_data(self, app, nearest_scope);
                return false;
            }

            let same_axis = matches!(
                (direction, head.direction),
                (
                    TraversalDirection::Down | TraversalDirection::Up,
                    TraversalDirection::Up | TraversalDirection::Down
                ) | (
                    TraversalDirection::Left | TraversalDirection::Right,
                    TraversalDirection::Left | TraversalDirection::Right
                )
            );
            if same_axis {
                if self.pop_or_invalidate(app, direction, nearest_scope, group_node) {
                    return true;
                }
            } else {
                // Reset the policy data if we change directions.
                FocusTraversalPolicy::invalidate_scope_data(self, app, nearest_scope);
            }
        }
        let history_is_empty = self
            .directional_data(app)
            .policy_data
            .get(&nearest_scope)
            .is_some_and(|data| data.history.is_empty());
        if history_is_empty {
            FocusTraversalPolicy::invalidate_scope_data(self, app, nearest_scope);
        }
        false
    }

    /// Dart's `popOrInvalidate` closure inside `_popPolicyDataIfNeeded`. Returns true if it
    /// successfully popped the history.
    fn pop_or_invalidate(
        self: Handle<Self>,
        app: &mut App,
        direction: TraversalDirection,
        nearest_scope: Handle<FocusScopeNode>,
        group_node: Option<Handle<FocusTraversalGroupNode>>,
    ) -> bool {
        let last_node = self
            .directional_data_mut(app)
            .policy_data
            .get_mut(&nearest_scope)
            .and_then(|data| data.history.pop())
            .expect("a non-empty history has a last entry")
            .node;
        let last_context = last_node
            .context(app)
            .expect("a focused node has a context");
        let focused_context = primary_focus(app)
            .expect("a directional traversal starts from the primary focus")
            .context(app)
            .expect("a focused node has a context");
        if Scrollable::maybe_of(app, last_context, None)
            != Scrollable::maybe_of(app, focused_context, None)
        {
            FocusTraversalPolicy::invalidate_scope_data(self, app, nearest_scope);
            return false;
        }
        let alignment_policy = match direction {
            TraversalDirection::Up | TraversalDirection::Left => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtStart
            }
            TraversalDirection::Right | TraversalDirection::Down => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
            }
        };
        self.request_focus(app, last_node, Some(alignment_policy), group_node);
        true
    }

    /// Dart's `DirectionalFocusTraversalPolicyMixin._pushPolicyData`.
    fn push_policy_data(
        self: Handle<Self>,
        app: &mut App,
        direction: TraversalDirection,
        nearest_scope: Handle<FocusScopeNode>,
        focused_child: AnyFocusNode,
    ) {
        let new_entry = DirectionalPolicyDataEntry {
            node: focused_child,
            direction,
        };
        self.directional_data_mut(app)
            .policy_data
            .entry(nearest_scope)
            .or_insert_with(|| DirectionalPolicyData {
                history: Vec::new(),
            })
            .history
            .push(new_entry);
    }

    /// Dart's `DirectionalFocusTraversalPolicyMixin._requestTraversalFocusInDirection`.
    fn request_traversal_focus_in_direction(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        node: AnyFocusNode,
        nearest_scope: Handle<FocusScopeNode>,
        direction: TraversalDirection,
        group_node: Option<Handle<FocusTraversalGroupNode>>,
    ) -> bool {
        if let Some(scope) = node.as_scope() {
            if let Some(focused_child) = scope.focused_child(app) {
                return self.request_traversal_focus_in_direction(
                    app,
                    current_node,
                    focused_child,
                    scope,
                    direction,
                    group_node,
                );
            }
            let first_node =
                FocusTraversalPolicy::find_first_focus_in_direction(self, app, node, direction)
                    .unwrap_or(current_node);
            let alignment_policy = match direction {
                TraversalDirection::Up | TraversalDirection::Left => {
                    ScrollPositionAlignmentPolicy::KeepVisibleAtStart
                }
                TraversalDirection::Right | TraversalDirection::Down => {
                    ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
                }
            };
            self.request_focus(app, first_node, Some(alignment_policy), group_node);
            return true;
        }
        let _ = nearest_scope;
        let node_had_primary_focus = node.has_primary_focus(app);
        let alignment_policy = match direction {
            TraversalDirection::Up | TraversalDirection::Left => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtStart
            }
            TraversalDirection::Right | TraversalDirection::Down => {
                ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
            }
        };
        self.request_focus(app, node, Some(alignment_policy), group_node);
        !node_had_primary_focus
    }

    /// Dart's `DirectionalFocusTraversalPolicyMixin._requestFocus`.
    fn request_focus(
        self: Handle<Self>,
        app: &mut App,
        node: AnyFocusNode,
        alignment_policy: Option<ScrollPositionAlignmentPolicy>,
        group_node: Option<Handle<FocusTraversalGroupNode>>,
    ) {
        if let Some(group_node) = group_node {
            app.get_mut(group_node).last_requested_focus = Some(node);
        }
        let request_focus_callback = self.request_focus_callback(app);
        request_focus_callback(app, node, alignment_policy, None, None, None);
    }

    /// Dart's `DirectionalFocusTraversalPolicyMixin._onEdgeForDirection`.
    fn on_edge_for_direction(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        focused_child: AnyFocusNode,
        group_node: Option<Handle<FocusTraversalGroupNode>>,
        direction: TraversalDirection,
        scope: Option<Handle<FocusScopeNode>>,
    ) -> bool {
        let mut nearest_scope = scope.unwrap_or_else(|| {
            current_node
                .nearest_scope(app)
                .expect("a node in the tree has a nearest scope")
        });
        let found;
        match nearest_scope.directional_traversal_edge_behavior(app) {
            TraversalEdgeBehavior::LeaveFlutterView => {
                focused_child.unfocus(app, Default::default());
                return false;
            }
            TraversalEdgeBehavior::ParentScope => {
                let parent_scope = nearest_scope.enclosing_scope(app);
                let root_scope = FocusManager::instance(app).root_scope(app);
                if let Some(parent_scope) = parent_scope
                    && parent_scope != root_scope
                {
                    FocusTraversalPolicy::invalidate_scope_data(self, app, nearest_scope);
                    nearest_scope = parent_scope;
                    FocusTraversalPolicy::invalidate_scope_data(self, app, nearest_scope);
                    let descendants = nearest_scope.traversal_descendants(app);
                    let next = self.find_next_focus_in_direction(
                        app,
                        focused_child,
                        descendants,
                        direction,
                        true,
                    );
                    match next {
                        None => {
                            return self.on_edge_for_direction(
                                app,
                                current_node,
                                focused_child,
                                group_node,
                                direction,
                                Some(nearest_scope),
                            );
                        }
                        Some(next) => found = Some(next),
                    }
                } else {
                    let descendants = nearest_scope.traversal_descendants(app);
                    found = self.find_next_focus_in_direction(
                        app,
                        focused_child,
                        descendants,
                        direction,
                        false,
                    );
                }
            }
            TraversalEdgeBehavior::ClosedLoop => {
                let descendants = nearest_scope.traversal_descendants(app);
                found = self.find_next_focus_in_direction(
                    app,
                    focused_child,
                    descendants,
                    direction,
                    false,
                );
            }
            TraversalEdgeBehavior::Stop => return false,
        }
        if let Some(found) = found {
            return self.request_traversal_focus_in_direction(
                app,
                current_node,
                found,
                nearest_scope,
                direction,
                group_node,
            );
        }
        false
    }

    /// Focuses the next widget in the given `direction` in the `FocusScope` that contains the
    /// `current_node`.
    ///
    /// This determines what the next node to receive focus in the given `direction` will be by
    /// inspecting the node tree, and then calling [`AnyFocusNode::request_focus`] on it.
    ///
    /// Returns true if it successfully found a node and requested focus.
    ///
    /// Maintains a stack of previous locations that have been visited on the policy data for the
    /// affected `FocusScopeNode`. If the previous direction was the opposite of the current
    /// direction, then this policy will request focus on the previously focused node. Changing
    /// to another direction other than the current one or its opposite will clear the stack.
    fn in_direction(
        self: Handle<Self>,
        app: &mut App,
        current_node: AnyFocusNode,
        direction: TraversalDirection,
    ) -> bool {
        let group_node = FocusTraversalGroup::get_group_node(app, current_node);
        let nearest_scope = current_node
            .nearest_scope(app)
            .expect("a node in the tree has a nearest scope");
        let focused_child = nearest_scope.focused_child(app);
        let Some(focused_child) = focused_child else {
            let first_focus = FocusTraversalPolicy::find_first_focus_in_direction(
                self,
                app,
                current_node,
                direction,
            )
            .unwrap_or(current_node);
            let alignment_policy = match direction {
                TraversalDirection::Up | TraversalDirection::Left => {
                    ScrollPositionAlignmentPolicy::KeepVisibleAtStart
                }
                TraversalDirection::Right | TraversalDirection::Down => {
                    ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
                }
            };
            self.request_focus(app, first_focus, Some(alignment_policy), group_node);
            return true;
        };
        if self.pop_policy_data_if_needed(app, direction, nearest_scope, group_node) {
            return true;
        }
        let descendants = nearest_scope.traversal_descendants(app);
        let found =
            self.find_next_focus_in_direction(app, focused_child, descendants, direction, true);
        if let Some(found) = found {
            self.push_policy_data(app, direction, nearest_scope, focused_child);
            return self.request_traversal_focus_in_direction(
                app,
                current_node,
                found,
                nearest_scope,
                direction,
                group_node,
            );
        }
        self.on_edge_for_direction(
            app,
            current_node,
            focused_child,
            group_node,
            direction,
            None,
        )
    }
}

/// `f64` has no total order; Dart's `compareTo` on a double does.
fn total_cmp(a: f64, b: f64) -> Ordering {
    a.total_cmp(&b)
}

/// The rectangle of every node, read once: `AnyFocusNode::rect` does a coordinate
/// transformation each time.
fn rects_of(app: &App, nodes: &[AnyFocusNode]) -> HashMap<AnyFocusNode, Rect> {
    nodes.iter().map(|node| (*node, node.rect(app))).collect()
}

fn vertical_compare(target: Offset, a: Offset, b: Offset) -> Ordering {
    total_cmp((a.dy() - target.dy()).abs(), (b.dy() - target.dy()).abs())
}

fn horizontal_compare(target: Offset, a: Offset, b: Offset) -> Ordering {
    total_cmp((a.dx() - target.dx()).abs(), (b.dx() - target.dx()).abs())
}

/// Sort the ones that are closest to target vertically first, and if two are the same vertical
/// distance, pick the one that is closest horizontally.
fn sort_by_distance_prefer_vertical(
    app: &App,
    target: Offset,
    nodes: &[AnyFocusNode],
) -> Vec<AnyFocusNode> {
    let rects = rects_of(app, nodes);
    let mut sorted = nodes.to_vec();
    sorted.sort_by(|node_a, node_b| {
        let a = rects[node_a].center();
        let b = rects[node_b].center();
        match vertical_compare(target, a, b) {
            Ordering::Equal => horizontal_compare(target, a, b),
            vertical => vertical,
        }
    });
    sorted
}

/// Sort the ones that are closest horizontally first, and if two are the same horizontal
/// distance, pick the one that is closest vertically.
fn sort_by_distance_prefer_horizontal(
    app: &App,
    target: Offset,
    nodes: &[AnyFocusNode],
) -> Vec<AnyFocusNode> {
    let rects = rects_of(app, nodes);
    let mut sorted = nodes.to_vec();
    sorted.sort_by(|node_a, node_b| {
        let a = rects[node_a].center();
        let b = rects[node_b].center();
        match horizontal_compare(target, a, b) {
            Ordering::Equal => vertical_compare(target, a, b),
            horizontal => horizontal,
        }
    });
    sorted
}

fn vertical_compare_closest_edge(target: Offset, a: Rect, b: Rect) -> Ordering {
    // Find which edge is closest to the target for each.
    let a_coord = if (a.top - target.dy()).abs() < (a.bottom - target.dy()).abs() {
        a.top
    } else {
        a.bottom
    };
    let b_coord = if (b.top - target.dy()).abs() < (b.bottom - target.dy()).abs() {
        b.top
    } else {
        b.bottom
    };
    total_cmp((a_coord - target.dy()).abs(), (b_coord - target.dy()).abs())
}

fn horizontal_compare_closest_edge(target: Offset, a: Rect, b: Rect) -> Ordering {
    // Find which edge is closest to the target for each.
    let a_coord = if (a.left - target.dx()).abs() < (a.right - target.dx()).abs() {
        a.left
    } else {
        a.right
    };
    let b_coord = if (b.left - target.dx()).abs() < (b.right - target.dx()).abs() {
        b.left
    } else {
        b.right
    };
    total_cmp((a_coord - target.dx()).abs(), (b_coord - target.dx()).abs())
}

/// Sort the ones that have edges that are closest horizontally first, and if two are the same
/// horizontal distance, pick the one that is closest vertically.
fn sort_closest_edges_by_distance_prefer_horizontal(
    app: &App,
    target: Offset,
    nodes: &[AnyFocusNode],
) -> Vec<AnyFocusNode> {
    let rects = rects_of(app, nodes);
    let mut sorted = nodes.to_vec();
    sorted.sort_by(|node_a, node_b| {
        let a = rects[node_a];
        let b = rects[node_b];
        match horizontal_compare_closest_edge(target, a, b) {
            // If they're the same distance horizontally, pick the closest one vertically.
            Ordering::Equal => vertical_compare(target, a.center(), b.center()),
            horizontal => horizontal,
        }
    });
    sorted
}

/// Sort the ones that have edges that are closest vertically first, and if two are the same
/// vertical distance, pick the one that is closest horizontally.
fn sort_closest_edges_by_distance_prefer_vertical(
    app: &App,
    target: Offset,
    nodes: &[AnyFocusNode],
) -> Vec<AnyFocusNode> {
    let rects = rects_of(app, nodes);
    let mut sorted = nodes.to_vec();
    sorted.sort_by(|node_a, node_b| {
        let a = rects[node_a];
        let b = rects[node_b];
        match vertical_compare_closest_edge(target, a, b) {
            // If they're the same distance vertically, pick the closest one horizontally.
            Ordering::Equal => horizontal_compare(target, a.center(), b.center()),
            vertical => vertical,
        }
    });
    sorted
}

/// Sorts nodes from left to right horizontally, and removes nodes that are either to the right
/// of the left side of the target node if we're going left, or to the left of the right side of
/// the target node if we're going right.
///
/// This doesn't need to take into account directionality because it is typically intending to
/// actually go left or right, not in a reading direction.
fn sort_and_filter_horizontally(
    app: &App,
    direction: TraversalDirection,
    target: Rect,
    nodes: &[AnyFocusNode],
    forward: bool,
) -> Vec<AnyFocusNode> {
    debug_assert!(direction == TraversalDirection::Left || direction == TraversalDirection::Right);
    let rects = rects_of(app, nodes);
    let mut sorted: Vec<AnyFocusNode> = nodes
        .iter()
        .copied()
        .filter(|node| {
            let rect = rects[node];
            rect != target
                && match direction {
                    TraversalDirection::Left => {
                        if forward {
                            rect.center().dx() <= target.left
                        } else {
                            rect.center().dx() >= target.left
                        }
                    }
                    TraversalDirection::Right => {
                        if forward {
                            rect.center().dx() >= target.right
                        } else {
                            rect.center().dx() <= target.right
                        }
                    }
                    TraversalDirection::Up | TraversalDirection::Down => {
                        panic!("Invalid direction {direction:?}")
                    }
                }
        })
        .collect();
    // Sort all nodes from left to right.
    sorted.sort_by(|a, b| total_cmp(rects[a].center().dx(), rects[b].center().dx()));
    sorted
}

/// Restricts the candidates to the focused node's own `Scrollable` on the given axis, unless none
/// of them share it.
///
/// Dart's `eligibleNodes.where(..)` inside `_findNextFocusInDirection`.
fn filter_to_scrollable(
    app: &mut App,
    focused_child: AnyFocusNode,
    eligible_nodes: &mut Vec<AnyFocusNode>,
    axis: Axis,
) {
    let context = focused_child
        .context(app)
        .expect("a focused node has a context");
    let Some(focused_scrollable) = Scrollable::maybe_of(app, context, Some(axis)) else {
        return;
    };
    let mut filtered = Vec::new();
    for node in eligible_nodes.iter().copied() {
        let context = node.context(app).expect("a traversable node has a context");
        if Scrollable::maybe_of(app, context, Some(axis)) == Some(focused_scrollable) {
            filtered.push(node);
        }
    }
    if !filtered.is_empty() {
        *eligible_nodes = filtered;
    }
}

/// Sorts nodes from top to bottom vertically, and removes nodes that are either below the top of
/// the target node if we're going up, or above the bottom of the target node if we're going
/// down.
fn sort_and_filter_vertically(
    app: &App,
    direction: TraversalDirection,
    target: Rect,
    nodes: &[AnyFocusNode],
    forward: bool,
) -> Vec<AnyFocusNode> {
    debug_assert!(direction == TraversalDirection::Up || direction == TraversalDirection::Down);
    let rects = rects_of(app, nodes);
    let mut sorted: Vec<AnyFocusNode> = nodes
        .iter()
        .copied()
        .filter(|node| {
            let rect = rects[node];
            rect != target
                && match direction {
                    TraversalDirection::Up => {
                        if forward {
                            rect.center().dy() <= target.top
                        } else {
                            rect.center().dy() >= target.top
                        }
                    }
                    TraversalDirection::Down => {
                        if forward {
                            rect.center().dy() >= target.bottom
                        } else {
                            rect.center().dy() <= target.bottom
                        }
                    }
                    TraversalDirection::Left | TraversalDirection::Right => {
                        panic!("Invalid direction {direction:?}")
                    }
                }
        })
        .collect();
    sorted.sort_by(|a, b| total_cmp(rects[a].center().dy(), rects[b].center().dy()));
    sorted
}

/// Dart's `eligibleNodes.where((node) => !node.rect.intersect(band).isEmpty)`.
fn nodes_in_band(app: &App, nodes: &[AnyFocusNode], band: Rect) -> Vec<AnyFocusNode> {
    nodes
        .iter()
        .copied()
        .filter(|node| !node.rect(app).intersect(band).is_empty())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// WidgetOrderTraversalPolicy

/// A [`FocusTraversalPolicy`] that traverses the focus order in widget hierarchy order.
///
/// This policy is used when the order desired is the order in which widgets are created in the
/// widget hierarchy.
///
/// See also:
///
///  * `FocusNode`, for a description of the focus system.
///  * [`FocusTraversalGroup`], a widget that groups together and imposes a traversal policy on
///    the `Focus` nodes below it in the widget hierarchy.
///  * [`ReadingOrderTraversalPolicy`], a policy that describes the order as the natural "reading
///    order" for the current [`Directionality`].
///  * [`DirectionalFocusTraversalPolicyMixin`], a trait that implements focus traversal in a
///    direction.
///  * [`OrderedTraversalPolicy`], a policy that describes the order explicitly using
///    [`FocusTraversalOrder`] widgets.
pub struct WidgetOrderTraversalPolicy {
    policy: FocusTraversalPolicyData,
    directional: DirectionalFocusTraversalPolicyMixinData,
}

impl WidgetOrderTraversalPolicy {
    /// Constructs a traversal policy that orders widgets for keyboard traversal based on the
    /// widget hierarchy order.
    ///
    /// Dart's optional `requestFocusCallback` argument is
    /// [`FocusTraversalPolicy::set_request_focus_callback`].
    pub fn new(app: &mut App) -> Handle<WidgetOrderTraversalPolicy> {
        app.create(WidgetOrderTraversalPolicy {
            policy: FocusTraversalPolicyData::new(),
            directional: DirectionalFocusTraversalPolicyMixinData::new(),
        })
    }
}

impl FocusTraversalPolicy for WidgetOrderTraversalPolicy {
    crate::focus_traversal_policy_accessors!();
    crate::directional_focus_traversal_policy_overrides!();

    fn sort_descendants(
        self: Handle<Self>,
        _app: &mut App,
        descendants: Vec<AnyFocusNode>,
        _current_node: AnyFocusNode,
    ) -> Vec<AnyFocusNode> {
        descendants
    }
}

impl DirectionalFocusTraversalPolicyMixin for WidgetOrderTraversalPolicy {
    fn directional_data(
        self: Handle<Self>,
        app: &App,
    ) -> &DirectionalFocusTraversalPolicyMixinData {
        &app.get(self).directional
    }

    fn directional_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut DirectionalFocusTraversalPolicyMixinData {
        &mut app.get_mut(self).directional
    }
}

// ---------------------------------------------------------------------------------------------
// ReadingOrderTraversalPolicy

/// The rect is copied out of the node, because it will be accessed many times in the reading
/// order algorithm, and [`AnyFocusNode::rect`] does coordinate transformation.
///
/// It's also a convenient place to put some utility functions having to do with the sort data.
#[derive(Clone)]
struct ReadingOrderSortData {
    directionality: Option<TextDirection>,
    rect: Rect,
    node: AnyFocusNode,
    /// The `Directionality` elements above this node, nearest first. Dart holds the widgets and
    /// compares them by identity; the element is the same identity within one sort.
    directional_ancestors: Vec<BuildContext>,
}

impl ReadingOrderSortData {
    fn new(app: &App, node: AnyFocusNode) -> ReadingOrderSortData {
        let context = node.context(app).expect("a traversable node has a context");
        ReadingOrderSortData {
            directionality: find_directionality(app, context),
            rect: node.rect(app),
            node,
            directional_ancestors: directionality_ancestors(app, context),
        }
    }

    /// Finds the common directional ancestor of an entire list of groups.
    fn common_directionality_of(app: &App, list: &[ReadingOrderSortData]) -> Option<TextDirection> {
        let mut common: Option<Vec<BuildContext>> = None;
        for member in list {
            common = Some(match common {
                None => member.directional_ancestors.clone(),
                Some(common) => common
                    .into_iter()
                    .filter(|ancestor| member.directional_ancestors.contains(ancestor))
                    .collect(),
            });
        }
        let common = common.expect("the list is never empty");
        if common.is_empty() {
            // If there is no common ancestor, then arbitrarily pick the directionality of the
            // first group, which is the equivalent of the "first strongly typed" item in a
            // bidirectional algorithm.
            return list[0].directionality;
        }
        // Find the closest common ancestor. The first member's ancestry was added in order from
        // nearest to furthest, so we can still use that to determine the closest one.
        let closest = list[0]
            .directional_ancestors
            .iter()
            .find(|ancestor| common.contains(ancestor))
            .expect("the intersection is a subset of the first member's ancestors");
        Some(text_direction_of(app, *closest))
    }

    fn sort_with_directionality(list: &mut [ReadingOrderSortData], directionality: TextDirection) {
        list.sort_by(|a, b| match directionality {
            TextDirection::Ltr => total_cmp(a.rect.left, b.rect.left),
            TextDirection::Rtl => total_cmp(b.rect.right, a.rect.right),
        });
    }
}

/// Find the directionality in force for a build context without creating a dependency.
fn find_directionality(app: &App, context: BuildContext) -> Option<TextDirection> {
    context
        .get_inherited_widget_of_exact_type::<Directionality>(app)
        .map(|directionality| directionality.text_direction)
}

/// The `Directionality` widget an element holds.
fn text_direction_of(app: &App, element: BuildContext) -> TextDirection {
    downcast_widget::<Directionality>(&**element.widget(app))
        .expect("the element of a Directionality holds one")
        .text_direction
}

/// Returns the list of `Directionality` elements, in order from nearest to furthest.
fn directionality_ancestors(app: &App, context: BuildContext) -> Vec<BuildContext> {
    let mut result = Vec::new();
    let mut directionality_element =
        context.get_element_for_inherited_widget_of_exact_type::<Directionality>(app);
    while let Some(element) = directionality_element {
        result.push(element);
        directionality_element = get_ancestor(app, element, 1).and_then(|ancestor| {
            ancestor.get_element_for_inherited_widget_of_exact_type::<Directionality>(app)
        });
    }
    result
}

/// A structure for containing group data while sorting in reading order while taking into
/// account the ambient directionality.
struct ReadingOrderDirectionalGroupData {
    members: Vec<ReadingOrderSortData>,
}

impl ReadingOrderDirectionalGroupData {
    fn directionality(&self) -> Option<TextDirection> {
        self.members[0].directionality
    }

    fn rect(&self) -> Rect {
        let mut rect: Option<Rect> = None;
        for member in &self.members {
            rect = Some(match rect {
                None => member.rect,
                Some(rect) => rect.expand_to_include(member.rect),
            });
        }
        rect.expect("a group always has a member")
    }

    fn sort_with_directionality(
        list: &mut [ReadingOrderDirectionalGroupData],
        directionality: TextDirection,
    ) {
        list.sort_by(|a, b| match directionality {
            TextDirection::Ltr => total_cmp(a.rect().left, b.rect().left),
            TextDirection::Rtl => total_cmp(b.rect().right, a.rect().right),
        });
    }
}

/// Traverses the focus order in "reading order".
///
/// By default, reading order traversal goes in the reading direction, and then down, using this
/// algorithm:
///
/// 1. Find the node rectangle that has the highest `top` on the screen.
/// 2. Find any other nodes that intersect the infinite horizontal band defined by the highest
///    rectangle's top and bottom edges.
/// 3. Pick the closest to the beginning of the reading order from among the nodes discovered
///    above.
///
/// It uses the ambient [`Directionality`] in the context for the enclosing
/// [`FocusTraversalGroup`] to determine which direction is "reading order".
pub struct ReadingOrderTraversalPolicy {
    policy: FocusTraversalPolicyData,
    directional: DirectionalFocusTraversalPolicyMixinData,
}

impl ReadingOrderTraversalPolicy {
    /// Constructs a traversal policy that orders the widgets in "reading order".
    ///
    /// Dart's optional `requestFocusCallback` argument is
    /// [`FocusTraversalPolicy::set_request_focus_callback`].
    pub fn new(app: &mut App) -> Handle<ReadingOrderTraversalPolicy> {
        app.create(ReadingOrderTraversalPolicy {
            policy: FocusTraversalPolicyData::new(),
            directional: DirectionalFocusTraversalPolicyMixinData::new(),
        })
    }

    /// Sorts the input focus nodes into reading order.
    pub fn sort(app: &App, nodes: Vec<AnyFocusNode>) -> Vec<AnyFocusNode> {
        if nodes.len() <= 1 {
            return nodes;
        }

        let mut unplaced: Vec<ReadingOrderSortData> = nodes
            .iter()
            .map(|node| ReadingOrderSortData::new(app, *node))
            .collect();

        let mut sorted_list = Vec::new();

        // Pick the initial widget as the one that is at the beginning of the band of the
        // topmost, or the topmost, if there are no others in its band.
        //
        // Go through each node, picking the next one after eliminating the previous one, since
        // removing the previously picked node will expose a new band in which to choose
        // candidates.
        while !unplaced.is_empty() {
            let current = ReadingOrderTraversalPolicy::pick_next(app, &mut unplaced);
            sorted_list.push(unplaced[current].node);
            unplaced.remove(current);
        }
        sorted_list
    }

    /// Collects the given candidates into groups by directionality. The candidates have already
    /// been sorted as if they all had the directionality of the nearest `Directionality`
    /// ancestor.
    fn collect_directionality_groups(
        candidates: &[ReadingOrderSortData],
    ) -> Vec<ReadingOrderDirectionalGroupData> {
        let mut current_direction = candidates[0].directionality;
        let mut current_group: Vec<ReadingOrderSortData> = Vec::new();
        let mut result: Vec<ReadingOrderDirectionalGroupData> = Vec::new();
        // Split candidates into runs of the same directionality.
        for candidate in candidates {
            if candidate.directionality == current_direction {
                current_group.push(candidate.clone());
                continue;
            }
            current_direction = candidate.directionality;
            result.push(ReadingOrderDirectionalGroupData {
                members: std::mem::take(&mut current_group),
            });
            current_group.push(candidate.clone());
        }
        if !current_group.is_empty() {
            result.push(ReadingOrderDirectionalGroupData {
                members: current_group,
            });
        }
        // Sort each group separately. Each group has the same directionality.
        for band_group in result.iter_mut() {
            if band_group.members.len() == 1 {
                continue; // No need to sort one node.
            }
            let directionality = band_group
                .directionality()
                .expect("a member with no directionality has no group to sort");
            ReadingOrderSortData::sort_with_directionality(&mut band_group.members, directionality);
        }
        result
    }

    /// Dart's `ReadingOrderTraversalPolicy._pickNext`: the index of the next node in
    /// `candidates`, which this also sorts by their tops.
    fn pick_next(app: &App, candidates: &mut [ReadingOrderSortData]) -> usize {
        // Find the topmost node by sorting on the top of the rectangles.
        candidates.sort_by(|a, b| total_cmp(a.rect.top, b.rect.top));
        let topmost = candidates[0].clone();

        // Find the candidates that are in the same horizontal band as the current one.
        let band = Rect::from_ltrb(
            f64::NEG_INFINITY,
            topmost.rect.top,
            f64::INFINITY,
            topmost.rect.bottom,
        );
        let mut in_band_of_top: Vec<ReadingOrderSortData> = candidates
            .iter()
            .filter(|item| !item.rect.intersect(band).is_empty())
            .cloned()
            .collect();
        // It has to have at least topmost in it if the topmost is not degenerate.
        debug_assert!(topmost.rect.is_empty() || !in_band_of_top.is_empty());

        // The topmost rect is in a band by itself, so just return that one.
        if in_band_of_top.len() <= 1 {
            return 0;
        }

        // Now that we know there are others in the same band as the topmost, then pick the one
        // at the beginning, depending on the text direction in force.
        //
        // Find out the directionality of the nearest common `Directionality` ancestor for all
        // nodes. This provides a base directionality to use for the ordering of the groups.
        let nearest_common_directionality =
            ReadingOrderSortData::common_directionality_of(app, &in_band_of_top)
                .expect("a node in the tree has a directionality");

        // Do an initial common-directionality-based sort to get consistent geometric ordering
        // for grouping into directionality groups. It has to use the common directionality to be
        // able to group into sane groups for the given directionality, since rectangles can
        // overlap and give different results for different directionalities.
        ReadingOrderSortData::sort_with_directionality(
            &mut in_band_of_top,
            nearest_common_directionality,
        );

        // Collect the top band into internally sorted groups with shared directionality.
        let mut band_groups =
            ReadingOrderTraversalPolicy::collect_directionality_groups(&in_band_of_top);
        let picked = if band_groups.len() == 1 {
            // There's only one directionality group, so just send back the first one in that
            // group, since it's already sorted.
            band_groups[0].members[0].node
        } else {
            // Sort the groups based on the common directionality and bounding boxes.
            ReadingOrderDirectionalGroupData::sort_with_directionality(
                &mut band_groups,
                nearest_common_directionality,
            );
            band_groups[0].members[0].node
        };
        candidates
            .iter()
            .position(|candidate| candidate.node == picked)
            .expect("the picked node came from the candidates")
    }
}

impl FocusTraversalPolicy for ReadingOrderTraversalPolicy {
    crate::focus_traversal_policy_accessors!();
    crate::directional_focus_traversal_policy_overrides!();

    /// Sorts the list of nodes based on their geometry into the desired reading order based on
    /// the directionality of the context for each node.
    fn sort_descendants(
        self: Handle<Self>,
        app: &mut App,
        descendants: Vec<AnyFocusNode>,
        _current_node: AnyFocusNode,
    ) -> Vec<AnyFocusNode> {
        ReadingOrderTraversalPolicy::sort(app, descendants)
    }
}

impl DirectionalFocusTraversalPolicyMixin for ReadingOrderTraversalPolicy {
    fn directional_data(
        self: Handle<Self>,
        app: &App,
    ) -> &DirectionalFocusTraversalPolicyMixinData {
        &app.get(self).directional
    }

    fn directional_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut DirectionalFocusTraversalPolicyMixinData {
        &mut app.get_mut(self).directional
    }
}

// ---------------------------------------------------------------------------------------------
// FocusOrder

/// Base trait for all sort orders for [`OrderedTraversalPolicy`] traversal.
///
/// Only orders of the same type are comparable. If a set of widgets in the same
/// [`FocusTraversalGroup`] contains orders that are not comparable with each other, it will
/// assert, since the ordering between such keys is undefined. To avoid collisions, use a
/// [`FocusTraversalGroup`] to group similarly ordered widgets together.
///
/// When implementing, [`do_compare`](Self::do_compare) must be implemented instead of
/// [`compare_to`](Self::compare_to), which calls [`do_compare`](Self::do_compare) to do the
/// actual comparison.
///
/// See also:
///
///  * [`FocusTraversalGroup`], a widget that groups together and imposes a traversal policy on
///    the `Focus` nodes below it in the widget hierarchy.
///  * [`FocusTraversalOrder`], a widget that assigns an order to a widget subtree for the
///    [`OrderedTraversalPolicy`] to use.
///  * [`NumericFocusOrder`], for a focus order that describes its order with an `f64`.
///  * [`LexicalFocusOrder`], a focus order that assigns a string-based lexical traversal order
///    to a [`FocusTraversalOrder`] widget.
pub trait FocusOrder: Debug {
    /// The value behind the erased order, for the downcast [`do_compare`](Self::do_compare)
    /// does.
    fn as_any(&self) -> &dyn Any;

    /// Compares this object to another [`FocusOrder`].
    ///
    /// When implementing [`FocusOrder`], implement [`do_compare`](Self::do_compare) instead of
    /// this function to do the actual comparison.
    ///
    /// Returns [`Ordering::Less`] if `self` is ordered before `other`, [`Ordering::Greater`] if
    /// `self` is ordered after `other`, and [`Ordering::Equal`] if they are ordered together.
    ///
    /// The `other` argument must be a value that is comparable to this object.
    fn compare_to(&self, other: &dyn FocusOrder) -> Ordering {
        debug_assert!(
            self.as_any().type_id() == other.as_any().type_id(),
            "The sorting algorithm must not compare incomparable keys, since they don't know \
             how to order themselves relative to each other. Comparing {self:?} with {other:?}"
        );
        self.do_compare(other)
    }

    /// The implementation called by [`compare_to`](Self::compare_to) to compare orders.
    ///
    /// The argument is guaranteed to be of the same type as this object.
    ///
    /// Returns [`Ordering::Less`] if this object comes earlier in the sort order than the
    /// `other` argument, and [`Ordering::Greater`] if it comes later. Returning
    /// [`Ordering::Equal`] causes the system to fall back to the secondary sort order defined by
    /// [`OrderedTraversalPolicy::secondary`].
    fn do_compare(&self, other: &dyn FocusOrder) -> Ordering;
}

/// The erased [`FocusOrder`]: what [`FocusTraversalOrder::order`] holds.
pub type FocusOrderRef = Rc<dyn FocusOrder>;

/// Can be given to a [`FocusTraversalOrder`] widget to assign a numerical order to a widget
/// subtree that is using an [`OrderedTraversalPolicy`] to define the order in which widgets
/// should be traversed with the keyboard.
///
/// Dart's `FocusOrder.numeric`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericFocusOrder {
    /// The numerical order to assign to the widget subtree using [`FocusTraversalOrder`].
    ///
    /// Determines the placement of this widget in a sequence of widgets that defines the order
    /// in which this node is traversed by the focus policy.
    ///
    /// Lower values will be traversed first.
    pub order: f64,
}

impl NumericFocusOrder {
    /// Creates an object that describes a focus traversal order numerically.
    pub fn new(order: f64) -> NumericFocusOrder {
        NumericFocusOrder { order }
    }
}

impl FocusOrder for NumericFocusOrder {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn do_compare(&self, other: &dyn FocusOrder) -> Ordering {
        let other = other
            .as_any()
            .downcast_ref::<NumericFocusOrder>()
            .expect("compare_to checked the type");
        total_cmp(self.order, other.order)
    }
}

/// Can be given to a [`FocusTraversalOrder`] widget to use a string to assign a lexical order to
/// a widget subtree that is using an [`OrderedTraversalPolicy`] to define the order in which
/// widgets should be traversed with the keyboard.
///
/// This sorts strings using Rust's default string comparison, which is not locale-specific.
///
/// Dart's `FocusOrder.lexical`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexicalFocusOrder {
    /// The string that defines the lexical order to assign to the widget subtree using
    /// [`FocusTraversalOrder`].
    ///
    /// Determines the placement of this widget in a sequence of widgets that defines the order
    /// in which this node is traversed by the focus policy.
    ///
    /// Lower lexical values will be traversed first (e.g. 'a' comes before 'z').
    pub order: String,
}

impl LexicalFocusOrder {
    /// Creates an object that describes a focus traversal order lexically.
    pub fn new(order: impl Into<String>) -> LexicalFocusOrder {
        LexicalFocusOrder {
            order: order.into(),
        }
    }
}

impl FocusOrder for LexicalFocusOrder {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn do_compare(&self, other: &dyn FocusOrder) -> Ordering {
        let other = other
            .as_any()
            .downcast_ref::<LexicalFocusOrder>()
            .expect("compare_to checked the type");
        self.order.cmp(&other.order)
    }
}

/// Used to help sort the focus nodes in an [`OrderedTraversalPolicy`].
struct OrderedFocusInfo {
    node: AnyFocusNode,
    order: FocusOrderRef,
}

/// A [`FocusTraversalPolicy`] that orders nodes by an explicit order that resides in the nearest
/// [`FocusTraversalOrder`] widget ancestor.
///
/// See also:
///
///  * [`FocusTraversalGroup`], a widget that groups together and imposes a traversal policy on
///    the `Focus` nodes below it in the widget hierarchy.
///  * [`WidgetOrderTraversalPolicy`], a policy that relies on the widget creation order to
///    describe the order of traversal.
///  * [`ReadingOrderTraversalPolicy`], a policy that describes the order as the natural "reading
///    order" for the current [`Directionality`].
///  * [`NumericFocusOrder`], a focus order that assigns a numeric traversal order to a
///    [`FocusTraversalOrder`] widget.
///  * [`LexicalFocusOrder`], a focus order that assigns a string-based lexical traversal order
///    to a [`FocusTraversalOrder`] widget.
///  * [`FocusOrder`], a base trait for all types of focus traversal orderings.
pub struct OrderedTraversalPolicy {
    policy: FocusTraversalPolicyData,
    directional: DirectionalFocusTraversalPolicyMixinData,
    secondary: Option<AnyFocusTraversalPolicy>,
}

impl OrderedTraversalPolicy {
    /// Constructs a traversal policy that orders widgets for keyboard traversal based on an
    /// explicit order.
    ///
    /// Dart's optional `secondary` and `requestFocusCallback` arguments are the setters.
    pub fn new(app: &mut App) -> Handle<OrderedTraversalPolicy> {
        app.create(OrderedTraversalPolicy {
            policy: FocusTraversalPolicyData::new(),
            directional: DirectionalFocusTraversalPolicyMixinData::new(),
            secondary: None,
        })
    }

    /// This is the policy that is used when a node doesn't have an order assigned, or when
    /// multiple nodes have orders which are identical.
    ///
    /// If not set, this defaults to [`ReadingOrderTraversalPolicy`].
    ///
    /// This policy determines the secondary sorting order of nodes which evaluate as having an
    /// identical order (including those with no order specified).
    ///
    /// Nodes with no order specified will be sorted after nodes with an explicit order.
    pub fn secondary(self: Handle<Self>, app: &App) -> Option<AnyFocusTraversalPolicy> {
        app.get(self).secondary
    }

    /// Dart `OrderedTraversalPolicy(secondary:)`.
    pub fn set_secondary(self: Handle<Self>, app: &mut App, secondary: AnyFocusTraversalPolicy) {
        app.get_mut(self).secondary = Some(secondary);
    }
}

impl FocusTraversalPolicy for OrderedTraversalPolicy {
    crate::focus_traversal_policy_accessors!();
    crate::directional_focus_traversal_policy_overrides!();

    fn sort_descendants(
        self: Handle<Self>,
        app: &mut App,
        descendants: Vec<AnyFocusNode>,
        current_node: AnyFocusNode,
    ) -> Vec<AnyFocusNode> {
        let secondary_policy = match app.get(self).secondary {
            Some(secondary) => secondary,
            None => ReadingOrderTraversalPolicy::new(app).as_policy(),
        };
        let sorted_descendants = secondary_policy.sort_descendants(app, descendants, current_node);
        let mut unordered: Vec<AnyFocusNode> = Vec::new();
        let mut ordered: Vec<OrderedFocusInfo> = Vec::new();
        for node in sorted_descendants {
            let context = node.context(app).expect("a traversable node has a context");
            match FocusTraversalOrder::maybe_of(app, context) {
                Some(order) => ordered.push(OrderedFocusInfo { node, order }),
                None => unordered.push(node),
            }
        }
        ordered.sort_by(|a, b| {
            debug_assert!(
                a.order.as_any().type_id() == b.order.as_any().type_id(),
                "When sorting nodes for determining focus order, the order ({:?}) of node \
                 {:?}, isn't the same type as the order ({:?}) of {:?}. Incompatible order \
                 types can't be compared. Use a FocusTraversalGroup to group similar orders \
                 together.",
                a.order,
                a.node,
                b.order,
                b.node
            );
            a.order.compare_to(&*b.order)
        });
        ordered
            .into_iter()
            .map(|info| info.node)
            .chain(unordered)
            .collect()
    }
}

impl DirectionalFocusTraversalPolicyMixin for OrderedTraversalPolicy {
    fn directional_data(
        self: Handle<Self>,
        app: &App,
    ) -> &DirectionalFocusTraversalPolicyMixinData {
        &app.get(self).directional
    }

    fn directional_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut DirectionalFocusTraversalPolicyMixinData {
        &mut app.get_mut(self).directional
    }
}

// ---------------------------------------------------------------------------------------------
// FocusTraversalOrder

/// An inherited widget that describes the order in which its child subtree should be traversed.
///
/// The order for a widget is determined by the [`FocusOrder`] returned by
/// [`FocusTraversalOrder::of`] for a particular context.
#[derive(Debug)]
pub struct FocusTraversalOrder {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The order for the widget descendants of this [`FocusTraversalOrder`].
    pub order: FocusOrderRef,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl FocusTraversalOrder {
    /// Creates an inherited widget used to describe the focus order of the `child` subtree.
    pub fn new<K>(order: FocusOrderRef, child: impl IntoWidget<K>) -> FocusTraversalOrder {
        FocusTraversalOrder {
            key: None,
            order,
            child: child.into_widget(),
        }
    }

    /// Dart `FocusTraversalOrder(key:)`.
    pub fn key(mut self, key: KeyRef) -> FocusTraversalOrder {
        self.key = Some(key);
        self
    }

    /// Finds the [`FocusOrder`] in the nearest ancestor [`FocusTraversalOrder`] widget.
    ///
    /// It does not create a rebuild dependency because changing the traversal order doesn't
    /// change the widget tree, so nothing needs to be rebuilt as a result of an order change.
    ///
    /// If no [`FocusTraversalOrder`] ancestor exists, this panics.
    pub fn of(app: &App, context: BuildContext) -> FocusOrderRef {
        FocusTraversalOrder::maybe_of(app, context).expect(
            "FocusTraversalOrder.of() was called with a context that does not contain a \
             FocusTraversalOrder widget.",
        )
    }

    /// Finds the [`FocusOrder`] in the nearest ancestor [`FocusTraversalOrder`] widget.
    ///
    /// It does not create a rebuild dependency because changing the traversal order doesn't
    /// change the widget tree, so nothing needs to be rebuilt as a result of an order change.
    ///
    /// If no [`FocusTraversalOrder`] ancestor exists, returns `None`.
    pub fn maybe_of(app: &App, context: BuildContext) -> Option<FocusOrderRef> {
        context
            .get_inherited_widget_of_exact_type::<FocusTraversalOrder>(app)
            .map(|marker| Rc::clone(&marker.order))
    }
}

impl InheritedWidget for FocusTraversalOrder {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    /// Since the order of traversal doesn't affect display of anything, we don't need to force a
    /// rebuild of anything that depends upon it.
    fn update_should_notify(&self, _old_widget: &FocusTraversalOrder) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------------------------
// FocusTraversalGroup

/// A special focus node that only [`FocusTraversalGroup`] uses so that it can be used to cache
/// the following in the focus tree:
///  - the focus traversal policy — this allows traversal code to find groups in the focus tree.
///  - the last focused node — this allows the policy to be invalidated when a focus change was
///    not triggered via a traversal request.
pub struct FocusTraversalGroupNode {
    change_notifier: ChangeNotifierData,
    focus_node: FocusNodeData,
    /// The policy this group imposes.
    pub policy: AnyFocusTraversalPolicy,
    /// The node the policy last requested focus for.
    pub last_requested_focus: Option<AnyFocusNode>,
}

impl FocusTraversalGroupNode {
    /// Creates a [`FocusTraversalGroupNode`] with the given policy.
    pub fn new(app: &mut App, policy: AnyFocusTraversalPolicy) -> Handle<FocusTraversalGroupNode> {
        app.create(FocusTraversalGroupNode {
            change_notifier: ChangeNotifierData::new(),
            focus_node: FocusNodeData::new(),
            policy,
            last_requested_focus: None,
        })
    }
}

impl ChangeNotifier for FocusTraversalGroupNode {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl FocusNodeLeaf for FocusTraversalGroupNode {
    crate::focus_node_accessors!();
}

/// A widget that describes the inherited focus policy for focus traversal for its descendants,
/// grouping them into a separate traversal group.
///
/// A traversal group is treated as one entity when sorted by the traversal algorithm, so it can
/// be used to segregate different parts of the widget tree that need to be sorted using
/// different algorithms and/or sort orders when using an [`OrderedTraversalPolicy`].
///
/// Within the group, it will use the given [`policy`](Self::policy) to order the elements. The
/// group itself will be ordered using the parent group's policy.
///
/// By default, traverses in reading order using [`ReadingOrderTraversalPolicy`].
///
/// To prevent the members of the group from being focused, set the
/// [`descendants_are_focusable`](Self::descendants_are_focusable) attribute to false.
pub struct FocusTraversalGroup {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The policy used to move the focus from one focus node to another when traversing them
    /// using a keyboard.
    ///
    /// If not specified, traverses in reading order using [`ReadingOrderTraversalPolicy`], which
    /// the state creates, since a policy is an arena object a widget cannot mint.
    ///
    /// See also:
    ///
    ///  * [`FocusTraversalPolicy`] for the API used to impose traversal order policy.
    ///  * [`WidgetOrderTraversalPolicy`] for a traversal policy that traverses nodes in the order
    ///    they are added to the widget tree.
    ///  * [`ReadingOrderTraversalPolicy`] for a traversal policy that traverses nodes in the
    ///    reading order defined in the widget tree, and then top to bottom.
    pub policy: Option<AnyFocusTraversalPolicy>,

    /// If false, will make this widget's descendants unfocusable.
    pub descendants_are_focusable: bool,

    /// If false, will make this widget's descendants untraversable.
    pub descendants_are_traversable: bool,

    /// Called when the `FocusNode` of this widget is created.
    pub on_focus_node_created: Option<ValueChanged<AnyFocusNode>>,

    /// The optional parent node to use when reparenting this group's `FocusNode`.
    ///
    /// If `None`, the enclosing `FocusScope` is used as the parent, which mirrors the widget
    /// tree and is typically what is desired.
    ///
    /// Setting this attaches the group (and the focus subtree below it) to the given node
    /// instead of the enclosing `FocusScope`, which is useful to keep a subtree's focus
    /// independent from the surrounding focus tree (for example, so each `View` forms its own
    /// focus root).
    ///
    /// Defaults to `None`.
    pub parent_node: Option<AnyFocusNode>,

    /// The child widget of this [`FocusTraversalGroup`].
    pub child: WidgetRef,
}

impl FocusTraversalGroup {
    /// Creates a [`FocusTraversalGroup`].
    pub fn new<K>(child: impl IntoWidget<K>) -> FocusTraversalGroup {
        FocusTraversalGroup {
            key: None,
            policy: None,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
            on_focus_node_created: None,
            parent_node: None,
            child: child.into_widget(),
        }
    }

    /// Dart `FocusTraversalGroup(key:)`.
    pub fn key(mut self, key: KeyRef) -> FocusTraversalGroup {
        self.key = Some(key);
        self
    }

    /// Dart `FocusTraversalGroup(policy:)`.
    pub fn policy(mut self, policy: AnyFocusTraversalPolicy) -> FocusTraversalGroup {
        self.policy = Some(policy);
        self
    }

    /// Dart `FocusTraversalGroup(descendantsAreFocusable:)`.
    pub fn descendants_are_focusable(
        mut self,
        descendants_are_focusable: bool,
    ) -> FocusTraversalGroup {
        self.descendants_are_focusable = descendants_are_focusable;
        self
    }

    /// Dart `FocusTraversalGroup(descendantsAreTraversable:)`.
    pub fn descendants_are_traversable(
        mut self,
        descendants_are_traversable: bool,
    ) -> FocusTraversalGroup {
        self.descendants_are_traversable = descendants_are_traversable;
        self
    }

    /// Dart `FocusTraversalGroup(onFocusNodeCreated:)`.
    pub fn on_focus_node_created(
        mut self,
        on_focus_node_created: impl Fn(&mut App, AnyFocusNode) + 'static,
    ) -> FocusTraversalGroup {
        self.on_focus_node_created = Some(Rc::new(on_focus_node_created));
        self
    }

    /// Dart `FocusTraversalGroup(parentNode:)`.
    pub fn parent_node(mut self, parent_node: AnyFocusNode) -> FocusTraversalGroup {
        self.parent_node = Some(parent_node);
        self
    }

    /// Returns the [`FocusTraversalPolicy`] that applies to the nearest ancestor of the given
    /// `FocusNode`.
    ///
    /// Will return `None` if no [`FocusTraversalPolicy`] ancestor applies to the given
    /// `FocusNode`.
    ///
    /// The [`FocusTraversalPolicy`] is set by introducing a [`FocusTraversalGroup`] into the
    /// widget tree, which will associate a policy with the focus tree under the nearest ancestor
    /// `Focus` widget.
    ///
    /// This function differs from [`maybe_of`](Self::maybe_of) in that it takes a `FocusNode`
    /// and only traverses the focus tree to determine the policy in effect.
    pub fn maybe_of_node(app: &App, node: AnyFocusNode) -> Option<AnyFocusTraversalPolicy> {
        FocusTraversalGroup::get_group_node(app, node).map(|group| app.get(group).policy)
    }

    /// Dart's `FocusTraversalGroup._getGroupNode`.
    fn get_group_node(app: &App, node: AnyFocusNode) -> Option<Handle<FocusTraversalGroupNode>> {
        let mut node = node;
        while node.parent(app).is_some() {
            node.context(app)?;
            if let Some(group) = node.downcast::<FocusTraversalGroupNode>(app) {
                return Some(group);
            }
            node = node.parent(app).expect("checked by the loop condition");
        }
        None
    }

    /// Returns the [`FocusTraversalPolicy`] that applies to the `FocusNode` of the nearest
    /// ancestor `Focus` widget, given a [`BuildContext`].
    ///
    /// Panics if no `Focus` ancestor is found, or if no [`FocusTraversalPolicy`] applies to the
    /// associated `FocusNode`.
    ///
    /// This function looks up the nearest ancestor `Focus` (or `FocusScope`) widget, and uses
    /// its `FocusNode` to walk up the focus tree to find the applicable
    /// [`FocusTraversalPolicy`] for that node.
    ///
    /// Calling this function does not create a rebuild dependency because changing the traversal
    /// order doesn't change the widget tree, so nothing needs to be rebuilt as a result of an
    /// order change.
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of) for a similar function that will return `None` if no
    ///    [`FocusTraversalGroup`] ancestor is found.
    ///  * [`maybe_of_node`](Self::maybe_of_node) for a function that will look for a policy
    ///    using a given `FocusNode`, and return `None` if no policy applies.
    pub fn of(app: &mut App, context: BuildContext) -> AnyFocusTraversalPolicy {
        FocusTraversalGroup::maybe_of(app, context).expect(
            "Unable to find a Focus or FocusScope widget in the given context, or the FocusNode \
             from the widget that was found is not associated with a FocusTraversalPolicy.\n\
             FocusTraversalGroup.of() was called with a context that does not contain a Focus \
             or FocusScope widget, or there was no FocusTraversalPolicy in effect.",
        )
    }

    /// Returns the [`FocusTraversalPolicy`] that applies to the `FocusNode` of the nearest
    /// ancestor `Focus` widget, or `None`, given a [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [`maybe_of_node`](Self::maybe_of_node) for a similar function that will look for a
    ///    policy using a given `FocusNode`.
    ///  * [`of`](Self::of) for a similar function that will panic if no
    ///    [`FocusTraversalPolicy`] applies.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<AnyFocusTraversalPolicy> {
        let node = Focus::maybe_of(app, context, true, false)?;
        FocusTraversalGroup::maybe_of_node(app, node)
    }
}

impl fmt::Debug for FocusTraversalGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FocusTraversalGroup")
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for FocusTraversalGroup {
    type State = FocusTraversalGroupState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> FocusTraversalGroupState {
        FocusTraversalGroupState {
            state: StateData::new(),
            focus_node: None,
        }
    }
}

/// Dart's `_FocusTraversalGroupState`.
pub struct FocusTraversalGroupState {
    state: StateData<FocusTraversalGroup>,
    focus_node: Option<Handle<FocusTraversalGroupNode>>,
}

impl FocusTraversalGroupState {
    /// The internal focus node used to collect the children of this node into a group, and to
    /// provide a context for the traversal algorithm to sort the group with. It's a special
    /// focus node just so that it can be identified when walking the focus tree during
    /// traversal, and hold the current policy.
    pub fn focus_node(self: Handle<Self>, app: &App) -> Handle<FocusTraversalGroupNode> {
        app.get(self).focus_node.expect("created in init_state")
    }

    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        let focus_node = self.focus_node(app);
        let primary_focus = FocusManager::instance(app).primary_focus(app);
        let Some(last_requested_focus) = app.get(focus_node).last_requested_focus else {
            return;
        };

        if primary_focus != Some(last_requested_focus) {
            let policy = app.get(focus_node).policy;
            let mut scope = primary_focus.and_then(|node| node.nearest_scope(app));
            while let Some(current) = scope {
                policy.invalidate_scope_data(app, current);
                scope = current.enclosing_scope(app);
            }
            app.get_mut(focus_node).last_requested_focus = None;
        }
    }
}

impl State for FocusTraversalGroupState {
    type Widget = FocusTraversalGroup;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let policy = match self.widget(app).policy {
            Some(policy) => policy,
            None => ReadingOrderTraversalPolicy::new(app).as_policy(),
        };
        let focus_node = FocusTraversalGroupNode::new(app, policy);
        focus_node.set_debug_label(app, Some("FocusTraversalGroup".to_string()));
        app.get_mut(self).focus_node = Some(focus_node);
        let listener =
            Listener::handle_method(self, FocusTraversalGroupState::handle_focus_changed);
        FocusManager::instance(app).add_listener(app, listener);
        if let Some(on_focus_node_created) = self.widget(app).on_focus_node_created.clone() {
            on_focus_node_created(app, focus_node.as_node());
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let listener =
            Listener::handle_method(self, FocusTraversalGroupState::handle_focus_changed);
        FocusManager::instance(app).remove_listener(app, &listener);
        let focus_node = self.focus_node(app);
        focus_node.dispose(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &FocusTraversalGroup) {
        if old_widget.policy != self.widget(app).policy
            && let Some(policy) = self.widget(app).policy
        {
            let focus_node = self.focus_node(app);
            app.get_mut(focus_node).policy = policy;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let focus_node = self.focus_node(app);
        let widget = self.widget(app);
        let (parent_node, descendants_are_focusable, descendants_are_traversable, child) = (
            widget.parent_node,
            widget.descendants_are_focusable,
            widget.descendants_are_traversable,
            widget.child.clone(),
        );
        let mut focus = Focus::new(child)
            .focus_node(focus_node.as_node())
            .can_request_focus(false)
            .skip_traversal(true)
            .include_semantics(false)
            .descendants_are_focusable(descendants_are_focusable)
            .descendants_are_traversable(descendants_are_traversable);
        if let Some(parent_node) = parent_node {
            focus = focus.parent_node(parent_node);
        }
        focus.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// The intents and actions

/// An intent for use with the [`RequestFocusAction`], which supplies the `FocusNode` that should
/// be focused.
pub struct RequestFocusIntent {
    /// The callback used to move the focus to the node [`focus_node`](Self::focus_node). By
    /// default it requests focus on the node.
    pub request_focus_callback: TraversalRequestFocusCallback,

    /// The `FocusNode` that is to be focused.
    pub focus_node: AnyFocusNode,
}

impl RequestFocusIntent {
    /// Creates an intent used with [`RequestFocusAction`].
    pub fn new(focus_node: AnyFocusNode) -> RequestFocusIntent {
        RequestFocusIntent {
            request_focus_callback: Rc::new(default_traversal_request_focus_callback),
            focus_node,
        }
    }

    /// Dart `RequestFocusIntent(requestFocusCallback:)`.
    pub fn request_focus_callback(
        mut self,
        request_focus_callback: TraversalRequestFocusCallback,
    ) -> RequestFocusIntent {
        self.request_focus_callback = request_focus_callback;
        self
    }
}

impl fmt::Debug for RequestFocusIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestFocusIntent")
            .field("focusNode", &self.focus_node)
            .finish_non_exhaustive()
    }
}

impl Intent for RequestFocusIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that requests the focus on the node it is given in its [`RequestFocusIntent`].
///
/// This action can be used to request focus for a particular node, by calling
/// `Actions::invoke(app, context, &RequestFocusIntent::new(focus_node))`.
///
/// The difference between requesting focus in this way versus calling
/// [`AnyFocusNode::request_focus`] directly is that it will use the [`Action`] registered in the
/// nearest `Actions` widget associated with [`RequestFocusIntent`] to make the request, rather
/// than just requesting focus directly. This allows the action to have additional side effects,
/// like logging, or undo and redo functionality.
pub struct RequestFocusAction {
    action: ActionData,
}

impl RequestFocusAction {
    /// Creates a [`RequestFocusAction`].
    pub fn new(app: &mut App) -> Handle<RequestFocusAction> {
        app.create(RequestFocusAction {
            action: ActionData::new(),
        })
    }
}

impl Action for RequestFocusAction {
    type Intent = RequestFocusIntent;
    crate::action_accessors!();

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &RequestFocusIntent,
    ) -> Option<Rc<dyn Any>> {
        let callback = Rc::clone(&intent.request_focus_callback);
        callback(app, intent.focus_node, None, None, None, None);
        None
    }
}

/// An [`Intent`] bound to [`NextFocusAction`], which moves the focus to the next focusable node
/// in the focus traversal order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NextFocusIntent;

impl NextFocusIntent {
    /// Creates an intent that is used with [`NextFocusAction`].
    pub const fn new() -> NextFocusIntent {
        NextFocusIntent
    }
}

impl Intent for NextFocusIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that moves the focus to the next focusable node in the focus order.
///
/// This action is the default action registered for the [`NextFocusIntent`], and by default is
/// bound to the tab key in `WidgetsApp`.
pub struct NextFocusAction {
    action: ActionData,
}

impl NextFocusAction {
    /// Creates a [`NextFocusAction`].
    pub fn new(app: &mut App) -> Handle<NextFocusAction> {
        app.create(NextFocusAction {
            action: ActionData::new(),
        })
    }
}

impl Action for NextFocusAction {
    type Intent = NextFocusIntent;
    crate::action_accessors!();

    /// Attempts to pass the focus to the next widget.
    ///
    /// Returns true if a widget was focused as a result of invoking this action.
    ///
    /// Returns false when the traversal reached the end and the engine must pass focus to
    /// platform UI.
    fn invoke(self: Handle<Self>, app: &mut App, _intent: &NextFocusIntent) -> Option<Rc<dyn Any>> {
        let focused = primary_focus(app)
            .expect("an action is invoked with a primary focus")
            .next_focus(app);
        Some(Rc::new(focused))
    }

    fn to_key_event_result(
        self: Handle<Self>,
        _app: &mut App,
        _intent: &NextFocusIntent,
        invoke_result: Option<&Rc<dyn Any>>,
    ) -> KeyEventResult {
        if invoke_result_is_true(invoke_result) {
            KeyEventResult::Handled
        } else {
            KeyEventResult::SkipRemainingHandlers
        }
    }
}

/// Dart's `bool invokeResult` parameter, which the erased result carries here.
fn invoke_result_is_true(invoke_result: Option<&Rc<dyn Any>>) -> bool {
    invoke_result
        .and_then(|result| result.downcast_ref::<bool>())
        .copied()
        .unwrap_or(false)
}

/// An [`Intent`] bound to [`PreviousFocusAction`], which moves the focus to the previous
/// focusable node in the focus traversal order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreviousFocusIntent;

impl PreviousFocusIntent {
    /// Creates an intent that is used with [`PreviousFocusAction`].
    pub const fn new() -> PreviousFocusIntent {
        PreviousFocusIntent
    }
}

impl Intent for PreviousFocusIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that moves the focus to the previous focusable node in the focus order.
///
/// This action is the default action registered for the [`PreviousFocusIntent`], and by default
/// is bound to a combination of the tab key and the shift key in `WidgetsApp`.
pub struct PreviousFocusAction {
    action: ActionData,
}

impl PreviousFocusAction {
    /// Creates a [`PreviousFocusAction`].
    pub fn new(app: &mut App) -> Handle<PreviousFocusAction> {
        app.create(PreviousFocusAction {
            action: ActionData::new(),
        })
    }
}

impl Action for PreviousFocusAction {
    type Intent = PreviousFocusIntent;
    crate::action_accessors!();

    /// Attempts to pass the focus to the previous widget.
    ///
    /// Returns true if a widget was focused as a result of invoking this action.
    ///
    /// Returns false when the traversal reached the beginning and the engine must pass focus to
    /// platform UI.
    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        _intent: &PreviousFocusIntent,
    ) -> Option<Rc<dyn Any>> {
        let focused = primary_focus(app)
            .expect("an action is invoked with a primary focus")
            .previous_focus(app);
        Some(Rc::new(focused))
    }

    fn to_key_event_result(
        self: Handle<Self>,
        _app: &mut App,
        _intent: &PreviousFocusIntent,
        invoke_result: Option<&Rc<dyn Any>>,
    ) -> KeyEventResult {
        if invoke_result_is_true(invoke_result) {
            KeyEventResult::Handled
        } else {
            KeyEventResult::SkipRemainingHandlers
        }
    }
}

/// An [`Intent`] that represents moving to the next focusable node in the given
/// [`direction`](Self::direction).
///
/// This is the [`Intent`] bound by default to the arrow keys in `WidgetsApp`, with the
/// appropriate associated directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectionalFocusIntent {
    /// The direction in which to look for the next focusable node when the associated
    /// [`DirectionalFocusAction`] is invoked.
    pub direction: TraversalDirection,

    /// If true, then directional focus actions that occur within a text field will not happen
    /// when the focus node which received the key is a text field.
    ///
    /// Defaults to true.
    pub ignore_text_fields: bool,
}

impl DirectionalFocusIntent {
    /// Creates an intent used to move the focus in the given `direction`.
    pub fn new(direction: TraversalDirection) -> DirectionalFocusIntent {
        DirectionalFocusIntent {
            direction,
            ignore_text_fields: true,
        }
    }

    /// Dart `DirectionalFocusIntent(ignoreTextFields:)`.
    pub fn ignore_text_fields(mut self, ignore_text_fields: bool) -> DirectionalFocusIntent {
        self.ignore_text_fields = ignore_text_fields;
        self
    }
}

impl Intent for DirectionalFocusIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that moves the focus to the focusable node in the direction configured by the
/// associated [`DirectionalFocusIntent::direction`].
///
/// This is the [`Action`] associated with [`DirectionalFocusIntent`] and bound by default to the
/// arrow keys in `WidgetsApp`, with the appropriate associated directions.
pub struct DirectionalFocusAction {
    action: ActionData,
    /// Whether this action is defined in a text field.
    is_for_text_field: bool,
}

impl DirectionalFocusAction {
    /// Creates a [`DirectionalFocusAction`].
    pub fn new(app: &mut App) -> Handle<DirectionalFocusAction> {
        app.create(DirectionalFocusAction {
            action: ActionData::new(),
            is_for_text_field: false,
        })
    }

    /// Creates a [`DirectionalFocusAction`] that ignores [`DirectionalFocusIntent`]s whose
    /// `ignore_text_fields` field is true.
    pub fn for_text_field(app: &mut App) -> Handle<DirectionalFocusAction> {
        app.create(DirectionalFocusAction {
            action: ActionData::new(),
            is_for_text_field: true,
        })
    }
}

impl Action for DirectionalFocusAction {
    type Intent = DirectionalFocusIntent;
    crate::action_accessors!();

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &DirectionalFocusIntent,
    ) -> Option<Rc<dyn Any>> {
        if !intent.ignore_text_fields || !app.get(self).is_for_text_field {
            primary_focus(app)
                .expect("an action is invoked with a primary focus")
                .focus_in_direction(app, intent.direction);
        }
        None
    }
}

// ---------------------------------------------------------------------------------------------
// ExcludeFocusTraversal

/// A widget that controls whether or not the descendants of this widget are traversable.
///
/// Does not affect the value of [`AnyFocusNode::skip_traversal`] of the descendants.
///
/// See also:
///
///  * `Focus`, a widget for adding and managing a `FocusNode` in the widget tree.
///  * `ExcludeFocus`, a widget that excludes its descendants from focusability.
///  * [`FocusTraversalGroup`], a widget that groups widgets for focus traversal, and can also be
///    used in the same way as this widget by setting its `descendants_are_focusable` attribute.
#[derive(Debug)]
pub struct ExcludeFocusTraversal {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// If true, will make this widget's descendants untraversable.
    ///
    /// Defaults to true.
    ///
    /// Does not affect the value of [`AnyFocusNode::skip_traversal`] on the descendants.
    pub excluding: bool,

    /// The child widget of this [`ExcludeFocusTraversal`].
    pub child: WidgetRef,
}

impl ExcludeFocusTraversal {
    /// Creates an [`ExcludeFocusTraversal`] widget.
    pub fn new<K>(child: impl IntoWidget<K>) -> ExcludeFocusTraversal {
        ExcludeFocusTraversal {
            key: None,
            excluding: true,
            child: child.into_widget(),
        }
    }

    /// Dart `ExcludeFocusTraversal(key:)`.
    pub fn key(mut self, key: KeyRef) -> ExcludeFocusTraversal {
        self.key = Some(key);
        self
    }

    /// Dart `ExcludeFocusTraversal(excluding:)`.
    pub fn excluding(mut self, excluding: bool) -> ExcludeFocusTraversal {
        self.excluding = excluding;
        self
    }
}

impl StatelessWidget for ExcludeFocusTraversal {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Focus::new(self.child.clone())
            .can_request_focus(false)
            .skip_traversal(true)
            .include_semantics(false)
            .descendants_are_traversable(!self.excluding)
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::{Rect, TextDirection};

    use super::*;
    use crate::widgets::basic::{Directionality, Positioned, SizedBox, Stack};
    use crate::widgets::focus_manager::tests::{app_with_view, mount};

    /// A `Focus` at a fixed place in a `Stack`, labelled so the traversal order can be read.
    fn placed(label: &'static str, rect: Rect) -> WidgetRef {
        Positioned::from_rect(rect, Focus::new(SizedBox::expand()).debug_label(label)).into_widget()
    }

    /// Mounts the given children in a `Stack` under a [`FocusTraversalGroup`] with `policy`.
    fn mount_group(app: &mut App, policy: AnyFocusTraversalPolicy, children: Vec<WidgetRef>) {
        mount(
            app,
            Directionality::new(
                TextDirection::Ltr,
                FocusTraversalGroup::new(Stack::new().children(children)).policy(policy),
            )
            .into_widget(),
        );
    }

    fn node(app: &mut App, label: &str) -> AnyFocusNode {
        let root = FocusManager::instance(app).root_scope(app).as_node();
        root.descendants(app)
            .into_iter()
            .find(|node| node.debug_label(app).as_deref() == Some(label))
            .expect("a node with that label")
    }

    fn label(app: &App, node: AnyFocusNode) -> String {
        node.debug_label(app)
            .unwrap_or_else(|| "<unlabelled>".to_string())
    }

    /// Focuses `start`, then moves the focus `steps` times, collecting the label of the primary
    /// focus after each step.
    fn order(app: &mut App, start: &str, steps: usize, forward: bool) -> Vec<String> {
        let start = node(app, start);
        start.request_focus(app, None);
        app.drain_microtasks();
        let mut seen = Vec::new();
        for _ in 0..steps {
            let focus = primary_focus(app).expect("a primary focus");
            if forward {
                focus.next_focus(app);
            } else {
                focus.previous_focus(app);
            }
            app.drain_microtasks();
            let focus = primary_focus(app).expect("a primary focus");
            seen.push(label(app, focus));
        }
        seen
    }

    /// The three nodes: "x" to the right of "y", and "z" below both, in an order that is neither
    /// the widget order nor the reading order of the others.
    fn scattered() -> Vec<WidgetRef> {
        vec![
            placed("x", Rect::from_ltwh(100.0, 0.0, 50.0, 20.0)),
            placed("y", Rect::from_ltwh(0.0, 0.0, 50.0, 20.0)),
            placed("z", Rect::from_ltwh(0.0, 100.0, 50.0, 20.0)),
        ]
    }

    #[test]
    fn widget_order_traverses_in_widget_creation_order() {
        let mut app = app_with_view();
        let policy = WidgetOrderTraversalPolicy::new(&mut app).as_policy();
        mount_group(&mut app, policy, scattered());

        assert_eq!(order(&mut app, "x", 3, true), ["y", "z", "x"]);
        assert_eq!(order(&mut app, "x", 3, false), ["z", "y", "x"]);
    }

    #[test]
    fn reading_order_traverses_top_to_bottom_in_the_reading_direction() {
        let mut app = app_with_view();
        let policy = ReadingOrderTraversalPolicy::new(&mut app).as_policy();
        mount_group(&mut app, policy, scattered());

        // "y" and "x" share the topmost band, left to right; "z" is below them.
        assert_eq!(order(&mut app, "y", 3, true), ["x", "z", "y"]);
        assert_eq!(order(&mut app, "y", 3, false), ["z", "x", "y"]);
    }

    #[test]
    fn reading_order_follows_the_directionality_in_force() {
        let mut app = app_with_view();
        let policy = ReadingOrderTraversalPolicy::new(&mut app).as_policy();
        mount(
            &mut app,
            Directionality::new(
                TextDirection::Rtl,
                FocusTraversalGroup::new(Stack::new().children(scattered())).policy(policy),
            )
            .into_widget(),
        );

        // Right to left: "x" comes before "y" in the topmost band.
        assert_eq!(order(&mut app, "x", 3, true), ["y", "z", "x"]);
    }

    #[test]
    fn ordered_traversal_follows_the_numeric_order() {
        let mut app = app_with_view();
        let policy = OrderedTraversalPolicy::new(&mut app).as_policy();
        let ordered = |label: &'static str, rect: Rect, order: f64| -> WidgetRef {
            FocusTraversalOrder::new(
                Rc::new(NumericFocusOrder::new(order)),
                Positioned::from_rect(rect, Focus::new(SizedBox::expand()).debug_label(label)),
            )
            .into_widget()
        };
        mount_group(
            &mut app,
            policy,
            vec![
                ordered("x", Rect::from_ltwh(100.0, 0.0, 50.0, 20.0), 3.0),
                ordered("y", Rect::from_ltwh(0.0, 0.0, 50.0, 20.0), 1.0),
                ordered("z", Rect::from_ltwh(0.0, 100.0, 50.0, 20.0), 2.0),
            ],
        );

        assert_eq!(order(&mut app, "y", 3, true), ["z", "x", "y"]);
    }

    #[test]
    fn ordered_traversal_follows_the_lexical_order() {
        let mut app = app_with_view();
        let policy = OrderedTraversalPolicy::new(&mut app).as_policy();
        let ordered = |label: &'static str, rect: Rect, order: &'static str| -> WidgetRef {
            FocusTraversalOrder::new(
                Rc::new(LexicalFocusOrder::new(order)),
                Positioned::from_rect(rect, Focus::new(SizedBox::expand()).debug_label(label)),
            )
            .into_widget()
        };
        mount_group(
            &mut app,
            policy,
            vec![
                ordered("x", Rect::from_ltwh(100.0, 0.0, 50.0, 20.0), "c"),
                ordered("y", Rect::from_ltwh(0.0, 0.0, 50.0, 20.0), "a"),
                ordered("z", Rect::from_ltwh(0.0, 100.0, 50.0, 20.0), "b"),
            ],
        );

        assert_eq!(order(&mut app, "y", 3, true), ["z", "x", "y"]);
    }

    #[test]
    fn the_traversal_edge_behavior_decides_what_happens_at_the_last_node() {
        let mut app = app_with_view();
        let policy = WidgetOrderTraversalPolicy::new(&mut app).as_policy();
        mount_group(&mut app, policy, scattered());

        // The default is a closed loop: the last node wraps around to the first.
        assert_eq!(order(&mut app, "z", 1, true), ["x"]);

        let scope = node(&mut app, "z")
            .nearest_scope(&mut app)
            .expect("a node in the tree has a nearest scope");
        scope.set_traversal_edge_behavior(&mut app, TraversalEdgeBehavior::Stop);
        assert_eq!(order(&mut app, "z", 1, true), ["z"]);
        assert_eq!(order(&mut app, "x", 1, false), ["x"]);

        scope.set_traversal_edge_behavior(&mut app, TraversalEdgeBehavior::LeaveFlutterView);
        let last = node(&mut app, "z");
        last.request_focus(&mut app, None);
        app.drain_microtasks();
        assert!(!last.next_focus(&mut app));
        app.drain_microtasks();
        assert_eq!(primary_focus(&mut app), Some(scope.as_node()));
    }

    #[test]
    fn directional_focus_moves_to_the_neighbour_in_that_direction() {
        let mut app = app_with_view();
        let policy = ReadingOrderTraversalPolicy::new(&mut app).as_policy();
        mount_group(&mut app, policy, scattered());

        let y = node(&mut app, "y");
        y.request_focus(&mut app, None);
        app.drain_microtasks();

        assert!(y.focus_in_direction(&mut app, TraversalDirection::Right));
        app.drain_microtasks();
        let focus = primary_focus(&mut app).expect("a primary focus");
        assert_eq!(label(&app, focus), "x");

        let x = node(&mut app, "x");
        assert!(x.focus_in_direction(&mut app, TraversalDirection::Left));
        app.drain_microtasks();
        let focus = primary_focus(&mut app).expect("a primary focus");
        assert_eq!(label(&app, focus), "y");

        let y = node(&mut app, "y");
        assert!(y.focus_in_direction(&mut app, TraversalDirection::Down));
        app.drain_microtasks();
        let focus = primary_focus(&mut app).expect("a primary focus");
        assert_eq!(label(&app, focus), "z");
    }

    #[test]
    fn a_shortcut_bound_to_next_focus_moves_the_focus() {
        use std::any::TypeId;
        use std::collections::HashMap;
        use std::time::Duration;

        use reveal_services::{
            HardwareKeyboard, KeyDownEvent, KeyEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
        };

        use crate::widgets::actions::Actions;
        use crate::widgets::shortcuts::{ShortcutMap, Shortcuts, SingleActivator};

        let mut app = app_with_view();
        let policy = WidgetOrderTraversalPolicy::new(&mut app).as_policy();
        let action = Action::as_action(NextFocusAction::new(&mut app));
        let shortcuts: ShortcutMap = vec![(
            Rc::new(SingleActivator::new(LogicalKeyboardKey::TAB)),
            Rc::new(NextFocusIntent::new()),
        )];
        mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                Shortcuts::new(
                    shortcuts,
                    Actions::new(
                        HashMap::from([(TypeId::of::<NextFocusIntent>(), action)]),
                        FocusTraversalGroup::new(Stack::new().children(scattered())).policy(policy),
                    ),
                ),
            )
            .into_widget(),
        );

        node(&mut app, "x").request_focus(&mut app, None);
        app.drain_microtasks();

        let tab = KeyEvent::Down(KeyDownEvent::new(
            PhysicalKeyboardKey::TAB,
            LogicalKeyboardKey::TAB,
            Duration::ZERO,
        ));
        let keyboard = HardwareKeyboard::instance(&mut app);
        assert!(keyboard.handle_key_event(&mut app, &tab));
        app.drain_microtasks();
        let focus = primary_focus(&mut app).expect("a primary focus");
        assert_eq!(label(&app, focus), "y");
    }

    #[test]
    fn exclude_focus_traversal_keeps_its_descendants_out_of_the_order() {
        let mut app = app_with_view();
        let policy = WidgetOrderTraversalPolicy::new(&mut app).as_policy();
        mount_group(
            &mut app,
            policy,
            vec![
                placed("x", Rect::from_ltwh(100.0, 0.0, 50.0, 20.0)),
                Positioned::from_rect(
                    Rect::from_ltwh(0.0, 0.0, 50.0, 20.0),
                    ExcludeFocusTraversal::new(Focus::new(SizedBox::expand()).debug_label("y")),
                )
                .into_widget(),
                placed("z", Rect::from_ltwh(0.0, 100.0, 50.0, 20.0)),
            ],
        );

        assert_eq!(order(&mut app, "x", 2, true), ["z", "x"]);
    }
}
