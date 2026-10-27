//! TabView's collection policy, keyboarding, width allocation and template from TabView.cpp/.xaml.

mod drag;
use drag::TabDragState;
pub use drag::{
    TabViewReorderRequestedEventArgs, TabViewTabDragCompletedEventArgs,
    TabViewTabDragStartingEventArgs, TabViewTabStripDropEventArgs,
};

use super::tab_view_item::{TabBottomBorderState, TabViewItemHeader};
use super::{TabScrollViewer, bottom_border_line};
use crate::*;
use inset_animation::Curves;
use inset_embedder::{FontWeight, TextDirection};
use inset_foundation::{App, Handle, Listenable, Listener, ValueKey};
use inset_painting::{Alignment, Axis, EdgeInsetsGeometry};
use inset_rendering::{BoxConstraints, StackFit};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    time::Duration,
};

/// Requests selection of an item, or no item when the collection becomes empty.
pub type TabViewSelectionChanged = Rc<dyn Fn(&mut App, Option<usize>)>;

/// Identifies the tab whose close was requested; removing it remains the owner's choice.
#[derive(Clone, Debug)]
pub struct TabViewTabCloseRequestedEventArgs {
    /// Collection index at the time of the request.
    pub index: usize,

    /// The stable tab item supplied by the owner.
    pub item: TabViewItem,
}

/// Receives a close request before the item's CloseRequested callback.
pub type TabViewTabCloseRequested = Rc<dyn Fn(&mut App, TabViewTabCloseRequestedEventArgs)>;

/// Receives an owner-managed collection reorder request.
pub type TabViewReorderRequested = Rc<dyn Fn(&mut App, TabViewReorderRequestedEventArgs)>;

/// Builds an independent native drag avatar at the measured tab width.
pub type TabViewDragFeedbackBuilder = Rc<dyn Fn(&mut App, &TabViewItem, f64) -> WidgetRef>;

/// Receives a cancelable drag-start event.
pub type TabViewTabDragStarting = Rc<dyn Fn(&mut App, &mut TabViewTabDragStartingEventArgs)>;

/// Receives the final acceptance of a completed drag.
pub type TabViewTabDragCompleted = Rc<dyn Fn(&mut App, TabViewTabDragCompletedEventArgs)>;

/// Receives an item dropped without an accepted target.
pub type TabViewTabDroppedOutside = Rc<dyn Fn(&mut App, TabViewItem)>;

/// Decides whether a strip accepts an incoming item.
pub type TabViewTabStripDragOver = Rc<dyn Fn(&mut App, &mut TabViewTabStripDropEventArgs)>;

/// Transfers an accepted incoming item to its owner.
pub type TabViewTabStripDrop = Rc<dyn Fn(&mut App, TabViewTabStripDropEventArgs)>;

/// A tab strip and selected content presenter, with source width and keyboard behavior.
#[derive(Clone)]
pub struct TabView {
    /// Identity retained when the surrounding widget tree rebuilds.
    pub key: Option<KeyRef>,

    /// Tabs in collection order; ids must be unique and stable across moves.
    pub tab_items: Vec<TabViewItem>,

    /// Index selected by the owner, or no selection.
    pub selected_index: Option<usize>,

    /// Requests selection changes from pointer, keyboard and collection reconciliation.
    pub selection_changed: TabViewSelectionChanged,

    /// How the available strip width is allocated to headers.
    pub tab_width_mode: TabViewWidthMode,

    /// When closable tabs expose their close button.
    pub close_button_overlay_mode: TabViewCloseButtonOverlayMode,

    /// Optional content before the first tab.
    pub tab_strip_header: Option<WidgetRef>,

    /// Optional content following the add button, stretched into remaining space.
    pub tab_strip_footer: Option<WidgetRef>,

    /// Whether the add-tab button is displayed.
    pub is_add_tab_button_visible: bool,

    /// Raised when the add-tab button activates; adding the tab remains the owner's choice.
    pub add_tab_button_click: Option<Listener>,

    /// Raised for close-button, middle-click and Ctrl+F4 close requests.
    pub tab_close_requested: Option<TabViewTabCloseRequested>,

    /// Allows drag events and drops into another tab strip; defaults to false.
    pub can_drag_tabs: bool,

    /// Enables pointer and Alt+Shift+arrow reordering; defaults to true.
    pub can_reorder_tabs: bool,

    /// Allows drops onto this tab strip; defaults to true.
    pub allow_drop_tabs: bool,

    /// Requests the owner move an item, preserving the selected identity.
    pub tab_reorder_requested: Option<TabViewReorderRequested>,

    /// Builds an independent visual subtree for native drag feedback.
    ///
    /// The feedback must not reuse GlobalKeys or externally owned focus nodes from the live header.
    pub tab_drag_feedback_builder: Option<TabViewDragFeedbackBuilder>,

    /// Raised before an externally enabled drag starts; set cancel to prevent it.
    pub tab_drag_starting: Option<TabViewTabDragStarting>,

    /// Raised after an externally enabled drag ends.
    pub tab_drag_completed: Option<TabViewTabDragCompleted>,

