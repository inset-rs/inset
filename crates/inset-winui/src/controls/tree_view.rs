//! TreeView.cpp, TreeView.xaml and TreeViewList.cpp, with the ListView behaviour TreeViewList
//! inherits from ListViewBase_Partial*.cpp and Selector_Partial.cpp: a tree of nodes the
//! application owns, flattened into the rows of a lazy list as ViewModel.cpp flattens it.
//!
//! The application owns the nodes, their `is_expanded`, and the selection. The tree reports what
//! it asks for: `expanding` and `collapsed` for a node it expands or collapses, `selection_changed`
//! for a selection, and `move_requested` for nodes a drop or a keyboard reorder moves. Until the
//! application next builds the tree, the tree shows what it asked for; then it shows what the
//! application passes.
//!
//! Selection follows the source per mode. In Single mode the selection is the list's own, as
//! TreeViewList takes it from ListView: a click or Enter selects, the arrow keys move it with the
//! focus, Ctrl with a click or Space toggles it, and a node that leaves the rows (an ancestor
//! collapses, or the node moves) is no longer selected. In Multiple mode each row has a check box,
//! and ViewModel's rules apply: a node takes its whole subtree with it, and its ancestors become
//! selected or partly selected from their children.

mod drag;
mod item;
mod view_model;

pub use drag::{
    DataPackageOperation, TreeViewDragItemsCompletedEventArgs, TreeViewDragItemsStartingEventArgs,
};

use crate::*;
// Inset exports a `FocusState` of its own; the XAML one is meant here.
use crate::FocusState;
use drag::TreeDragState;
use inset_embedder::{Offset, Rect, Size, TextDirection};
use inset_foundation::{App, Handle, ValueKey};
use inset_painting::EdgeInsetsGeometry;
use inset_scheduler::{FrameCallback, Ticker};
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use item::{ItemFocus, PressedPart, TreeViewItemContainer, estimated_row_height};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
};
use view_model::{MovePlan, NodeSelectionState, NodeTree, Rows, SelectionChange};

/// XAML `TreeViewSelectionMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TreeViewSelectionMode {
    /// Nothing can be selected.
    None,
    /// One node at a time, the list's selection.
    #[default]
    Single,
    /// Any nodes, through each row's check box.
    Multiple,
}

/// XAML `TreeViewNode`: one node of the tree the application passes.
#[derive(Clone)]
pub struct TreeViewNode {
    /// Identity across rebuilds and moves; unique within the whole tree.
    pub id: String,

    /// `Content`: what the node's row shows.
    pub content: WidgetRef,

    /// `Children`, in order.
    pub children: Vec<TreeViewNode>,

    /// `IsExpanded`: whether the node's children are rows of the list.
    pub is_expanded: bool,

    /// `HasUnrealizedChildren`: the node has children the application has not passed yet, which
    /// it loads in `expanding`. Such a node shows its chevron.
    pub has_unrealized_children: bool,
}

impl TreeViewNode {
    /// Creates a node showing `content`, without children.
    pub fn new<K>(id: impl Into<String>, content: impl IntoWidget<K>) -> Self {
        Self {
            id: id.into(),
            content: content.into_widget(),
            children: Vec::new(),
            is_expanded: false,
            has_unrealized_children: false,
        }
    }

    /// Creates a node that is `text`, as a node whose `Content` is a string shows it.
    pub fn text(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, Text::new(text).soft_wrap(false))
    }

    /// Sets Children.
    pub fn children(mut self, value: Vec<TreeViewNode>) -> Self {
        self.children = value;
        self
    }

    /// Sets IsExpanded.
    pub fn is_expanded(mut self, value: bool) -> Self {
        self.is_expanded = value;
        self
    }

    /// Sets HasUnrealizedChildren.
    pub fn has_unrealized_children(mut self, value: bool) -> Self {
        self.has_unrealized_children = value;
        self
    }
}

impl fmt::Debug for TreeViewNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeViewNode")
            .field("id", &self.id)
            .field("is_expanded", &self.is_expanded)
            .field("has_unrealized_children", &self.has_unrealized_children)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

/// XAML `TreeViewSelectionChangedEventArgs`, with the selection the tree asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewSelectionChangedEventArgs {
    /// `AddedItems`: the nodes that became selected, in order.
    pub added_items: Vec<String>,

    /// `RemovedItems`: the nodes that stopped being selected, in order.
    pub removed_items: Vec<String>,

    /// `SelectedNodes` after the change, which the application passes back in `selected_nodes`.
    pub selected_nodes: Vec<String>,
}

/// XAML `TreeViewItemInvokedEventArgs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewItemInvokedEventArgs {
    /// `InvokedItem`: the node clicked, or the focused node of Enter or Space.
    pub invoked_item: String,
}

/// XAML `TreeViewExpandingEventArgs`: the tree expanded a node, which the application stores in
/// its `is_expanded`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewExpandingEventArgs {
    /// `Node`.
    pub node: String,
}

/// XAML `TreeViewCollapsedEventArgs`: the tree collapsed a node, which the application stores in
/// its `is_expanded`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewCollapsedEventArgs {
    /// `Node`.
    pub node: String,
}

/// One move of a node, which the application applies to its tree: the node leaves its parent
/// and becomes the child of `new_parent` at `index`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewNodeMove {
    /// The node, with its subtree.
    pub node: String,

    /// The node's new parent; `None` for the tree's root nodes.
    pub new_parent: Option<String>,

    /// The node's index among `new_parent`'s children, counted after the node has left its
    /// parent.
    pub index: usize,
}

/// The moves a drop or a keyboard reorder makes, which TreeView makes on its own nodes in
/// WinUI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewMoveRequestedEventArgs {
    /// The moves, applied one after another.
    pub moves: Vec<TreeViewNodeMove>,
}

/// Receives XAML `SelectionChanged`.
pub type TreeViewSelectionChangedHandler = Rc<dyn Fn(&mut App, TreeViewSelectionChangedEventArgs)>;

/// Receives XAML `ItemInvoked`.
pub type TreeViewItemInvokedHandler = Rc<dyn Fn(&mut App, TreeViewItemInvokedEventArgs)>;

/// Receives XAML `Expanding`.
pub type TreeViewExpandingHandler = Rc<dyn Fn(&mut App, TreeViewExpandingEventArgs)>;

/// Receives XAML `Collapsed`.
pub type TreeViewCollapsedHandler = Rc<dyn Fn(&mut App, TreeViewCollapsedEventArgs)>;

/// Receives the moves the tree asks the application to make.
pub type TreeViewMoveRequestedHandler = Rc<dyn Fn(&mut App, TreeViewMoveRequestedEventArgs)>;

/// Receives XAML `DragItemsStarting`.
pub type TreeViewDragItemsStartingHandler =
    Rc<dyn Fn(&mut App, &mut TreeViewDragItemsStartingEventArgs)>;

