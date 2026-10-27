//! TreeView: a layers panel of a few hundred nodes to expand, select and reorder by dragging or
//! with Shift+Alt and the arrow keys; the same layers with a check box on each row for selecting
//! several; and a disabled tree. The page owns the layers, their expansion and the selection,
//! and applies the moves the trees ask for.

use crate::{column, example, label, section};
use inset_foundation::{App, Handle};
use inset_widgets::*;
use inset_winui::*;

/// Builds the TreeView feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("TreeView", resources, TreeViewDemo.into_widget())
}

/// The pages of the design, the groups on each and the layers in each group: 318 nodes.
const ARTBOARDS: [&str; 6] = ["Home", "Pricing", "Gallery", "About", "Blog", "Contact"];
const GROUPS: [&str; 4] = ["Header", "Hero", "Content", "Footer"];
const LAYERS: [&str; 12] = [
    "Logo",
    "Title",
    "Subtitle",
    "Button",
    "Image",
    "Caption",
    "Divider",
    "Icon",
    "Link",
    "Background",
    "Shadow",
    "Badge",
];

/// A layer of the design, which the page owns.
#[derive(Clone, Debug)]
struct Layer {
    /// The node's id: the layer's path.
    id: String,
    /// What the row shows.
    name: String,
    /// The layers in it.
    children: Vec<Layer>,
    /// Whether its row shows its children.
    expanded: bool,
}