    /// Raised when an externally enabled drag finishes without an accepted drop.
    pub tab_dropped_outside: Option<TabViewTabDroppedOutside>,

    /// Decides whether an incoming drag is accepted; the owner sets accepted.
    pub tab_strip_drag_over: Option<TabViewTabStripDragOver>,

    /// Receives accepted cross-strip drops; ownership transfer remains the owner's responsibility.
    pub tab_strip_drop: Option<TabViewTabStripDrop>,
}

impl fmt::Debug for TabView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabView")
            .field("selected_index", &self.selected_index)
            .field("tab_items", &self.tab_items)
            .finish_non_exhaustive()
    }
}

impl TabView {
    /// Creates tabs with owner-managed selection, matching the kit's other value controls.
    pub fn new(
        items: Vec<TabViewItem>,
        selected_index: Option<usize>,
        selection_changed: impl Fn(&mut App, Option<usize>) + 'static,
    ) -> Self {
        Self {
            key: None,
            tab_items: items,
            selected_index,
            selection_changed: Rc::new(selection_changed),
            tab_width_mode: TabViewWidthMode::Equal,
            close_button_overlay_mode: TabViewCloseButtonOverlayMode::Auto,
            tab_strip_header: None,
            tab_strip_footer: None,
            is_add_tab_button_visible: true,
            add_tab_button_click: None,
            tab_close_requested: None,
            can_drag_tabs: false,
            can_reorder_tabs: true,
            allow_drop_tabs: true,
            tab_reorder_requested: None,
            tab_drag_feedback_builder: None,
            tab_drag_starting: None,
            tab_drag_completed: None,
            tab_dropped_outside: None,
            tab_strip_drag_over: None,
            tab_strip_drop: None,
        }
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// Selects Equal, SizeToContent or Compact header allocation.
    pub fn tab_width_mode(mut self, mode: TabViewWidthMode) -> Self {
        self.tab_width_mode = mode;
        self
    }

    /// Sets close-button visibility policy.
    pub fn close_button_overlay_mode(mut self, mode: TabViewCloseButtonOverlayMode) -> Self {
        self.close_button_overlay_mode = mode;
        self
    }

    /// Supplies the leading strip content.
    pub fn tab_strip_header<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.tab_strip_header = Some(child.into_widget());
        self
    }

    /// Supplies the trailing strip content.
    pub fn tab_strip_footer<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.tab_strip_footer = Some(child.into_widget());
        self
    }

    /// Controls visibility of the add-tab button.
    pub fn is_add_tab_button_visible(mut self, value: bool) -> Self {
        self.is_add_tab_button_visible = value;
        self
    }

    /// Receives activation of the add-tab button.
    pub fn add_tab_button_click(mut self, callback: Listener) -> Self {
        self.add_tab_button_click = Some(callback);
        self
    }

    /// Receives requests to close individual tabs.
    pub fn tab_close_requested(mut self, callback: TabViewTabCloseRequested) -> Self {
        self.tab_close_requested = Some(callback);
        self
    }

    /// Returns the selected item if its index is in range.
    pub fn selected_item(&self) -> Option<&TabViewItem> {
        self.selected_index
            .and_then(|index| self.tab_items.get(index))
    }
}

/// Persistent native identity and focus handles for one logical tab.
#[derive(Clone)]
struct TabContainer {
    /// Identifies the realized header independently of its collection index.
    key: Rc<GlobalKey>,

    /// Owns keyboard focus for the header surface.
    tab_focus: AnyFocusNode,

    /// Owns keyboard focus for the close button.
    close_focus: AnyFocusNode,

    /// Retains traversal history for this tab’s page.
    content_focus: Handle<FocusScopeNode>,
}

impl TabContainer {
    /// Creates identity and focus handles without realizing the header or page.
    fn new(app: &mut App, id: &str) -> Self {
        let content_focus = FocusScopeNode::new(app);
        content_focus.set_traversal_edge_behavior(app, TraversalEdgeBehavior::ParentScope);
        Self {
            key: Rc::new(GlobalKey::labeled(format!("Tab {id}"))),
            tab_focus: FocusNode::new(app).as_node(),
            close_focus: FocusNode::new(app).as_node(),
            content_focus,
        }
    }

    /// Releases caller-owned focus nodes after their widgets have detached.
    fn dispose(self, app: &mut App) {
        for node in [
            self.tab_focus,
            self.close_focus,
            self.content_focus.as_node(),
        ] {
            node.dispose(app);
            app.destroy(node.id());
        }
    }
}

/// Retains containers across reorders and tracks source strip sizing and hover state.
pub struct TabViewState {
    /// Native widget lifecycle and element association.
    state: StateData<TabView>,

    /// Persistent handles indexed by stable tab identity.
    containers: HashMap<String, TabContainer>,

    /// Pages that have been selected and must retain their native subtree.
    visited: HashSet<String>,

    /// Controller for the native horizontal list viewport.
    scroll: Option<Handle<ScrollViewportController>>,

