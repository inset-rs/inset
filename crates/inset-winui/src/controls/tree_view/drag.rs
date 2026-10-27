//! TreeViewList.cpp's drag and drop of nodes, with what it relies on in ListViewBase
//! (ListViewBase_Partial_Reorder.cpp), ListViewBaseItem_Partial.cpp, TreeViewItem.cpp and
//! LiveReorderHelper.cpp. Inset's `Draggable` carries the drag from a row, and a `DragTarget`
//! over the list receives it.
//!
//! The drag reaches the tree's elements as the source's drag events do:
//! - The row drawn under the pointer has DragEnter, DragOver and DragLeave first, which
//!   TreeViewItem handles: a row accepts a drag of nodes it is not under, and a collapsed row
//!   with children hovered for a second expands. Over the middle three fifths of a row,
//!   ListViewBaseItem takes the drag there: the row looks pointed at.
//! - Elsewhere the list has the drag: TreeViewList finds the empty slot the nodes would move to,
//!   and ListViewBase opens a gap there after 200 ms and scrolls near the list's edges.
//! - The drop goes where the last drag over did: into the row under the pointer when that row
//!   accepted the drag, else, over no row, as in the gap, to the empty slot.

use super::item::{ItemDrag, PressedPart};
use super::view_model::MovePlan;
use super::{TreeViewSelectionMode, TreeViewState};
use crate::*;
use inset_embedder::{Offset, Rect};
use inset_foundation::{App, Handle, Listener, Timer};
use inset_gestures::{AnyGestureRecognizer, GestureMultiDragStartCallback};
use inset_widgets::*;
use std::{cell::Cell, rc::Rc, time::Duration};

/// XAML `DataPackageOperation`, as a drag within the tree ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DataPackageOperation {
    /// The drop moved nothing, or the drag ended outside the tree.
    #[default]
    None,
    /// The drop moved the nodes.
    Move,
}

/// XAML `TreeViewDragItemsStartingEventArgs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewDragItemsStartingEventArgs {
    /// `Items`: the node the drag starts on, or in Multiple mode with several nodes selected,
    /// `SelectedNodes`. `DragItemsCompleted` reports them again.
    pub items: Vec<String>,

    /// `Cancel`: set it to keep the drag from starting.
    pub cancel: bool,
}

/// XAML `TreeViewDragItemsCompletedEventArgs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeViewDragItemsCompletedEventArgs {
    /// `DropResult`.
    pub drop_result: DataPackageOperation,

    /// `Items`, as `DragItemsStarting` left them.
    pub items: Vec<String>,

    /// `NewParentItem`: the parent of the first of `items` after the drop; `None` for a root
    /// node.
    pub new_parent_item: Option<String>,
}

/// `LISTVIEWITEM_DRAGOVER_OUTSIDE_MARGIN`: the share of a row at each end outside its middle,
/// where the list has the drag.
const DRAG_OVER_OUTSIDE_MARGIN: f64 = 0.2;

/// `LISTVIEW_LIVEREORDER_TIMER`: how long the drag stays at a slot before the gap opens.
const LIVE_REORDER_DELAY: Duration = Duration::from_millis(200);

/// `c_dragOverInterval`: how long a drag hovers a collapsed row before it expands.
const DRAG_OVER_EXPAND_DELAY: Duration = Duration::from_secs(1);

/// `LISTVIEWBASE_EDGE_SCROLL_EDGE_WIDTH_PX`, `_MIN_SPEED`, `_MAX_SPEED` and
/// `_START_DELAY_MSEC`: edge scrolling, in pixels and pixels per second.
const EDGE_SCROLL_EDGE_WIDTH: f64 = 100.0;
const EDGE_SCROLL_MIN_SPEED: f64 = 150.0;
const EDGE_SCROLL_MAX_SPEED: f64 = 1500.0;
const EDGE_SCROLL_START_DELAY: Duration = Duration::from_millis(50);

/// `ScrollViewerScrollRoundingTolerance`.
const SCROLL_ROUNDING_TOLERANCE: f64 = 0.05;

/// What Inset's drag carries from a row.
#[derive(Clone, Debug)]
pub(super) struct TreeDragData {
    /// The tree the drag started in.
    source: Handle<TreeViewState>,
    /// The node the drag started on.
    id: String,
    /// Where the pointer is on the preview, which turns the preview's place into the pointer's.
    anchor: Rc<Cell<Offset>>,
}

