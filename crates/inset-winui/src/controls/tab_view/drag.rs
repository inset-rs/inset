//! TabView delegates drag interaction to ListViewBase; Inset supplies the native gesture and feedback.
use super::*;
use inset_embedder::{Offset, Rect};
use inset_foundation::Timer;
use inset_gestures::{AnyGestureRecognizer, GestureMultiDragStartCallback};
use std::cell::Cell;

/// Requests moving a tab within the owner's collection. `new_index` is its index after removal.
#[derive(Clone, Debug)]
pub struct TabViewReorderRequestedEventArgs {
    /// Index in the owner collection before removal.
    pub old_index: usize,

    /// Destination index after removal.
    pub new_index: usize,

    /// Stable item being moved.
    pub item: TabViewItem,

    /// The selected index after the move, preserving the same selected item.
    pub selected_index: Option<usize>,
}

/// TabDragStarting, raised only when CanDragTabs is enabled.
#[derive(Clone, Debug)]
pub struct TabViewTabDragStartingEventArgs {
    /// Item snapshot supplied before the native drag starts.
    pub item: TabViewItem,

    /// Set true to prevent creation of a drag avatar.
    pub cancel: bool,
}

/// TabDragCompleted; `was_accepted` corresponds to a non-None drop result.
#[derive(Clone, Debug)]
pub struct TabViewTabDragCompletedEventArgs {
    /// Item that completed its drag.
    pub item: TabViewItem,

    /// Whether an in-window target accepted the drop.
    pub was_accepted: bool,
}

/// In-window TabStripDragOver/TabStripDrop data. The owner decides whether to accept a cross-strip move.
#[derive(Clone, Debug)]
pub struct TabViewTabStripDropEventArgs {
    /// Strip that supplied the item.
    pub source: Handle<TabViewState>,

    /// Item snapshot to transfer through the owner collection.
    pub item: TabViewItem,

    /// Insertion boundary in the destination collection, before any source removal.
    pub insertion_index: usize,

    /// Global pointer position at the target.
    pub position: Offset,

    /// Whether the destination owner accepts this move.
    pub accepted: bool,
}

impl TabView {
    /// Controls whether drag events and cross-strip transfers are enabled.
    pub fn can_drag_tabs(mut self, value: bool) -> Self {
        self.can_drag_tabs = value;
        self
    }

    /// Controls pointer and Alt+Shift+arrow reordering within this strip.
    pub fn can_reorder_tabs(mut self, value: bool) -> Self {
        self.can_reorder_tabs = value;
        self
    }

    /// Controls whether incoming pointer drops can be accepted.
    pub fn allow_drop_tabs(mut self, value: bool) -> Self {
        self.allow_drop_tabs = value;
        self
    }

    /// Receives a move request; the owner must reorder its collection and reconcile selection.
    pub fn tab_reorder_requested(
        mut self,
        callback: impl Fn(&mut App, TabViewReorderRequestedEventArgs) + 'static,
    ) -> Self {
        self.tab_reorder_requested = Some(Rc::new(callback));
        self
    }
    /// Supplies independent feedback for custom headers with keys or externally owned state.
    pub fn tab_drag_feedback_builder(
        mut self,
        builder: impl Fn(&mut App, &TabViewItem, f64) -> WidgetRef + 'static,
    ) -> Self {
        self.tab_drag_feedback_builder = Some(Rc::new(builder));
        self
    }

    /// Receives a cancelable event before an externally enabled drag starts.
    pub fn tab_drag_starting(
        mut self,
        callback: impl Fn(&mut App, &mut TabViewTabDragStartingEventArgs) + 'static,
    ) -> Self {
        self.tab_drag_starting = Some(Rc::new(callback));
        self
    }

    /// Receives completion of an externally enabled drag.
    pub fn tab_drag_completed(
        mut self,
        callback: impl Fn(&mut App, TabViewTabDragCompletedEventArgs) + 'static,
    ) -> Self {
        self.tab_drag_completed = Some(Rc::new(callback));
        self
    }

    /// Receives a drag that ended without an accepted in-window target.
    pub fn tab_dropped_outside(
        mut self,
        callback: impl Fn(&mut App, TabViewItem) + 'static,
    ) -> Self {
        self.tab_dropped_outside = Some(Rc::new(callback));
        self
    }