/// The design's layers, with the first page and its header open.
fn layers() -> Vec<Layer> {
    ARTBOARDS
        .iter()
        .enumerate()
        .map(|(page, artboard)| {
            let page_id = artboard.to_lowercase();
            Layer {
                id: page_id.clone(),
                name: (*artboard).to_owned(),
                expanded: page == 0,
                children: GROUPS
                    .iter()
                    .enumerate()
                    .map(|(group, name)| {
                        let group_id = format!("{page_id}/{}", name.to_lowercase());
                        Layer {
                            id: group_id.clone(),
                            name: (*name).to_owned(),
                            expanded: page == 0 && group == 0,
                            children: LAYERS
                                .iter()
                                .map(|layer| Layer {
                                    id: format!("{group_id}/{}", layer.to_lowercase()),
                                    name: (*layer).to_owned(),
                                    expanded: false,
                                    children: Vec::new(),
                                })
                                .collect(),
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The tree's nodes for `layers`.
fn nodes(layers: &[Layer]) -> Vec<TreeViewNode> {
    layers
        .iter()
        .map(|layer| {
            TreeViewNode::text(layer.id.clone(), layer.name.clone())
                .is_expanded(layer.expanded)
                .children(nodes(&layer.children))
        })
        .collect()
}

/// The layer `id`.
fn find<'a>(layers: &'a mut [Layer], id: &str) -> Option<&'a mut Layer> {
    for layer in layers.iter_mut() {
        if layer.id == id {
            return Some(layer);
        }
        if let Some(found) = find(&mut layer.children, id) {
            return Some(found);
        }
    }
    None
}

/// Takes the layer `id` out of the layers.
fn take(layers: &mut Vec<Layer>, id: &str) -> Option<Layer> {
    if let Some(index) = layers.iter().position(|layer| layer.id == id) {
        return Some(layers.remove(index));
    }
    layers
        .iter_mut()
        .find_map(|layer| take(&mut layer.children, id))
}

/// Applies one move the tree asks for.
fn apply_move(layers: &mut Vec<Layer>, node_move: &TreeViewNodeMove) {
    let Some(layer) = take(layers, &node_move.node) else {
        return;
    };
    let siblings = match &node_move.new_parent {
        Some(parent) => match find(layers, parent) {
            Some(parent) => &mut parent.children,
            None => return,
        },
        None => layers,
    };
    let index = node_move.index.min(siblings.len());
    siblings.insert(index, layer);
}

/// Sets the expansion of the layer `id`.
fn set_expanded(layers: &mut [Layer], id: &str, expanded: bool) {
    if let Some(layer) = find(layers, id) {
        layer.expanded = expanded;
    }
}

/// The TreeView examples.
#[derive(Debug)]
struct TreeViewDemo;

/// The layers and selections the examples keep.
struct TreeViewDemoState {
    /// Native widget state.
    state: StateData<TreeViewDemo>,
    /// The layers panel's layers.
    layers: Vec<Layer>,
    /// The layers panel's selection.
    selected: Vec<String>,
    /// The layers with check boxes.
    checked_layers: Vec<Layer>,
    /// Their selection.
    checked: Vec<String>,
    /// The last event.
    event: String,
}

impl StatefulWidget for TreeViewDemo {
    type State = TreeViewDemoState;

    fn create_state(&self) -> Self::State {
        TreeViewDemoState {
            state: StateData::new(),
            layers: layers(),
            selected: vec!["home/header/title".to_owned()],
            checked_layers: layers(),
            checked: vec![
                "home/header/logo".to_owned(),
                "home/header/title".to_owned(),
            ],
            event: "No event yet".to_owned(),
        }
    }
}

/// Which of the examples' trees a handler serves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Panel {
    Layers,
    Checked,
}

impl TreeViewDemoState {
    /// The layers of `panel`.
    fn layers_of(&mut self, panel: Panel) -> &mut Vec<Layer> {
        match panel {
            Panel::Layers => &mut self.layers,
            Panel::Checked => &mut self.checked_layers,
        }
    }

    /// The tree of `panel`, its layers stored as the tree asks.
    fn tree(self: Handle<Self>, app: &App, panel: Panel) -> TreeView {
        let state = app.get(self);
        let (layers, selected, mode) = match panel {
            Panel::Layers => (
                &state.layers,
                &state.selected,
                TreeViewSelectionMode::Single,
            ),
            Panel::Checked => (
                &state.checked_layers,
                &state.checked,
                TreeViewSelectionMode::Multiple,
            ),
        };
        TreeView::new(nodes(layers), selected.clone(), move |app, args| {
            self.set_state(app, |state| {
                match panel {
                    Panel::Layers => state.selected = args.selected_nodes.clone(),
                    Panel::Checked => state.checked = args.selected_nodes.clone(),
                }
                state.event = format!(
                    "SelectionChanged: +{:?} −{:?}",
                    args.added_items, args.removed_items
                );
            })
        })
        .selection_mode(mode)
        .expanding(move |app, args| {
            self.set_state(app, |state| {
                set_expanded(state.layers_of(panel), &args.node, true)
            })
        })
        .collapsed(move |app, args| {
            self.set_state(app, |state| {
                set_expanded(state.layers_of(panel), &args.node, false)
            })
        })
        .move_requested(move |app, args| {
            self.set_state(app, |state| {
                for node_move in &args.moves {
                    apply_move(state.layers_of(panel), node_move);
                }
                let last = args.moves.last().unwrap();
                state.event = format!(
                    "Moved {} to {} at {}",
                    last.node,
                    last.new_parent.as_deref().unwrap_or("the top level"),
                    last.index
                );
            })
        })
        .item_invoked(move |app, args| {
            self.set_state(app, |state| {
                state.event = format!("ItemInvoked: {}", args.invoked_item)
            })
        })
        .drag_items_completed(move |app, args| {
            self.set_state(app, |state| {
                state.event = format!(
                    "DragItemsCompleted: {:?} {:?} into {}",
                    args.drop_result,
                    args.items,
                    args.new_parent_item.as_deref().unwrap_or("the top level")
                )
            })
        })
    }
}

impl State for TreeViewDemoState {
    type Widget = TreeViewDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let secondary = r.text_fill_color_secondary;
        let panel = |tree: TreeView| {
            SizedBox::new()
                .width(360.0)
                .height(420.0)
                .child(tree)
                .into_widget()
        };
        let state = app.get(self);
        let selected = state.selected.first().cloned();
        let checked = state.checked.len();
        let event = state.event.clone();
        let disabled = TreeView::new(
            vec![
                TreeViewNode::text("assets", "Assets")
                    .is_expanded(true)
                    .children(vec![
                        TreeViewNode::text("assets/icons", "Icons"),
                        TreeViewNode::text("assets/photos", "Photos"),
                    ]),
                TreeViewNode::text("styles", "Styles"),
            ],
            vec!["assets/photos".to_owned()],
            |_, _| {},
        )
        .is_enabled(false);
        column(
            vec![
                example(
                    "A layers panel",
                    "Click a layer to select it, or its chevron to open it. Drag a layer onto another to put it inside, or between two to reorder; hover a closed group to open it. With the keyboard, the arrows move and open, Enter invokes, and Shift+Alt with Up or Down reorders.",
                    &r,
                    column(
                        vec![
                            panel(self.tree(app, Panel::Layers)),
                            label(
                                format!("Selected: {}", selected.as_deref().unwrap_or("nothing")),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Multiple selection",
                    "Check a group to check everything in it; a group with some of its layers checked is partly checked. Dragging a checked layer moves every checked layer.",
                    &r,
                    column(
                        vec![
                            panel(
                                self.tree(app, Panel::Checked)
                                    .selection_mode(TreeViewSelectionMode::Multiple),
                            ),
                            label(
                                format!("{checked} layers checked"),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "A disabled TreeView",
                    "",
                    &r,
                    SizedBox::new()
                        .width(360.0)
                        .height(120.0)
                        .child(disabled)
                        .into_widget(),
                ),
                label(event, TextBlockStyle::Caption, secondary),
            ],
            20.0,
        )
    }
}