/// The drag state of TreeViewList and the ListViewBase under it.
#[derive(Default)]
pub(super) struct TreeDragState {
    /// `m_tpPrimaryDraggedContainer`: the node the drag started on, while the list is in it.
    primary: Option<String>,
    /// `m_draggedTreeViewNode`, which TreeViewList's `DragItemsStarting` handler sets.
    dragged: Option<String>,
    /// The drag's items, as `DragItemsStarting` left them.
    items: Vec<String>,
    /// `TreeViewItemTemplateSettings.DragItemsCount` of a drag of several selected nodes, in
    /// `MultipleDraggingPrimary`.
    multiple: Option<usize>,
    /// `m_isDraggingOverSelf`.
    over_self: bool,
    /// The row the last drag over reached: `m_dragEnterDO`.
    entered: Option<String>,
    /// `m_tpDragOverItem`: the row whose middle the drag is over.
    drag_over_item: Option<String>,
    /// The accepted operation, carried from one drag event to the next: Move when true.
    accepted: bool,
    /// `m_emptySlotIndex`.
    empty_slot: Option<usize>,
    /// `m_liveReorderIndices.draggedOverIndex`.
    live_index: Option<usize>,
    /// The drag-over index whose gap shows: `m_movedItems`.
    live_shown: Option<usize>,
    /// `m_tpLiveReorderTimer`.
    live_timer: Option<Timer>,
    /// TreeViewItem's `m_expandContentTimer`, of the entered row.
    expand_timer: Option<Timer>,
    /// Where the pointer last was, in global coordinates.
    position: Option<Offset>,
    /// `m_currentAutoPanVelocity`: pixels per second toward the start of the list.
    velocity: f64,
    /// `m_pendingAutoPanVelocity`.
    pending_velocity: f64,
    /// `m_tpStartEdgeScrollTimer`.
    edge_timer: Option<Timer>,
    /// The edge scroll's last frame.
    last_tick: Option<Duration>,
    /// The drop's accepted operation.
    result: DataPackageOperation,
}

/// `GetDragOverIndex`: the row whose slot the gap takes, which moves past a row only once the
/// drag is past the row's middle.
fn drag_over_index(closest: usize, insertion: usize, previous: usize) -> usize {
    if insertion == closest {
        if previous < insertion {
            closest - 1
        } else {
            closest
        }
    } else if previous >= insertion {
        closest + 1
    } else {
        closest
    }
}

/// `ComputeEdgeScrollVelocityFromEdgeDistance`.
fn edge_scroll_speed(distance: f64) -> f64 {
    if distance <= EDGE_SCROLL_EDGE_WIDTH {
        EDGE_SCROLL_MAX_SPEED
            - distance / EDGE_SCROLL_EDGE_WIDTH * (EDGE_SCROLL_MAX_SPEED - EDGE_SCROLL_MIN_SPEED)
    } else {
        0.0
    }
}

/// Whether `y` is in the middle of `rect`, outside `DRAG_OVER_OUTSIDE_MARGIN` at either end:
/// `ListViewBaseItem::IsDragOver`.
fn is_drag_over(rect: Rect, y: f64) -> bool {
    let location = y - rect.top;
    let size = rect.height();
    location > DRAG_OVER_OUTSIDE_MARGIN * size && location < (1.0 - DRAG_OVER_OUTSIDE_MARGIN) * size
}

/// The draggable around a row, whose start TreeViewList can decline.
#[derive(Clone, Debug)]
struct TreeViewItemDraggable {
    draggable: Draggable<TreeDragData>,
}

impl DraggableWidget for TreeViewItemDraggable {
    type Data = TreeDragData;

    fn draggable(&self) -> &Draggable<TreeDragData> {
        &self.draggable
    }

    fn create_recognizer(
        &self,
        app: &mut App,
        on_start: GestureMultiDragStartCallback,
    ) -> AnyGestureRecognizer {
        let data = self.draggable.data.clone().expect("a row's drag data");
        self.draggable.create_recognizer(
            app,
            Rc::new(move |app, position| {
                if !data.source.alive(app) || !data.source.start_drag(app, &data.id) {
                    return None;
                }
                on_start(app, position)
            }),
        )
    }
}

impl StatefulWidget for TreeViewItemDraggable {
    type State = DraggableState<Self>;

    fn create_state(&self) -> Self::State {
        DraggableState::new()
    }
}

impl TreeViewState {
    /// `IsInDragDrop`: a drag started on one of the rows.
    pub(super) fn is_in_drag(self: Handle<Self>, app: &App) -> bool {
        app.get(self).drag.primary.is_some()
    }

    /// What the drag means for the row of `id`; for the drag preview, the row as the drag
    /// visual takes it, before it becomes a placeholder.
    pub(super) fn item_drag(self: Handle<Self>, app: &App, id: &str, preview: bool) -> ItemDrag {
        let drag = &app.get(self).drag;
        let primary = drag.primary.as_deref() == Some(id);
        ItemDrag {
            in_drag: drag.primary.is_some(),
            primary,
            captured: !preview,
            dragged_over: !preview && drag.drag_over_item.as_deref() == Some(id),
            can_reorder: self.widget(app).can_reorder_items,
            drag_items_count: drag.items.len(),
            multiple_dragging_primary: primary && drag.multiple.is_some(),
            drag_items_count_display: drag.multiple.unwrap_or(0),
        }
    }

