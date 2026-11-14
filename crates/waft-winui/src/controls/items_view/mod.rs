//! `ItemsView` (`controls/dev/ItemsView`: `ItemsView.cpp`, `ItemsViewInteractions.cpp` and the selectors) over Waft's lazy list: items an application builds as `ItemContainer`s, one or several selected by the selection mode's rules, the current item moved by the arrow keys, Home, End, Page Up and Page Down, and an item invoked.
//!
//! WinUI makes the items through an `ItemsRepeater` that can make any item on demand. Here a vertical `StackLayout` is Waft's `ListView`, which builds only what is in sight and a margin around it; an item out of that range is brought into view by its estimated offset, from the extents measured so far, and focused once built. A horizontal `StackLayout`, as `SelectorBar` uses, builds every item in a row. The view keeps the built items by index ([`ItemsViewState::register`]), and a later editor must keep that registry exact: focus, Tab and page navigation read it.

mod selection;
mod selection_model;
mod selector;

pub use selection::ItemsViewSelection;
pub use selector::ItemsViewSelectionMode;

use crate::FocusState;
use crate::*;
use std::{collections::HashMap, fmt, rc::Rc};
use waft_embedder::{Clip, Offset, Radius, Rect, Size, TextDirection};
use waft_foundation::{App, Handle, ListenableObject, Listener};
use waft_painting::{Axis, BorderRadius, EdgeInsetsGeometry};
use waft_rendering::MainAxisSize;
use waft_scheduler::FrameCallback;
use waft_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use waft_widgets::*;

/// Builds the `ItemContainer` of the item at an index: XAML's `ItemTemplate` over `ItemsSource`.
pub type ItemsViewItemTemplate = Rc<dyn Fn(&mut App, BuildContext, usize) -> ItemContainer>;

/// `ItemInvoked`: the index of the item invoked.
pub type ItemsViewItemInvoked = Rc<dyn Fn(&mut App, usize)>;

/// XAML `StackLayout`, the one `Layout` this port has: items one after another along `orientation`, `spacing` apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StackLayout {
    /// `Orientation`.
    pub orientation: Axis,
    /// `Spacing`.
    pub spacing: f64,
}

impl Default for StackLayout {
    fn default() -> Self {
        Self {
            orientation: Axis::Vertical,
            spacing: 0.0,
        }
    }
}