    /// Updates scroll-button state when the viewport moves.
    scroll_listener: Option<Listener>,

    /// Owned focus node for the add-tab button.
    add_focus: Option<AnyFocusNode>,

    /// Stable identity of the currently hovered tab.
    hovered: Option<String>,

    /// Whether closing a tab should defer equal-width expansion.
    pointer_in_strip: bool,

    /// Keeps neighboring close buttons stationary until the pointer leaves.
    deferred_width_update: bool,

    /// Most recently allocated common header width; the theme's `TabViewItemMaxWidth` before
    /// the first layout.
    equal_width: Option<f64>,

    /// Measured width reserved before the first tab.
    header_width: f64,

    /// Measured width reserved after the add button.
    footer_width: f64,

    /// Measured variable widths used for realization estimates.
    tab_widths: HashMap<String, f64>,

    /// Invalidates older asynchronous bring-into-view requests.
    bring_epoch: u64,

    /// Source drag and destination insertion state.
    drag: TabDragState,

    /// Selected object identity, reconciled before the owner receives collection changes.
    selected_id: Option<String>,

    /// Invalidates queued collection selection notifications.
    selection_epoch: u64,

    /// Carries page focus across owner-driven selection reconciliation.
    pending_content_focus: bool,
}

impl StatefulWidget for TabView {
    type State = TabViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        Self::State {
            state: StateData::new(),
            containers: HashMap::new(),
            visited: HashSet::new(),
            scroll: None,
            scroll_listener: None,
            add_focus: None,
            hovered: None,
            pointer_in_strip: false,
            deferred_width_update: false,
            equal_width: None,
            header_width: 2.0,
            footer_width: 0.0,
            tab_widths: HashMap::new(),
            bring_epoch: 0,
            drag: TabDragState::default(),
            pending_content_focus: false,
            selected_id: self.selected_item().map(|item| item.id.clone()),
            selection_epoch: 0,
        }
    }
}

impl TabViewState {
    /// Resolves the retained selected object against the current collection.
    fn selected_index(self: Handle<Self>, app: &App) -> Option<usize> {
        let id = app.get(self).selected_id.as_ref()?;
        self.widget(app)
            .tab_items
            .iter()
            .position(|item| &item.id == id)
    }

    /// Supplies the reconciled selection to template and interaction code.
    fn effective_widget(self: Handle<Self>, app: &App) -> TabView {
        let mut widget = self.widget(app).clone();
        widget.selected_index = self.selected_index(app);
        widget
    }

    /// Looks up a realized header without forcing every list item to mount.
    pub fn container_from_index(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
    ) -> Option<BuildContext> {
        let id = self.widget(app).tab_items.get(index)?.id.clone();
        let key = app.get(self).containers.get(&id)?.key.clone();
        key.current_context(app)
    }

    /// Resolves stable item identity to its currently realized header.
    pub fn container_from_item(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
    ) -> Option<BuildContext> {
        let key = app.get(self).containers.get(id)?.key.clone();
        key.current_context(app)
    }