    /// How far the gap of a live reorder moves the row at `row`: each row between the dragged
    /// one and the drag-over index takes its neighbour's slot toward the dragged one.
    fn displacement(self: Handle<Self>, app: &App, row: usize) -> f64 {
        let state = app.get(self);
        let (Some(primary), Some(shown)) = (&state.drag.primary, state.drag.live_shown) else {
            return 0.0;
        };
        let Some(dragged) = state.rows.index_of(primary) else {
            return 0.0;
        };
        if dragged < row && row <= shown {
            -self.row_height(app, state.rows.get(row - 1).unwrap())
        } else if shown <= row && row < dragged {
            self.row_height(app, state.rows.get(row).unwrap())
        } else {
            0.0
        }
    }

    /// The row at `row` as the list draws it: displaced by a live reorder, and draggable while
    /// the tree can drag or reorder.
    pub(super) fn drag_source(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        row: usize,
        container: super::item::TreeViewItemContainer,
    ) -> WidgetRef {
        let displaced = Transform::translate(Offset::new(0.0, self.displacement(app, row)))
            .child(container)
            .into_widget();
        let widget = self.widget(app).clone();
        if !widget.is_enabled || !(widget.can_drag_items || widget.can_reorder_items) {
            return displaced;
        }
        let anchor = Rc::new(Cell::new(Offset::ZERO));
        let data = TreeDragData {
            source: self,
            id: id.to_owned(),
            anchor: anchor.clone(),
        };
        let resources = ThemeResources::of(app, self.context(app));
        let preview = id.to_owned();
        // The preview is a widget of its own, in the overlay, where the tree's theme does not
        // reach.
        let feedback = ThemeScope::with_resources(
            resources,
            Builder::new(move |app, _| self.drag_preview(app, &preview)),
        );
        let draggable = Draggable::new(displaced, feedback)
            .data(data)
            .max_simultaneous_drags(1)
            .drag_anchor_strategy(Rc::new(move |app, draggable, context, position| {
                let offset = child_drag_anchor_strategy(app, draggable, context, position);
                anchor.set(offset);
                offset
            }))
            .on_drag_completed(Listener::new(move |app| self.drag_ended(app, true)))
            .on_draggable_canceled(Rc::new(move |app, _, _| self.drag_ended(app, false)));
        TreeViewItemDraggable { draggable }.into_widget()
    }

    /// The drag visual: the row as it was when the drag started, at its width.
    fn drag_preview(self: Handle<Self>, app: &mut App, id: &str) -> WidgetRef {
        if !self.alive(app) || !app.get(self).tree.contains(id) {
            return SizedBox::shrink().into_widget();
        }
        let width = app.get(self).widths.get(id).copied().unwrap_or(0.0);
        let container = self.container(app, id, None);
        SizedBox::new().width(width).child(container).into_widget()
    }

    /// `ListViewBase::OnDragGesture` for a drag starting on the row of `id`, with TreeViewList's
    /// `OnDragItemsStarting` and TreeView's `DragItemsStarting`: whether the drag starts.
    fn start_drag(self: Handle<Self>, app: &mut App, id: &str) -> bool {
        let widget = self.widget(app).clone();
        if self.is_in_drag(app)
            || !widget.is_enabled
            || !(widget.can_drag_items || widget.can_reorder_items)
            || self.pressed_part_is(app, id, PressedPart::Chevron)
            || self.pressed_part_is(app, id, PressedPart::CheckBox)
            || app.get(self).rows.index_of(id).is_none()
        {
            return false;
        }
        let mut items = vec![id.to_owned()];
        // `DragItemsStarting` is raised only for a tree that can drag items.
        if widget.can_drag_items {
            app.get_mut(self).drag.dragged = Some(id.to_owned());
            if self.is_multi_select_with_selected_items(app) {
                let selected = self.selected_nodes(app);
                if selected.len() > 1 {
                    app.get_mut(self).drag.multiple = Some(selected.len());
                    items = selected;
                }
            } else {
                // Outside a selection of several, the dragged node collapses.
                self.set_node_expanded(app, id, false);
            }
            let mut args = TreeViewDragItemsStartingEventArgs {
                items,
                cancel: false,
            };
            if let Some(handler) = widget.drag_items_starting.clone() {
                handler(app, &mut args);
            }
            if args.cancel || !self.alive(app) {
                let drag = &mut app.get_mut(self).drag;
                drag.dragged = None;
                drag.multiple = None;
                return false;
            }
            items = args.items;
        }
        let drag = &mut app.get_mut(self).drag;
        drag.primary = Some(id.to_owned());
        drag.items = items;
        drag.over_self = false;
        drag.accepted = false;
        drag.result = DataPackageOperation::None;
        self.set_state(app, |_| {});
        true
    }