/// XAML `ItemsView`.
#[derive(Clone)]
pub struct ItemsView {
    /// How many items there are: `ItemsSource`'s count.
    pub item_count: usize,
    /// `ItemTemplate`.
    pub item_template: ItemsViewItemTemplate,
    /// `Layout`.
    pub layout: StackLayout,
    /// `SelectionMode`.
    pub selection_mode: ItemsViewSelectionMode,
    /// `IsItemInvokedEnabled`.
    pub is_item_invoked_enabled: bool,
    /// The selection and current item; `None` makes the view keep its own.
    pub selection: Option<Handle<ItemsViewSelection>>,
    /// `ItemInvoked`.
    pub item_invoked: Option<ItemsViewItemInvoked>,
    /// `SelectionChanged`.
    pub selection_changed: Option<Listener>,
    /// `Padding`: around the items, inside the scrolling.
    pub padding: [f64; 4],
    /// The extent of every item along a vertical stack, which lets the list place an item far away without building every item before it, as Flutter's `ListView.itemExtent` does; `None` measures each item as it is built.
    pub item_extent: Option<f64>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl ItemsView {
    /// Creates a view of `item_count` items, each the container `item_template` builds.
    pub fn new(
        item_count: usize,
        item_template: impl Fn(&mut App, BuildContext, usize) -> ItemContainer + 'static,
    ) -> Self {
        Self {
            item_count,
            item_template: Rc::new(item_template),
            layout: StackLayout::default(),
            selection_mode: ItemsViewSelectionMode::default(),
            is_item_invoked_enabled: false,
            selection: None,
            item_invoked: None,
            selection_changed: None,
            padding: [0.0; 4],
            item_extent: None,
            key: None,
        }
    }

    /// Sets the XAML `Layout`.
    pub fn layout(mut self, value: StackLayout) -> Self {
        self.layout = value;
        self
    }

    /// Sets the XAML `SelectionMode`.
    pub fn selection_mode(mut self, value: ItemsViewSelectionMode) -> Self {
        self.selection_mode = value;
        self
    }

    /// Sets the XAML `IsItemInvokedEnabled`.
    pub fn is_item_invoked_enabled(mut self, value: bool) -> Self {
        self.is_item_invoked_enabled = value;
        self
    }

    /// Gives the view the application's selection.
    pub fn selection(mut self, value: Handle<ItemsViewSelection>) -> Self {
        self.selection = Some(value);
        self
    }

    /// Sets the XAML `ItemInvoked` handler.
    pub fn item_invoked(mut self, handler: impl Fn(&mut App, usize) + 'static) -> Self {
        self.item_invoked = Some(Rc::new(handler));
        self
    }

    /// Sets the XAML `SelectionChanged` handler.
    pub fn selection_changed(mut self, handler: impl Fn(&mut App) + 'static) -> Self {
        self.selection_changed = Some(Listener::new(handler));
        self
    }

    /// Sets the XAML `Padding`.
    pub fn padding(mut self, value: [f64; 4]) -> Self {
        self.padding = value;
        self
    }

    /// Gives every item the same extent along a vertical stack.
    pub fn item_extent(mut self, value: f64) -> Self {
        self.item_extent = Some(value);
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for ItemsView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemsView")
            .field("item_count", &self.item_count)
            .field("layout", &self.layout)
            .field("selection_mode", &self.selection_mode)
            .field("is_item_invoked_enabled", &self.is_item_invoked_enabled)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ItemsView {
    type State = ItemsViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ItemsViewState {
        ItemsViewState {
            state: StateData::new(),
            own_selection: None,
            listening: None,
            seen_revision: 0,
            scroll: None,
            items: HashMap::new(),
            extents: HashMap::new(),
            tab_stop: None,
            focus_epoch: 0,
        }
    }
}

/// An item's index and the view it is an item of, which the view puts around each container it builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemsViewItem {
    pub(crate) index: usize,
    pub(crate) view: Handle<ItemsViewState>,
}

impl ItemsViewItem {
    /// The item `context` is in, if it is in an `ItemsView`.
    pub(crate) fn of(app: &App, context: BuildContext) -> Option<ItemsViewItem> {
        context
            .get_inherited_widget_of_exact_type::<ItemsViewItemScope>(app)
            .map(|scope| scope.item)
    }
}

/// Tells the container under it which item of which view it is.
#[derive(Debug)]
struct ItemsViewItemScope {
    item: ItemsViewItem,
    child: WidgetRef,
}

impl InheritedWidget for ItemsViewItemScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &Self) -> bool {
        self.item != old.item
    }
}

/// A container the list has built.
#[derive(Clone, Copy, Debug)]
struct BuiltItem {
    container: Handle<ItemContainerState>,
    focus: Handle<FocusNode>,
}

/// The selection the view listens to, the built items, their measured extents and the scrolling.
pub struct ItemsViewState {
    state: StateData<ItemsView>,
    /// The selection made for a view given none.
    own_selection: Option<Handle<ItemsViewSelection>>,
    /// The selection listened to, and the listener.
    listening: Option<(Handle<ItemsViewSelection>, Listener)>,
    /// The selection's revision `SelectionChanged` was last raised for.
    seen_revision: u64,
    scroll: Option<Handle<ScrollViewportController>>,
    /// The built containers by index.
    items: HashMap<usize, BuiltItem>,
    /// Each built item's extent along the layout, as last laid out.
    extents: HashMap<usize, f64>,
    /// The item Tab stops at, as last told to the containers.
    tab_stop: Option<usize>,
    /// Counts focus requests, so that a request waiting for its item to be built gives way to a later one.
    focus_epoch: u64,
}

// MARK: The containers' view of the view

impl ItemsViewState {
    pub(crate) fn selection(self: Handle<Self>, app: &App) -> Handle<ItemsViewSelection> {
        self.widget(app)
            .selection
            .or(app.get(self).own_selection)
            .expect("made in `init_state`")
    }

