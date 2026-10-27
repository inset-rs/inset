//! ViewModel.cpp and TreeViewNode.cpp, with the node operations of TreeViewList.cpp and
//! TreeViewItem.cpp: the tree the application passes, flattened into the rows the list shows,
//! the selection rules of Multiple mode, and the moves a drop or a keyboard reorder makes.

use super::{TreeViewNode, TreeViewNodeMove};
use inset_widgets::WidgetRef;
use std::collections::{HashMap, HashSet};

/// `TreeNodeSelectionState`: a node's state in Multiple mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NodeSelectionState {
    UnSelected,
    PartialSelected,
    Selected,
}

/// One node as the view model reads it: `Parent`, `Children`, `IsExpanded` and
/// `HasUnrealizedChildren` of a `TreeViewNode`, with its content.
#[derive(Clone)]
pub(super) struct NodeData {
    /// `Parent`; `None` for a node of `RootNodes`, whose parent is the tree's hidden root.
    pub(super) parent: Option<String>,
    /// `Children`, by id.
    pub(super) children: Vec<String>,
    /// `IsExpanded`.
    pub(super) is_expanded: bool,
    /// `HasUnrealizedChildren`.
    pub(super) has_unrealized_children: bool,
    /// `Content`.
    pub(super) content: WidgetRef,
}

/// The nodes under TreeView's hidden root node, by id.
#[derive(Clone)]
pub(super) struct NodeTree {
    /// `RootNodes`, by id.
    roots: Vec<String>,
    /// Every node, by id.
    nodes: HashMap<String, NodeData>,
}

impl NodeTree {
    /// Indexes `roots` and their descendants. Ids must be unique in the whole tree.
    pub(super) fn new(roots: &[TreeViewNode]) -> NodeTree {
        let mut tree = NodeTree {
            roots: roots.iter().map(|node| node.id.clone()).collect(),
            nodes: HashMap::new(),
        };
        let mut pending: Vec<(Option<String>, &TreeViewNode)> =
            roots.iter().map(|node| (None, node)).collect();
        while let Some((parent, node)) = pending.pop() {
            let data = NodeData {
                parent,
                children: node.children.iter().map(|child| child.id.clone()).collect(),
                is_expanded: node.is_expanded,
                has_unrealized_children: node.has_unrealized_children,
                content: node.content.clone(),
            };
            let previous = tree.nodes.insert(node.id.clone(), data);
            assert!(
                previous.is_none(),
                "TreeView node ids must be unique: {}",
                node.id
            );
            pending.extend(
                node.children
                    .iter()
                    .map(|child| (Some(node.id.clone()), child)),
            );
        }
        tree
    }

    /// Whether the tree has a node `id`.
    pub(super) fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// The node `id`, which the tree must have.
    pub(super) fn node(&self, id: &str) -> &NodeData {
        &self.nodes[id]
    }

    /// The children of `parent`, or `RootNodes` for `None`.
    pub(super) fn children(&self, parent: Option<&str>) -> &[String] {
        match parent {
            Some(parent) => &self.nodes[parent].children,
            None => &self.roots,
        }
    }

    /// `Parent`, `None` for a node of `RootNodes`.
    pub(super) fn parent(&self, id: &str) -> Option<&str> {
        self.nodes[id].parent.as_deref()
    }

    /// `Depth`: 0 for a node of `RootNodes`.
    pub(super) fn depth(&self, id: &str) -> usize {
        let mut depth = 0;
        let mut current = self.parent(id);
        while let Some(parent) = current {
            depth += 1;
            current = self.parent(parent);
        }
        depth
    }

    /// `HasChildren`: children, realized or not.
    pub(super) fn has_children(&self, id: &str) -> bool {
        let node = &self.nodes[id];
        !node.children.is_empty() || node.has_unrealized_children
    }

    /// `IsExpanded`.
    pub(super) fn is_expanded(&self, id: &str) -> bool {
        self.nodes[id].is_expanded
    }