    /// The drag ended: dropped on the list, or released elsewhere. `CompleteDrop` ends the
    /// list's drag, then `DragItemsCompleted` is raised for a tree that can drag items, after
    /// TreeViewList forgets the dragged node.
    fn drag_ended(self: Handle<Self>, app: &mut App, dropped: bool) {
        if !self.alive(app) || !self.is_in_drag(app) {
            return;
        }
        let result = if dropped {
            app.get(self).drag.result
        } else {
            DataPackageOperation::None
        };
        self.clear_drag_over(app);
        let drag = std::mem::take(&mut app.get_mut(self).drag);
        let widget = self.widget(app).clone();
        if widget.can_drag_items
            && let Some(handler) = widget.drag_items_completed
        {
            let tree = app.get(self).tree.clone();
            let new_parent_item = drag
                .items
                .first()
                .filter(|id| tree.contains(id))
                .and_then(|id| tree.parent(id).map(str::to_owned));
            handler(
                app,
                TreeViewDragItemsCompletedEventArgs {
                    drop_result: result,
                    items: drag.items,
                    new_parent_item,
                },
            );
        }
        self.set_state(app, |_| {});
    }

    /// The list as the drag's target.
    pub(super) fn drop_target(self: Handle<Self>, _app: &mut App, child: WidgetRef) -> WidgetRef {
        DragTarget::<TreeDragData>::new(move |_, _, _, _| child.clone())
            .on_will_accept_with_details(Rc::new(move |app, details| {
                self.drag_entered(app, &details.data)
            }))
            .on_move(Rc::new(move |app, details| {
                if details.data.source == self && self.alive(app) {
                    let position = details.offset + details.data.anchor.get();
                    self.drag_over(app, position);
                }
            }))
            .on_leave(Rc::new(move |app, data| {
                if data.is_some_and(|data| data.source == self) && self.alive(app) {
                    self.drag_left(app);
                }
            }))
            .on_accept_with_details(Rc::new(move |app, details| {
                if details.data.source == self && self.alive(app) {
                    let position = details.offset + details.data.anchor.get();
                    self.drop(app, position);
                }
            }))
            .into_widget()
    }

    /// The drag entered the list: `ListViewBase::OnDragEnter` where the list allows drops. The
    /// tree takes only drags of its own rows.
    fn drag_entered(self: Handle<Self>, app: &mut App, data: &TreeDragData) -> bool {
        if data.source != self || !self.alive(app) {
            return false;
        }
        if self.widget(app).allow_drop {
            let drag = &mut app.get_mut(self).drag;
            drag.over_self = drag.primary.is_some();
        }
        true
    }

    /// The drag left the list: the entered row's DragLeave, `TreeViewList::OnDragLeave` and
    /// `ListViewBase::OnDragLeave`.
    fn drag_left(self: Handle<Self>, app: &mut App) {
        if let Some(entered) = app.get_mut(self).drag.entered.take() {
            self.row_drag_left(app, &entered);
        }
        let drag = &mut app.get_mut(self).drag;
        // No element that accepted the drag is under it any more.
        drag.accepted = false;
        drag.empty_slot = None;
        drag.position = None;
        if self.widget(app).allow_drop {
            self.set_pending_velocity(app, 0.0);
            self.reset_live_reorder(app);
            app.get_mut(self).drag.over_self = false;
        }
        self.set_state(app, |_| {});
    }

    /// One drag over at `position`: the row drawn there has it first, then the list where the
    /// row leaves it.
    fn drag_over(self: Handle<Self>, app: &mut App, position: Offset) {
        if !self.is_in_drag(app) {
            return;
        }
        app.get_mut(self).drag.position = Some(position);
        let hit = self.row_drawn_at(app, position);
        let hit_id = hit.as_ref().map(|(_, id, _)| id.clone());
        if hit_id != app.get(self).drag.entered {
            if let Some(entered) = app.get_mut(self).drag.entered.take() {
                self.row_drag_left(app, &entered);
            }
            app.get_mut(self).drag.entered = hit_id.clone();
            if let Some(id) = &hit_id {
                self.row_drag_entered(app, id);
            }
        }
        let mut handled = false;
        if let Some((_, id, rect)) = hit {
            self.row_drag_over(app, &id);
            // `ListViewBaseItem::OnDragOver`: the middle of the row takes the drag.
            handled = is_drag_over(rect, position.dy());
            app.get_mut(self).drag.drag_over_item = handled.then_some(id);
        }
        if !handled && self.widget(app).allow_drop {
            self.list_drag_over(app, position);
        }
        self.set_state(app, |_| {});
    }