    pub(crate) fn is_selected(self: Handle<Self>, app: &App, index: usize) -> bool {
        self.selection(app).is_selected(app, index)
    }

    pub(crate) fn is_tab_stop(self: Handle<Self>, app: &App, index: usize) -> bool {
        self.tab_stop_index(app) == Some(index)
    }

    /// `ItemsView::OnItemsRepeaterElementPrepared`'s `MultiSelectMode` for `Auto`.
    pub(crate) fn multi_select_mode(self: Handle<Self>, app: &App) -> ItemContainerMultiSelectMode {
        match self.widget(app).selection_mode {
            ItemsViewSelectionMode::None | ItemsViewSelectionMode::Single => {
                ItemContainerMultiSelectMode::Single
            }
            ItemsViewSelectionMode::Extended => ItemContainerMultiSelectMode::Extended,
            ItemsViewSelectionMode::Multiple => ItemContainerMultiSelectMode::Multiple,
        }
    }

    /// A container's `CanUserSelect`, `Auto` resolved from the selection mode.
    pub(crate) fn can_user_select(
        self: Handle<Self>,
        app: &App,
        mode: ItemContainerUserSelectMode,
    ) -> bool {
        match mode {
            ItemContainerUserSelectMode::Auto => {
                self.widget(app).selection_mode != ItemsViewSelectionMode::None
            }
            ItemContainerUserSelectMode::UserCanSelect => true,
            ItemContainerUserSelectMode::UserCannotSelect => false,
        }
    }

    /// A container's `CanUserInvoke`, `Auto` resolved from `IsItemInvokedEnabled`.
    pub(crate) fn can_user_invoke(
        self: Handle<Self>,
        app: &App,
        mode: ItemContainerUserInvokeMode,
    ) -> bool {
        match mode {
            ItemContainerUserInvokeMode::Auto => self.widget(app).is_item_invoked_enabled,
            ItemContainerUserInvokeMode::UserCanInvoke => true,
            ItemContainerUserInvokeMode::UserCannotInvoke => false,
        }
    }

    /// A container was built for `index`.
    pub(crate) fn register(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        container: Handle<ItemContainerState>,
        focus: Handle<FocusNode>,
    ) {
        app.get_mut(self)
            .items
            .insert(index, BuiltItem { container, focus });
        self.sync_tab_stop_after_frame(app);
    }

    /// The container for `index` went, unless another has taken its place.
    pub(crate) fn unregister(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        container: Handle<ItemContainerState>,
    ) {
        let items = &mut app.get_mut(self).items;
        if items
            .get(&index)
            .is_some_and(|item| item.container == container)
        {
            items.remove(&index);
            self.sync_tab_stop_after_frame(app);
        }
    }

    /// `ItemsView::OnItemsViewItemContainerItemInvoked`: answers whether it handled the interaction.
    pub(crate) fn item_container_invoked(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        container: &ItemContainer,
        trigger: ItemContainerInteractionTrigger,
    ) -> bool {
        let handled = match trigger {
            ItemContainerInteractionTrigger::PointerReleased => {
                self.process_interaction(app, index, FocusState::Pointer)
            }
            ItemContainerInteractionTrigger::EnterKey
            | ItemContainerInteractionTrigger::SpaceKey => {
                self.process_interaction(app, index, FocusState::Keyboard)
            }
            ItemContainerInteractionTrigger::Tap | ItemContainerInteractionTrigger::DoubleTap => {
                false
            }
        };
        if trigger != ItemContainerInteractionTrigger::PointerReleased
            && self.can_raise_item_invoked(app, trigger, container)
            && let Some(invoked) = self.widget(app).item_invoked.clone()
        {
            invoked(app, index);
        }
        handled
    }

    /// `OnItemsViewElementGettingFocus`: the focused item becomes the current one.
    pub(crate) fn item_got_focus(self: Handle<Self>, app: &mut App, index: usize) {
        self.selection(app).set_current_item_index(app, Some(index));
        self.sync_tab_stop(app);
    }
}