/// Receives XAML `DragItemsCompleted`.
pub type TreeViewDragItemsCompletedHandler =
    Rc<dyn Fn(&mut App, TreeViewDragItemsCompletedEventArgs)>;

/// XAML `TreeView` with the template of `DefaultTreeViewStyle`: a `TreeViewList` of
/// `TreeViewItem`s.
#[derive(Clone)]
pub struct TreeView {
    /// `RootNodes`, with their descendants.
    pub root_nodes: Vec<TreeViewNode>,

    /// `SelectedNodes`, by id: the nodes drawn selected. In Single mode the first is the list's
    /// selection, which clicks and the arrow keys replace, and every one is drawn selected, so an
    /// application can keep several selected. In Multiple mode a node not among them is drawn
    /// partly selected when one of its descendants is among them.
    pub selected_nodes: Vec<String>,

    /// `SelectionChanged`: asks the application for another selection.
    pub selection_changed: TreeViewSelectionChangedHandler,

    /// `SelectionMode`; `Single` by default.
    pub selection_mode: TreeViewSelectionMode,

    /// `IsEnabled`.
    pub is_enabled: bool,

    /// `Expanding`.
    pub expanding: Option<TreeViewExpandingHandler>,

    /// `Collapsed`.
    pub collapsed: Option<TreeViewCollapsedHandler>,

    /// `ItemInvoked`.
    pub item_invoked: Option<TreeViewItemInvokedHandler>,

    /// `CanDragItems`: whether a drag raises `DragItemsStarting` and `DragItemsCompleted` and can
    /// move nodes; true by default.
    pub can_drag_items: bool,

    /// `CanReorderItems`: whether a drag or Shift+Alt with an arrow key can move nodes; true by
    /// default.
    pub can_reorder_items: bool,

    /// `AllowDrop`: whether the list takes drops between rows; true by default. Drops onto a row
    /// are the row's, which allows them either way.
    pub allow_drop: bool,

    /// Asks the application to move nodes.
    pub move_requested: Option<TreeViewMoveRequestedHandler>,

    /// `DragItemsStarting`.
    pub drag_items_starting: Option<TreeViewDragItemsStartingHandler>,

    /// `DragItemsCompleted`.
    pub drag_items_completed: Option<TreeViewDragItemsCompletedHandler>,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl TreeView {
    /// Creates a tree of `root_nodes` showing `selected_nodes` selected, which asks for other
    /// selections through `selection_changed`.
    pub fn new(
        root_nodes: Vec<TreeViewNode>,
        selected_nodes: Vec<String>,
        selection_changed: impl Fn(&mut App, TreeViewSelectionChangedEventArgs) + 'static,
    ) -> TreeView {
        TreeView {
            root_nodes,
            selected_nodes,
            selection_changed: Rc::new(selection_changed),
            selection_mode: TreeViewSelectionMode::Single,
            is_enabled: true,
            expanding: None,
            collapsed: None,
            item_invoked: None,
            can_drag_items: true,
            can_reorder_items: true,
            allow_drop: true,
            move_requested: None,
            drag_items_starting: None,
            drag_items_completed: None,
            key: None,
        }
    }

    /// Sets SelectionMode.
    pub fn selection_mode(mut self, value: TreeViewSelectionMode) -> Self {
        self.selection_mode = value;
        self
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets Expanding.
    pub fn expanding(
        mut self,
        value: impl Fn(&mut App, TreeViewExpandingEventArgs) + 'static,
    ) -> Self {
        self.expanding = Some(Rc::new(value));
        self
    }

    /// Sets Collapsed.
    pub fn collapsed(
        mut self,
        value: impl Fn(&mut App, TreeViewCollapsedEventArgs) + 'static,
    ) -> Self {
        self.collapsed = Some(Rc::new(value));
        self
    }

    /// Sets ItemInvoked.
    pub fn item_invoked(
        mut self,
        value: impl Fn(&mut App, TreeViewItemInvokedEventArgs) + 'static,
    ) -> Self {
        self.item_invoked = Some(Rc::new(value));
        self
    }

    /// Sets CanDragItems.
    pub fn can_drag_items(mut self, value: bool) -> Self {
        self.can_drag_items = value;
        self
    }

    /// Sets CanReorderItems.
    pub fn can_reorder_items(mut self, value: bool) -> Self {
        self.can_reorder_items = value;
        self
    }

    /// Sets AllowDrop.
    pub fn allow_drop(mut self, value: bool) -> Self {
        self.allow_drop = value;
        self
    }

    /// Sets the handler of the moves the tree asks for.
    pub fn move_requested(
        mut self,
        value: impl Fn(&mut App, TreeViewMoveRequestedEventArgs) + 'static,
    ) -> Self {
        self.move_requested = Some(Rc::new(value));
        self
    }

    /// Sets DragItemsStarting.
    pub fn drag_items_starting(
        mut self,
        value: impl Fn(&mut App, &mut TreeViewDragItemsStartingEventArgs) + 'static,
    ) -> Self {
        self.drag_items_starting = Some(Rc::new(value));
        self
    }

    /// Sets DragItemsCompleted.
    pub fn drag_items_completed(
        mut self,
        value: impl Fn(&mut App, TreeViewDragItemsCompletedEventArgs) + 'static,
    ) -> Self {
        self.drag_items_completed = Some(Rc::new(value));
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for TreeView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeView")
            .field("root_nodes", &self.root_nodes)
            .field("selected_nodes", &self.selected_nodes)
            .field("selection_mode", &self.selection_mode)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// `ScrollViewerLineDelta`: the scroll of a line, which an arrow key no row takes scrolls.
const LINE_DELTA: f64 = 16.0;

impl StatefulWidget for TreeView {
    type State = TreeViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TreeViewState {
        let tree = NodeTree::new(&self.root_nodes);
        let rows = tree.rows();
        TreeViewState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            tree: Rc::new(tree),
            rows,
            requested_expansion: HashMap::new(),
            requested_selection: None,
            row_estimate: 0.0,
            item_focus: HashMap::new(),
            row_keys: HashMap::new(),
            viewport_key: GlobalKey::new(),
            heights: HashMap::new(),
            widths: HashMap::new(),
            realized: HashSet::new(),
            focused: None,
            last_focused: None,
            focus_epoch: 0,
            pressed_part: None,
            scroll: None,
            building: false,
            drag: TreeDragState::default(),
        }
    }
}

/// TreeView's view model and list: the nodes as the tree reads them, the rows, the focus, and
/// what the tree asked of the application since it last built the tree.
pub struct TreeViewState {
    state: StateData<TreeView>,

    /// The height of a row of one line of text, as the last build resolved it from the theme.
    row_estimate: f64,

    /// TickerMode subscription.
    single_ticker_provider: SingleTickerProviderStateMixinData,

    /// The clock of edge scrolling.
    ticker: Option<Handle<Ticker>>,

    /// The nodes, with the expansions and moves the tree asked for.
    tree: Rc<NodeTree>,

    /// The view model's flat list.
    rows: Rows,

    /// The `IsExpanded` the tree set, by node, until the application next builds it.
    requested_expansion: HashMap<String, bool>,

    /// The `SelectedNodes` the tree asked for, until the application next builds it.
    requested_selection: Option<Vec<String>>,

    /// The rows' focus, by node, owned here so that the tree can focus a row it scrolls to.
    item_focus: HashMap<String, Handle<FocusNode>>,

    /// Identifies each row's container, where the row is laid out.
    row_keys: HashMap<String, GlobalKey>,

    /// Identifies the list's viewport.
    viewport_key: GlobalKey,

    /// Each row's height and width, by node, as last laid out.
    heights: HashMap<String, f64>,
    widths: HashMap<String, f64>,

    /// The rows the lazy list has built.
    realized: HashSet<String>,

    /// The node whose row has the focus.
    focused: Option<String>,

    /// `LastFocusedIndex`, by node: the row Tab enters the list at.
    last_focused: Option<String>,

    /// Invalidates focus requests a later one replaced.
    focus_epoch: u64,

    /// The row part the last press went down on, with the row and the pointer.
    pressed_part: Option<(String, PressedPart, i64)>,

    /// The list's scroll position.
    scroll: Option<Handle<ScrollViewportController>>,

    /// The application's values are being applied, during a build: callbacks to the application
    /// wait for the frame's end.
    building: bool,

    /// TreeViewList's and ListViewBase's drag and drop.
    drag: TreeDragState,
}

impl SingleTickerProviderStateMixin for TreeViewState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// The keyboard modifiers `GetKeyboardModifiers` reports.
#[derive(Clone, Copy, Debug)]
struct Modifiers {
    alt: bool,
    control: bool,
    shift: bool,
}

impl Modifiers {
    fn of(app: &mut App) -> Modifiers {
        let keyboard = HardwareKeyboard::instance(app);
        Modifiers {
            alt: keyboard.is_alt_pressed(app),
            control: keyboard.is_control_pressed(app),
            shift: keyboard.is_shift_pressed(app),
        }
    }
}

/// Whether `key` is one of `IsDirectionalKey`'s.
fn is_directional(key: LogicalKeyboardKey) -> bool {
    matches!(
        key,
        LogicalKeyboardKey::ARROW_UP
            | LogicalKeyboardKey::ARROW_DOWN
            | LogicalKeyboardKey::ARROW_LEFT
            | LogicalKeyboardKey::ARROW_RIGHT
    )
}

impl TreeViewState {
    fn alive(self: Handle<Self>, app: &App) -> bool {
        app.contains(self) && self.mounted(app)
    }

    fn view(self: Handle<Self>, app: &App) -> Option<inset_embedder::ViewId> {
        View::maybe_id_of(app, self.context(app))
    }

    /// Runs `work` once the frame is over, if the tree is still mounted.
    fn after_frame(self: Handle<Self>, app: &mut App, work: impl Fn(&mut App) + 'static) {
        let view = self.view(app);
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if self.alive(app) {
                    work(app);
                }
            }),
        );
    }