    /// `TreeViewItem::OnDragEnter`: the row accepts a drag of nodes it is not within, and a
    /// collapsed row with children other than the dragged one starts its expand timer.
    fn row_drag_entered(self: Handle<Self>, app: &mut App, id: &str) {
        app.get_mut(self).drag.accepted = false;
        let Some(dragged) = app.get(self).drag.dragged.clone() else {
            return;
        };
        if !self.widget(app).can_reorder_items {
            return;
        }
        let tree = app.get(self).tree.clone();
        if !tree.is_within(id, &dragged) {
            app.get_mut(self).drag.accepted = true;
        }
        if id != dragged && !tree.is_expanded(id) && tree.has_children(id) {
            if let Some(timer) = app.get_mut(self).drag.expand_timer.take() {
                timer.cancel(app);
            }
            let row = id.to_owned();
            let timer = Timer::new(
                app,
                DRAG_OVER_EXPAND_DELAY,
                Listener::new(move |app| {
                    if self.alive(app) {
                        self.expand_on_hover(app, &row);
                    }
                }),
            );
            app.get_mut(self).drag.expand_timer = Some(timer);
        }
    }

    /// `TreeViewItem::OnExpandContentTimerTick`: the hovered row expands, and the drag goes over
    /// what is now under it.
    fn expand_on_hover(self: Handle<Self>, app: &mut App, id: &str) {
        app.get_mut(self).drag.expand_timer = None;
        if !app.get(self).tree.contains(id) || app.get(self).tree.is_expanded(id) {
            return;
        }
        self.set_node_expanded(app, id, true);
        if let Some(position) = app.get(self).drag.position {
            self.drag_over(app, position);
        }
    }

    /// The row's DragLeave: TreeViewItem stops its expand timer and ListViewBaseItem stops being
    /// dragged over.
    fn row_drag_left(self: Handle<Self>, app: &mut App, _id: &str) {
        if let Some(timer) = app.get_mut(self).drag.expand_timer.take() {
            timer.cancel(app);
        }
        app.get_mut(self).drag.drag_over_item = None;
    }

    /// `TreeViewItem::OnDragOver`: in Multiple mode with a selection, the row accepts unless it
    /// is selected; otherwise it accepts a drag of nodes it is not within, and leaves the
    /// operation as it was for one it is.
    fn row_drag_over(self: Handle<Self>, app: &mut App, id: &str) {
        let Some(dragged) = app.get(self).drag.dragged.clone() else {
            return;
        };
        if !self.widget(app).can_reorder_items {
            return;
        }
        if self.is_multi_select_with_selected_items(app) {
            app.get_mut(self).drag.accepted = !self.is_node_selected(app, id);
        } else if !app.get(self).tree.is_within(id, &dragged) {
            app.get_mut(self).drag.accepted = true;
        }
    }

    /// `TreeViewList::OnDragOver`, then `ListViewBase::OnDragOver` when TreeViewList leaves it.
    fn list_drag_over(self: Handle<Self>, app: &mut App, position: Offset) {
        let widget = self.widget(app).clone();
        app.get_mut(self).drag.accepted = false;
        if app.get(self).drag.dragged.is_some() && widget.can_reorder_items {
            let slot = self.empty_slot(app, position);
            let rows = app.get(self).rows.clone();
            let allow = match slot.and_then(|slot| rows.get(slot)) {
                // A drop beside a selected node would move it among itself.
                Some(insert_at) if widget.selection_mode == TreeViewSelectionMode::Multiple => {
                    !self.is_node_selected(app, insert_at)
                }
                Some(_) => true,
                None => false,
            };
            let drag = &mut app.get_mut(self).drag;
            if allow {
                drag.empty_slot = slot;
                drag.accepted = true;
            } else {
                // Refused, and handled: ListViewBase neither reorders nor scrolls.
                drag.empty_slot = None;
                return;
            }
        }
        self.process_live_reorder(app, position);
        // `DropCausesReorder`.
        let drag = &app.get(self).drag;
        if drag.over_self && widget.allow_drop && widget.can_reorder_items {
            app.get_mut(self).drag.accepted = true;
        }
        let velocity = self.edge_scroll_velocity(app, position);
        self.set_pending_velocity(app, velocity);
    }