    /// Sets `IsExpanded` of a node the tree has.
    pub(super) fn set_expanded(&mut self, id: &str, expanded: bool) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.is_expanded = expanded;
        }
    }

    /// The node's index among its parent's children.
    pub(super) fn index_in_parent(&self, id: &str) -> usize {
        self.children(self.parent(id))
            .iter()
            .position(|child| child == id)
            .expect("a node is among its parent's children")
    }

    /// Whether `id` is `ancestor` or below it.
    pub(super) fn is_within(&self, id: &str, ancestor: &str) -> bool {
        let mut current = Some(id);
        while let Some(node) = current {
            if node == ancestor {
                return true;
            }
            current = self.parent(node);
        }
        false
    }

    /// Every node in `id`'s subtree, `id` first, in depth-first order.
    #[cfg(test)]
    fn subtree(&self, id: &str) -> Vec<String> {
        let mut nodes = Vec::new();
        let mut pending = vec![id.to_owned()];
        while let Some(node) = pending.pop() {
            pending.extend(self.nodes[&node].children.iter().rev().cloned());
            nodes.push(node);
        }
        nodes
    }

    /// The view model's flat list: depth first, every node of an expanded node right after it,
    /// as `AddNodeToView` and `AddNodeDescendantsToView` build it.
    pub(super) fn rows(&self) -> Rows {
        let mut ids = Vec::new();
        let mut pending: Vec<&String> = self.roots.iter().rev().collect();
        while let Some(id) = pending.pop() {
            let node = &self.nodes[id];
            if node.is_expanded {
                pending.extend(node.children.iter().rev());
            }
            ids.push(id.clone());
        }
        Rows::new(ids)
    }

    /// `RemoveNodeFromParent`, with `TreeViewNodeVector::RemoveAt` collapsing a parent left
    /// without children: returns the parent it collapsed.
    fn remove(&mut self, id: &str) -> Option<String> {
        let parent = self.nodes[id].parent.clone();
        let siblings = match &parent {
            Some(parent) => &mut self.nodes.get_mut(parent).unwrap().children,
            None => &mut self.roots,
        };
        siblings.retain(|child| child != id);
        let emptied = siblings.is_empty();
        self.nodes.get_mut(id).unwrap().parent = None;
        let parent = parent?;
        let node = self.nodes.get_mut(&parent).unwrap();
        (emptied && std::mem::take(&mut node.is_expanded)).then_some(parent)
    }

    /// `TreeViewNodeVector::InsertAt`.
    fn insert(&mut self, parent: Option<&str>, index: usize, id: &str) {
        let siblings = match parent {
            Some(parent) => &mut self.nodes.get_mut(parent).unwrap().children,
            None => &mut self.roots,
        };
        siblings.insert(index, id.to_owned());
        self.nodes.get_mut(id).unwrap().parent = parent.map(str::to_owned);
    }

    /// Moves `id` under `parent` at `index`, counted after its removal: returns the parent the
    /// removal collapses.
    pub(super) fn move_node(
        &mut self,
        id: &str,
        parent: Option<&str>,
        index: usize,
    ) -> Option<String> {
        let collapsed = self.remove(id);
        self.insert(parent, index, id);
        collapsed
    }

    /// `move_node`, recording the move and the parent the removal collapses in `plan`.
    fn apply(&mut self, plan: &mut MovePlan, id: &str, parent: Option<&str>, index: usize) {
        plan.collapsed.extend(self.move_node(id, parent, index));
        plan.moves.push(TreeViewNodeMove {
            node: id.to_owned(),
            new_parent: parent.map(str::to_owned),
            index,
        });
    }

    /// `TreeViewList::OnDrop` between rows: `MoveNodeInto` for each of `nodes`, the last first,
    /// with `insert_at` the node at `empty_slot` of the rows when the drop began. A node not
    /// shown in the rows stays where it is.
    pub(super) fn plan_drop_between(
        &self,
        nodes: &[String],
        insert_at: &str,
        empty_slot: usize,
    ) -> MovePlan {
        let mut tree = self.clone();
        let mut plan = MovePlan::default();
        for node in nodes.iter().rev() {
            let Some(flat_index) = tree.rows().index_of(node) else {
                continue;
            };
            // A node dropped beside its own descendant would end up inside itself.
            if node == insert_at || tree.is_within(insert_at, node) {
                continue;
            }
            plan.collapsed.extend(tree.remove(node));
            let offset = usize::from(flat_index < empty_slot);
            // Below an expanded node, the node becomes its first child.
            let (parent, index) = if tree.is_expanded(insert_at) && offset == 1 {
                (Some(insert_at.to_owned()), 0)
            } else {
                (
                    tree.parent(insert_at).map(str::to_owned),
                    tree.index_in_parent(insert_at) + offset,
                )
            };
            tree.insert(parent.as_deref(), index, node);
            plan.moves.push(TreeViewNodeMove {
                node: node.clone(),
                new_parent: parent,
                index,
            });
        }
        plan
    }

    /// `TreeViewItem::OnDrop` on `target`: each of `nodes` shown in the rows becomes the last
    /// child of `target`, in order.
    pub(super) fn plan_drop_into(&self, nodes: &[String], target: &str) -> MovePlan {
        let mut tree = self.clone();
        let mut plan = MovePlan::default();
        for node in nodes {
            if tree.rows().index_of(node).is_none() || tree.is_within(target, node) {
                continue;
            }
            // The node's removal can shorten the target's children only when it is one of them.
            let index =
                tree.children(Some(target)).len() - usize::from(tree.parent(node) == Some(target));
            tree.apply(&mut plan, node, Some(target), index);
        }
        plan
    }

    /// `TreeViewItem::HandleReorder`: the node trades places with its previous or next sibling,
    /// collapsing first; nothing at either end of its siblings.
    pub(super) fn plan_reorder(&self, node: &str, forward: bool) -> Option<MovePlan> {
        let index = self.index_in_parent(node);
        let count = self.children(self.parent(node)).len();
        let new_index = if forward {
            (index + 1 < count).then_some(index + 1)?
        } else {
            index.checked_sub(1)?
        };
        let mut tree = self.clone();
        let mut plan = MovePlan::default();
        if tree.is_expanded(node) {
            tree.set_expanded(node, false);
            plan.collapsed.push(node.to_owned());
        }
        let parent = tree.parent(node).map(str::to_owned);
        tree.apply(&mut plan, node, parent.as_deref(), new_index);
        Some(plan)
    }

    /// `TreeViewList::GetRootsOfSelectedSubtrees`: for each selected node in order, the highest
    /// selected node above it with only selected nodes between, each once.
    pub(super) fn roots_of_selected_subtrees(&self, selected: &[String]) -> Vec<String> {
        let selected_set: HashSet<&str> = selected.iter().map(String::as_str).collect();
        let mut roots: Vec<String> = Vec::new();
        for node in selected.iter().filter(|node| self.contains(node)) {
            let mut root = node.as_str();
            while let Some(parent) = self.parent(root)
                && selected_set.contains(parent)
            {
                root = parent;
            }
            if !roots.iter().any(|existing| existing == root) {
                roots.push(root.to_owned());
            }
        }
        roots
    }
}