    /// Synchronizes persistent identity without creating offscreen widgets.
    fn sync_items(self: Handle<Self>, app: &mut App) {
        let ids: Vec<_> = self
            .widget(app)
            .tab_items
            .iter()
            .map(|item| item.id.clone())
            .collect();
        let unique: HashSet<_> = ids.iter().cloned().collect();
        assert_eq!(unique.len(), ids.len(), "TabView item ids must be unique");
        let removed: Vec<_> = app
            .get(self)
            .containers
            .keys()
            .filter(|id| !unique.contains(*id))
            .cloned()
            .collect();
        for id in removed {
            let container = app.get_mut(self).containers.remove(&id).unwrap();
            app.get_mut(self).visited.remove(&id);
            app.get_mut(self).tab_widths.remove(&id);
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| container.clone().dispose(app)),
                View::maybe_id_of(app, self.context(app)),
            );
        }
        for id in ids {
            if !app.get(self).containers.contains_key(&id) {
                let container = TabContainer::new(app, &id);
                app.get_mut(self).containers.insert(id, container);
            }
        }
        if let Some(id) = app.get(self).selected_id.clone() {
            app.get_mut(self).visited.insert(id);
        }
    }

    /// Requests selection only for a visible, enabled tab.
    fn select(self: Handle<Self>, app: &mut App, index: usize) {
        let widget = self.effective_widget(app);
        if widget
            .tab_items
            .get(index)
            .is_some_and(|item| item.is_enabled && item.is_visible)
            && widget.selected_index != Some(index)
        {
            let callback = widget.selection_changed.clone();
            callback(app, Some(index));
        }
    }

    /// Raises parent and item close events without removing the tab itself.
    fn request_close(self: Handle<Self>, app: &mut App, index: usize) {
        let widget = self.effective_widget(app);
        let Some(item) = widget
            .tab_items
            .get(index)
            .cloned()
            .filter(|item| item.is_closable && item.is_enabled)
        else {
            return;
        };
        let focus_in_header =
            app.get(self)
                .containers
                .get(&item.id)
                .cloned()
                .is_some_and(|container| {
                    container.tab_focus.has_focus(app) || container.close_focus.has_focus(app)
                });
        let focus_candidates: Vec<_> = ((index + 1)..widget.tab_items.len())
            .chain((0..index).rev())
            .map(|index| widget.tab_items[index].id.clone())
            .collect();
        let closing_id = item.id.clone();
        if let Some(callback) = widget.tab_close_requested {
            callback(
                app,
                TabViewTabCloseRequestedEventArgs {
                    index,
                    item: item.clone(),
                },
            );
        }
        if let Some(callback) = item.close_requested {
            callback.call(app);
        }
        if focus_in_header {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if !app.contains(self)
                        || !self.mounted(app)
                        || self
                            .widget(app)
                            .tab_items
                            .iter()
                            .any(|item| item.id == closing_id)
                    {
                        return;
                    }
                    let next = focus_candidates.iter().find_map(|id| {
                        self.widget(app)
                            .tab_items
                            .iter()
                            .position(|item| &item.id == id && item.is_enabled && item.is_visible)
                    });
                    if let Some(index) = next {
                        let id = self.widget(app).tab_items[index].id.clone();
                        let focus = app.get(self).containers[&id].tab_focus;
                        self.bring_tab_into_view(app, index, Some(focus));
                    } else if self.widget(app).is_add_tab_button_visible
                        && let Some(focus) = app.get(self).add_focus
                    {
                        focus.request_focus(app, None);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
    }

    /// Moves selection with wraparound, skipping unavailable tabs.
    fn move_selection(self: Handle<Self>, app: &mut App, forward: bool) -> bool {
        let widget = self.effective_widget(app);
        let count = widget.tab_items.len();
        if count == 0 {
            return false;
        }
        let start = widget
            .selected_index
            .unwrap_or(if forward { count - 1 } else { 0 })
            .min(count - 1);
        let candidate = (1..=count)
            .map(|step| {
                if forward {
                    (start + step) % count
                } else {
                    (start + count - step % count) % count
                }
            })
            .find(|index| {
                widget.tab_items[*index].is_enabled && widget.tab_items[*index].is_visible
            });
        if let Some(index) = candidate {
            self.select(app, index);
            true
        } else {
            false
        }
    }

    /// The source arrow order includes each close button and wraps through AddButton.
    fn move_focus(self: Handle<Self>, app: &mut App, forward: bool) -> bool {
        let widget = self.effective_widget(app);
        let mut order = Vec::new();
        for (index, item) in widget
            .tab_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_enabled && item.is_visible)
        {
            let container = app.get(self).containers[&item.id].clone();
            order.push((Some(index), container.tab_focus));
            let show_close = item.is_closable
                && (widget.close_button_overlay_mode
                    != TabViewCloseButtonOverlayMode::OnPointerOver
                    || widget.selected_index == Some(index)
                    || app.get(self).hovered.as_ref() == Some(&item.id));
            if show_close {
                order.push((Some(index), container.close_focus));
            }
        }
        if widget.is_add_tab_button_visible {
            order.push((None, app.get(self).add_focus.unwrap()));
        }
        let Some(primary) = primary_focus(app) else {
            return false;
        };
        let Some(current) = order.iter().position(|(_, node)| *node == primary) else {
            return false;
        };
        let next = if forward {
            (current + 1) % order.len()
        } else {
            (current + order.len() - 1) % order.len()
        };
        let (index, node) = order[next];
        if let Some(index) = index {
            self.bring_tab_into_view(app, index, Some(node));
        } else {
            node.request_focus(app, None);
        }
        true
    }

    /// Handles source accelerators in the TabView scope, including page content.
    fn on_key(self: Handle<Self>, app: &mut App, event: &KeyEvent, rtl: bool) -> KeyEventResult {
        if !matches!(event, KeyEvent::Down(_)) {
            return KeyEventResult::Ignored;
        }
        let keyboard = HardwareKeyboard::instance(app);
        let control = keyboard.is_control_pressed(app);
        let shift = keyboard.is_shift_pressed(app);
        let key = event.logical_key();
        if app.get(self).drag.item.is_some() {
            return KeyEventResult::Handled;
        }
        let arrow = matches!(
            key,
            LogicalKeyboardKey::ARROW_LEFT
                | LogicalKeyboardKey::ARROW_RIGHT
                | LogicalKeyboardKey::ARROW_UP
                | LogicalKeyboardKey::ARROW_DOWN
        );
        let handled = if arrow && keyboard.is_alt_pressed(app) && shift && !control {
            self.keyboard_reorder(
                app,
                (matches!(
                    key,
                    LogicalKeyboardKey::ARROW_RIGHT | LogicalKeyboardKey::ARROW_DOWN
                )) != rtl,
            )
        } else if control && key == LogicalKeyboardKey::TAB {
            self.move_selection(app, !shift)
        } else if control && key == LogicalKeyboardKey::F4 {
            if let Some(index) = self.selected_index(app).filter(|index| {
                self.widget(app)
                    .tab_items
                    .get(*index)
                    .is_some_and(|item| item.is_closable)
            }) {
                self.request_close(app, index);
                true
            } else {
                false
            }
        } else if (key == LogicalKeyboardKey::ARROW_LEFT || key == LogicalKeyboardKey::ARROW_RIGHT)
            && !(keyboard.is_alt_pressed(app) && shift)
        {
            self.move_focus(app, (key == LogicalKeyboardKey::ARROW_RIGHT) != rtl)
        } else {
            false
        };
        if handled {
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }

    /// Uses native realization and ensure-visible; equal-width offsets are exact.
    fn bring_tab_into_view(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        focus: Option<AnyFocusNode>,
    ) {
        app.get_mut(self).bring_epoch += 1;
        let epoch = app.get(self).bring_epoch;
        self.bring_step(app, index, focus, epoch, 0);
    }

    /// Estimates an offscreen variable-width position, then corrects after native layout.
    fn bring_step(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        focus: Option<AnyFocusNode>,
        epoch: u64,
        attempt: usize,
    ) {
        let r = ThemeResources::of(app, self.context(app));
        after_next_frame(
            app,
            View::maybe_id_of(app, self.context(app)),
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || !self.mounted(app) || app.get(self).bring_epoch != epoch {
                    return;
                }
                if let Some(context) = self.container_from_index(app, index) {
                    drop(Scrollable::ensure_visible(
                        app,
                        context,
                        0.0,
                        Duration::ZERO,
                        Curves::linear(),
                        ScrollPositionAlignmentPolicy::KeepVisibleAtStart,
                    ));
                    drop(Scrollable::ensure_visible(
                        app,
                        context,
                        1.0,
                        Duration::ZERO,
                        Curves::linear(),
                        ScrollPositionAlignmentPolicy::KeepVisibleAtEnd,
                    ));
                    if let Some(node) = focus {
                        node.request_focus(app, None);
                    }
                    return;
                }
                let widget = self.effective_widget(app);
                if index >= widget.tab_items.len() {
                    return;
                }
                let widths = &app.get(self).tab_widths;
                let estimate = if widget.tab_width_mode == TabViewWidthMode::Equal {
                    app.get(self)
                        .equal_width
                        .unwrap_or(r.tab_view_item_max_width)
                } else if widths.is_empty() {
                    r.tab_view_item_min_width
                } else {
                    widths.values().sum::<f64>() / widths.len() as f64
                };
                let target = widget
                    .tab_items
                    .iter()
                    .take(index)
                    .filter(|item| item.is_visible)
                    .map(|item| widths.get(&item.id).copied().unwrap_or(estimate))
                    .sum::<f64>();
                let scroll = app.get(self).scroll.unwrap();
                let moved = scroll.change_view(app, target, true);
                if moved && attempt < self.widget(app).tab_items.len() {
                    self.bring_step(app, index, focus, epoch, attempt + 1);
                }
            }),
        );
    }
}