    /// TreeViewList's empty slot: the row index the dragged node would take, from
    /// `GetInsertionIndexes` and which half of the row beside the insertion the pointer is in.
    fn empty_slot(self: Handle<Self>, app: &mut App, position: Offset) -> Option<usize> {
        let size = app.get(self).rows.len();
        let below = self.insertion_index(app, position)?;
        if below > size.checked_sub(1)? {
            return Some(size - 1);
        }
        if below == 0 {
            return Some(0);
        }
        let dragged = app.get(self).drag.dragged.clone()?;
        let dragged_row = app.get(self).rows.index_of(&dragged)?;
        let row = if dragged_row < below {
            below
        } else {
            below - 1
        };
        let rect = self.row_drawn_rect(app, row)?;
        Some(if position.dy() - rect.top < rect.height() / 2.0 {
            below - 1
        } else {
            below
        })
    }

    /// `ProcessDragOverAt`'s live reorder: the drag-over index follows the pointer, and the gap
    /// moves to it once it has stayed for `LIVE_REORDER_DELAY`.
    fn process_live_reorder(self: Handle<Self>, app: &mut App, position: Offset) {
        let widget = self.widget(app).clone();
        let drag = &app.get(self).drag;
        // `ShouldPlayLiveReorder`: not while the list edge-scrolls.
        if !widget.can_reorder_items || !widget.allow_drop || drag.velocity != 0.0 {
            return;
        }
        let Some(primary) = drag.primary.clone() else {
            return;
        };
        let count = app.get(self).rows.len();
        let Some(dragged) = app.get(self).rows.index_of(&primary) else {
            return;
        };
        let (Some(closest), Some(insertion)) = (
            self.closest_row(app, position),
            self.insertion_index(app, position),
        ) else {
            return;
        };
        let previous = app.get(self).drag.live_index.unwrap_or(dragged);
        let index = if insertion == count {
            count
        } else {
            drag_over_index(closest, insertion, previous)
        };
        app.get_mut(self).drag.live_index = Some(index);
        if previous == dragged || previous != index {
            if let Some(timer) = app.get_mut(self).drag.live_timer.take() {
                timer.cancel(app);
            }
            let timer = Timer::new(
                app,
                LIVE_REORDER_DELAY,
                Listener::new(move |app| {
                    if self.alive(app) && self.is_in_drag(app) {
                        self.show_live_reorder(app);
                    }
                }),
            );
            app.get_mut(self).drag.live_timer = Some(timer);
        }
    }

    /// `LiveReorderTimerTickHandler`: the gap moves to the drag-over index. The drag then goes
    /// over what the gap put under the pointer, as the system's next drag over would.
    fn show_live_reorder(self: Handle<Self>, app: &mut App) {
        let drag = &mut app.get_mut(self).drag;
        drag.live_timer = None;
        drag.live_shown = drag.live_index;
        let position = drag.position;
        self.set_state(app, |_| {});
        if let Some(position) = position {
            self.drag_over(app, position);
        }
    }