// MARK: Interaction

impl ItemsViewState {
    /// `ItemsView::CanRaiseItemInvoked`: invoking is on, and with a selection mode a tap or Space selects rather than invokes, and without one a double tap does nothing more than a tap.
    fn can_raise_item_invoked(
        self: Handle<Self>,
        app: &App,
        trigger: ItemContainerInteractionTrigger,
        container: &ItemContainer,
    ) -> bool {
        use ItemContainerInteractionTrigger::*;
        if !self.can_user_invoke(app, container.can_user_invoke) {
            return false;
        }
        let widget = self.widget(app);
        let none = widget.selection_mode == ItemsViewSelectionMode::None;
        widget.is_item_invoked_enabled
            && !(none && trigger == DoubleTap)
            && !(!none && matches!(trigger, Tap | SpaceKey))
    }

    /// `ItemsView::ProcessInteraction`: the item becomes current and takes the focus, brought into view for a pointer, and the selector acts on it.
    fn process_interaction(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        focus: FocusState,
    ) -> bool {
        let handled = self.set_current_element_index(app, index, focus);
        if focus == FocusState::Pointer {
            self.bring_item_into_view(app, index, None);
        }
        let (ctrl, shift) = modifiers(app);
        let selection = self.selection(app);
        selection.interacted(app, index, ctrl, shift);
        self.deselect_unselectable_items(app);
        handled
    }

    /// `ApplySelectionModelSelectionChange` while processing an interaction: a built item that cannot be selected is deselected, the anchor kept.
    fn deselect_unselectable_items(self: Handle<Self>, app: &mut App) {
        let selection = self.selection(app);
        let items: Vec<(usize, BuiltItem)> = app
            .get(self)
            .items
            .iter()
            .map(|(i, item)| (*i, *item))
            .collect();
        for (index, item) in items {
            let mode = item.container.widget(app).can_user_select;
            if mode == ItemContainerUserSelectMode::UserCannotSelect
                && selection.is_selected(app, index)
            {
                selection.deselect_with_anchor_preservation(app, index);
            }
        }
    }

    /// `SetCurrentElementIndex` with `SetFocusElementIndex`: answers whether the item took the focus.
    fn set_current_element_index(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        focus: FocusState,
    ) -> bool {
        self.selection(app).set_current_item_index(app, Some(index));
        self.sync_tab_stop(app);
        let Some(item) = app.get(self).items.get(&index).copied() else {
            return false;
        };
        FocusState::name(app, item.focus.as_node(), focus);
        item.focus.request_focus(app, None);
        true
    }