    /// Calls the application now, or at the frame's end while its values are being applied.
    fn raise(self: Handle<Self>, app: &mut App, call: impl Fn(&mut App) + 'static) {
        if app.get(self).building {
            let view = self.view(app);
            after_next_frame(app, view, FrameCallback::new(move |app, _| call(app)));
        } else {
            call(app);
        }
    }

    fn mode(self: Handle<Self>, app: &App) -> TreeViewSelectionMode {
        self.widget(app).selection_mode
    }

    fn scroll(self: Handle<Self>, app: &App) -> Handle<ScrollViewportController> {
        app.get(self).scroll.expect("an initialized TreeView")
    }

    // MARK: The application's values

    /// Reads the application's nodes, with the expansions the tree asked for, and checks its
    /// selection: unique ids, which `NodeTree::new` checks.
    fn sync_tree(self: Handle<Self>, app: &mut App) {
        let mut tree = NodeTree::new(&self.widget(app).root_nodes);
        for (id, expanded) in &app.get(self).requested_expansion {
            tree.set_expanded(id, *expanded);
        }
        let rows = tree.rows();
        let state = app.get_mut(self);
        state.tree = Rc::new(tree);
        state.rows = rows;
        self.prune_rows(app);
    }

    /// Releases the focus of the nodes no longer in the tree, once their rows are gone.
    fn prune_rows(self: Handle<Self>, app: &mut App) {
        let tree = app.get(self).tree.clone();
        let removed: Vec<String> = app
            .get(self)
            .item_focus
            .keys()
            .filter(|id| !tree.contains(id))
            .cloned()
            .collect();
        for id in removed {
            let state = app.get_mut(self);
            let node = state.item_focus.remove(&id).unwrap();
            state.row_keys.remove(&id);
            state.heights.remove(&id);
            state.widths.remove(&id);
            let view = self.view(app);
            after_next_frame(
                app,
                view,
                FrameCallback::new(move |app, _| {
                    if app.contains(node) {
                        node.dispose(app);
                        app.destroy(node);
                    }
                }),
            );
        }
    }

    // MARK: Selection

    /// `SelectedNodes` as the tree reads them: what it last asked for, until the application
    /// answers, else the application's.
    fn selected_nodes(self: Handle<Self>, app: &App) -> Vec<String> {
        let state = app.get(self);
        state
            .requested_selection
            .clone()
            .unwrap_or_else(|| self.widget(app).selected_nodes.clone())
    }

    /// The list's selection in Single mode: the first of `SelectedNodes`.
    fn list_selection(self: Handle<Self>, app: &App) -> Option<String> {
        if self.mode(app) != TreeViewSelectionMode::Single {
            return None;
        }
        self.selected_nodes(app).into_iter().next()
    }

    /// Whether the node is among `SelectedNodes`: `TreeViewList::IsSelected`.
    fn is_node_selected(self: Handle<Self>, app: &App, id: &str) -> bool {
        self.selected_nodes(app).iter().any(|node| node == id)
    }

    /// `IsMutiSelectWithSelectedItems`.
    fn is_multi_select_with_selected_items(self: Handle<Self>, app: &App) -> bool {
        self.mode(app) == TreeViewSelectionMode::Multiple && !self.selected_nodes(app).is_empty()
    }