    /// Allows the destination owner to accept an incoming item.
    pub fn tab_strip_drag_over(
        mut self,
        callback: impl Fn(&mut App, &mut TabViewTabStripDropEventArgs) + 'static,
    ) -> Self {
        self.tab_strip_drag_over = Some(Rc::new(callback));
        self
    }

    /// Transfers an accepted cross-strip payload to the destination owner.
    pub fn tab_strip_drop(
        mut self,
        callback: impl Fn(&mut App, TabViewTabStripDropEventArgs) + 'static,
    ) -> Self {
        self.tab_strip_drop = Some(Rc::new(callback));
        self
    }
}

/// Payload carried by Inset’s in-window drag avatar.
#[derive(Clone, Debug)]
struct TabDragData {
    /// Original strip, retained as identity for in-window drop ownership.
    source: Handle<TabViewState>,

    /// Item snapshot taken when the drag surface was built.
    item: TabViewItem,

    /// Whether this payload may leave its original strip.
    external: bool,

    /// Measured source width used by feedback and insertion visuals.
    width: f64,

    /// Local pickup offset, used to recover pointer position from the avatar offset.
    anchor: Rc<Cell<Offset>>,
}

/// Transient drag state shared by the source and target strip policies.
#[derive(Default)]
pub(super) struct TabDragState {
    /// Identity currently being dragged out of this strip.
    pub item: Option<String>,

    /// Payload accepted by this strip while the pointer hovers.
    incoming: Option<Rc<TabDragData>>,

    /// Most recent global pointer position for stationary edge scrolling.
    position: Option<Offset>,

    /// Current collection insertion boundary before source removal.
    insertion: Option<usize>,

    /// Insertion boundary displayed after the source hover delay.
    live_insertion: Option<usize>,

    /// Pending 200 ms live-reorder hover delay.
    timer: Option<Timer>,

    /// Native edge scroller associated with this strip’s viewport.
    auto_scroll: Option<Handle<EdgeDraggingAutoScroller>>,
}

/// Allows TabDragStarting to cancel before the native recognizer creates its drag avatar.
#[derive(Clone, Debug)]
struct TabDraggable {
    /// Native drag widget whose recognizer start is cancelable.
    draggable: Draggable<TabDragData>,
}

impl DraggableWidget for TabDraggable {
    type Data = TabDragData;
    fn draggable(&self) -> &Draggable<TabDragData> {
        &self.draggable
    }

    fn create_recognizer(
        &self,
        app: &mut App,
        on_start: GestureMultiDragStartCallback,
    ) -> AnyGestureRecognizer {
        let data = self.draggable.data.as_ref().unwrap().clone();
        self.draggable.create_recognizer(
            app,
            Rc::new(move |app, position| {
                if !app.contains(data.source)
                    || !data.source.mounted(app)
                    || app.get(data.source).drag.item.is_some()
                {
                    return None;
                }
                let widget = data.source.widget(app).clone();
                if !widget
                    .tab_items
                    .iter()
                    .any(|item| item.id == data.item.id && item.is_enabled && item.is_visible)
                {
                    return None;
                }
                if widget.can_drag_tabs {
                    let item = widget
                        .tab_items
                        .iter()
                        .find(|item| item.id == data.item.id)
                        .unwrap()
                        .clone();
                    let mut args = TabViewTabDragStartingEventArgs {
                        item,
                        cancel: false,
                    };
                    if let Some(callback) = widget.tab_drag_starting {
                        callback(app, &mut args);
                    }
                    if args.cancel || !data.source.mounted(app) {
                        return None;
                    }
                }
                on_start(app, position)
            }),
        )
    }
}

impl StatefulWidget for TabDraggable {
    type State = DraggableState<Self>;
    fn create_state(&self) -> Self::State {
        DraggableState::new()
    }
}

impl TabViewState {
    /// Moves only a focused header or close button; page content keeps its own arrow keys.
    pub(super) fn keyboard_reorder(self: Handle<Self>, app: &mut App, forward: bool) -> bool {
        let focused = self.widget(app).tab_items.clone().iter().position(|item| {
            app.get(self)
                .containers
                .get(&item.id)
                .cloned()
                .is_some_and(|container| {
                    container.tab_focus.has_focus(app) || container.close_focus.has_focus(app)
                })
        });
        let Some(from) = focused else {
            return false;
        };
        if self.widget(app).can_reorder_tabs {
            let to = if forward {
                from.checked_add(1)
                    .filter(|next| *next < self.widget(app).tab_items.len())
            } else {
                from.checked_sub(1)
            };
            if let Some(to) = to {
                self.request_reorder(app, from, to);
            }
        }
        true
    }