    /// `ItemsView::OnKeyDown`'s Control+A and `OnItemsViewElementKeyDown`'s navigation keys.
    fn key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        if key == LogicalKeyboardKey::KEY_A {
            return self.select_all_key(app);
        }
        match self.navigation_target(app, key) {
            Some(Navigation::To(index)) => {
                self.focus_by_keyboard(app, index, None);
                KeyEventResult::Handled
            }
            Some(Navigation::Edge(index, alignment)) => {
                self.focus_by_keyboard(app, index, Some(alignment));
                KeyEventResult::Handled
            }
            Some(Navigation::Page(forward)) => {
                self.page(app, forward);
                KeyEventResult::Handled
            }
            None => KeyEventResult::Ignored,
        }
    }

    /// Control+A selects every item in `Multiple` and `Extended` modes.
    fn select_all_key(self: Handle<Self>, app: &mut App) -> KeyEventResult {
        let mode = self.widget(app).selection_mode;
        let multiple = matches!(
            mode,
            ItemsViewSelectionMode::Multiple | ItemsViewSelectionMode::Extended
        );
        if multiple && HardwareKeyboard::instance(app).is_control_pressed(app) {
            self.selection(app).select_all(app);
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }

    /// `ProcessNavigationKeys` for a stack: the arrows along it move to the adjacent focusable item, Home and End to the first and last, and Page Up and Page Down page along a vertical one.
    fn navigation_target(
        self: Handle<Self>,
        app: &mut App,
        key: LogicalKeyboardKey,
    ) -> Option<Navigation> {
        let count = self.widget(app).item_count;
        if count == 0 {
            return None;
        }
        let vertical = self.widget(app).layout.orientation == Axis::Vertical;
        let rtl = Directionality::maybe_of(app, self.context(app)) == Some(TextDirection::Rtl);
        let previous = match key {
            LogicalKeyboardKey::ARROW_UP if vertical => Some(true),
            LogicalKeyboardKey::ARROW_DOWN if vertical => Some(false),
            LogicalKeyboardKey::ARROW_LEFT if !vertical => Some(!rtl),
            LogicalKeyboardKey::ARROW_RIGHT if !vertical => Some(rtl),
            _ => None,
        };
        if let Some(previous) = previous {
            let target = self.adjacent_focusable_index(app, previous)?;
            return Some(Navigation::To(target));
        }
        match key {
            LogicalKeyboardKey::HOME => Some(Navigation::Edge(0, 0.0)),
            LogicalKeyboardKey::END => Some(Navigation::Edge(count - 1, 1.0)),
            LogicalKeyboardKey::PAGE_UP if vertical => Some(Navigation::Page(false)),
            LogicalKeyboardKey::PAGE_DOWN if vertical => Some(Navigation::Page(true)),
            _ => None,
        }
    }

    /// `GetAdjacentFocusableElementByIndex`: the nearest item before or after the current one that can take the focus; an item not built is taken to be able to.
    fn adjacent_focusable_index(self: Handle<Self>, app: &App, previous: bool) -> Option<usize> {
        let count = self.widget(app).item_count;
        let current = self.selection(app).current_item_index(app)?;
        let mut index = current;
        loop {
            index = if previous {
                index.checked_sub(1)?
            } else if index + 1 < count {
                index + 1
            } else {
                return None;
            };
            if self.is_focusable(app, index) {
                return Some(index);
            }
        }
    }

    fn is_focusable(self: Handle<Self>, app: &App, index: usize) -> bool {
        app.get(self)
            .items
            .get(&index)
            .is_none_or(|item| item.container.widget(app).is_enabled)
    }

    /// `SetFocusElementIndex` with `FocusState::Keyboard` and the selector's `OnFocusedAction`: the item is brought into view, aligned when `alignment` says so, and takes the focus once built.
    fn focus_by_keyboard(self: Handle<Self>, app: &mut App, index: usize, alignment: Option<f64>) {
        self.bring_item_into_view(app, index, alignment);
        let epoch = {
            let state = app.get_mut(self);
            state.focus_epoch += 1;
            state.focus_epoch
        };
        self.focus_when_built(app, index, epoch, 0);
    }

    /// Focuses the item at `index` once the list has built it, a few frames at most, unless a later request came.
    fn focus_when_built(self: Handle<Self>, app: &mut App, index: usize, epoch: u64, attempt: u32) {
        if app.get(self).focus_epoch != epoch {
            return;
        }
        if app.get(self).items.contains_key(&index) {
            self.set_current_element_index(app, index, FocusState::Keyboard);
            self.bring_item_into_view(app, index, None);
            let (ctrl, shift) = modifiers(app);
            self.selection(app).focused(app, index, ctrl, shift);
            self.deselect_unselectable_items(app);
            return;
        }
        if attempt < 3 {
            self.bring_item_into_view(app, index, None);
            self.after_frame(app, move |app| {
                self.focus_when_built(app, index, epoch, attempt + 1)
            });
        }
    }

    /// Page Up and Page Down: the last item wholly in view in that direction, or when the current item already is, the last a page further.
    fn page(self: Handle<Self>, app: &mut App, forward: bool) {
        let current = self.selection(app).current_item_index(app);
        if let Some(target) = self.last_visible_index(app, forward)
            && Some(target) != current
        {
            self.focus_by_keyboard(app, target, None);
            return;
        }
        let scroll = self.scroll(app);
        let page = scroll.metrics(app).viewport_length;
        scroll.scroll_by(app, if forward { page } else { -page }, true);
        self.after_frame(app, move |app| {
            if let Some(target) = self.last_visible_index(app, forward) {
                self.focus_by_keyboard(app, target, None);
            }
        });
    }

    /// The built item wholly in view furthest along, or back; an item larger than the view counts when partly in view.
    fn last_visible_index(self: Handle<Self>, app: &mut App, forward: bool) -> Option<usize> {
        let viewport = self.viewport_rect(app)?;
        let mut indices: Vec<usize> = app.get(self).items.keys().copied().collect();
        indices.sort_unstable();
        indices.retain(|index| self.is_focusable(app, *index));
        let rects: Vec<(usize, Rect)> = indices
            .into_iter()
            .filter_map(|index| Some((index, self.item_rect(app, index)?)))
            .collect();
        let wholly =
            |rect: &Rect| rect.top >= viewport.top - 0.5 && rect.bottom <= viewport.bottom + 0.5;
        let partly = |rect: &Rect| rect.top < viewport.bottom && rect.bottom > viewport.top;
        let pick = |test: &dyn Fn(&Rect) -> bool| {
            let mut found = rects
                .iter()
                .filter(|(_, rect)| test(rect))
                .map(|(index, _)| *index);
            if forward { found.last() } else { found.next() }
        };
        pick(&wholly).or_else(|| pick(&partly))
    }
}