    /// Asks the application for `selected`, after `added` and `removed`.
    fn request_selection(
        self: Handle<Self>,
        app: &mut App,
        selected: Vec<String>,
        added: Vec<String>,
        removed: Vec<String>,
    ) {
        app.get_mut(self).requested_selection = Some(selected.clone());
        let args = TreeViewSelectionChangedEventArgs {
            added_items: added,
            removed_items: removed,
            selected_nodes: selected,
        };
        let handler = self.widget(app).selection_changed.clone();
        self.raise(app, move |app| handler(app, args.clone()));
        self.set_state(app, |_| {});
    }

    /// The list's `MakeSingleSelection` and `MakeToggleSelection`: `id` becomes the selection,
    /// or nothing does.
    fn set_list_selection(self: Handle<Self>, app: &mut App, id: Option<&str>) {
        let old = self.selected_nodes(app);
        let new: Vec<String> = id.map(str::to_owned).into_iter().collect();
        if old == new {
            return;
        }
        let removed = old
            .iter()
            .filter(|node| !new.contains(node))
            .cloned()
            .collect();
        let added = new
            .iter()
            .filter(|node| !old.contains(node))
            .cloned()
            .collect();
        self.request_selection(app, new, added, removed);
    }

    /// The selection a node leaving the rows clears in Single mode: ListView unselects a removed
    /// item, and TreeView follows its `SelectedItem`.
    fn deselect_if_hidden(self: Handle<Self>, app: &mut App, before: &Rows) {
        if let Some(selected) = self.list_selection(app)
            && before.index_of(&selected).is_some()
            && app.get(self).rows.index_of(&selected).is_none()
        {
            self.set_list_selection(app, None);
        }
    }

    /// `TreeViewItem::OnCheckToggle`: the check box of a row in Multiple mode changed, and
    /// `SelectByIndex` applies ViewModel's rules.
    fn check_toggled(self: Handle<Self>, app: &mut App, id: &str, is_checked: Option<bool>) {
        let state = match is_checked {
            Some(true) => NodeSelectionState::Selected,
            Some(false) => NodeSelectionState::UnSelected,
            None => NodeSelectionState::PartialSelected,
        };
        self.update_multiple_selection(app, id, state);
    }

    /// `ViewModel::SelectByIndex` between `BeginSelectionChanges` and `EndSelectionChanges`,
    /// which raises `SelectionChanged` when anything changed.
    fn update_multiple_selection(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        state: NodeSelectionState,
    ) {
        let tree = app.get(self).tree.clone();
        if !tree.contains(id) {
            return;
        }
        let selected = self.selected_nodes(app);
        let mut change = SelectionChange::new(&tree, &selected);
        change.update_selection(id, state);
        if !change.added.is_empty() || !change.removed.is_empty() {
            let SelectionChange {
                selected,
                added,
                removed,
                ..
            } = change;
            self.request_selection(app, selected, added, removed);
        }
    }

    /// Each node's state in Multiple mode, as the check boxes draw it.
    fn multiple_selection_states(
        self: Handle<Self>,
        app: &App,
    ) -> HashMap<String, NodeSelectionState> {
        let tree = app.get(self).tree.clone();
        view_model::selection_states(&tree, &self.selected_nodes(app))
    }

    // MARK: Expansion and moves

    /// `TreeViewNode::IsExpanded` set by the tree: ViewModel adds or removes the node's
    /// descendants from the rows, then TreeView raises `Expanding` or `Collapsed`.
    fn set_node_expanded(self: Handle<Self>, app: &mut App, id: &str, expanded: bool) {
        let tree = app.get(self).tree.clone();
        if !tree.contains(id) || tree.is_expanded(id) == expanded {
            return;
        }
        let before = app.get(self).rows.clone();
        let state = app.get_mut(self);
        state.requested_expansion.insert(id.to_owned(), expanded);
        Rc::make_mut(&mut state.tree).set_expanded(id, expanded);
        state.rows = state.tree.rows();
        if !expanded {
            self.deselect_if_hidden(app, &before);
        }
        self.raise_expansion(app, id, expanded);
        self.set_state(app, |_| {});
    }

    /// Raises `Expanding` or `Collapsed`.
    fn raise_expansion(self: Handle<Self>, app: &mut App, id: &str, expanded: bool) {
        let node = id.to_owned();
        if expanded {
            if let Some(handler) = self.widget(app).expanding.clone() {
                self.raise(app, move |app| {
                    handler(app, TreeViewExpandingEventArgs { node: node.clone() })
                });
            }
        } else if let Some(handler) = self.widget(app).collapsed.clone() {
            self.raise(app, move |app| {
                handler(app, TreeViewCollapsedEventArgs { node: node.clone() })
            });
        }
    }

    /// Asks the application for the moves of `plan` and shows them until it answers: the moved
    /// nodes leave the rows, which in Single mode unselects the selected one among them, and the
    /// parents the moves leave without children collapse.
    fn request_moves(self: Handle<Self>, app: &mut App, plan: MovePlan) {
        if plan.moves.is_empty() && plan.collapsed.is_empty() {
            return;
        }
        let before_rows = app.get(self).rows.clone();
        let before_tree = app.get(self).tree.clone();
        let state = app.get_mut(self);
        let tree = Rc::make_mut(&mut state.tree);
        for node_move in &plan.moves {
            tree.move_node(
                &node_move.node,
                node_move.new_parent.as_deref(),
                node_move.index,
            );
        }
        for id in &plan.collapsed {
            tree.set_expanded(id, false);
        }
        state.rows = state.tree.rows();
        for id in &plan.collapsed {
            state.requested_expansion.insert(id.clone(), false);
        }
        if !plan.moves.is_empty()
            && let Some(handler) = self.widget(app).move_requested.clone()
        {
            let args = TreeViewMoveRequestedEventArgs {
                moves: plan.moves.clone(),
            };
            self.raise(app, move |app| handler(app, args.clone()));
        }
        if let Some(selected) = self.list_selection(app)
            && before_rows.index_of(&selected).is_some()
            && plan
                .moves
                .iter()
                .any(|node_move| before_tree.is_within(&selected, &node_move.node))
        {
            self.set_list_selection(app, None);
        }
        for id in &plan.collapsed {
            self.raise_expansion(app, id, false);
        }
        self.set_state(app, |_| {});
    }

    // MARK: Focus

    /// The focus of the node's row, made the first time it is asked for.
    fn item_focus_node(self: Handle<Self>, app: &mut App, id: &str) -> Handle<FocusNode> {
        if let Some(node) = app.get(self).item_focus.get(id) {
            return *node;
        }
        let node = FocusNode::new(app);
        app.get_mut(self).item_focus.insert(id.to_owned(), node);
        node
    }

    /// The key of the node's row.
    fn row_key(self: Handle<Self>, app: &mut App, id: &str) -> GlobalKey {
        app.get_mut(self)
            .row_keys
            .entry(id.to_owned())
            .or_insert_with(GlobalKey::new)
            .clone()
    }