/// The view model's flat list of the rows the list shows, by id.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Rows {
    /// The rows in order.
    ids: Vec<String>,
    /// Each row's index.
    index: HashMap<String, usize>,
}

impl Rows {
    fn new(ids: Vec<String>) -> Rows {
        let index = ids
            .iter()
            .enumerate()
            .map(|(row, id)| (id.clone(), row))
            .collect();
        Rows { ids, index }
    }

    /// `Size`.
    pub(super) fn len(&self) -> usize {
        self.ids.len()
    }

    /// `GetNodeAt`.
    pub(super) fn get(&self, row: usize) -> Option<&str> {
        self.ids.get(row).map(String::as_str)
    }

    /// `IndexOfNode`.
    pub(super) fn index_of(&self, id: &str) -> Option<usize> {
        self.index.get(id).copied()
    }

    /// The rows in order.
    pub(super) fn ids(&self) -> &[String] {
        &self.ids
    }
}

/// What a drop or a keyboard reorder asks of the application: the moves, in order, and the
/// nodes it collapses, a parent its last child leaves among them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct MovePlan {
    pub(super) moves: Vec<TreeViewNodeMove>,
    pub(super) collapsed: Vec<String>,
}

/// `SelectedNodes` and each node's `TreeNodeSelectionState` as ViewModel keeps them, with what
/// was added to and removed from `SelectedNodes` since the change began, as
/// `BeginSelectionChanges` and `EndSelectionChanges` track them.
pub(super) struct SelectionChange<'a> {
    tree: &'a NodeTree,
    /// States other than `UnSelected`.
    states: HashMap<String, NodeSelectionState>,
    /// `SelectedNodes`.
    pub(super) selected: Vec<String>,
    /// Nodes that became selected, in order.
    pub(super) added: Vec<String>,
    /// Nodes that stopped being selected, in order.
    pub(super) removed: Vec<String>,
}