/// Where a navigation key moves the focus.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Navigation {
    /// To an item, brought into view as little as it takes.
    To(usize),
    /// To the first or last item, aligned to that end of the view.
    Edge(usize, f64),
    /// A page forward or back.
    Page(bool),
}

/// Whether Control and Shift are down, as the selectors take them.
fn modifiers(app: &mut App) -> (bool, bool) {
    let keyboard = HardwareKeyboard::instance(app);
    (
        keyboard.is_control_pressed(app),
        keyboard.is_shift_pressed(app),
    )
}

// MARK: Scrolling

impl ItemsViewState {
    fn scroll(self: Handle<Self>, app: &App) -> Handle<ScrollViewportController> {
        app.get(self).scroll.expect("made in `init_state`")
    }

    fn axis(self: Handle<Self>, app: &App) -> Axis {
        self.widget(app).layout.orientation
    }

    /// `StartBringItemIntoView`: scrolls so the item is in view, its start at the view's start for an `alignment` of 0 and its end at the view's end for 1, or as little as it takes for none. An item not built is placed by the extents of the items between it and a built one.
    fn bring_item_into_view(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        alignment: Option<f64>,
    ) {
        let Some((start, extent)) = self.item_offset_in_view(app, index) else {
            return;
        };
        let scroll = self.scroll(app);
        let view = scroll.metrics(app).viewport_length;
        let end = start + extent;
        let delta = match alignment {
            Some(alignment) => start - alignment * (view - extent),
            None if start < 0.0 => start,
            None if end > view => (end - view).min(start),
            None => return,
        };
        scroll.scroll_by(app, delta, true);
    }

    /// The item's start along the layout, from the start of the view, and its extent.
    fn item_offset_in_view(self: Handle<Self>, app: &mut App, index: usize) -> Option<(f64, f64)> {
        let viewport = self.viewport_rect(app)?;
        let axis = self.axis(app);
        let start_of = |rect: Rect| match axis {
            Axis::Vertical => rect.top - viewport.top,
            Axis::Horizontal => rect.left - viewport.left,
        };
        if let Some(rect) = self.item_rect(app, index) {
            return Some((start_of(rect), self.extent(app, index)));
        }
        let anchor = app
            .get(self)
            .items
            .keys()
            .copied()
            .min_by_key(|built| built.abs_diff(index))?;
        let mut start = start_of(self.item_rect(app, anchor)?);
        let spacing = self.widget(app).layout.spacing;
        if index > anchor {
            for between in anchor..index {
                start += self.extent(app, between) + spacing;
            }
        } else {
            for between in index..anchor {
                start -= self.extent(app, between) + spacing;
            }
        }
        Some((start, self.extent(app, index)))
    }