    /// `ListViewBase::FocusItem`: the row becomes the last focused and takes the focus once it
    /// is built.
    fn focus_item(self: Handle<Self>, app: &mut App, id: &str, state: FocusState) {
        if !app.get(self).tree.contains(id) {
            return;
        }
        let node = self.item_focus_node(app, id);
        let tree_state = app.get_mut(self);
        tree_state.last_focused = Some(id.to_owned());
        tree_state.focus_epoch += 1;
        let epoch = tree_state.focus_epoch;
        FocusState::name(app, node.as_node(), state);
        if node.has_primary_focus(app) {
            self.set_state(app, |_| {});
            return;
        }
        self.request_item_focus(app, id.to_owned(), node, epoch, 0);
    }

    /// Requests focus for a row, retrying for a few frames while the list builds it.
    fn request_item_focus(
        self: Handle<Self>,
        app: &mut App,
        id: String,
        node: Handle<FocusNode>,
        epoch: u64,
        attempt: u32,
    ) {
        if node.as_node().context(app).is_some() {
            if let Some(row) = app.get(self).rows.index_of(&id) {
                self.scroll_into_view(app, row);
            }
            node.request_focus(app, None);
            return;
        }
        if attempt < 3 {
            self.after_frame(app, move |app| {
                if app.get(self).focus_epoch == epoch {
                    self.request_item_focus(app, id.clone(), node, epoch, attempt + 1);
                }
            });
        }
    }

    /// `SetFocusedItem` with the row scrolled into view when asked.
    fn set_focused_row(
        self: Handle<Self>,
        app: &mut App,
        row: usize,
        scroll: bool,
        state: FocusState,
    ) {
        let Some(id) = app.get(self).rows.get(row).map(str::to_owned) else {
            return;
        };
        if scroll {
            self.scroll_into_view(app, row);
        }
        self.focus_item(app, &id, state);
    }

    /// A row's focus changed: `ListViewBase::ItemFocused`, which records the row as the last
    /// focused, and `ItemUnfocused`.
    fn item_focus_changed(self: Handle<Self>, app: &mut App, id: &str, focused: bool) {
        if focused {
            let tree_state = app.get_mut(self);
            tree_state.focused = Some(id.to_owned());
            tree_state.last_focused = Some(id.to_owned());
        } else if app.get(self).focused.as_deref() == Some(id) {
            app.get_mut(self).focused = None;
        }
        self.set_state(app, |_| {});
    }

    /// The row Tab reaches: TreeViewList's `TabNavigation="Once"` makes the list one tab stop,
    /// entered at the last focused row, else the selected one, else the first.
    fn tab_stop(self: Handle<Self>, app: &App) -> Option<String> {
        let state = app.get(self);
        let rows = &state.rows;
        state
            .last_focused
            .clone()
            .filter(|id| rows.index_of(id).is_some())
            .or_else(|| {
                self.list_selection(app)
                    .filter(|id| rows.index_of(id).is_some())
            })
            .or_else(|| rows.get(0).map(str::to_owned))
    }

    // MARK: Pointer

    /// A press went down on a part of a row. A press handled by the chevron or the check box
    /// neither invokes nor drags the row.
    fn part_pressed(self: Handle<Self>, app: &mut App, id: &str, part: PressedPart, pointer: i64) {
        let state = app.get_mut(self);
        let same_press = state
            .pressed_part
            .as_ref()
            .is_some_and(|(pressed, _, down)| pressed == id && *down == pointer);
        if part != PressedPart::Item || !same_press {
            state.pressed_part = Some((id.to_owned(), part, pointer));
        }
    }

    /// Whether the last press on the row went down on `part`.
    fn pressed_part_is(self: Handle<Self>, app: &App, id: &str, part: PressedPart) -> bool {
        app.get(self)
            .pressed_part
            .as_ref()
            .is_some_and(|(pressed, pressed_part, _)| pressed == id && *pressed_part == part)
    }

    /// `TreeViewItem::OnExpandCollapseChevronPointerPressed`: the node's `IsExpanded` toggles,
    /// whether the chevron shows or not.
    fn chevron_pressed(self: Handle<Self>, app: &mut App, id: &str) {
        if self.is_in_drag(app) {
            return;
        }
        let expanded = app.get(self).tree.is_expanded(id);
        self.set_node_expanded(app, id, !expanded);
    }

    /// `ListViewBaseItem::DoTapInteraction`: a click is the primary gesture, a click with Ctrl
    /// the secondary.
    fn item_tapped(self: Handle<Self>, app: &mut App, id: &str) {
        if self.is_in_drag(app) || !app.get(self).tree.contains(id) {
            return;
        }
        if Modifiers::of(app).control {
            self.secondary_gesture(app, id, false);
        } else {
            self.primary_gesture(app, id, false);
        }
    }

    /// `ListViewBaseItem::OnRightTapped`: the row takes the focus.
    fn item_right_tapped(self: Handle<Self>, app: &mut App, id: &str) {
        let state = FocusState::coerce_programmatic(app);
        self.focus_item(app, id, state);
    }

    /// `OnItemPrimaryInteractionGesture` with `IsItemClickEnabled`, which TreeViewList sets: the
    /// row takes the focus, `ItemInvoked` is raised, and in Single mode the row is selected.
    fn primary_gesture(self: Handle<Self>, app: &mut App, id: &str, keyboard: bool) {
        let state = if keyboard {
            FocusState::Keyboard
        } else {
            FocusState::Pointer
        };
        self.focus_item(app, id, state);
        self.raise_item_invoked(app, id);
        if self.mode(app) == TreeViewSelectionMode::Single {
            self.set_list_selection(app, Some(id));
        }
    }

    /// `OnItemSecondaryInteractionGesture`: `ItemInvoked` is raised, then in Single mode the row
    /// takes the focus and its selection toggles.
    fn secondary_gesture(self: Handle<Self>, app: &mut App, id: &str, keyboard: bool) {
        self.raise_item_invoked(app, id);
        if self.mode(app) == TreeViewSelectionMode::Single {
            let state = if keyboard {
                FocusState::Keyboard
            } else {
                FocusState::Pointer
            };
            self.focus_item(app, id, state);
            let selected = self.list_selection(app).as_deref() == Some(id);
            self.set_list_selection(app, (!selected).then_some(id));
        }
    }

    /// `TreeView::OnItemClick`.
    fn raise_item_invoked(self: Handle<Self>, app: &mut App, id: &str) {
        if let Some(handler) = self.widget(app).item_invoked.clone() {
            let args = TreeViewItemInvokedEventArgs {
                invoked_item: id.to_owned(),
            };
            self.raise(app, move |app| handler(app, args.clone()));
        }
    }

    /// A row's container mounted or unmounted in the lazy list.
    fn row_realized(self: Handle<Self>, app: &mut App, id: &str, realized: bool) {
        if !app.contains(self) {
            return;
        }
        let state = app.get_mut(self);
        if realized {
            state.realized.insert(id.to_owned());
        } else {
            state.realized.remove(id);
        }
    }