impl State for TabViewState {
    type Widget = TabView;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let scroll = ScrollViewportController::new(app);
        let listener = Listener::new(move |app| {
            if self.mounted(app) {
                self.set_state(app, |_| {});
            }
        });
        scroll.add_listener(app, listener.clone());
        app.get_mut(self).scroll = Some(scroll);
        app.get_mut(self).scroll_listener = Some(listener);
        app.get_mut(self).add_focus = Some(FocusNode::new(app).as_node());
        self.sync_items(app);
        if let Some(index) = self.selected_index(app) {
            self.bring_tab_into_view(app, index, None);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &TabView) {
        if old.tab_width_mode != self.widget(app).tab_width_mode {
            app.get_mut(self).tab_widths.clear();
        }
        let old_selected = app.get(self).selected_id.clone();
        let content_had_focus = app.get(self).pending_content_focus
            || old_selected
                .as_ref()
                .and_then(|id| app.get(self).containers.get(id))
                .map(|entry| entry.content_focus.as_node())
                .is_some_and(|node| node.has_focus(app));
        let widget = self.widget(app);
        let old_index = old_selected
            .as_ref()
            .and_then(|id| old.tab_items.iter().position(|item| &item.id == id));
        let retained_index = old_selected
            .as_ref()
            .and_then(|id| widget.tab_items.iter().position(|item| &item.id == id));
        let single_removal = if old.tab_items.len() == widget.tab_items.len() + 1 {
            let removed = old
                .tab_items
                .iter()
                .zip(&widget.tab_items)
                .position(|(old, new)| old.id != new.id)
                .unwrap_or(widget.tab_items.len());
            old.tab_items
                .iter()
                .skip(removed + 1)
                .map(|item| &item.id)
                .eq(widget.tab_items.iter().skip(removed).map(|item| &item.id))
                .then_some(removed)
        } else {
            None
        };
        let removed_index = old_index
            .filter(|_| retained_index.is_none())
            .or(single_removal.filter(|_| old_selected.is_none()));
        let owner_changed = old.selected_index != widget.selected_index;
        let next = if owner_changed && widget.selected_item().is_some() {
            widget.selected_index
        } else if let Some(removed) = removed_index {
            let count = widget.tab_items.len();
            if count == 0 {
                None
            } else {
                let start = removed.min(count - 1);
                (0..count).map(|step| (start + step) % count).find(|index| {
                    widget.tab_items[*index].is_enabled && widget.tab_items[*index].is_visible
                })
            }
        } else if owner_changed {
            widget
                .selected_index
                .filter(|index| *index < widget.tab_items.len())
        } else {
            retained_index
        };
        let selected_id = next.map(|index| widget.tab_items[index].id.clone());
        let notify = next != widget.selected_index;
        let callback = widget.selection_changed.clone();
        let state = app.get_mut(self);
        state.selected_id = selected_id.clone();
        state.selection_epoch += 1;
        let epoch = state.selection_epoch;
        self.sync_items(app);
        if notify {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self)
                        && self.mounted(app)
                        && app.get(self).selection_epoch == epoch
                    {
                        callback(app, next);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
        if single_removal
            .or(removed_index)
            .is_some_and(|removed| removed < self.widget(app).tab_items.len())
            && app.get(self).pointer_in_strip
        {
            app.get_mut(self).deferred_width_update = true;
        }
        if (old_selected != selected_id || app.get(self).pending_content_focus)
            && let Some(index) = next
        {
            self.bring_tab_into_view(app, index, None);
            if content_had_focus {
                app.get_mut(self).pending_content_focus = false;
                let id = selected_id.unwrap();
                let container = app.get(self).containers[&id].clone();
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _| {
                        if app.contains(self)
                            && self.mounted(app)
                            && app.get(self).selected_id.as_ref() == Some(&id)
                        {
                            let candidates =
                                container.content_focus.as_node().traversal_descendants(app);
                            if let Some(first) = candidates.first() {
                                first.request_focus(app, None);
                            } else {
                                container.tab_focus.request_focus(app, None);
                            }
                        }
                    }),
                    View::maybe_id_of(app, self.context(app)),
                );
            }
        }
        if self.widget(app).tab_items.is_empty() {
            app.get_mut(self).pending_content_focus = false;
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_drag(app);
        let scroll = app.get(self).scroll.unwrap();
        if let Some(listener) = app.get_mut(self).scroll_listener.take() {
            scroll.remove_listener(app, &listener);
        }
        scroll.dispose(app);
        app.destroy(scroll);
        for (_, container) in std::mem::take(&mut app.get_mut(self).containers) {
            container.dispose(app);
        }
        if let Some(node) = app.get_mut(self).add_focus.take() {
            node.dispose(app);
            app.destroy(node.id());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let rtl = Directionality::of(app, context) == TextDirection::Rtl;
        Focus::new(
            LayoutBuilder::new(move |app, context, constraints| {
                self.template(app, context, constraints)
            })
            .into_widget(),
        )
        .can_request_focus(false)
        .skip_traversal(true)
        .on_key_event(Rc::new(move |app, _, event| self.on_key(app, event, rtl)))
        .into_widget()
    }
}

impl TabViewState {
    /// LayoutRoot and TabContainerGrid, with source row/column ownership.
    fn template(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        constraints: BoxConstraints,
    ) -> WidgetRef {
        let widget = self.effective_widget(app);
        let theme = ThemeResources::of(app, context);
        let r = theme.clone();
        let visible: Vec<_> = widget
            .tab_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_visible)
            .map(|(index, _)| index)
            .collect();
        let taken = app.get(self).header_width
            + app.get(self).footer_width
            + if widget.is_add_tab_button_visible {
                r.tab_view_item_add_button_width + r.tab_view_item_add_button_container_padding[0]
            } else {
                0.0
            };
        let available = (constraints.max_width - taken).max(0.0);
        if !app.get(self).deferred_width_update {
            app.get_mut(self).equal_width = Some(if visible.is_empty() {
                r.tab_view_item_max_width
            } else {
                ((available - 8.0) / visible.len() as f64)
                    .clamp(r.tab_view_item_min_width, r.tab_view_item_max_width)
            });
        }
        let width = app
            .get(self)
            .equal_width
            .unwrap_or(r.tab_view_item_max_width);
        let equal = widget.tab_width_mode == TabViewWidthMode::Equal;
        let required = if equal {
            width * visible.len() as f64 + 8.0
        } else {
            visible
                .iter()
                .map(|index| {
                    app.get(self)
                        .tab_widths
                        .get(&widget.tab_items[*index].id)
                        .copied()
                        .unwrap_or(r.tab_view_item_min_width)
                })
                .sum::<f64>()
                + 8.0
        };
        let strip_width = required.min(available);
        let scroll = app.get(self).scroll.unwrap();
        let show_scroll = if equal {
            required > available
        } else {
            scroll.metrics(app).scrollable_length > 0.1
        };
        let item_indices = visible.clone();
        let source = widget.clone();
        let list = ListView::builder(move |app, _, visual_index| {
            let index = *item_indices.get(visual_index as usize)?;
            let item = source.tab_items[index].clone();
            let id = item.id.clone();
            let container = app.get(self).containers[&id].clone();
            let hovered = app.get(self).hovered.clone();
            let selected = source.selected_index == Some(index);
            let separator_visible = source.selected_index != Some(index)
                && source.selected_index != Some(index + 1)
                && hovered.as_ref() != Some(&id)
                && source
                    .tab_items
                    .get(index + 1)
                    .is_none_or(|next| hovered.as_ref() != Some(&next.id))
                && index + 1 < source.tab_items.len();
            let bottom = if app.get(self).drag.item.is_some() || selected {
                TabBottomBorderState::NoBottomBorderLine
            } else if source.selected_index == Some(index + 1) {
                TabBottomBorderState::LeftOfSelectedTab
            } else if source
                .selected_index
                .is_some_and(|selected| index == selected + 1)
            {
                TabBottomBorderState::RightOfSelectedTab
            } else {
                TabBottomBorderState::Normal
            };
            let header = TabViewItemHeader {
                item,
                selected,
                width_mode: source.tab_width_mode,
                close_overlay_mode: source.close_button_overlay_mode,
                bottom_border: bottom,
                separator_visible,
                focus_node: container.tab_focus,
                close_focus_node: container.close_focus,
                select: Listener::new(move |app| self.select(app, index)),
                close: Listener::new(move |app| self.request_close(app, index)),
                hover_changed: Rc::new(move |app, hovered| {
                    let id = self
                        .widget(app)
                        .tab_items
                        .get(index)
                        .map(|item| item.id.clone());
                    self.set_state(app, |state| {
                        if hovered {
                            state.hovered = id;
                        } else if state.hovered == id {
                            state.hovered = None;
                        }
                    });
                }),
            };
            let header = self.draggable_header(
                app,
                header,
                if equal {
                    width
                } else {
                    app.get(self)
                        .tab_widths
                        .get(&id)
                        .copied()
                        .unwrap_or(r.tab_view_item_min_width)
                },
            );
            let measured_id = id.clone();
            let header = SizeObserver::new(
                header,
                Rc::new(move |app, _, size| {
                    if app.contains(self)
                        && self.mounted(app)
                        && app.get(self).tab_widths.get(&measured_id).copied() != Some(size.width())
                    {
                        self.set_state(app, |state| {
                            state.tab_widths.insert(measured_id.clone(), size.width());
                        });
                    }
                }),
            )
            .key(container.key);
            let mut sized = SizedBox::new().child(header);
            if equal {
                sized = sized.width(width);
            }
            Some(sized.key(Rc::new(ValueKey::new(id))).into_widget())
        })
        .item_count(visible.len() as i32)
        .find_child_index_callback(Rc::new(move |key| {
            (key.as_ref() as &dyn std::any::Any)
                .downcast_ref::<ValueKey<String>>()
                .and_then(|key| {
                    visible
                        .iter()
                        .position(|index| widget.tab_items[*index].id == key.value)
                })
                .map(|index| index as i32)
        }))
        .build()
        .scroll_direction(Axis::Horizontal)
        .controller(scroll.native_controller(app))
        .padding(EdgeInsetsGeometry::only(4.0, 0.0, 4.0, 0.0));
        let list = if equal { list.item_extent(width) } else { list };
        let strip = SizedBox::new()
            .width(strip_width)
            .height(theme.tab_view_item_min_height)
            .child(
                TabScrollViewer::new(scroll, list)
                    .scroll_buttons_visible(show_scroll)
                    .bottom_border_visible(app.get(self).drag.item.is_none()),
            );
        let strip = Padding::new(EdgeInsetsGeometry::only(
            0.0,
            theme.tab_view_header_padding[1],
            0.0,
            0.0,
        ))
        .child(strip);
        let strip = self.drop_target(strip.into_widget());
        let strip = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                app.get_mut(self).pointer_in_strip = true
            }))
            .on_exit(Rc::new(move |app, _| {
                self.set_state(app, |state| {
                    state.pointer_in_strip = false;
                    state.deferred_width_update = false;
                })
            }))
            .child(strip);
        let widget = self.effective_widget(app);
        let leading = widget
            .tab_strip_header
            .clone()
            .unwrap_or_else(|| SizedBox::new().width(2.0).into_widget());
        let trailing = widget
            .tab_strip_footer
            .clone()
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let leading = SizeObserver::new(
            leading,
            Rc::new(move |app, _, size| {
                if app.contains(self)
                    && self.mounted(app)
                    && app.get(self).header_width != size.width()
                {
                    self.set_state(app, |state| state.header_width = size.width());
                }
            }),
        );
        let trailing = SizeObserver::new(
            trailing,
            Rc::new(move |app, _, size| {
                if app.contains(self)
                    && self.mounted(app)
                    && app.get(self).footer_width != size.width()
                {
                    self.set_state(app, |state| state.footer_width = size.width());
                }
            }),
        );
        let trailing = Align::new()
            .alignment(Alignment::CENTER_LEFT.into())
            .width_factor(1.0)
            .height_factor(1.0)
            .child(trailing);
        let mut strip_children = vec![
            GridCell::new(leading).into_widget(),
            GridCell::new(strip).column(1).into_widget(),
            GridCell::new(trailing).column(3).into_widget(),
        ];
        if widget.is_add_tab_button_visible {
            let click = widget
                .add_tab_button_click
                .unwrap_or_else(|| Listener::new(|_| {}));
            let button = Button::new(
                FontIcon::symbol(FluentSymbol::Add)
                    .font_size(theme.tab_view_item_add_button_font_size),
                click,
            )
            .focus_node(app.get(self).add_focus.unwrap())
            .template(add_button_template);
            strip_children.push(
                GridCell::new(
                    Align::new()
                        .alignment(Alignment::BOTTOM_CENTER.into())
                        .child(
                            Padding::new(EdgeInsetsGeometry::only(
                                theme.tab_view_item_add_button_container_padding[0],
                                0.0,
                                0.0,
                                theme.tab_view_item_add_button_container_padding[3],
                            ))
                            .child(ToolTipService::new(button, ToolTip::text("Add new tab"))),
                        ),
                )
                .column(2)
                .into_widget(),
            );
        }
        // LeftBottomBorderLine / RightBottomBorderLine remain beneath their column content. While
        // a tab is dragged, `SingleBottomBorderLine` spans the left one across the strip and
        // collapses the right one.
        let lines: &[_] = if app.get(self).drag.item.is_some() {
            &[("LeftBottomBorderLine", 0, 4)]
        } else {
            &[
                ("LeftBottomBorderLine", 0, 1),
                ("RightBottomBorderLine", 2, 2),
            ]
        };
        for &(name, column, span) in lines {
            strip_children.insert(
                0,
                bottom_border_line(name, theme.tab_view_border_brush)
                    .column(column)
                    .column_span(span)
                    .into_widget(),
            );
        }
        let strip = Grid::new()
            .background(Brush::Solid(theme.tab_view_background))
            .column_definitions([
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::star(1.0)),
            ])
            .children(strip_children);
        let pages: Vec<_> = widget
            .tab_items
            .iter()
            .enumerate()
            .filter(|(_, item)| app.get(self).visited.contains(&item.id))
            .map(|(index, item)| {
                let selected = widget.selected_index == Some(index);
                let scope = app.get(self).containers[&item.id].content_focus;
                Offstage::new()
                    .offstage(!selected)
                    .child(TickerMode::new(
                        selected,
                        ExcludeFocus::new(FocusScope::new(item.content.clone()).node(scope))
                            .excluding(!selected),
                    ))
                    .key(Rc::new(ValueKey::new(item.id.clone())))
                    .into_widget()
            })
            .collect();
        let content = Stack::new().fit(StackFit::Passthrough).children(pages);
        let layout_root = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::star(1.0)),
            ])
            .children([
                GridCell::new(strip).into_widget(),
                GridCell::new(content).row(1).into_widget(),
            ]);
        // Like a page-level container, the TabView fills a bounded width; in an unbounded one it
        // is as wide as its strip.
        if constraints.has_bounded_width() {
            SizedBox::new()
                .width(constraints.max_width)
                .child(layout_root)
                .into_widget()
        } else {
            layout_root.into_widget()
        }
    }
}