    /// An item's extent: the view's `item_extent`, else as last laid out, else the average of those measured.
    fn extent(self: Handle<Self>, app: &App, index: usize) -> f64 {
        if let Some(extent) = self.widget(app).item_extent {
            return extent;
        }
        let extents = &app.get(self).extents;
        extents.get(&index).copied().unwrap_or_else(|| {
            if extents.is_empty() {
                0.0
            } else {
                extents.values().sum::<f64>() / extents.len() as f64
            }
        })
    }

    fn item_measured(self: Handle<Self>, app: &mut App, index: usize, size: Size) {
        if !app.contains(self) {
            return;
        }
        let extent = match self.axis(app) {
            Axis::Vertical => size.height(),
            Axis::Horizontal => size.width(),
        };
        app.get_mut(self).extents.insert(index, extent);
    }

    /// Where a built item is laid out, in global coordinates.
    fn item_rect(self: Handle<Self>, app: &mut App, index: usize) -> Option<Rect> {
        let focus = app.get(self).items.get(&index)?.focus;
        global_rect(app, focus.as_node().context(app)?)
    }

    /// Where the view is laid out, in global coordinates.
    fn viewport_rect(self: Handle<Self>, app: &mut App) -> Option<Rect> {
        global_rect(app, self.context(app))
    }
}

fn global_rect(app: &mut App, context: BuildContext) -> Option<Rect> {
    let render = context.find_render_object(app)?.as_box()?;
    let origin = render.local_to_global(app, Offset::ZERO, None);
    Some(origin & render.size(app))
}

// MARK: Tab stop and selection

impl ItemsViewState {
    /// `TabNavigation="Once"`: Tab stops at the current item, else at the selected one in `Single` mode, else at the first built.
    fn tab_stop_index(self: Handle<Self>, app: &App) -> Option<usize> {
        let selection = self.selection(app);
        if let Some(current) = selection.current_item_index(app) {
            return Some(current);
        }
        let single = self.widget(app).selection_mode == ItemsViewSelectionMode::Single;
        let selected = selection.selected_index(app).filter(|_| single);
        selected.or_else(|| app.get(self).items.keys().copied().min())
    }

    /// Tells the containers when the item Tab stops at changed.
    fn sync_tab_stop(self: Handle<Self>, app: &mut App) {
        let tab_stop = self.tab_stop_index(app);
        if app.get(self).tab_stop != tab_stop {
            app.get_mut(self).tab_stop = tab_stop;
            self.selection(app).notify_items(app);
        }
    }

    /// The same after the frame, for the registry changing while the list builds.
    fn sync_tab_stop_after_frame(self: Handle<Self>, app: &mut App) {
        self.after_frame(app, move |app| self.sync_tab_stop(app));
    }

    /// The selection heard: `SelectionChanged` is raised for a change of the selection, not of the current item.
    fn selection_heard(self: Handle<Self>, app: &mut App) {
        let revision = self.selection(app).revision(app);
        if revision == app.get(self).seen_revision {
            return;
        }
        app.get_mut(self).seen_revision = revision;
        if let Some(changed) = self.widget(app).selection_changed.clone() {
            changed.call(app);
        }
    }

    /// Listens to the selection in use, and gives it the view's mode and count.
    fn sync_selection(self: Handle<Self>, app: &mut App) {
        let selection = self.selection(app);
        if app
            .get(self)
            .listening
            .as_ref()
            .map(|(listened, _)| *listened)
            != Some(selection)
        {
            if let Some((old, listener)) = app.get_mut(self).listening.take()
                && app.contains(old)
            {
                old.remove_listener(app, &listener);
            }
            let listener = Listener::new(move |app| self.selection_heard(app));
            selection.add_listener(app, listener.clone());
            let revision = selection.revision(app);
            let state = app.get_mut(self);
            state.listening = Some((selection, listener));
            state.seen_revision = revision;
        }
        let widget = self.widget(app).clone();
        selection.set_item_count(app, widget.item_count);
        selection.set_selection_mode(app, widget.selection_mode);
    }

    fn alive(self: Handle<Self>, app: &App) -> bool {
        app.contains(self) && self.mounted(app)
    }