    /// A row reported its size.
    fn row_measured(self: Handle<Self>, app: &mut App, id: &str, size: Size) {
        if !app.contains(self) {
            return;
        }
        let state = app.get_mut(self);
        state.heights.insert(id.to_owned(), size.height());
        state.widths.insert(id.to_owned(), size.width());
    }

    // MARK: Keyboard

    /// The keys of the focused row, as `TreeViewItem::OnKeyDown`, `ListViewBaseItem::OnKeyDown`
    /// and `ListViewBase::OnKeyDown` take them in turn.
    fn on_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if !matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            return KeyEventResult::Ignored;
        }
        let Some(id) = app.get(self).focused.clone() else {
            return KeyEventResult::Ignored;
        };
        if !app.get(self).tree.contains(&id) || app.get(self).rows.index_of(&id).is_none() {
            return KeyEventResult::Ignored;
        }
        // During a drag, the list takes every key.
        if self.is_in_drag(app) {
            return KeyEventResult::Handled;
        }
        let key = event.logical_key();
        let modifiers = Modifiers::of(app);
        let handled = self.item_key_down(app, &id, key, modifiers)
            || self.list_view_item_key_down(key, modifiers)
            || self.list_key_down(app, &id, key, modifiers);
        if handled {
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }

    /// `TreeViewItem::OnKeyDown`: Shift+Alt with an arrow reorders, an arrow alone expands,
    /// collapses or moves to the parent or first child, and Space in Multiple mode toggles the
    /// check box.
    fn item_key_down(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        key: LogicalKeyboardKey,
        modifiers: Modifiers,
    ) -> bool {
        let directional = is_directional(key);
        if directional
            && modifiers.shift
            && modifiers.alt
            && !modifiers.control
            && self.widget(app).can_reorder_items
        {
            self.keyboard_reorder(app, id, key);
            return true;
        }
        if directional && !modifiers.shift && !modifiers.alt && !modifiers.control {
            return self.expand_collapse_key(app, id, key);
        }
        if key == LogicalKeyboardKey::SPACE && self.mode(app) == TreeViewSelectionMode::Multiple {
            let selected = self.multiple_selection_states(app).get(id).copied()
                == Some(NodeSelectionState::Selected);
            self.check_toggled(app, id, Some(!selected));
            return true;
        }
        false
    }

    /// `ListViewBaseItem::OnKeyDown`: Shift+Alt with an arrow is ListView's keyboard reorder,
    /// which does nothing for a tree that cannot reorder, and takes the key.
    fn list_view_item_key_down(
        self: Handle<Self>,
        key: LogicalKeyboardKey,
        modifiers: Modifiers,
    ) -> bool {
        is_directional(key) && modifiers.alt && modifiers.shift && !modifiers.control
    }

    /// `TreeViewItem::HandleReorder`: Up, or Left past the first row, moves the node before its
    /// previous sibling; Down or Right, short of the last row, after its next sibling.
    fn keyboard_reorder(self: Handle<Self>, app: &mut App, id: &str, key: LogicalKeyboardKey) {
        let Some(row) = app.get(self).rows.index_of(id) else {
            return;
        };
        let last = app.get(self).rows.len() - 1;
        let forward = if key == LogicalKeyboardKey::ARROW_UP
            || (key == LogicalKeyboardKey::ARROW_LEFT && row != 0)
        {
            false
        } else if (key == LogicalKeyboardKey::ARROW_DOWN || key == LogicalKeyboardKey::ARROW_RIGHT)
            && row != last
        {
            true
        } else {
            return;
        };
        let tree = app.get(self).tree.clone();
        if let Some(plan) = tree.plan_reorder(id, forward) {
            self.request_moves(app, plan);
            self.focus_item(app, id, FocusState::Keyboard);
        }
    }

    /// `TreeViewItem::HandleExpandCollapse`, with Left and Right swapped right to left.
    fn expand_collapse_key(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        key: LogicalKeyboardKey,
    ) -> bool {
        let rtl = Directionality::maybe_of(app, self.context(app)) == Some(TextDirection::Rtl);
        let (collapse, expand) = if rtl {
            (
                LogicalKeyboardKey::ARROW_RIGHT,
                LogicalKeyboardKey::ARROW_LEFT,
            )
        } else {
            (
                LogicalKeyboardKey::ARROW_LEFT,
                LogicalKeyboardKey::ARROW_RIGHT,
            )
        };
        let tree = app.get(self).tree.clone();
        let expanded = tree.is_expanded(id);
        if key == collapse {
            if expanded {
                self.set_node_expanded(app, id, false);
                self.focus_item(app, id, FocusState::Keyboard);
                return true;
            }
            // A root node's parent is the tree's hidden root, which has no row.
            if let Some(parent) = tree.parent(id)
                && let Some(row) = app.get(self).rows.index_of(parent)
            {
                self.set_focused_row(app, row, true, FocusState::Keyboard);
                return true;
            }
        } else if key == expand {
            if !expanded && tree.has_children(id) {
                self.set_node_expanded(app, id, true);
                return true;
            }
            if let Some(child) = tree.node(id).children.first()
                && let Some(row) = app.get(self).rows.index_of(child)
            {
                self.set_focused_row(app, row, true, FocusState::Keyboard);
                return true;
            }
        }
        false
    }

    /// `ListViewBase::OnKeyDown`: Enter and Space invoke, Ctrl+Space toggles the selection, and
    /// the navigation keys move the focus, and in Single mode without Ctrl the selection with it.
    fn list_key_down(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        key: LogicalKeyboardKey,
        modifiers: Modifiers,
    ) -> bool {
        match key {
            LogicalKeyboardKey::ENTER | LogicalKeyboardKey::NUMPAD_ENTER => {
                self.primary_gesture(app, id, true);
                true
            }
            // Alt+Space opens the window's menu.
            LogicalKeyboardKey::SPACE if !modifiers.alt => {
                if modifiers.control {
                    self.secondary_gesture(app, id, true);
                } else {
                    self.primary_gesture(app, id, true);
                }
                true
            }
            LogicalKeyboardKey::ARROW_UP
            | LogicalKeyboardKey::ARROW_DOWN
            | LogicalKeyboardKey::ARROW_LEFT
            | LogicalKeyboardKey::ARROW_RIGHT
            | LogicalKeyboardKey::HOME
            | LogicalKeyboardKey::END
            | LogicalKeyboardKey::PAGE_UP
            | LogicalKeyboardKey::PAGE_DOWN => self.keyboard_navigation(app, id, key, modifiers),
            _ => false,
        }
    }

    /// `OnKeyboardNavigation` on the vertical ItemsStackPanel, then the selection and focus that
    /// follow. A key that moves to no other row scrolls the list instead and is not taken.
    fn keyboard_navigation(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        key: LogicalKeyboardKey,
        modifiers: Modifiers,
    ) -> bool {
        let rows = app.get(self).rows.clone();
        let Some(row) = rows.index_of(id) else {
            return false;
        };
        let last = rows.len() - 1;
        let target = match key {
            LogicalKeyboardKey::ARROW_UP => row.checked_sub(1),
            LogicalKeyboardKey::ARROW_DOWN => (row < last).then_some(row + 1),
            LogicalKeyboardKey::HOME => Some(0),
            LogicalKeyboardKey::END => Some(last),
            LogicalKeyboardKey::PAGE_UP => self.page_target(app, row, false),
            LogicalKeyboardKey::PAGE_DOWN => self.page_target(app, row, true),
            // Left and Right run across a vertical panel.
            _ => None,
        }
        .filter(|target| *target != row);
        let Some(target) = target else {
            self.scroll_in_direction(app, key);
            return false;
        };
        if self.mode(app) == TreeViewSelectionMode::Single && !modifiers.control {
            let target_id = rows.get(target).unwrap().to_owned();
            self.set_list_selection(app, Some(&target_id));
        }
        self.set_focused_row(app, target, true, FocusState::Keyboard);
        true
    }

    /// `ElementScrollViewerScrollInDirection`: a line for Up and Down, a page for Page Up and Page
    /// Down, and the ends for Home and End.
    fn scroll_in_direction(self: Handle<Self>, app: &mut App, key: LogicalKeyboardKey) {
        let scroll = self.scroll(app);
        let metrics = scroll.metrics(app);
        let offset = match key {
            LogicalKeyboardKey::ARROW_UP => metrics.offset - LINE_DELTA,
            LogicalKeyboardKey::ARROW_DOWN => metrics.offset + LINE_DELTA,
            LogicalKeyboardKey::PAGE_UP => metrics.offset - metrics.viewport_length,
            LogicalKeyboardKey::PAGE_DOWN => metrics.offset + metrics.viewport_length,
            LogicalKeyboardKey::HOME => 0.0,
            LogicalKeyboardKey::END => metrics.scrollable_length,
            _ => return,
        };
        scroll.change_view(app, offset, true);
    }

    // MARK: Geometry

    /// The list's viewport, in global coordinates.
    fn viewport_rect(self: Handle<Self>, app: &mut App) -> Option<Rect> {
        let key = app.get(self).viewport_key.clone();
        let render = key
            .current_context(app)?
            .find_render_object(app)?
            .as_box()?;
        let origin = render.local_to_global(app, Offset::ZERO, None);
        Some(origin & render.size(app))
    }

    /// Where a built row's container is laid out, before a drag displaces it, in global
    /// coordinates.
    fn row_rect(self: Handle<Self>, app: &mut App, id: &str) -> Option<Rect> {
        let key = app.get(self).row_keys.get(id)?.clone();
        let render = key
            .current_context(app)?
            .find_render_object(app)?
            .as_box()?;
        let origin = render.local_to_global(app, Offset::ZERO, None);
        Some(origin & render.size(app))
    }

    /// A row's height, as last laid out, or the height of a row of one line of text.
    fn row_height(self: Handle<Self>, app: &App, id: &str) -> f64 {
        app.get(self)
            .heights
            .get(id)
            .copied()
            .unwrap_or(app.get(self).row_estimate)
    }

    /// Each row's top relative to the top of the viewport: the built rows where they are laid
    /// out, the others counted from them by their heights.
    fn row_tops(self: Handle<Self>, app: &mut App) -> Option<Vec<f64>> {
        let viewport = self.viewport_rect(app)?;
        let rows = app.get(self).rows.clone();
        let anchor = app
            .get(self)
            .realized
            .clone()
            .into_iter()
            .filter_map(|id| Some((rows.index_of(&id)?, id)))
            .min_by_key(|(row, _)| *row)?;
        let anchor_top = self.row_rect(app, &anchor.1)?.top - viewport.top;
        let heights: Vec<f64> = rows
            .ids()
            .iter()
            .map(|id| self.row_height(app, id))
            .collect();
        let mut tops = vec![0.0; rows.len()];
        tops[anchor.0] = anchor_top;
        for row in anchor.0 + 1..rows.len() {
            tops[row] = tops[row - 1] + heights[row - 1];
        }
        for row in (0..anchor.0).rev() {
            tops[row] = tops[row + 1] - heights[row];
        }
        Some(tops)
    }

    /// `ScrollIntoView` with the default alignment: the least scroll that shows the row.
    fn scroll_into_view(self: Handle<Self>, app: &mut App, row: usize) {
        let scroll = self.scroll(app);
        let viewport = scroll.metrics(app).viewport_length;
        let Some(id) = app.get(self).rows.get(row).map(str::to_owned) else {
            return;
        };
        let Some(tops) = self.row_tops(app) else {
            return;
        };
        let top = tops[row];
        let bottom = top + self.row_height(app, &id);
        if top < 0.0 {
            scroll.scroll_by(app, top, true);
        } else if bottom > viewport {
            scroll.scroll_by(app, (bottom - viewport).min(top), true);
        }
    }

    /// `GetTargetIndexFromNavigationActionByPage`: the last row fully in view in the direction
    /// of travel, or when the row already is, the last one a page further; a row out of view is
    /// first brought to the edge it pages from.
    fn page_target(self: Handle<Self>, app: &mut App, source: usize, down: bool) -> Option<usize> {
        let metrics = self.scroll(app).metrics(app);
        let viewport = metrics.viewport_length;
        let tops = self.row_tops(app)?;
        let rows = app.get(self).rows.clone();
        let heights: Vec<f64> = rows
            .ids()
            .iter()
            .map(|id| self.row_height(app, id))
            .collect();
        let count = rows.len();
        // The scrolls the list can make from where it is.
        let clamp =
            |delta: f64| delta.clamp(-metrics.offset, metrics.scrollable_length - metrics.offset);
        // `GetLastVisibleContainer` with the window scrolled by `delta`: fully in view, or for a
        // row taller than the window, partly.
        let last_visible = |delta: f64| {
            let fully = |row: &usize| {
                tops[*row] - delta >= -0.5 && tops[*row] - delta + heights[*row] <= viewport + 0.5
            };
            let partly = |row: &usize| {
                tops[*row] - delta < viewport && tops[*row] - delta + heights[*row] > 0.0
            };
            if down {
                (0..count)
                    .rev()
                    .find(fully)
                    .or_else(|| (0..count).rev().find(partly))
            } else {
                (0..count).find(fully).or_else(|| (0..count).find(partly))
            }
        };
        let in_view = tops[source] < viewport && tops[source] + heights[source] > 0.0;
        if !in_view {
            // `ScrollItemIntoView` with the leading alignment, then a page from there.
            let leading = clamp(tops[source]);
            let page = (if down { viewport } else { -viewport }).clamp(
                -(metrics.offset + leading),
                metrics.scrollable_length - metrics.offset - leading,
            );
            return last_visible(leading + page);
        }
        let last = last_visible(0.0)?;
        let past_last = if down { source >= last } else { source <= last };
        let at_end = if down {
            source == count - 1
        } else {
            source == 0
        };
        if !past_last || at_end {
            return Some(last);
        }
        let mut last = last_visible(clamp(if down { viewport } else { -viewport }))?;
        if source == last {
            if down && last + 1 < count {
                last += 1;
            } else if !down && last > 0 {
                last -= 1;
            }
        }
        Some(last)
    }

    // MARK: Template

    /// The row container for the node `id`, a row of the list with `focus`, or else the drag
    /// preview.
    fn container(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        focus: Option<ItemFocus>,
    ) -> TreeViewItemContainer {
        let widget = self.widget(app).clone();
        let tree = app.get(self).tree.clone();
        let node = tree.node(id);
        let multi_select = (widget.selection_mode == TreeViewSelectionMode::Multiple).then(|| {
            self.multiple_selection_states(app)
                .get(id)
                .copied()
                .unwrap_or(NodeSelectionState::UnSelected)
        });
        TreeViewItemContainer {
            owner: self,
            id: id.to_owned(),
            content: node.content.clone(),
            depth: tree.depth(id),
            has_children: tree.has_children(id),
            is_expanded: node.is_expanded,
            is_enabled: widget.is_enabled,
            multi_select,
            is_selected: self.mode(app) == TreeViewSelectionMode::Single
                && self.is_node_selected(app, id),
            drag: self.item_drag(app, id, focus.is_none()),
            focus,
        }
    }

    /// The row at `index` of the list: the container, displaced and draggable, measured where it
    /// is laid out.
    fn row(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        tab_stop: Option<&str>,
    ) -> Option<WidgetRef> {
        let id = app.get(self).rows.get(index)?.to_owned();
        let node = self.item_focus_node(app, &id);
        let keyboard = FocusState::current(app).shows_focus_visual();
        let state = app.get(self);
        let focus = ItemFocus {
            node,
            is_tab_stop: tab_stop == Some(id.as_str()),
            shows_focus_visual: state.focused.as_deref() == Some(id.as_str()) && keyboard,
        };
        let container = self.container(app, &id, Some(focus));
        let row = self.drag_source(app, &id, index, container);
        let key = self.row_key(app, &id);
        let measured = id.clone();
        Some(
            KeyedSubtree::new(
                SizeObserver::new(
                    row,
                    Rc::new(move |app, _, size| self.row_measured(app, &measured, size)),
                )
                .key(Rc::new(key)),
            )
            .key(Rc::new(ValueKey::new(id)))
            .into_widget(),
        )
    }

    /// `ListControl`: TreeViewList's `ScrollViewer` around the lazy list of rows. Without a bound
    /// on its height, the list is as tall as its rows.
    fn list(self: Handle<Self>, app: &mut App, shrink_wrap: bool) -> WidgetRef {
        let scroll = self.scroll(app);
        let ids = app.get(self).rows.ids().to_vec();
        let tab_stop = self.tab_stop(app);
        let list = ListView::builder(move |app, _, index| {
            self.row(app, index as usize, tab_stop.as_deref())
        })
        .item_count(ids.len() as i32)
        .find_child_index_callback(Rc::new(move |key| {
            (key.as_ref() as &dyn std::any::Any)
                .downcast_ref::<ValueKey<String>>()
                .and_then(|key| ids.iter().position(|id| *id == key.value))
                .map(|index| index as i32)
        }))
        .build()
        .controller(scroll.native_controller(app))
        .shrink_wrap(shrink_wrap)
        .padding(EdgeInsetsGeometry::ZERO);
        let viewport = KeyedSubtree::new(list).key(Rc::new(app.get(self).viewport_key.clone()));
        ScrollBarViewport::scroller(scroll, viewport).into_widget()
    }
}