    /// `ResetAllItemsForLiveReorder`.
    fn reset_live_reorder(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).drag.live_timer.take() {
            timer.cancel(app);
        }
        let drag = &mut app.get_mut(self).drag;
        drag.live_index = None;
        drag.live_shown = None;
    }

    /// `ComputeEdgeScrollVelocity`: toward the start within the edge width of the list's top,
    /// toward the end within it of its bottom, faster closer to the edge; none against an end.
    fn edge_scroll_velocity(self: Handle<Self>, app: &mut App, position: Offset) -> f64 {
        let Some(list) = self.viewport_rect(app) else {
            return 0.0;
        };
        let metrics = self.scroll(app).metrics(app);
        let y = position.dy() - list.top;
        let mut velocity = edge_scroll_speed(y);
        let mut bound = 0.0;
        if velocity == 0.0 {
            velocity = -edge_scroll_speed(list.height() - y);
            bound = metrics.scrollable_length;
        }
        if (bound - metrics.offset).abs() < SCROLL_ROUNDING_TOLERANCE {
            velocity = 0.0;
        }
        velocity
    }

    /// `SetPendingAutoPanVelocity`: a scroll starts after `EDGE_SCROLL_START_DELAY`, changes
    /// speed at once, and stops at once.
    fn set_pending_velocity(self: Handle<Self>, app: &mut App, velocity: f64) {
        if velocity == 0.0 {
            let drag = &mut app.get_mut(self).drag;
            drag.velocity = 0.0;
            drag.pending_velocity = 0.0;
            if let Some(timer) = drag.edge_timer.take() {
                timer.cancel(app);
            }
            if let Some(ticker) = app.get(self).ticker
                && ticker.is_active(app)
            {
                ticker.stop(app, false);
            }
            return;
        }
        if app.get(self).drag.velocity != 0.0 {
            app.get_mut(self).drag.velocity = velocity;
            return;
        }
        app.get_mut(self).drag.pending_velocity = velocity;
        if app.get(self).drag.edge_timer.is_none() {
            let timer = Timer::new(
                app,
                EDGE_SCROLL_START_DELAY,
                Listener::new(move |app| {
                    if self.alive(app) {
                        self.start_edge_scroll(app);
                    }
                }),
            );
            app.get_mut(self).drag.edge_timer = Some(timer);
        }
    }

    /// `StartEdgeScrollTimerTick`: the pending velocity becomes the scroll's.
    fn start_edge_scroll(self: Handle<Self>, app: &mut App) {
        let drag = &mut app.get_mut(self).drag;
        drag.edge_timer = None;
        drag.velocity = std::mem::take(&mut drag.pending_velocity);
        drag.last_tick = None;
        if drag.velocity != 0.0
            && let Some(ticker) = app.get(self).ticker
            && !ticker.is_active(app)
        {
            ticker.start(app);
        }
    }

    /// One frame of edge scrolling at the current velocity. The source's drag keeps reporting
    /// where it is while the list scrolls under it.
    pub(super) fn edge_scroll_tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        let drag = &mut app.get_mut(self).drag;
        let delta = elapsed
            .saturating_sub(drag.last_tick.unwrap_or(elapsed))
            .as_secs_f64();
        drag.last_tick = Some(elapsed);
        let velocity = drag.velocity;
        let position = drag.position;
        if velocity == 0.0 {
            return;
        }
        let scroll = self.scroll(app);
        scroll.scroll_by(app, -velocity * delta, true);
        if let Some(position) = position {
            self.drag_over(app, position);
        }
    }

    /// The drop at `position`: the system's drag over there, then `TreeViewItem::OnDrop` on the
    /// row the drag over reached, and `TreeViewList::OnDrop` where the row leaves it.
    fn drop(self: Handle<Self>, app: &mut App, position: Offset) {
        self.drag_over(app, position);
        let widget = self.widget(app).clone();
        let tree = app.get(self).tree.clone();
        let rows = app.get(self).rows.clone();
        let drag = &app.get(self).drag;
        let (entered, dragged, slot) =
            (drag.entered.clone(), drag.dragged.clone(), drag.empty_slot);
        let mut accepted = drag.accepted;
        let selected_roots = || tree.roots_of_selected_subtrees(&self.selected_nodes(app));
        let mut handled = false;
        let mut plan = MovePlan::default();
        if let Some(target) = &entered
            && accepted
            && let Some(dragged) = &dragged
        {
            if self.is_multi_select_with_selected_items(app) {
                plan = tree.plan_drop_into(&selected_roots(), target);
                handled = true;
            } else if dragged != target {
                plan = tree.plan_drop_into(std::slice::from_ref(dragged), target);
                handled = true;
            } else {
                accepted = false;
            }
        }
        if !handled
            && widget.allow_drop
            && accepted
            && let Some(dragged) = &dragged
            && let Some(slot) = slot
            && let Some(insert_at) = rows.get(slot)
        {
            let nodes = if self.is_multi_select_with_selected_items(app) {
                selected_roots()
            } else {
                vec![dragged.clone()]
            };
            plan = tree.plan_drop_between(&nodes, insert_at, slot);
        }
        app.get_mut(self).drag.result = if accepted {
            DataPackageOperation::Move
        } else {
            DataPackageOperation::None
        };
        self.request_moves(app, plan);
        self.clear_drag_over(app);
    }

    /// Ends what a drag over set: the timers, the gap, edge scrolling and the dragged-over row.
    fn clear_drag_over(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).drag.expand_timer.take() {
            timer.cancel(app);
        }
        self.reset_live_reorder(app);
        self.set_pending_velocity(app, 0.0);
        let drag = &mut app.get_mut(self).drag;
        drag.entered = None;
        drag.drag_over_item = None;
        drag.empty_slot = None;
        drag.over_self = false;
        self.set_state(app, |_| {});
    }

    /// The application rebuilt the tree during a drag: a gap is kept only while the dragged row
    /// is still a row.
    pub(super) fn drag_tree_changed(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        if let Some(primary) = &state.drag.primary
            && state.rows.index_of(primary).is_none()
        {
            let drag = &mut app.get_mut(self).drag;
            drag.live_index = None;
            drag.live_shown = None;
        }
    }

    /// Cancels the drag's timers when the tree goes.
    pub(super) fn dispose_drag(self: Handle<Self>, app: &mut App) {
        let drag = std::mem::take(&mut app.get_mut(self).drag);
        for timer in [drag.live_timer, drag.expand_timer, drag.edge_timer]
            .into_iter()
            .flatten()
        {
            timer.cancel(app);
        }
    }

    // MARK: Geometry

    /// The built rows, in order, with where each is laid out before a gap displaces it.
    fn laid_out_rows(self: Handle<Self>, app: &mut App) -> Vec<(usize, String, Rect)> {
        let rows = app.get(self).rows.clone();
        let mut laid_out: Vec<(usize, String, Rect)> = app
            .get(self)
            .realized
            .clone()
            .into_iter()
            .filter_map(|id| Some((rows.index_of(&id)?, id)))
            .filter_map(|(row, id)| {
                let rect = self.row_rect(app, &id)?;
                Some((row, id, rect))
            })
            .collect();
        laid_out.sort_by_key(|(row, _, _)| *row);
        laid_out
    }

    /// Where the row at `row` is drawn, displaced by a gap, if it is built.
    fn row_drawn_rect(self: Handle<Self>, app: &mut App, row: usize) -> Option<Rect> {
        let id = app.get(self).rows.get(row)?.to_owned();
        let rect = self.row_rect(app, &id)?;
        Some(rect.translate(0.0, self.displacement(app, row)))
    }

    /// The row drawn at `position`. Where a displaced row covers the dragged row's placeholder,
    /// the displaced one is the row.
    fn row_drawn_at(
        self: Handle<Self>,
        app: &mut App,
        position: Offset,
    ) -> Option<(usize, String, Rect)> {
        let primary = app.get(self).drag.primary.clone();
        let hits: Vec<(usize, String, Rect)> = self
            .laid_out_rows(app)
            .into_iter()
            .map(|(row, id, rect)| {
                let rect = rect.translate(0.0, self.displacement(app, row));
                (row, id, rect)
            })
            .filter(|(_, _, rect)| rect.contains(position))
            .collect();
        hits.iter()
            .find(|(_, id, _)| Some(id) != primary.as_ref())
            .or(hits.first())
            .cloned()
    }

    /// `GetInsertionIndex` of the ItemsStackPanel, from where the rows are laid out: the row
    /// under `position`, or the one after it below its middle; the row count below the last.
    fn insertion_index(self: Handle<Self>, app: &mut App, position: Offset) -> Option<usize> {
        let count = app.get(self).rows.len();
        if count == 0 {
            return Some(0);
        }
        let laid_out = self.laid_out_rows(app);
        let y = position.dy();
        for (row, _, rect) in &laid_out {
            if y >= rect.top && y < rect.bottom {
                return Some(if y - rect.top >= rect.height() / 2.0 {
                    row + 1
                } else {
                    *row
                });
            }
        }
        let (first, _, first_rect) = laid_out.first()?;
        let (last, _, _) = laid_out.last()?;
        Some(if y < first_rect.top { *first } else { last + 1 })
    }

    /// `GetClosestElementInfo`: the row under `position`, or the nearest built one.
    fn closest_row(self: Handle<Self>, app: &mut App, position: Offset) -> Option<usize> {
        let laid_out = self.laid_out_rows(app);
        let y = position.dy();
        laid_out
            .iter()
            .find(|(_, _, rect)| y >= rect.top && y < rect.bottom)
            .or_else(|| {
                if y < laid_out.first()?.2.top {
                    laid_out.first()
                } else {
                    laid_out.last()
                }
            })
            .map(|(row, _, _)| *row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gap_moves_past_a_row_only_beyond_its_middle() {
        // Dragging down from row 1: over the top half of row 3, the gap stays at row 2.
        assert_eq!(drag_over_index(3, 3, 2), 2);
        // Over the bottom half of row 3, it takes row 3.
        assert_eq!(drag_over_index(3, 4, 2), 3);
        // Dragging up from row 5: over the bottom half of row 2, it stays at row 3.
        assert_eq!(drag_over_index(2, 3, 3), 3);
        // Over the top half of row 2, it takes row 2.
        assert_eq!(drag_over_index(2, 2, 3), 2);
    }

    #[test]
    fn edge_scrolling_slows_linearly_away_from_the_edge() {
        assert_eq!(edge_scroll_speed(0.0), 1500.0);
        assert_eq!(edge_scroll_speed(50.0), 825.0);
        assert_eq!(edge_scroll_speed(100.0), 150.0);
        assert_eq!(edge_scroll_speed(100.5), 0.0);
    }

    #[test]
    fn the_middle_three_fifths_of_a_row_take_a_drop_into_it() {
        let row = Rect::from_ltwh(0.0, 100.0, 200.0, 30.0);
        assert!(!is_drag_over(row, 105.0));
        assert!(is_drag_over(row, 107.0));
        assert!(is_drag_over(row, 123.0));
        assert!(!is_drag_over(row, 125.0));
    }
}