    /// Runs `work` once the frame is over, if the view is still mounted.
    fn after_frame(self: Handle<Self>, app: &mut App, work: impl Fn(&mut App) + 'static) {
        let view = View::maybe_id_of(app, self.context(app));
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
}

// MARK: Building

impl ItemsViewState {
    /// One item: the application's container in the scope that tells it its index, measured, `Spacing` after it but the last.
    fn item(self: Handle<Self>, app: &mut App, context: BuildContext, index: usize) -> WidgetRef {
        let widget = self.widget(app).clone();
        let container = (widget.item_template)(app, context, index);
        let scoped = ItemsViewItemScope {
            item: ItemsViewItem { index, view: self },
            child: container.into_widget(),
        };
        let measured = SizeObserver::new(
            scoped,
            Rc::new(move |app, _, size| self.item_measured(app, index, size)),
        );
        let spacing = if index + 1 < widget.item_count {
            widget.layout.spacing
        } else {
            0.0
        };
        let gap = match widget.layout.orientation {
            Axis::Vertical => EdgeInsetsGeometry::from_ltrb(0.0, 0.0, 0.0, spacing),
            Axis::Horizontal => EdgeInsetsGeometry::from_ltrb(0.0, 0.0, spacing, 0.0),
        };
        Padding::new(gap).child(measured).into_widget()
    }

    /// `PART_ItemsRepeater` in `PART_ScrollView`: a lazy list along a vertical stack, a row of every item along a horizontal one.
    fn items(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.widget(app).clone();
        let scroll = self.scroll(app);
        let [left, top, right, bottom] = widget.padding;
        let padding = EdgeInsetsGeometry::from_ltrb(left, top, right, bottom);
        match widget.layout.orientation {
            Axis::Vertical => {
                let list = ListView::builder(move |app, context, index| {
                    Some(self.item(app, context, index as usize))
                })
                .item_count(widget.item_count as i32)
                .build()
                .controller(scroll.native_controller(app))
                .padding(padding);
                let list = match widget.item_extent {
                    Some(extent) => list.item_extent(extent + widget.layout.spacing),
                    None => list,
                };
                ScrollBarViewport::scroller(scroll, list).into_widget()
            }
            Axis::Horizontal => {
                let context = self.context(app);
                let items: Vec<WidgetRef> = (0..widget.item_count)
                    .map(|index| self.item(app, context, index))
                    .collect();
                let row = Padding::new(padding)
                    .child(Row::new().main_axis_size(MainAxisSize::Min).children(items));
                ScrollBarViewport::new(scroll, row)
                    .axis(Axis::Horizontal)
                    .into_widget()
            }
        }
    }
}

impl State for ItemsViewState {
    type Widget = ItemsView;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        let scroll = ScrollViewportController::new(app);
        app.get_mut(self).scroll = Some(scroll);
        if self.widget(app).selection.is_none() {
            let own = ItemsViewSelection::new(app);
            app.get_mut(self).own_selection = Some(own);
        }
        self.sync_selection(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &ItemsView) {
        if self.widget(app).selection.is_none() && app.get(self).own_selection.is_none() {
            let own = ItemsViewSelection::new(app);
            app.get_mut(self).own_selection = Some(own);
        }
        self.sync_selection(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some((selection, listener)) = app.get_mut(self).listening.take()
            && app.contains(selection)
        {
            selection.remove_listener(app, &listener);
        }
        if let Some(own) = app.get_mut(self).own_selection.take() {
            own.dispose(app);
            app.destroy(own);
        }
        if let Some(scroll) = app.get_mut(self).scroll.take() {
            scroll.dispose(app);
            app.destroy(scroll);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let corner = ThemeResources::of(app, context).control_corner_radius;
        let items = self.items(app);
        let [top_left, top_right, bottom_right, bottom_left] = corner.map(Radius::circular);
        let clipped = ClipRRect::new()
            .border_radius(
                BorderRadius::only(top_left, top_right, bottom_left, bottom_right).into(),
            )
            .clip_behavior(Clip::AntiAlias)
            .child(items);
        Focus::new(clipped)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.key(app, event)))
            .into_widget()
    }
}