    /// Preserves selected identity and restores the moved header after owner mutation.
    fn request_reorder(self: Handle<Self>, app: &mut App, from: usize, to: usize) {
        if from == to {
            return;
        }
        let widget = self.effective_widget(app);
        let Some(callback) = widget.tab_reorder_requested else {
            return;
        };
        let Some(item) = widget.tab_items.get(from).cloned() else {
            return;
        };
        let selected = widget.selected_index.map(|selected| {
            if selected == from {
                to
            } else if from < selected && selected <= to {
                selected - 1
            } else if to <= selected && selected < from {
                selected + 1
            } else {
                selected
            }
        });
        let id = item.id.clone();
        let restore_focus = app
            .get(self)
            .containers
            .get(&id)
            .cloned()
            .is_some_and(|container| {
                container.tab_focus.has_focus(app) || container.close_focus.has_focus(app)
            });
        callback(
            app,
            TabViewReorderRequestedEventArgs {
                old_index: from,
                new_index: to,
                item,
                selected_index: selected,
            },
        );
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if app.contains(self)
                    && self.mounted(app)
                    && let Some(index) = self
                        .widget(app)
                        .tab_items
                        .iter()
                        .position(|item| item.id == id)
                {
                    let focus = if restore_focus {
                        app.get(self)
                            .containers
                            .get(&id)
                            .map(|entry| entry.tab_focus)
                    } else {
                        None
                    };
                    self.bring_tab_into_view(app, index, focus);
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    /// Wraps a header in the native drag surface without duplicating owned focus nodes.
    pub(super) fn draggable_header(
        self: Handle<Self>,
        app: &mut App,
        header: TabViewItemHeader,
        width: f64,
    ) -> WidgetRef {
        let widget = self.effective_widget(app);
        let id = header.item.id.clone();
        let draggable = header.item.is_enabled && (widget.can_drag_tabs || widget.can_reorder_tabs);
        let displacement = self.live_displacement(app, &id);
        let child = Transform::translate(Offset::new(displacement, 0.0))
            .transform_hit_tests(false)
            .child(header.clone())
            .into_widget();
        if !draggable {
            return child;
        }
        let data = TabDragData {
            source: self,
            item: header.item.clone(),
            external: widget.can_drag_tabs,
            width,
            anchor: Rc::new(Cell::new(Offset::ZERO)),
        };
        let anchor = data.anchor.clone();
        let resources = ThemeResources::of(app, self.context(app));
        let feedback_builder = widget.tab_drag_feedback_builder.clone();
        let feedback = ThemeScope::with_resources(
            resources.clone(),
            SizedBox::new()
                .width(width)
                .height(resources.tab_view_item_min_height)
                .child(Builder::new(move |app, context| {
                    if let Some(builder) = &feedback_builder {
                        let r = ThemeResources::of(app, context);
                        let color = r.text_fill_color_primary;
                        DefaultTextStyle::new(
                            control_text_style(
                                &resources,
                                r.tab_view_item_header_font_size,
                                FontWeight::NORMAL,
                                color,
                            ),
                            builder(app, &header.item, width),
                        )
                        .into_widget()
                    } else {
                        header.feedback(app, context)
                    }
                })),
        );
        let end_data = data.clone();
        let source = Draggable::new(Opacity::new(1.0).child(child.clone()), feedback)
            .data(data)
            .hit_test_behavior(inset_rendering::HitTestBehavior::Opaque)
            .max_simultaneous_drags(1)
            .child_when_dragging(Opacity::new(0.0).child(child))
            .drag_anchor_strategy(Rc::new(move |app, draggable, context, position| {
                let offset = child_drag_anchor_strategy(app, draggable, context, position);
                anchor.set(offset);
                offset
            }))
            .on_drag_started(Listener::new(move |app| {
                self.set_state(app, |state| state.drag.item = Some(id.clone()));
            }))
            .on_drag_completed(Listener::new({
                let data = end_data.clone();
                move |app| self.drag_finished(app, &data, true)
            }))
            .on_draggable_canceled(Rc::new(move |app, _, _| {
                self.drag_finished(app, &end_data, false)
            }));

        TabDraggable { draggable: source }.into_widget()
    }

    /// Clears source visuals and emits source completion events.
    fn drag_finished(self: Handle<Self>, app: &mut App, data: &TabDragData, was_accepted: bool) {
        if !app.contains(self) || !self.mounted(app) {
            return;
        }
        self.clear_drag(app);
        self.set_state(app, |state| state.drag.item = None);
        let widget = self.effective_widget(app);
        if let Some(index) = widget.selected_index {
            self.bring_tab_into_view(app, index, None);
        }
        if data.external {
            if let Some(callback) = widget.tab_drag_completed {
                callback(
                    app,
                    TabViewTabDragCompletedEventArgs {
                        item: data.item.clone(),
                        was_accepted,
                    },
                );
            }
            if !was_accepted && let Some(callback) = widget.tab_dropped_outside {
                callback(app, data.item.clone());
            }
        }
    }

    /// Offsets intervening headers to expose the delayed insertion gap.
    fn live_displacement(self: Handle<Self>, app: &mut App, id: &str) -> f64 {
        let rtl = Directionality::of(app, self.context(app)) == TextDirection::Rtl;
        let state = app.get(self);
        let (Some(data), Some(slot)) = (&state.drag.incoming, state.drag.live_insertion) else {
            return 0.0;
        };
        if data.source != self {
            return 0.0;
        }
        let items = &self.widget(app).tab_items;
        let (Some(from), Some(index)) = (
            items.iter().position(|item| item.id == data.item.id),
            items.iter().position(|item| item.id == id),
        ) else {
            return 0.0;
        };
        let shift = if slot > from && from < index && index < slot {
            -data.width
        } else if slot <= index && index < from {
            data.width
        } else {
            0.0
        };
        if rtl { -shift } else { shift }
    }

    /// Routes native candidate, movement, leave and accepted-drop events.
    pub(super) fn drop_target(self: Handle<Self>, child: WidgetRef) -> WidgetRef {
        DragTarget::<TabDragData>::new(move |_, _, _, _| child.clone())
            .on_will_accept_with_details(Rc::new(move |app, details| {
                self.drag_over(app, details.data, details.offset)
            }))
            .on_move(Rc::new(move |app, details| {
                self.drag_over(app, details.data, details.offset);
            }))
            .on_leave(Rc::new(move |app, _| {
                if app.contains(self) && self.mounted(app) {
                    self.clear_drag(app);
                    self.set_state(app, |_| {});
                }
            }))
            .on_accept_with_details(Rc::new(move |app, details| {
                self.drop_tab(app, details.data, details.offset)
            }))
            .into_widget()
    }

    /// Maps the pointer to a collection boundary using realized header centers.
    fn insertion_at(self: Handle<Self>, app: &mut App, position: Offset) -> usize {
        let rtl = Directionality::of(app, self.context(app)) == TextDirection::Rtl;
        let items = self.widget(app).tab_items.clone();
        let mut last = None;
        for (index, _item) in items.iter().enumerate().filter(|(_, item)| item.is_visible) {
            let Some(context) = self.container_from_index(app, index) else {
                continue;
            };
            let Some(render) = context
                .find_render_object(app)
                .and_then(|render| render.as_box())
            else {
                continue;
            };
            let center =
                render.local_to_global(app, Offset::new(render.size(app).width() / 2.0, 0.0), None);
            if if rtl {
                position.dx() > center.dx()
            } else {
                position.dx() < center.dx()
            } {
                return index;
            }
            last = Some(index + 1);
        }
        last.unwrap_or(items.len())
    }

    /// Revalidates owner acceptance and starts insertion visuals and edge scrolling.
    fn drag_over(self: Handle<Self>, app: &mut App, data: Rc<TabDragData>, offset: Offset) -> bool {
        if !app.contains(self) || !self.mounted(app) || !self.widget(app).allow_drop_tabs {
            return false;
        }
        let position = offset + data.anchor.get();
        let insertion = self.insertion_at(app, position);
        let widget = self.effective_widget(app);
        let internal = data.source == self;
        let mut args = TabViewTabStripDropEventArgs {
            source: data.source,
            item: data.item.clone(),
            insertion_index: insertion,
            position,
            accepted: internal && widget.can_reorder_tabs && widget.tab_reorder_requested.is_some(),
        };
        if !internal && !data.external {
            return false;
        }
        if let Some(callback) = widget.tab_strip_drag_over {
            callback(app, &mut args);
        }
        if !args.accepted {
            return false;
        }
        let changed = app.get(self).drag.insertion != Some(insertion);
        app.get_mut(self).drag.incoming = Some(data);
        app.get_mut(self).drag.position = Some(position);
        if changed {
            if let Some(timer) = app.get_mut(self).drag.timer.take() {
                timer.cancel(app);
            }
            app.get_mut(self).drag.insertion = Some(insertion);
            // ListViewBase starts its live reorder visuals after the source 200 ms hover delay.
            let timer = Timer::new(
                app,
                Duration::from_millis(200),
                Listener::new(move |app| {
                    if app.contains(self) && self.mounted(app) {
                        self.set_state(app, |state| {
                            state.drag.timer = None;
                            state.drag.live_insertion = state.drag.insertion;
                        });
                    }
                }),
            );
            app.get_mut(self).drag.timer = Some(timer);
        }
        self.auto_scroll(app, position);
        true
    }

    /// Keeps the native viewport moving while a drag remains at an edge.
    fn auto_scroll(self: Handle<Self>, app: &mut App, position: Offset) {
        if app.get(self).drag.auto_scroll.is_none() {
            let keys: Vec<_> = app
                .get(self)
                .containers
                .values()
                .map(|entry| entry.key.clone())
                .collect();
            let scrollable = keys.into_iter().find_map(|key| {
                key.current_context(app)
                    .and_then(|context| Scrollable::maybe_of(app, context, None))
            });
            if let Some(scrollable) = scrollable {
                let scroller = EdgeDraggingAutoScroller::new(
                    app,
                    scrollable,
                    Some(Listener::new(move |app| {
                        if app.contains(self)
                            && self.mounted(app)
                            && let (Some(data), Some(position)) = (
                                app.get(self).drag.incoming.clone(),
                                app.get(self).drag.position,
                            )
                        {
                            let offset = position - data.anchor.get();
                            self.drag_over(app, data, offset);
                        }
                    })),
                    50.0,
                );
                app.get_mut(self).drag.auto_scroll = Some(scroller);
            }
        }
        if let Some(scroller) = app.get(self).drag.auto_scroll {
            scroller.start_auto_scroll_if_necessary(
                app,
                Rect::from_ltwh(position.dx() - 10.0, position.dy(), 20.0, 1.0),
            );
        }
    }

    /// Commits an internal reorder request or an owner-managed cross-strip transfer.
    fn drop_tab(self: Handle<Self>, app: &mut App, data: Rc<TabDragData>, offset: Offset) {
        if !self.drag_over(app, data.clone(), offset) {
            return;
        }
        let insertion = self.insertion_at(app, offset + data.anchor.get());
        if data.source == self && self.widget(app).can_reorder_tabs {
            if let Some(from) = self
                .widget(app)
                .tab_items
                .iter()
                .position(|item| item.id == data.item.id)
            {
                self.request_reorder(
                    app,
                    from,
                    if insertion > from {
                        insertion - 1
                    } else {
                        insertion
                    },
                );
            }
        } else if let Some(callback) = self.widget(app).tab_strip_drop.clone() {
            callback(
                app,
                TabViewTabStripDropEventArgs {
                    source: data.source,
                    item: data.item.clone(),
                    insertion_index: insertion,
                    position: offset + data.anchor.get(),
                    accepted: true,
                },
            );
        }
        self.clear_drag(app);
        self.set_state(app, |_| {});
    }

    /// Stops destination hover and scrolling state without ending the source drag.
    pub(super) fn clear_drag(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).drag.timer.take() {
            timer.cancel(app);
        }
        if let Some(scroller) = app.get(self).drag.auto_scroll {
            scroller.stop_auto_scroll(app);
        }
        let drag = &mut app.get_mut(self).drag;
        drag.incoming = None;
        drag.position = None;
        drag.insertion = None;
        drag.live_insertion = None;
    }

    /// Cancels timers and releases the owned native scroller.
    pub(super) fn dispose_drag(self: Handle<Self>, app: &mut App) {
        self.clear_drag(app);
        if let Some(scroller) = app.get_mut(self).drag.auto_scroll.take() {
            app.destroy(scroller);
        }
    }
}