impl<'a> SelectionChange<'a> {
    /// Starts from `selected`, the nodes in `Selected` state; a node above one of them that is
    /// not among them is `PartialSelected`.
    pub(super) fn new(tree: &'a NodeTree, selected: &[String]) -> SelectionChange<'a> {
        SelectionChange {
            tree,
            states: selection_states(tree, selected),
            selected: selected.to_vec(),
            added: Vec::new(),
            removed: Vec::new(),
        }
    }

    /// The node's state.
    pub(super) fn state(&self, id: &str) -> NodeSelectionState {
        self.states
            .get(id)
            .copied()
            .unwrap_or(NodeSelectionState::UnSelected)
    }

    /// `ViewModel::UpdateSelection` in Multiple mode: the node takes `state`, its descendants
    /// follow it, and its ancestors take the state their children give them.
    pub(super) fn update_selection(&mut self, id: &str, state: NodeSelectionState) {
        if self.state(id) != state {
            self.update_node_selection(id, state);
            self.update_descendants(id, state);
            self.update_ancestors(id);
        }
    }

    /// `UpdateNodeSelection`: a node entering `Selected` is appended to `SelectedNodes`, one
    /// leaving it is removed.
    fn update_node_selection(&mut self, id: &str, state: NodeSelectionState) {
        if self.state(id) == state {
            return;
        }
        match state {
            NodeSelectionState::UnSelected => {
                self.states.remove(id);
            }
            _ => {
                self.states.insert(id.to_owned(), state);
            }
        }
        if state == NodeSelectionState::Selected {
            self.selected.push(id.to_owned());
            self.added.push(id.to_owned());
        } else if let Some(index) = self.selected.iter().position(|node| node == id) {
            self.selected.remove(index);
            self.removed.push(id.to_owned());
        }
    }

    /// `UpdateSelectionStateOfDescendants`: every descendant, collapsed or not, takes a
    /// selected or unselected state; unselecting goes backwards.
    fn update_descendants(&mut self, id: &str, state: NodeSelectionState) {
        let children = self.tree.node(id).children.clone();
        match state {
            NodeSelectionState::PartialSelected => {}
            NodeSelectionState::UnSelected => {
                for child in children.iter().rev() {
                    self.update_descendants(child, state);
                    self.update_node_selection(child, state);
                }
            }
            NodeSelectionState::Selected => {
                for child in &children {
                    self.update_node_selection(child, state);
                    self.update_descendants(child, state);
                }
            }
        }
    }

    /// `UpdateSelectionStateOfAncestors`, up to the tree's hidden root.
    fn update_ancestors(&mut self, id: &str) {
        let Some(parent) = self.tree.parent(id).map(str::to_owned) else {
            return;
        };
        let state = self.state_based_on_children(&parent);
        if self.state(&parent) != state {
            self.update_node_selection(&parent, state);
            self.update_ancestors(&parent);
        }
    }

    /// `SelectionStateBasedOnChildren`.
    fn state_based_on_children(&self, id: &str) -> NodeSelectionState {
        let mut has_selected = false;
        let mut has_unselected = false;
        for child in &self.tree.node(id).children {
            match self.state(child) {
                NodeSelectionState::Selected => has_selected = true,
                NodeSelectionState::UnSelected => has_unselected = true,
                NodeSelectionState::PartialSelected => {
                    return NodeSelectionState::PartialSelected;
                }
            }
            if has_selected && has_unselected {
                return NodeSelectionState::PartialSelected;
            }
        }
        if has_selected {
            NodeSelectionState::Selected
        } else {
            NodeSelectionState::UnSelected
        }
    }
}

/// The states Multiple mode draws for `selected`: `Selected` for those of them in the tree, and
/// `PartialSelected` for the nodes above them not among them.
pub(super) fn selection_states(
    tree: &NodeTree,
    selected: &[String],
) -> HashMap<String, NodeSelectionState> {
    let mut states = HashMap::new();
    for id in selected.iter().filter(|id| tree.contains(id)) {
        states.insert(id.clone(), NodeSelectionState::Selected);
    }
    for id in selected.iter().filter(|id| tree.contains(id)) {
        let mut current = tree.parent(id);
        while let Some(parent) = current {
            states
                .entry(parent.to_owned())
                .or_insert(NodeSelectionState::PartialSelected);
            current = tree.parent(parent);
        }
    }
    states
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_widgets::SizedBox;

    fn node(id: &str, children: Vec<TreeViewNode>) -> TreeViewNode {
        TreeViewNode::new(id, SizedBox::shrink()).children(children)
    }

    fn expanded(id: &str, children: Vec<TreeViewNode>) -> TreeViewNode {
        node(id, children).is_expanded(true)
    }

    /// `Root` expanded with `Root.0`, `Root.1` (with `Root.1.0` and `Root.1.1`) and `Root.2`.
    fn tree(root_1_expanded: bool) -> NodeTree {
        NodeTree::new(&[expanded(
            "Root",
            vec![
                node("Root.0", vec![]),
                node(
                    "Root.1",
                    vec![node("Root.1.0", vec![]), node("Root.1.1", vec![])],
                )
                .is_expanded(root_1_expanded),
                node("Root.2", vec![]),
            ],
        )])
    }

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn moved(node: &str, parent: Option<&str>, index: usize) -> TreeViewNodeMove {
        TreeViewNodeMove {
            node: node.to_owned(),
            new_parent: parent.map(str::to_owned),
            index,
        }
    }

    #[test]
    fn the_rows_are_the_expanded_nodes_depth_first() {
        assert_eq!(
            tree(true).rows().ids(),
            ids(&["Root", "Root.0", "Root.1", "Root.1.0", "Root.1.1", "Root.2"])
        );
        assert_eq!(
            tree(false).rows().ids(),
            ids(&["Root", "Root.0", "Root.1", "Root.2"])
        );
        let tree = tree(true);
        assert_eq!(tree.depth("Root.1.1"), 2);
        assert!(tree.has_children("Root.1") && !tree.has_children("Root.2"));
    }

    #[test]
    fn unrealized_children_count_as_children() {
        let tree = NodeTree::new(&[node("Lazy", vec![]).has_unrealized_children(true)]);
        assert!(tree.has_children("Lazy"));
    }

    #[test]
    #[should_panic(expected = "unique")]
    fn ids_must_be_unique() {
        NodeTree::new(&[node("A", vec![node("A", vec![])])]);
    }

    #[test]
    fn selecting_a_node_selects_its_collapsed_descendants_and_completes_its_parent() {
        let tree = tree(false);
        let mut change = SelectionChange::new(&tree, &[]);
        change.update_selection("Root.1", NodeSelectionState::Selected);
        assert_eq!(change.selected, ids(&["Root.1", "Root.1.0", "Root.1.1"]));
        assert_eq!(change.state("Root"), NodeSelectionState::PartialSelected);
        change.update_selection("Root.0", NodeSelectionState::Selected);
        change.update_selection("Root.2", NodeSelectionState::Selected);
        // Every child selected: the parent is selected too, after them.
        assert_eq!(
            change.selected,
            ids(&["Root.1", "Root.1.0", "Root.1.1", "Root.0", "Root.2", "Root"])
        );
        assert_eq!(change.added, change.selected);
    }

    #[test]
    fn unselecting_a_child_makes_its_ancestors_partly_selected() {
        let tree = tree(true);
        let all = tree.subtree("Root");
        let mut change = SelectionChange::new(&tree, &all);
        change.update_selection("Root.1.0", NodeSelectionState::UnSelected);
        assert_eq!(change.state("Root.1"), NodeSelectionState::PartialSelected);
        assert_eq!(change.state("Root"), NodeSelectionState::PartialSelected);
        assert_eq!(change.removed, ids(&["Root.1.0", "Root.1", "Root"]));
        assert!(change.added.is_empty());
    }

    #[test]
    fn a_partly_selected_node_unchecked_unselects_its_subtree() {
        let tree = tree(true);
        let mut change = SelectionChange::new(&tree, &ids(&["Root.1.0"]));
        assert_eq!(change.state("Root.1"), NodeSelectionState::PartialSelected);
        change.update_selection("Root.1", NodeSelectionState::UnSelected);
        assert!(change.selected.is_empty());
        assert_eq!(change.removed, ids(&["Root.1.0"]));
        assert_eq!(change.state("Root"), NodeSelectionState::UnSelected);
    }

    #[test]
    fn a_drop_between_rows_moves_the_node_to_the_slot() {
        let tree = tree(false);
        // `Root.0` (row 1) dropped at slot 2: after `Root.1`.
        let plan = tree.plan_drop_between(&ids(&["Root.0"]), "Root.1", 2);
        assert_eq!(plan.moves, vec![moved("Root.0", Some("Root"), 1)]);
        // `Root.2` (row 3) dropped at slot 1: before `Root.0`.
        let plan = tree.plan_drop_between(&ids(&["Root.2"]), "Root.0", 1);
        assert_eq!(plan.moves, vec![moved("Root.2", Some("Root"), 0)]);
    }

    #[test]
    fn a_drop_below_an_expanded_node_makes_the_node_its_first_child() {
        let tree = tree(true);
        let plan = tree.plan_drop_between(&ids(&["Root.0"]), "Root.1", 2);
        assert_eq!(plan.moves, vec![moved("Root.0", Some("Root.1"), 0)]);
    }

    #[test]
    fn several_nodes_dropped_between_rows_keep_their_order() {
        let tree = tree(false);
        // `TreeViewMultiSelectDragAndDropBetweenNodes`: `Root.2`, `Root.0`, `Root.1`.
        let plan = tree.plan_drop_between(&ids(&["Root.0", "Root.1"]), "Root.2", 3);
        assert_eq!(
            plan.moves,
            vec![
                moved("Root.1", Some("Root"), 2),
                moved("Root.0", Some("Root"), 1),
            ]
        );
    }

    #[test]
    fn a_drop_into_a_node_appends_and_collapses_the_parent_it_empties() {
        let tree = NodeTree::new(&[
            expanded("A", vec![node("A.0", vec![])]),
            node("B", vec![node("B.0", vec![])]),
        ]);
        let plan = tree.plan_drop_into(&ids(&["A.0"]), "B");
        assert_eq!(plan.moves, vec![moved("A.0", Some("B"), 1)]);
        assert_eq!(plan.collapsed, ids(&["A"]));
    }

    #[test]
    fn a_hidden_node_and_a_node_onto_its_descendant_do_not_move() {
        let tree = tree(false);
        assert!(
            tree.plan_drop_into(&ids(&["Root.1.0"]), "Root.0")
                .moves
                .is_empty()
        );
        let tree = self::tree(true);
        assert!(
            tree.plan_drop_into(&ids(&["Root.1"]), "Root.1.0")
                .moves
                .is_empty()
        );
        assert!(
            tree.plan_drop_between(&ids(&["Root.1"]), "Root.1.1", 4)
                .moves
                .is_empty()
        );
    }

    #[test]
    fn a_keyboard_reorder_collapses_and_trades_places_with_a_sibling() {
        let tree = tree(true);
        let plan = tree.plan_reorder("Root.1", true).unwrap();
        assert_eq!(plan.moves, vec![moved("Root.1", Some("Root"), 2)]);
        assert_eq!(plan.collapsed, ids(&["Root.1"]));
        assert!(tree.plan_reorder("Root.0", false).is_none());
        assert!(tree.plan_reorder("Root.2", true).is_none());
    }

    #[test]
    fn the_roots_of_selected_subtrees_skip_selected_descendants() {
        let tree = tree(true);
        assert_eq!(
            tree.roots_of_selected_subtrees(&ids(&["Root.1.0", "Root.1", "Root.1.1", "Root.2"])),
            ids(&["Root.1", "Root.2"])
        );
    }
}