impl State for TreeViewState {
    type Widget = TreeView;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        // Programmatic focus shows the focus visual only after keyboard input.
        InputDevice::initialize(app);
        let scroll = ScrollViewportController::new(app);
        app.get_mut(self).scroll = Some(scroll);
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.edge_scroll_tick(app, elapsed)),
        );
        app.get_mut(self).ticker = Some(ticker);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &TreeView) {
        app.get_mut(self).building = true;
        let before = app.get(self).rows.clone();
        let state = app.get_mut(self);
        state.requested_expansion.clear();
        state.requested_selection = None;
        self.sync_tree(app);
        // A selected node the application's change hides is no longer the list's selection.
        if let Some(selected) = self.list_selection(app)
            && before.index_of(&selected).is_some()
            && app.get(self).rows.index_of(&selected).is_none()
        {
            self.after_frame(app, move |app| {
                if self.list_selection(app).as_deref() == Some(selected.as_str())
                    && app.get(self).rows.index_of(&selected).is_none()
                {
                    self.set_list_selection(app, None);
                }
            });
        }
        self.drag_tree_changed(app);
        app.get_mut(self).building = false;
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_drag(app);
        let state = app.get_mut(self);
        let scroll = state.scroll.take().expect("an initialized TreeView");
        let ticker = state.ticker.take().expect("an initialized TreeView");
        let nodes: Vec<_> = state.item_focus.drain().map(|(_, node)| node).collect();
        for node in nodes {
            node.dispose(app);
            app.destroy(node);
        }
        ticker.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        scroll.dispose(app);
        app.destroy(scroll);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let estimate = estimated_row_height(&ThemeResources::of(app, context));
        app.get_mut(self).row_estimate = estimate;
        let list = LayoutBuilder::new(move |app, _, constraints| {
            self.list(app, constraints.max_height.is_infinite())
        });
        let target = self.drop_target(app, list.into_widget());
        Focus::new(target)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.on_key(app, event)))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_is_built_collapsed_without_children() {
        let node = TreeViewNode::text("a", "A");
        assert!(!node.is_expanded && node.children.is_empty() && !node.has_unrealized_children);
        let node = node
            .children(vec![TreeViewNode::text("b", "B")])
            .is_expanded(true);
        assert_eq!(node.children.len(), 1);
        assert!(node.is_expanded);
    }

    #[test]
    fn a_tree_defaults_to_single_selection_with_drag_and_reorder() {
        let tree = TreeView::new(Vec::new(), Vec::new(), |_, _| {});
        assert_eq!(tree.selection_mode, TreeViewSelectionMode::Single);
        assert!(tree.can_drag_items && tree.can_reorder_items && tree.allow_drop);
        assert!(tree.is_enabled);
    }
}