/// TabViewButtonStyle's ContentPresenter and discrete CommonStates.
fn add_button_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
) -> WidgetRef {
    let theme = ThemeResources::of(app, context);
    let r = &theme;
    let (background, foreground, border) = match states.common {
        CommonState::Normal => (
            r.tab_view_button_background,
            r.tab_view_button_foreground,
            r.tab_view_button_border_brush,
        ),
        CommonState::PointerOver => (
            r.tab_view_button_background_pointer_over,
            r.tab_view_button_foreground_pointer_over,
            r.tab_view_button_border_brush_pointer_over,
        ),
        CommonState::Pressed => (
            r.tab_view_button_background_pressed,
            r.tab_view_button_foreground_pressed,
            r.tab_view_button_border_brush_pressed,
        ),
        CommonState::Disabled => (
            r.tab_view_button_background_disabled,
            r.tab_view_button_foreground_disabled,
            r.tab_view_button_border_brush_disabled,
        ),
    };
    FocusVisual::new(
        SizedBox::new()
            .width(r.tab_view_item_add_button_width)
            .height(r.tab_view_item_add_button_height)
            .child(
                ControlBorder::new(Brush::Solid(background), Brush::Solid(border))
                    .border_thickness_ltrb(r.tab_view_button_border_thickness)
                    .corner_radius(r.control_corner_radius[0])
                    .child(Center::new().child(DefaultTextStyle::new(
                        control_text_style(
                            &theme,
                            r.tab_view_item_add_button_font_size,
                            FontWeight::NORMAL,
                            foreground,
                        ),
                        content,
                    ))),
            ),
        theme.theme,
    )
    .visible(states.focused)
    .margin([-3.0; 4])
    .corner_radius(r.control_corner_radius[0])
    .into_widget()
}
