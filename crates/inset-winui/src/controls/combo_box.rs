//! ComboBox_themeresources.xaml and ComboBox_Partial.cpp: a box showing the selected item that
//! opens a drop-down of the items, or, editable, a text box whose text the box matches to the
//! items. The drop-down puts the selected item over the box where it fits, at least as wide as
//! the box and within the window, and closes when a press lands outside it.
//!
//! The application owns the selection: `selected_index`, and in an editable box `custom_value`,
//! are what it passes, and `selection_changed` asks for another. The selection the box asks for
//! is what the box itself reads as its selection until the application next builds it; if the
//! application passed another by then, the box shows that one. The drop-down is the box's own
//! unless the application takes it over with `is_drop_down_open`.

mod item;
mod popup;
mod search;

pub use item::ComboBoxItem;

use super::flyout::shadow::PopupShadow;
use super::text_control::TextControl;
use crate::*;
// Inset exports a `FocusState` of its own; the XAML one is meant here.
use crate::FocusState;
use inset_embedder::{Color, FontWeight, Offset, Rect, Size, TextAffinity, TextSelection};
use inset_foundation::{App, Handle, Listenable, Listener, ValueKey};
use inset_gestures::{GestureBinding, K_PRIMARY_BUTTON, PointerEvent};
use inset_painting::{
    Alignment, EdgeInsetsGeometry, PaintingBinding, TextPainter, TextSpan, transform_rect,
};
use inset_rendering::{BoxConstraints, HitTestBehavior};
use inset_scheduler::FrameCallback;
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey, TextEditingValue};
use inset_widgets::Listener as PointerListener;
use inset_widgets::*;
use item::{ComboBoxItemContainer, ITEM_MARGIN};
use popup::{ArrangeInput, Arrangement, ItemProbe, PopupPosition};
use search::{Match, TypeAhead};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    time::Duration,
};

/// XAML `ComboBoxSelectionChangedTrigger`: what moving through the open drop-down with the
/// keyboard does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ComboBoxSelectionChangedTrigger {
    /// The item moved to is drawn selected; Enter or a click selects it.
    #[default]
    Committed,
    /// The item moved to is selected.
    Always,
}

/// Something a `ComboBox` selects: an element of `RemovedItems` or `AddedItems`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComboBoxSelectedItem {
    /// The item at `index` of `items`, with its `id`.
    Item {
        /// Its index in `items`.
        index: usize,
        /// Its `id`.
        id: String,
    },
    /// A custom value: text committed in an editable box that matches no item.
    CustomValue(String),
}

impl ComboBoxSelectedItem {
    /// The item's index, for an item of `items`.
    pub fn index(&self) -> Option<usize> {
        match self {
            Self::Item { index, .. } => Some(*index),
            Self::CustomValue(_) => None,
        }
    }

    /// The text, for a custom value.
    pub fn custom_value(&self) -> Option<&str> {
        match self {
            Self::Item { .. } => None,
            Self::CustomValue(text) => Some(text),
        }
    }
}

/// XAML `SelectionChangedEventArgs` of a ComboBox: what was selected and what is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboBoxSelectionChangedEventArgs {
    /// `RemovedItems`: what was selected, if anything.
    pub removed_item: Option<ComboBoxSelectedItem>,

    /// `AddedItems`: what the box asks to select, if anything.
    pub added_item: Option<ComboBoxSelectedItem>,
}

impl ComboBoxSelectionChangedEventArgs {
    /// The `selected_index` the box asks for.
    pub fn selected_index(&self) -> Option<usize> {
        self.added_item
            .as_ref()
            .and_then(ComboBoxSelectedItem::index)
    }

    /// The `custom_value` the box asks for.
    pub fn custom_value(&self) -> Option<&str> {
        self.added_item
            .as_ref()
            .and_then(ComboBoxSelectedItem::custom_value)
    }
}

/// Receives XAML `SelectionChanged`.
pub type ComboBoxSelectionChangedHandler = Rc<dyn Fn(&mut App, ComboBoxSelectionChangedEventArgs)>;

/// XAML `ComboBoxTextSubmittedEventArgs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboBoxTextSubmittedEventArgs {
    /// `Text`: the text committed, which matches no item.
    pub text: String,

    /// `Handled`: set it to keep the box from selecting the text as a custom value.
    pub handled: bool,
}

/// Receives XAML `TextSubmitted`.
pub type ComboBoxTextSubmittedHandler = Rc<dyn Fn(&mut App, &mut ComboBoxTextSubmittedEventArgs)>;

/// Asks the application to open or close a drop-down it owns.
pub type ComboBoxDropDownOpenChangedHandler = Rc<dyn Fn(&mut App, bool)>;

/// XAML `ComboBox` with the template of `DefaultComboBoxStyle`.
#[derive(Clone)]
pub struct ComboBox {
    /// `Items`, in order; ids are unique and stay the same across rebuilds.
    pub items: Vec<ComboBoxItem>,

    /// `SelectedIndex`; an index of `items`.
    pub selected_index: Option<usize>,

    /// `SelectionChanged`: asks the application for another selection; the application owns
    /// `selected_index` and `custom_value`.
    pub selection_changed: ComboBoxSelectionChangedHandler,

    /// `SelectedItem` when it is a custom value of an editable box: text matching no item,
    /// selected while `selected_index` is `None`.
    pub custom_value: Option<String>,

    /// `Header`: shown above the box.
    pub header: Option<WidgetRef>,

    /// `PlaceholderText`: shown while nothing is selected.
    pub placeholder_text: String,

    /// `PlaceholderForeground`: overrides the placeholder's foreground in every state.
    pub placeholder_foreground: Option<Color>,

    /// `Description`: shown below the box.
    pub description: Option<WidgetRef>,

    /// `IsDropDownOpen`, when the application owns it; `None` leaves the drop-down to the box.
    pub is_drop_down_open: Option<bool>,

    /// Asks the application, when it owns `is_drop_down_open`, to open or close the drop-down.
    pub is_drop_down_open_changed: Option<ComboBoxDropDownOpenChangedHandler>,

    /// `DropDownOpened`.
    pub drop_down_opened: Option<Listener>,

    /// `DropDownClosed`.
    pub drop_down_closed: Option<Listener>,

    /// `MaxDropDownHeight`: the tallest the drop-down is; 504 by default.
    pub max_drop_down_height: f64,

    /// `IsEnabled`.
    pub is_enabled: bool,

    /// `IsEditable`: whether the box is a text box whose text is matched to the items.
    pub is_editable: bool,

    /// `TextSubmitted`: an editable box committing text that matches no item.
    pub text_submitted: Option<ComboBoxTextSubmittedHandler>,

    /// `IsTextSearchEnabled`: whether typing selects an item by its text; true by default.
    pub is_text_search_enabled: bool,

    /// `SelectionChangedTrigger`.
    pub selection_changed_trigger: ComboBoxSelectionChangedTrigger,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl ComboBox {
    /// Creates a box choosing among `items`, showing `selected_index`, which asks for other
    /// selections through `selection_changed`.
    pub fn new(
        items: Vec<ComboBoxItem>,
        selected_index: Option<usize>,
        selection_changed: impl Fn(&mut App, ComboBoxSelectionChangedEventArgs) + 'static,
    ) -> ComboBox {
        ComboBox {
            items,
            selected_index,
            selection_changed: Rc::new(selection_changed),
            custom_value: None,
            header: None,
            placeholder_text: String::new(),
            placeholder_foreground: None,
            description: None,
            is_drop_down_open: None,
            is_drop_down_open_changed: None,
            drop_down_opened: None,
            drop_down_closed: None,
            max_drop_down_height: MAX_DROP_DOWN_HEIGHT,
            is_enabled: true,
            is_editable: false,
            text_submitted: None,
            is_text_search_enabled: true,
            selection_changed_trigger: ComboBoxSelectionChangedTrigger::Committed,
            key: None,
        }
    }

    /// Sets SelectedItem to a custom value.
    pub fn custom_value(mut self, value: impl Into<String>) -> Self {
        self.custom_value = Some(value.into());
        self
    }

    /// Sets Header.
    pub fn header<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.header = Some(value.into_widget());
        self
    }

    /// Sets PlaceholderText.
    pub fn placeholder_text(mut self, value: impl Into<String>) -> Self {
        self.placeholder_text = value.into();
        self
    }

    /// Sets PlaceholderForeground.
    pub fn placeholder_foreground(mut self, value: Color) -> Self {
        self.placeholder_foreground = Some(value);
        self
    }

    /// Sets Description.
    pub fn description<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.description = Some(value.into_widget());
        self
    }

    /// Gives the application IsDropDownOpen: the box shows `open` and asks for changes through
    /// `changed`.
    pub fn is_drop_down_open(
        mut self,
        open: bool,
        changed: impl Fn(&mut App, bool) + 'static,
    ) -> Self {
        self.is_drop_down_open = Some(open);
        self.is_drop_down_open_changed = Some(Rc::new(changed));
        self
    }

    /// Sets DropDownOpened.
    pub fn drop_down_opened(mut self, value: Listener) -> Self {
        self.drop_down_opened = Some(value);
        self
    }

    /// Sets DropDownClosed.
    pub fn drop_down_closed(mut self, value: Listener) -> Self {
        self.drop_down_closed = Some(value);
        self
    }

    /// Sets MaxDropDownHeight.
    pub fn max_drop_down_height(mut self, value: f64) -> Self {
        self.max_drop_down_height = value;
        self
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets IsEditable.
    pub fn is_editable(mut self, value: bool) -> Self {
        self.is_editable = value;
        self
    }

    /// Sets TextSubmitted.
    pub fn text_submitted(
        mut self,
        value: impl Fn(&mut App, &mut ComboBoxTextSubmittedEventArgs) + 'static,
    ) -> Self {
        self.text_submitted = Some(Rc::new(value));
        self
    }

    /// Sets IsTextSearchEnabled.
    pub fn is_text_search_enabled(mut self, value: bool) -> Self {
        self.is_text_search_enabled = value;
        self
    }

    /// Sets SelectionChangedTrigger.
    pub fn selection_changed_trigger(mut self, value: ComboBoxSelectionChangedTrigger) -> Self {
        self.selection_changed_trigger = value;
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for ComboBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComboBox")
            .field("items", &self.items)
            .field("selected_index", &self.selected_index)
            .field("custom_value", &self.custom_value)
            .field("placeholder_text", &self.placeholder_text)
            .field("is_editable", &self.is_editable)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// `DefaultComboBoxStyle`'s `MaxDropDownHeight`.
const MAX_DROP_DOWN_HEIGHT: f64 = 504.0;

/// `ColumnDefinition` 1 of the template: the glyph's column.
const GLYPH_COLUMN_WIDTH: f64 = 38.0;

/// `DropDownGlyph`'s `Width`, `Height`, `Margin` and its fallback icon's `FontSize`.
const GLYPH_SIZE: f64 = 12.0;
const GLYPH_MARGIN: [f64; 4] = [0.0, 0.0, 14.0, 0.0];

/// `DropDownOverlay`'s `Width` and `Margin`.
const DROP_DOWN_OVERLAY_WIDTH: f64 = 30.0;
const DROP_DOWN_OVERLAY_MARGIN: [f64; 4] = [4.0; 4];

/// `HighlightBackground`'s `Margin`.
const HIGHLIGHT_BACKGROUND_MARGIN: [f64; 4] = [-4.0; 4];

/// `Pill`'s `Margin` in the box.
const PILL_MARGIN: [f64; 4] = [1.0, 0.0, 0.0, 0.0];

/// `HeaderContentPresenter`'s `LineHeight`.
const HEADER_LINE_HEIGHT: f64 = 20.0;

/// `ScrollViewerLineDelta` of ScrollViewer_Partial.h: one line of `ScrollInDirection`.
const LINE_DELTA: f64 = 16.0;

/// `ComboBoxTextBoxStyle`: the TextBox template of an editable box's `EditableText`, with the
/// corner radius `ComboBoxHelper` gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ComboBoxTextBoxStyle {
    /// `CornerRadius`: the box's, with the corners next to an open drop-down square.
    pub(super) corner_radius: [f64; 4],
}

/// `SelectedIndex` and `SelectedItem` together.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Selection {
    /// Nothing selected.
    None,
    /// The item at this index.
    Index(usize),
    /// A custom value.
    Custom(String),
}

impl Selection {
    /// The selection `widget` passes.
    fn of(widget: &ComboBox) -> Selection {
        match (widget.selected_index, &widget.custom_value) {
            (Some(index), _) => Selection::Index(index),
            (None, Some(text)) => Selection::Custom(text.clone()),
            (None, None) => Selection::None,
        }
    }

    /// `SelectedIndex`.
    fn index(&self) -> Option<usize> {
        match self {
            Selection::Index(index) => Some(*index),
            _ => None,
        }
    }

    /// `SelectedItem`, as `SelectionChangedEventArgs` reports it.
    fn item(&self, items: &[ComboBoxItem]) -> Option<ComboBoxSelectedItem> {
        match self {
            Selection::None => None,
            Selection::Index(index) => Some(ComboBoxSelectedItem::Item {
                index: *index,
                id: items[*index].id.clone(),
            }),
            Selection::Custom(text) => Some(ComboBoxSelectedItem::CustomValue(text.clone())),
        }
    }
}

/// `FocusStates` of the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComboBoxFocusState {
    /// `HighlightBackground` and `Pill` show.
    Focused,
    /// As `Focused`, pressed.
    FocusedPressed,
    Unfocused,
    PointerFocused,
    /// The drop-down is open.
    FocusedDropDown,
}

/// `EditableModeStates` of an editable box: whether its text box has the focus, and the pointer
/// over `DropDownOverlay`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names, reason = "the template's state names")]
enum EditableModeState {
    TextBoxFocused,
    TextBoxFocusedOverlayPointerOver,
    TextBoxFocusedOverlayPressed,
    TextBoxOverlayPointerOver,
    TextBoxOverlayPressed,
    TextBoxUnfocused,
}

/// What a key typed in an editable box's text box was to `ProcessSearch`: a character, or
/// Backspace, which searches for exact matches only.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Typed {
    Character(String),
    Backspace,
}

impl Typed {
    /// The `L' '` that `ProcessSearch` is called with when no key prompted it.
    fn none() -> Typed {
        Typed::Character(" ".to_owned())
    }
}

impl StatefulWidget for ComboBox {
    type State = ComboBoxState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ComboBoxState {
        ComboBoxState {
            state: StateData::new(),
            scope_focus: None,
            focus: None,
            text: None,
            text_focus: None,
            popup: None,
            item_focus: HashMap::new(),
            scroll: None,
            background_key: GlobalKey::new(),
            overlay_key: GlobalKey::new(),
            heights: HashMap::new(),
            widths: HashMap::new(),
            last_popup_height: Rc::new(Cell::new(0.0)),
            window_at_open: None,
            selection: Selection::of(self),
            requested: None,
            open: false,
            closing: false,
            opened_up: false,
            pointer_over_main: false,
            pointer_over_popup: false,
            pointer_over_overlay: false,
            pressed: false,
            restore_index_set: false,
            index_to_restore: None,
            closing_due_to_cancel: false,
            ignore_cancel_key_downs: false,
            focused_index: None,
            forced_selected: None,
            forced_unselected: None,
            selection_active: false,
            type_ahead: TypeAhead::default(),
            search_result: None,
            custom_value_ref: None,
            pending_typed: None,
            text_box_enabled: false,
            should_move_focus_to_text_box: false,
            building: false,
            open_epoch: 0,
            key_time: None,
            pending_open: None,
        }
    }
}

/// The parts ComboBox_Partial.cpp keeps of its template, its flags and its search.
pub struct ComboBoxState {
    state: StateData<ComboBox>,

    /// Focus anywhere in the box, its text box or its drop-down: what `HasFocus` asks.
    scope_focus: Option<Handle<FocusNode>>,

    /// The ComboBox's own focus, which `Focus` gives it.
    focus: Option<Handle<FocusNode>>,

    /// `EditableText`'s text.
    text: Option<Handle<TextEditingController>>,

    /// `EditableText`'s focus.
    text_focus: Option<Handle<FocusNode>>,

    /// Shows `Popup`.
    popup: Option<Handle<OverlayPortalController>>,

    /// The items' focus, by id, owned here so that the box can focus an item it scrolls to.
    item_focus: HashMap<String, Handle<FocusNode>>,

    /// The drop-down's scroll position, made at each opening where the opening starts.
    scroll: Option<Handle<ScrollViewportController>>,

    /// Identifies `Background`, the drop-down's anchor.
    background_key: GlobalKey,

    /// Identifies `DropDownOverlay`.
    overlay_key: GlobalKey,

    /// Each item's height in the drop-down, by id, as last laid out.
    heights: HashMap<String, f64>,

    /// The width each item laid out in this opening asks for, by id; the drop-down is as wide
    /// as the widest.
    widths: HashMap<String, f64>,

    /// The drop-down's height in its last layout.
    last_popup_height: Rc<Cell<f64>>,

    /// The window's size when the drop-down opened; a change closes it.
    window_at_open: Option<Size>,

    /// The application's selection, as last passed.
    selection: Selection,

    /// The selection the box last asked for, until the application next builds it.
    requested: Option<Selection>,

    /// `IsDropDownOpen`, as the box has it.
    open: bool,

    /// `m_isDropDownClosing`.
    closing: bool,

    /// `m_openedUp`: an editable box's drop-down is above the box.
    opened_up: bool,

    /// `m_IsPointerOverMain`.
    pointer_over_main: bool,

    /// `m_IsPointerOverPopup`.
    pointer_over_popup: bool,

    /// `m_IsPointerOverDropDownOverlay`.
    pointer_over_overlay: bool,

    /// `m_bIsPressed`.
    pressed: bool,

    /// `m_restoreIndexSet`.
    restore_index_set: bool,

    /// `m_indexToRestoreOnCancel`.
    index_to_restore: Option<usize>,

    /// `m_isClosingDueToCancel`.
    closing_due_to_cancel: bool,

    /// `m_ignoreCancelKeyDowns`.
    ignore_cancel_key_downs: bool,

    /// The Selector's `m_focusedIndex`.
    focused_index: Option<usize>,

    /// `m_indexForcedToSelectedVisual`.
    forced_selected: Option<usize>,

    /// `m_indexForcedToUnselectedVisual`.
    forced_unselected: Option<usize>,

    /// `IsSelectionActive`: the box has the focus.
    selection_active: bool,

    /// A non-editable box's typed search, and both kinds' `m_searchString`.
    type_ahead: TypeAhead,

    /// `m_searchResultIndexSet` and `m_searchResultIndex`.
    search_result: Option<Option<usize>>,

    /// `m_customValueRef`.
    custom_value_ref: Option<String>,

    /// The key typed in the text box whose edit has not arrived yet.
    pending_typed: Option<Typed>,

    /// `EditableText` has its size: it is shown instead of `ContentPresenter`.
    text_box_enabled: bool,

    /// `m_shouldMoveFocusToTextBox`.
    should_move_focus_to_text_box: bool,

    /// The application's values are being applied, during a build: callbacks to the
    /// application wait for the frame's end.
    building: bool,

    /// Invalidates work queued for an earlier opening.
    open_epoch: u64,

    /// When the last key event arrived, the clock of the typed search.
    key_time: Option<Duration>,

    /// The drop-down state an application that owns it passed, waiting for the frame's end.
    pending_open: Option<bool>,
}

impl ComboBoxState {
    fn scope_focus(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).scope_focus.expect("an initialized ComboBox")
    }

    fn box_focus(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).focus.expect("an initialized ComboBox")
    }

    fn text(self: Handle<Self>, app: &App) -> Handle<TextEditingController> {
        app.get(self).text.expect("an initialized ComboBox")
    }

    fn text_focus(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).text_focus.expect("an initialized ComboBox")
    }

    fn popup(self: Handle<Self>, app: &App) -> Handle<OverlayPortalController> {
        app.get(self).popup.expect("an initialized ComboBox")
    }

    fn alive(self: Handle<Self>, app: &App) -> bool {
        app.contains(self) && self.mounted(app)
    }

    fn view(self: Handle<Self>, app: &App) -> Option<inset_embedder::ViewId> {
        View::maybe_id_of(app, self.context(app))
    }

    /// Runs `work` once the frame is over, if the box is still mounted.
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

    fn is_editable(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).is_editable
    }

    fn item_count(self: Handle<Self>, app: &App) -> usize {
        self.widget(app).items.len()
    }

    // MARK: Selection

    /// `SelectedIndex` and `SelectedItem` as the box's own logic reads them: what it last asked
    /// for, until the application answers, else the application's.
    fn selected(self: Handle<Self>, app: &App) -> Selection {
        let state = app.get(self);
        state
            .requested
            .clone()
            .unwrap_or_else(|| state.selection.clone())
    }

    /// `SelectedIndex`.
    fn selected_index(self: Handle<Self>, app: &App) -> Option<usize> {
        self.selected(app).index()
    }

    /// The `put_SelectedIndex` and `put_SelectedItem` of the box's own logic: a different
    /// selection becomes what the box reads, runs what `SelectedItem` and `OnSelectionChanged`
    /// do, and is asked of the application through `SelectionChanged`.
    fn set_selection(self: Handle<Self>, app: &mut App, new: Selection) {
        let old = self.selected(app);
        if old == new {
            return;
        }
        app.get_mut(self).requested = Some(new.clone());
        self.selection_did_change(app, &old, &new);
        let items = self.widget(app).items.clone();
        let args = ComboBoxSelectionChangedEventArgs {
            removed_item: old.item(&items),
            added_item: new.item(&items),
        };
        let handler = self.widget(app).selection_changed.clone();
        self.raise(app, move |app| handler(app, args.clone()));
        self.set_state(app, |_| {});
    }

    /// What follows a change of selection: `OnPropertyChanged2` for `SelectedItem`, then
    /// `OnSelectionChanged`.
    fn selection_did_change(self: Handle<Self>, app: &mut App, old: &Selection, new: &Selection) {
        self.selected_item_changed(app);
        self.on_selection_changed(app, old.index(), new.index());
    }

    /// `OnPropertyChanged2` for `SelectedItem` in an editable box: the search result becomes the
    /// selected index, a custom value is kept for a revert, the text box shows the selection
    /// with its text selected, and a search the closed box has not committed is committed.
    fn selected_item_changed(self: Handle<Self>, app: &mut App) {
        if !self.is_editable(app) {
            return;
        }
        let selected = self.selected(app);
        app.get_mut(self).search_result = Some(selected.index());
        if let Selection::Custom(text) = &selected {
            app.get_mut(self).custom_value_ref = Some(text.clone());
        }
        if let Some(text) = self.selection_text(app, &selected) {
            self.update_editable_text_box(app, &text, true, true);
        }
        if matches!(app.get(self).search_result, Some(Some(_))) && !app.get(self).open {
            self.commit_revert_editable_search(app, false);
        }
    }

    /// `ComboBox::OnSelectionChanged`: in the open drop-down, the Selector focuses the newly
    /// selected item, except in an editable box; a closed editable box remembers the selection
    /// to restore.
    fn on_selection_changed(
        self: Handle<Self>,
        app: &mut App,
        old: Option<usize>,
        new: Option<usize>,
    ) {
        if app.get(self).open {
            if !self.is_editable(app)
                && let Some(new) = new
            {
                self.set_focused_item_keeping_state(app, Some(new), true);
            }
        } else if self.is_editable(app) && !app.get(self).restore_index_set {
            let state = app.get_mut(self);
            state.restore_index_set = true;
            state.index_to_restore = old;
        }
    }

    /// The text `TryGetStringValue` gets from a selection: an item's text, which is empty for an
    /// item without, or the custom value; none without a selection.
    fn selection_text(self: Handle<Self>, app: &App, selection: &Selection) -> Option<String> {
        match selection {
            Selection::None => None,
            Selection::Index(index) => Some(self.item_text(app, *index)),
            Selection::Custom(text) => Some(text.clone()),
        }
    }

    /// An item's text, empty for an item without.
    fn item_text(self: Handle<Self>, app: &App, index: usize) -> String {
        self.widget(app).items[index]
            .plain_text
            .clone()
            .unwrap_or_default()
    }

    /// `OverrideSelectedIndexForVisualStates`: draws `index` selected and the selected item
    /// unselected, while they differ.
    fn override_selected_index_for_visual_states(
        self: Handle<Self>,
        app: &mut App,
        index: Option<usize>,
    ) {
        self.clear_selected_index_override_for_visual_states(app);
        let selected = self.selected_index(app);
        if index != selected {
            let state = app.get_mut(self);
            state.forced_selected = index;
            if selected.is_some() {
                state.forced_unselected = selected;
            }
        }
        self.set_state(app, |_| {});
    }

    /// `ClearSelectedIndexOverrideForVisualStates`.
    fn clear_selected_index_override_for_visual_states(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.forced_selected = None;
            state.forced_unselected = None;
        });
    }

    /// `ComboBoxItem::ChangeVisualState`'s `IsSelected`: the search result in an editable box
    /// that has one, else the selection, unless the box draws another item in its place.
    fn appears_selected(self: Handle<Self>, app: &App, index: usize) -> bool {
        let state = app.get(self);
        let mut selected = match state.search_result {
            Some(result) => result == Some(index),
            None => self.selected_index(app) == Some(index),
        };
        if state.forced_selected == Some(index) {
            selected = true;
        } else if state.forced_unselected == Some(index) {
            selected = false;
        }
        selected
    }

    // MARK: Opening and closing

    /// `put_IsDropDownOpen` from the box's own logic: a change opens or closes the drop-down and
    /// is asked of an application that owns `is_drop_down_open`.
    fn put_is_drop_down_open(self: Handle<Self>, app: &mut App, open: bool) {
        if app.get(self).open == open {
            return;
        }
        if let Some(changed) = self.widget(app).is_drop_down_open_changed.clone() {
            self.raise(app, move |app| changed(app, open));
        }
        self.set_drop_down_open(app, open);
    }

    /// `OnIsDropDownOpenChanged`: an opening while the drop-down is still closing finishes the
    /// closing first.
    fn set_drop_down_open(self: Handle<Self>, app: &mut App, open: bool) {
        if app.get(self).open == open {
            return;
        }
        app.get_mut(self).open = open;
        if app.get(self).closing && open {
            self.finish_closing_drop_down(app);
        }
        if open {
            self.on_open(app);
        } else {
            self.on_close(app);
        }
    }

    /// `OnOpen`: remembers the selection to restore on cancel, shows the drop-down, and moves the
    /// focus into it: to the text box of an editable box, else to the selected item or, with
    /// nothing selected, to the first, which the box draws selected.
    fn on_open(self: Handle<Self>, app: &mut App) {
        let selected = self.selected_index(app);
        let state = app.get_mut(self);
        if !state.restore_index_set {
            state.restore_index_set = true;
            state.index_to_restore = selected;
        }
        state.closing_due_to_cancel = false;
        self.show_popup(app);
        if let Some(opened) = self.widget(app).drop_down_opened.clone() {
            self.raise(app, move |app| opened.call(app));
        }
        self.set_state(app, |_| {});
        let programmatic = FocusState::coerce_programmatic(app);
        if self.is_editable(app) {
            if !self.editable_text_has_focus(app) {
                self.text_focus(app).request_focus(app, None);
            }
        } else if let Some(index) = selected {
            self.set_focused_item(app, Some(index), false, true, programmatic);
        } else if self.item_count(app) > 0 {
            if self.widget(app).selection_changed_trigger == ComboBoxSelectionChangedTrigger::Always
            {
                self.set_selection(app, Selection::Index(0));
            } else {
                self.override_selected_index_for_visual_states(app, Some(0));
            }
            self.set_focused_item(app, Some(0), false, true, programmatic);
        }
    }

    /// `OnClose`: an editable box commits its search, or reverts it when cancelled; a cancelled
    /// non-editable box goes back to the selection it opened with. Without the closing theme
    /// animation, the drop-down finishes closing at once.
    fn on_close(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).closing = true;
        let cancel = app.get(self).closing_due_to_cancel;
        if self.is_editable(app) {
            self.commit_revert_editable_search(app, cancel);
        } else if cancel
            && let Some(index) = app.get(self).index_to_restore
            && index < self.item_count(app)
        {
            self.set_selection(app, Selection::Index(index));
        }
        let state = app.get_mut(self);
        state.index_to_restore = None;
        state.restore_index_set = false;
        state.closing_due_to_cancel = false;
        self.clear_selected_index_override_for_visual_states(app);
        self.finish_closing_drop_down(app);
        self.set_state(app, |state| {
            state.pointer_over_popup = false;
            state.pointer_over_main = false;
            state.pointer_over_overlay = false;
        });
    }

    /// `FinishClosingDropDown`: the focus goes back from the drop-down to the box, the drop-down
    /// hides, and `DropDownClosed` is raised.
    fn finish_closing_drop_down(self: Handle<Self>, app: &mut App) {
        self.set_focused_item_keeping_state(app, None, false);
        let state = app.get_mut(self);
        state.should_move_focus_to_text_box = true;
        state.pointer_over_popup = false;
        state.closing = false;
        // `ResetMinimumDesiredWindowWidth`.
        state.widths.clear();
        state.window_at_open = None;
        state.focused_index = None;
        self.hide_popup(app);
        if let Some(closed) = self.widget(app).drop_down_closed.clone() {
            self.raise(app, move |app| closed.call(app));
        }
        self.set_state(app, |_| {});
    }

    /// Shows `Popup`, with its list starting where `ArrangePopup` scrolls a pannable list: at the
    /// first item it shows.
    fn show_popup(self: Handle<Self>, app: &mut App) {
        let geometry = self.popup_geometry(app);
        let context = self.context(app);
        let mut offset = 0.0;
        if let Some((target, window)) = geometry
            && let Some(arrangement) = self.arrangement(app, context, target, window)
            && arrangement.pannable
            && !self.is_editable(app)
        {
            let heights = self.item_heights(app, context);
            offset = heights[..arrangement.first_item].iter().sum();
        }
        let old = app.get_mut(self).scroll.take();
        self.retire_scroll(app, old);
        let scroll = ScrollViewportController::with_initial_offset(app, offset);
        let state = app.get_mut(self);
        state.scroll = Some(scroll);
        state.window_at_open = geometry.map(|(_, window)| window);
        state.open_epoch += 1;
        let epoch = state.open_epoch;
        self.popup(app).show(app);
        // `OnElementPopupChildLoaded`.
        self.after_frame(app, move |app| {
            if app.get(self).open && app.get(self).open_epoch == epoch {
                self.popup_child_loaded(app);
            }
        });
    }

    /// Hides `Popup`; its list's scroll position is released once the list is gone.
    fn hide_popup(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).open_epoch += 1;
        self.popup(app).hide(app);
        let old = app.get_mut(self).scroll.take();
        self.retire_scroll(app, old);
    }

    /// Disposes a drop-down's scroll position after its list unmounts.
    fn retire_scroll(
        self: Handle<Self>,
        app: &mut App,
        scroll: Option<Handle<ScrollViewportController>>,
    ) {
        let Some(scroll) = scroll else {
            return;
        };
        let view = self.view(app);
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if app.contains(scroll) {
                    scroll.dispose(app);
                    app.destroy(scroll);
                }
            }),
        );
    }

    /// `OnElementPopupChildLoaded`: an editable box scrolls its selection, or the item its search
    /// found, into view, drawing a found item selected while selection waits for a commit.
    fn popup_child_loaded(self: Handle<Self>, app: &mut App) {
        if !self.is_editable(app) {
            return;
        }
        let trigger = self.widget(app).selection_changed_trigger;
        let current = match app.get(self).search_result {
            Some(result) if trigger != ComboBoxSelectionChangedTrigger::Always => {
                self.override_selected_index_for_visual_states(app, result);
                result
            }
            _ => self.selected_index(app),
        };
        if let Some(index) = current {
            self.scroll_into_view(app, index);
        }
    }

    /// The light-dismiss canvas: a press of the left button outside the drop-down cancels it.
    fn light_dismiss(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).closing_due_to_cancel = true;
        self.put_is_drop_down_open(app, false);
    }

    // MARK: Focus

    /// `HasFocus`: whether the focus is in the box, its text box or its drop-down.
    fn has_focus(self: Handle<Self>, app: &mut App) -> bool {
        self.scope_focus(app).has_focus(app)
    }

    /// `EditableTextHasFocus`.
    fn editable_text_has_focus(self: Handle<Self>, app: &App) -> bool {
        self.is_editable(app) && self.text_focus(app).has_primary_focus(app)
    }

    /// `Focus(state)` on the box itself.
    fn focus_box(self: Handle<Self>, app: &mut App, state: FocusState) {
        let node = self.box_focus(app);
        FocusState::name(app, node.as_node(), state);
        node.request_focus(app, None);
    }

    /// `SetFocusedItem` with the focus state of the element that has the focus, or programmatic
    /// focus without.
    fn set_focused_item_keeping_state(
        self: Handle<Self>,
        app: &mut App,
        index: Option<usize>,
        scroll: bool,
    ) {
        let state = if self.has_focus(app) {
            FocusState::current(app)
        } else {
            FocusState::coerce_programmatic(app)
        };
        self.set_focused_item(app, index, scroll, false, state);
    }

    /// `SetFocusedItem`: records the focused index, scrolls the item into view when asked and the
    /// drop-down is open, and focuses it when `force` or the box has the focus. Without an item,
    /// the box focuses itself.
    fn set_focused_item(
        self: Handle<Self>,
        app: &mut App,
        index: Option<usize>,
        scroll: bool,
        force: bool,
        state: FocusState,
    ) {
        let count = self.item_count(app);
        let index = index.filter(|index| *index < count);
        let should_focus = force || self.has_focus(app);
        if should_focus
            || app
                .get(self)
                .focused_index
                .is_some_and(|focused| focused >= count)
        {
            app.get_mut(self).focused_index = index;
        }
        let Some(index) = index else {
            if should_focus {
                self.focus_box(app, state);
            }
            return;
        };
        if scroll && app.get(self).open {
            self.scroll_into_view(app, index);
        }
        if should_focus {
            self.focus_item(app, index, state);
        }
    }

    /// `FocusSelfOrChild` on the item at `index`: once it is built, if the drop-down is still
    /// open then.
    fn focus_item(self: Handle<Self>, app: &mut App, index: usize, state: FocusState) {
        let id = self.widget(app).items[index].id.clone();
        let node = self.item_focus_node(app, &id);
        FocusState::name(app, node.as_node(), state);
        self.request_item_focus(app, node, app.get(self).open_epoch, 0);
    }

    /// Requests focus for an item, retrying for a few frames while the list builds it.
    fn request_item_focus(
        self: Handle<Self>,
        app: &mut App,
        node: Handle<FocusNode>,
        epoch: u64,
        attempt: u32,
    ) {
        if node.as_node().context(app).is_some() {
            node.request_focus(app, None);
            return;
        }
        if attempt < 3 {
            self.after_frame(app, move |app| {
                if app.get(self).open && app.get(self).open_epoch == epoch {
                    self.request_item_focus(app, node, epoch, attempt + 1);
                }
            });
        }
    }

    /// The focus of the item `id`, made the first time it is asked for.
    fn item_focus_node(self: Handle<Self>, app: &mut App, id: &str) -> Handle<FocusNode> {
        if let Some(node) = app.get(self).item_focus.get(id) {
            return *node;
        }
        let node = FocusNode::new(app);
        // The items are reached with the arrow keys, never by Tab.
        node.as_node().set_skip_traversal(app, true);
        app.get_mut(self).item_focus.insert(id.to_owned(), node);
        node
    }

    /// The index of the item with the primary focus.
    fn focused_item_index(self: Handle<Self>, app: &App) -> Option<usize> {
        let focus = app.get(self);
        let id = focus
            .item_focus
            .iter()
            .find(|(_, node)| node.has_primary_focus(app))?
            .0;
        self.widget(app)
            .items
            .iter()
            .position(|item| &item.id == id)
    }

    /// `FocusChanged`: the box is selection-active while it has the focus, and losing the focus
    /// cancels the drop-down.
    fn focus_changed(self: Handle<Self>, app: &mut App, has_focus: bool) {
        app.get_mut(self).selection_active = has_focus;
        if !has_focus {
            app.get_mut(self).closing_due_to_cancel = true;
            self.put_is_drop_down_open(app, false);
        }
        self.set_state(app, |_| {});
    }

    /// `OnGotFocus`, for the focus arriving anywhere in the box. In an editable box, a focused
    /// text box takes its size and selects its text, matching it to the items; the focus arriving
    /// elsewhere moves on to the text box when it has no size yet or has to.
    fn on_got_focus(self: Handle<Self>, app: &mut App) {
        let has_focus = self.has_focus(app);
        self.focus_changed(app, has_focus);
        if !self.is_editable(app) {
            return;
        }
        if self.editable_text_has_focus(app) {
            app.get_mut(self).should_move_focus_to_text_box = false;
            self.ensure_text_box_is_enabled(app, false);
            self.select_all(app);
            self.process_search(app, &Typed::none());
        } else if !app.get(self).text_box_enabled || app.get(self).should_move_focus_to_text_box {
            self.ensure_text_box_is_enabled(app, true);
        }
    }

    /// `OnLostFocus`, for the focus leaving the box: an editable box commits its text, and its
    /// text box gives way to `ContentPresenter`.
    fn on_lost_focus(self: Handle<Self>, app: &mut App) {
        let has_focus = self.has_focus(app);
        self.focus_changed(app, has_focus);
        if self.is_editable(app) && !has_focus {
            self.commit_revert_editable_search(app, false);
            self.set_state(app, |state| {
                state.text_box_enabled = false;
                state.should_move_focus_to_text_box = true;
            });
        }
    }

    /// The box's focus changed: arriving focus runs `OnGotFocus`.
    fn box_focus_changed(self: Handle<Self>, app: &mut App) {
        if self.box_focus(app).has_primary_focus(app) {
            self.on_got_focus(app);
        }
        self.set_state(app, |_| {});
    }

    /// The text box's focus changed: arriving focus runs `OnGotFocus`.
    fn text_focus_changed(self: Handle<Self>, app: &mut App) {
        if self.text_focus(app).has_primary_focus(app) {
            self.on_got_focus(app);
        }
        self.set_state(app, |_| {});
    }

    /// `ItemFocused` and `ItemUnfocused`, then `OnGotFocus`.
    fn item_focus_changed(self: Handle<Self>, app: &mut App, index: usize, focused: bool) {
        if focused {
            app.get_mut(self).focused_index = Some(index);
            self.on_got_focus(app);
        } else if app.get(self).focused_index == Some(index) {
            app.get_mut(self).focused_index = None;
        }
        self.set_state(app, |_| {});
    }

    /// The focus entered or left the box as a whole.
    fn scope_focus_changed(self: Handle<Self>, app: &mut App, has_focus: bool) {
        if !has_focus {
            self.on_lost_focus(app);
        }
    }

    // MARK: Editable mode

    /// `EnsureTextBoxIsEnabled`: the text box takes its size in place of `ContentPresenter`, and
    /// the focus when asked.
    fn ensure_text_box_is_enabled(self: Handle<Self>, app: &mut App, move_focus: bool) {
        self.set_state(app, |state| state.text_box_enabled = true);
        if move_focus {
            self.text_focus(app).request_focus(app, None);
            app.get_mut(self).should_move_focus_to_text_box = false;
        }
    }

    /// `SetupEditableMode`: the text box shows the selection, with no search or revert pending.
    fn setup_editable_mode(self: Handle<Self>, app: &mut App) {
        let selected = self.selected(app);
        if let Some(text) = self.selection_text(app, &selected) {
            self.update_editable_text_box(app, &text, false, false);
        }
        let state = app.get_mut(self);
        state.restore_index_set = false;
        state.index_to_restore = None;
        state.search_result = None;
        state.type_ahead.reset();
    }

    /// `DisableEditableMode`: no search or revert pending, and a custom value is no longer
    /// selected.
    fn disable_editable_mode(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        state.search_result = None;
        state.type_ahead.reset();
        state.should_move_focus_to_text_box = false;
        state.restore_index_set = false;
        state.index_to_restore = None;
        state.text_box_enabled = false;
        if state.custom_value_ref.take().is_some() {
            self.set_selection(app, Selection::None);
        }
        self.text(app).set_value(app, TextEditingValue::new());
    }

    /// `CommitRevertEditableSearch`. Reverting goes back to the selection remembered when the
    /// drop-down opened or the selection last changed. Committing selects the item the search
    /// found; text that matches no item and is not only spaces raises `TextSubmitted`, and
    /// unless the application handles it, selects an item it matches exactly or else becomes a
    /// custom value. The text box then shows the selection with its text selected.
    fn commit_revert_editable_search(self: Handle<Self>, app: &mut App, restore: bool) {
        if restore {
            if app.get(self).restore_index_set {
                let restore = match app.get(self).index_to_restore {
                    Some(index) if index < self.item_count(app) => Selection::Index(index),
                    _ => match app.get(self).custom_value_ref.clone() {
                        Some(text) => Selection::Custom(text),
                        None => Selection::None,
                    },
                };
                self.set_selection(app, restore);
            }
        } else {
            if app.get(self).search_result.is_none() {
                self.process_search(app, &Typed::none());
            }
            if let Some(Some(index)) = app.get(self).search_result {
                self.set_selection(app, Selection::Index(index));
                app.get_mut(self).custom_value_ref = None;
            } else {
                let text = self.text(app).text_value(app).to_owned();
                if search::is_search_string_valid(&text) {
                    let send = app
                        .get(self)
                        .custom_value_ref
                        .as_ref()
                        .is_none_or(|stored| !search::are_strings_equal(stored, &text));
                    if send {
                        if !self.raise_text_submitted(app, &text) {
                            match self.search_item_source_index(
                                app,
                                &Typed::none(),
                                false,
                                Match::Exact,
                            ) {
                                Some(index) => {
                                    app.get_mut(self).custom_value_ref = None;
                                    self.set_selection(app, Selection::Index(index));
                                }
                                None => self.set_selection(app, Selection::Custom(text)),
                            }
                        }
                    } else if self.selected_index(app).is_some() {
                        self.set_selection(app, Selection::Custom(text));
                    }
                }
            }
        }
        let selected = self.selected(app);
        match self.selection_text(app, &selected) {
            Some(text) => self.update_editable_text_box(app, &text, true, true),
            None => {
                self.text(app).set_value(app, TextEditingValue::new());
                self.select_all(app);
            }
        }
        let state = app.get_mut(self);
        state.search_result = None;
        state.restore_index_set = false;
        state.index_to_restore = None;
        self.set_state(app, |_| {});
    }

    /// `RaiseTextSubmittedEvent`: whether the application handled the text.
    fn raise_text_submitted(self: Handle<Self>, app: &mut App, text: &str) -> bool {
        let Some(handler) = self.widget(app).text_submitted.clone() else {
            return false;
        };
        let mut args = ComboBoxTextSubmittedEventArgs {
            text: text.to_owned(),
            handled: false,
        };
        handler(app, &mut args);
        args.handled
    }

    /// `UpdateEditableTextBox`: the text box shows `text`, unless it already does but for the
    /// case of ASCII letters. With `select_all`, all of it is selected; otherwise, completing
    /// typed text, what follows the caret is.
    fn update_editable_text_box(
        self: Handle<Self>,
        app: &mut App,
        text: &str,
        select_text: bool,
        select_all: bool,
    ) {
        let controller = self.text(app);
        if search::are_strings_equal(text, controller.text_value(app)) {
            return;
        }
        app.get_mut(self).type_ahead.search = text.to_owned();
        let length = text.encode_utf16().count() as i32;
        let start = controller.selection(app).range().start.max(0);
        let selection = if select_text && select_all {
            TextSelection::new(0, length)
        } else if select_text {
            TextSelection::new(start.min(length), length)
        } else {
            // Setting `Text` puts the caret at the start.
            TextSelection::collapsed(0, TextAffinity::Downstream)
        };
        controller.set_value(app, TextEditingValue::new().text(text).selection(selection));
    }

    /// `TextBox::SelectAll`.
    fn select_all(self: Handle<Self>, app: &mut App) {
        let controller = self.text(app);
        let end = controller.text_value(app).encode_utf16().count() as i32;
        controller.set_selection(app, TextSelection::new(0, end));
    }

    /// The text box's text changed through an edit: a character or Backspace typed for it is
    /// what `OnCharacterReceived` receives once the text box has taken it.
    fn text_edited(self: Handle<Self>, app: &mut App) {
        let typed = app.get_mut(self).pending_typed.take();
        if let Some(typed) = typed
            && self.widget(app).is_text_search_enabled
        {
            self.process_search(app, &typed);
        }
    }

    /// A key reached the text box's focus before the text box: a character or Backspace is
    /// remembered for the edit it makes.
    fn text_box_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            let typed = if event.logical_key() == LogicalKeyboardKey::BACKSPACE {
                Some(Typed::Backspace)
            } else {
                event
                    .character()
                    .map(|text| Typed::Character(text.to_owned()))
            };
            app.get_mut(self).pending_typed = typed;
        }
        KeyEventResult::Ignored
    }

    // MARK: Search

    /// `ProcessSearch`. An editable box matches its text to the items, completing the text with
    /// the item that starts with it, and draws the item selected in an open drop-down; a
    /// non-editable box adds `typed` to its typed search and selects the item it finds.
    fn process_search(self: Handle<Self>, app: &mut App, typed: &Typed) {
        if self.is_editable(app) {
            let text = self.text(app).text_value(app).to_owned();
            if search::are_strings_equal(&text, &app.get(self).type_ahead.search) {
                return;
            }
            let found = if text.is_empty() {
                app.get_mut(self).type_ahead.reset();
                None
            } else {
                self.search_item_source_index(app, typed, false, Match::Prefix)
            };
            app.get_mut(self).search_result = Some(found);
            if self.widget(app).selection_changed_trigger == ComboBoxSelectionChangedTrigger::Always
                && let Some(index) = found
            {
                self.set_selection(app, Selection::Index(index));
            }
            if app.get(self).open {
                self.override_selected_index_for_visual_states(app, found);
                if let Some(index) = found {
                    self.scroll_into_view(app, index);
                }
            }
            self.set_state(app, |_| {});
        } else if let Some(index) = self.search_item_source_index(app, typed, true, Match::Prefix) {
            self.set_selection(app, Selection::Index(index));
        }
    }

    /// `SearchItemSourceIndex`: an editable box searches with its text from the first item; a
    /// non-editable box adds `typed` and searches from the selection. Backspace in an editable
    /// box searches for exact matches only; a prefix match there completes the text.
    fn search_item_source_index(
        self: Handle<Self>,
        app: &mut App,
        typed: &Typed,
        from_current: bool,
        rule: Match,
    ) -> Option<usize> {
        let editable = self.is_editable(app);
        let count = self.item_count(app);
        let created = if editable {
            let text = self.text(app).text_value(app).to_owned();
            app.get_mut(self).type_ahead.search = text;
            false
        } else {
            let character = match typed {
                Typed::Character(character) => character.clone(),
                Typed::Backspace => return None,
            };
            let now = app.get(self).last_key_time();
            app.get_mut(self).type_ahead.append(&character, now)
        };
        let start = if from_current {
            search::search_start(self.selected_index(app), count, created)
        } else {
            0
        };
        let rule = if editable && *typed == Typed::Backspace {
            Match::Exact
        } else {
            rule
        };
        let widget = self.widget(app).clone();
        let texts: Vec<Option<&str>> = widget
            .items
            .iter()
            .map(|item| item.plain_text.as_deref())
            .collect();
        let found = search::find_item(&texts, &app.get(self).type_ahead.search, start, rule);
        if editable
            && rule == Match::Prefix
            && let Some(index) = found
        {
            let text = self.item_text(app, index);
            self.update_editable_text_box(app, &text, true, false);
        }
        found
    }

    /// `IsInSearchingMode`.
    fn is_in_searching_mode(self: Handle<Self>, app: &mut App) -> bool {
        let now = app.get(self).last_key_time();
        self.widget(app).is_text_search_enabled && app.get_mut(self).type_ahead.is_searching(now)
    }

    // MARK: Keyboard

    /// The box's key handler, which every key of the box, its text box and its drop-down reaches:
    /// `OnKeyDownPrivate` and `OnKeyUpPrivate`, and for a key the box leaves unhandled,
    /// `OnCharacterReceived`. Keys the text box takes first in WinUI are left to it.
    fn on_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        app.get_mut(self).key_time = Some(event.time_stamp());
        match event {
            KeyEvent::Down(_) | KeyEvent::Repeat(_) => {
                if self.editable_text_has_focus(app) && is_text_box_key(event.logical_key()) {
                    return KeyEventResult::Ignored;
                }
                if self.key_down(app, event) {
                    app.get_mut(self).pending_typed = None;
                    return KeyEventResult::Handled;
                }
                if let Some(character) = event.character()
                    && character != "\u{1b}"
                    && !self.is_editable(app)
                    && self.widget(app).is_enabled
                    && self.widget(app).is_text_search_enabled
                {
                    self.process_search(app, &Typed::Character(character.to_owned()));
                }
                KeyEventResult::Ignored
            }
            KeyEvent::Up(_) => {
                if app.get(self).ignore_cancel_key_downs
                    && event.logical_key() == LogicalKeyboardKey::ESCAPE
                {
                    app.get_mut(self).ignore_cancel_key_downs = false;
                    return KeyEventResult::Handled;
                }
                KeyEventResult::Ignored
            }
        }
    }

    /// `OnKeyDownPrivate`: whether the box handled the key.
    fn key_down(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> bool {
        let key = event.logical_key();
        if app.get(self).ignore_cancel_key_downs && key == LogicalKeyboardKey::ESCAPE {
            return true;
        }
        if !self.widget(app).is_enabled {
            return false;
        }
        if app.get(self).open {
            self.popup_key_down(app, key)
        } else {
            self.main_key_down(app, key)
        }
    }

    /// `MainKeyDown`, for a closed drop-down: F4, Alt+Up and Alt+Down, Enter and Space open it,
    /// Up, Down, Home and End select. An editable box opens with Up and Down too, commits on
    /// Enter and Tab and reverts on Escape.
    fn main_key_down(self: Handle<Self>, app: &mut App, key: LogicalKeyboardKey) -> bool {
        let modifiers = Modifiers::of(app);
        let editable = self.is_editable(app);
        let mut handled = true;
        let mut new_index = None;
        match key {
            LogicalKeyboardKey::ESCAPE => {
                if editable {
                    self.commit_revert_editable_search(app, true);
                    self.clear_selected_index_override_for_visual_states(app);
                } else {
                    handled = false;
                }
            }
            LogicalKeyboardKey::ENTER | LogicalKeyboardKey::NUMPAD_ENTER => {
                if editable {
                    self.commit_revert_editable_search(app, false);
                } else if !self.is_in_searching_mode(app) {
                    self.put_is_drop_down_open(app, true);
                }
            }
            LogicalKeyboardKey::TAB => {
                if editable {
                    self.commit_revert_editable_search(app, false);
                }
                handled = false;
            }
            LogicalKeyboardKey::SPACE => {
                if editable {
                    handled = false;
                } else if self.is_in_searching_mode(app) {
                    self.process_search(app, &Typed::none());
                } else {
                    self.put_is_drop_down_open(app, true);
                }
            }
            LogicalKeyboardKey::ARROW_DOWN | LogicalKeyboardKey::ARROW_UP
                if editable || modifiers.alt =>
            {
                self.put_is_drop_down_open(app, true);
            }
            LogicalKeyboardKey::ARROW_DOWN
            | LogicalKeyboardKey::ARROW_UP
            | LogicalKeyboardKey::HOME
            | LogicalKeyboardKey::END => {
                let selected = self.selected_index(app);
                new_index = self.handle_navigation_key(app, key, false, selected);
            }
            LogicalKeyboardKey::F4 => {
                if modifiers.none() {
                    self.put_is_drop_down_open(app, true);
                } else {
                    handled = false;
                }
            }
            _ => handled = false,
        }
        if let Some(index) = new_index {
            self.set_selection(app, Selection::Index(index));
        }
        handled
    }

    /// `PopupKeyDown`, for an open drop-down: Escape cancels it, Tab, F4 and Alt+Up or Alt+Down
    /// close it, Enter and Space select the focused item and close it, and Up, Down, Home, End,
    /// Page Up and Page Down move through the items. Space while typing a search is typed.
    fn popup_key_down(self: Handle<Self>, app: &mut App, key: LogicalKeyboardKey) -> bool {
        let modifiers = Modifiers::of(app);
        let editable = self.is_editable(app);
        let text_focus = self.editable_text_has_focus(app);
        let mut handled = false;
        let mut new_focused: Option<usize> = None;
        match key {
            LogicalKeyboardKey::ESCAPE => {
                let state = FocusState::coerce_programmatic(app);
                self.focus_box(app, state);
                app.get_mut(self).closing_due_to_cancel = true;
                self.put_is_drop_down_open(app, false);
                app.get_mut(self).ignore_cancel_key_downs = true;
                handled = true;
            }
            LogicalKeyboardKey::TAB => {
                // Tab then moves on from the box. The text box of an editable box is where it
                // moves on from: its box is no tab stop of its own.
                if !editable {
                    let state = FocusState::coerce_programmatic(app);
                    self.focus_box(app, state);
                    FocusManager::instance(app).apply_focus_changes_if_needed(app);
                }
                self.put_is_drop_down_open(app, false);
            }
            LogicalKeyboardKey::SPACE if editable || self.is_in_searching_mode(app) => {}
            LogicalKeyboardKey::SPACE
            | LogicalKeyboardKey::ENTER
            | LogicalKeyboardKey::NUMPAD_ENTER => {
                if !(modifiers.alt && !modifiers.control) {
                    if editable && text_focus {
                        self.put_is_drop_down_open(app, false);
                        handled = true;
                    } else if let Some(index) = self.focused_item_index(app) {
                        if modifiers.control && self.selected_index(app) == Some(index) {
                            self.set_selection(app, Selection::None);
                        } else {
                            let focused = app.get(self).focused_index;
                            self.set_selection(
                                app,
                                focused.map_or(Selection::None, Selection::Index),
                            );
                            self.put_is_drop_down_open(app, false);
                        }
                        handled = true;
                    }
                }
            }
            LogicalKeyboardKey::ARROW_UP | LogicalKeyboardKey::ARROW_DOWN
                if editable && text_focus =>
            {
                let current = match app.get(self).search_result {
                    Some(result) => result,
                    None => self.selected_index(app),
                };
                new_focused = Some(match (current, key) {
                    (Some(index), LogicalKeyboardKey::ARROW_DOWN) => index + 1,
                    (None, LogicalKeyboardKey::ARROW_DOWN) => 0,
                    (Some(index), _) => index.saturating_sub(1),
                    (None, _) => 0,
                });
            }
            LogicalKeyboardKey::ARROW_UP | LogicalKeyboardKey::ARROW_DOWN if modifiers.alt => {
                self.put_is_drop_down_open(app, false);
                handled = true;
            }
            LogicalKeyboardKey::ARROW_UP
            | LogicalKeyboardKey::ARROW_DOWN
            | LogicalKeyboardKey::HOME
            | LogicalKeyboardKey::END
            | LogicalKeyboardKey::PAGE_UP
            | LogicalKeyboardKey::PAGE_DOWN => {
                let focused = app.get(self).focused_index;
                new_focused = self.handle_navigation_key(app, key, true, focused);
                handled = true;
            }
            LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_RIGHT => handled = true,
            LogicalKeyboardKey::F4 if modifiers.none() => {
                self.put_is_drop_down_open(app, false);
                handled = true;
            }
            _ => {}
        }
        if let Some(new) = new_focused {
            handled = true;
            let count = self.item_count(app);
            if count > 0 {
                let new = new.min(count - 1);
                if editable && text_focus {
                    app.get_mut(self).search_result = Some(Some(new));
                    let text = self.item_text(app, new);
                    self.update_editable_text_box(app, &text, true, true);
                    self.scroll_into_view(app, new);
                } else {
                    self.set_focused_item(app, Some(new), true, false, FocusState::Keyboard);
                }
                if self.widget(app).selection_changed_trigger
                    == ComboBoxSelectionChangedTrigger::Always
                {
                    self.set_selection(app, Selection::Index(new));
                } else {
                    self.override_selected_index_for_visual_states(app, Some(new));
                }
            }
        }
        handled
    }

    /// `HandleNavigationKey` for the vertical list: Up and Down move to the previous and next
    /// enabled item, scrolling a line instead where there is none and `scroll_viewport` is set;
    /// Home and End go to the first and last item; Page Up and Page Down by a page.
    fn handle_navigation_key(
        self: Handle<Self>,
        app: &mut App,
        key: LogicalKeyboardKey,
        scroll_viewport: bool,
        index: Option<usize>,
    ) -> Option<usize> {
        let count = self.item_count(app);
        let focused = app.get(self).focused_index;
        let new = match key {
            LogicalKeyboardKey::ARROW_UP => {
                let new = self.select_prev(app, index);
                if focused == new && scroll_viewport {
                    self.scroll_by(app, -LINE_DELTA);
                }
                new
            }
            LogicalKeyboardKey::ARROW_DOWN => {
                let new = self.select_next(app, index);
                if focused == new && scroll_viewport {
                    self.scroll_by(app, LINE_DELTA);
                }
                new
            }
            LogicalKeyboardKey::HOME => Some(0),
            LogicalKeyboardKey::END => count.checked_sub(1),
            LogicalKeyboardKey::PAGE_UP => self.navigate_by_page(app, false),
            LogicalKeyboardKey::PAGE_DOWN => self.navigate_by_page(app, true),
            _ => index,
        };
        new.filter(|new| *new < count)
    }

    /// `SelectNext`: the next enabled item after `index`, or `index`.
    fn select_next(self: Handle<Self>, app: &App, index: Option<usize>) -> Option<usize> {
        let start = index.map_or(0, |index| index + 1);
        let items = &self.widget(app).items;
        (start..items.len())
            .find(|index| items[*index].is_enabled)
            .or(index)
    }

    /// `SelectPrev`: the previous enabled item before `index`, or `index`.
    fn select_prev(self: Handle<Self>, app: &App, index: Option<usize>) -> Option<usize> {
        let items = &self.widget(app).items;
        let start = index?;
        (0..start)
            .rev()
            .find(|index| items[*index].is_enabled)
            .or(Some(start))
    }

    /// `NavigateByPage`: the last item fully on the page in the direction of travel, scrolling a
    /// page when the focused item already is, and advancing at least one item.
    fn navigate_by_page(self: Handle<Self>, app: &mut App, forward: bool) -> Option<usize> {
        let focused = app.get(self).focused_index;
        let Some(focused) = focused else {
            return self.first_item_on_current_page(app, None, forward);
        };
        if !self.is_on_current_page(app, focused, true) {
            self.scroll_into_view(app, focused);
        }
        let mut new = self.first_item_on_current_page(app, Some(focused), forward);
        if new == Some(focused) {
            let viewport = self
                .scroll_metrics(app)
                .map_or(0.0, |metrics| metrics.viewport_length);
            self.scroll_by(app, if forward { viewport } else { -viewport });
            new = self.first_item_on_current_page(app, Some(focused), forward);
            if new == Some(focused) {
                let count = self.item_count(app);
                if forward && count > 0 && focused < count - 1 {
                    new = Some(focused + 1);
                } else if !forward && focused > 0 {
                    new = Some(focused - 1);
                }
            }
        }
        new
    }

    /// `GetFirstItemOnCurrentPage`: from `start`, the first item at least partly on the page in
    /// the direction of travel, then the last one after it that is fully on it.
    fn first_item_on_current_page(
        self: Handle<Self>,
        app: &mut App,
        start: Option<usize>,
        forward: bool,
    ) -> Option<usize> {
        let count = self.item_count(app) as i64;
        let delta = if forward { 1 } else { -1 };
        let mut result = start.map_or(-1, |start| start as i64);
        let mut probe = result;
        let in_range = |probe: i64| 0 <= probe && probe < count;
        let mut on_page = false;
        while in_range(probe) {
            on_page = self.is_on_current_page(app, probe as usize, true);
            if on_page {
                break;
            }
            result = probe;
            probe += delta;
        }
        if on_page {
            result = probe;
            probe += delta;
            on_page = in_range(probe) && self.is_on_current_page(app, probe as usize, false);
            if on_page {
                result = probe;
                probe += delta;
                while in_range(probe) && self.is_on_current_page(app, probe as usize, false) {
                    result = probe;
                    probe += delta;
                }
            }
        }
        (result >= 0).then_some(result as usize)
    }

    /// `IsOnCurrentPage`: whether the item is on the drop-down's visible page, at least partly
    /// with `partial`, else entirely.
    fn is_on_current_page(self: Handle<Self>, app: &mut App, index: usize, partial: bool) -> bool {
        let Some(metrics) = self.scroll_metrics(app) else {
            return false;
        };
        let r = ThemeResources::of(app, self.context(app));
        let heights = self.item_heights(app, self.context(app));
        let top = heights[..index].iter().sum::<f64>() - metrics.offset;
        let bottom = top + heights[index];
        // The scroll host is the viewport with the items' fixed margin around it.
        let margin = r.combo_box_dropdown_content_margin;
        let (host_top, host_bottom) = (-margin[1], metrics.viewport_length + margin[3]);
        if partial {
            top < host_bottom && host_top < bottom
        } else {
            host_top <= top && bottom <= host_bottom
        }
    }

    /// `ScrollIntoView` with the default alignment: the least scrolling that shows the item.
    fn scroll_into_view(self: Handle<Self>, app: &mut App, index: usize) {
        let Some(metrics) = self.scroll_metrics(app) else {
            // The list has not been laid out yet.
            let epoch = app.get(self).open_epoch;
            self.after_frame(app, move |app| {
                if app.get(self).open
                    && app.get(self).open_epoch == epoch
                    && self.scroll_metrics(app).is_some()
                    && index < self.item_count(app)
                {
                    self.scroll_into_view(app, index);
                }
            });
            return;
        };
        let heights = self.item_heights(app, self.context(app));
        let top: f64 = heights[..index].iter().sum();
        let bottom = top + heights[index];
        let target = if top < metrics.offset {
            top
        } else if bottom > metrics.offset + metrics.viewport_length {
            bottom - metrics.viewport_length
        } else {
            return;
        };
        let scroll = app.get(self).scroll.unwrap();
        scroll.change_view(app, target, true);
    }

    /// Scrolls the drop-down by `delta` at once.
    fn scroll_by(self: Handle<Self>, app: &mut App, delta: f64) {
        if let Some(scroll) = app.get(self).scroll {
            scroll.scroll_by(app, delta, true);
        }
    }

    /// The drop-down list's scroll metrics, once it is laid out.
    fn scroll_metrics(self: Handle<Self>, app: &App) -> Option<ScrollViewportMetrics> {
        let scroll = app.get(self).scroll?;
        let metrics = scroll.metrics(app);
        (metrics.viewport_length > 0.0).then_some(metrics)
    }

    // MARK: Pointer

    /// `OnPointerPressed` on the box: the left button presses it.
    fn pointer_pressed(self: Handle<Self>, app: &mut App) {
        if self.widget(app).is_enabled {
            self.set_state(app, |state| state.pressed = true);
        }
    }

    /// `OnPointerReleased` and `PerformPointerUpAction`: the box takes the focus, then opens, or
    /// in an editable box, opens or closes when the press was on `DropDownOverlay`.
    fn pointer_released(self: Handle<Self>, app: &mut App, on_overlay: bool) {
        if !std::mem::take(&mut app.get_mut(self).pressed) {
            return;
        }
        self.set_state(app, |_| {});
        self.focus_box(app, FocusState::Pointer);
        if !self.is_editable(app) {
            self.put_is_drop_down_open(app, true);
        } else if on_overlay {
            let open = app.get(self).open;
            self.put_is_drop_down_open(app, !open);
        }
    }

    /// `OnPointerWheelChanged`: while the box has the focus it takes the wheel, and while closed
    /// selects the next or previous enabled item.
    fn pointer_signal(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        if !matches!(event, PointerEvent::Scroll(_))
            || !self.widget(app).is_enabled
            || !self.has_focus(app)
        {
            return;
        }
        let resolver = GestureBinding::instance(app).pointer_signal_resolver(app);
        resolver.register(
            app,
            event,
            Rc::new(move |app, event| self.wheel(app, event)),
        );
    }

    /// One wheel event the box took. `MouseWheelDelta` is positive for a wheel turned away from
    /// the user, where Inset's vertical scroll delta is negative, and for a wheel tilted right.
    fn wheel(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        let PointerEvent::Scroll(scroll) = event else {
            return;
        };
        if !self.alive(app) || app.get(self).open {
            return;
        }
        let delta = if scroll.scroll_delta.dy() != 0.0 {
            -scroll.scroll_delta.dy()
        } else {
            scroll.scroll_delta.dx()
        };
        let selected = self.selected_index(app);
        let new = if delta < 0.0 {
            self.select_next(app, selected)
        } else {
            self.select_prev(app, selected)
        };
        if let Some(new) = new {
            self.set_selection(app, Selection::Index(new));
        }
    }

    /// A click on the item at `index`: `OnSelectorItemClicked` focuses it and selects it, or with
    /// Ctrl toggles its selection, then the drop-down closes.
    fn item_clicked(self: Handle<Self>, app: &mut App, index: usize) {
        self.focus_item(app, index, FocusState::Pointer);
        let control = HardwareKeyboard::instance(app).is_control_pressed(app);
        let is_selected = self.selected_index(app) == Some(index);
        let select = if control { !is_selected } else { true };
        self.set_selection(
            app,
            if select {
                Selection::Index(index)
            } else {
                Selection::None
            },
        );
        self.put_is_drop_down_open(app, false);
    }

    /// An item reported its size in the drop-down.
    fn item_measured(self: Handle<Self>, app: &mut App, id: &str, size: Size) {
        if !self.alive(app) {
            return;
        }
        let state = app.get(self);
        if state.heights.get(id) == Some(&size.height())
            && state.widths.get(id) == Some(&size.width())
        {
            return;
        }
        let id = id.to_owned();
        self.set_state(app, |state| {
            state.heights.insert(id.clone(), size.height());
            state.widths.insert(id, size.width());
        });
    }

    // MARK: Popup placement

    /// `Background`'s bounds in the root overlay, and the overlay's size: what `ArrangePopup`
    /// reads at the opening.
    fn popup_geometry(self: Handle<Self>, app: &mut App) -> Option<(Rect, Size)> {
        let target = app
            .get(self)
            .background_key
            .clone()
            .current_context(app)?
            .find_render_object(app)?
            .as_box()?;
        let overlay = Overlay::of(app, self.context(app), true);
        let root = overlay.context(app).find_render_object(app)?.as_box()?;
        let transform = target
            .as_object()
            .get_transform_to(app, Some(root.as_object()));
        Some((
            transform_rect(&transform, Offset::ZERO & target.size(app)),
            root.size(app),
        ))
    }

    /// Each item's height in the drop-down: as last laid out, or for an item not laid out yet,
    /// what an item of one line of text takes.
    fn item_heights(self: Handle<Self>, app: &mut App, context: BuildContext) -> Vec<f64> {
        let estimate = text_item_size(app, context, "").height();
        let widget = self.widget(app).clone();
        let heights = &app.get(self).heights;
        widget
            .items
            .iter()
            .map(|item| heights.get(&item.id).copied().unwrap_or(estimate))
            .collect()
    }

    /// The width the drop-down's content asks for: the widest item laid out in this opening, and
    /// the text of the items it is about to show that are not laid out yet.
    fn content_width(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        first_item: usize,
        window: Size,
    ) -> f64 {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let shown = first_item..first_item + r.combo_box_popup_max_number_of_items as usize;
        let mut widest: f64 = 0.0;
        for (index, item) in widget.items.iter().enumerate() {
            let measured = app.get(self).widths.get(&item.id).copied();
            if let Some(width) = measured {
                widest = widest.max(width);
            } else if shown.contains(&index)
                && let Some(text) = &item.plain_text
            {
                widest = widest.max(text_item_size(app, context, text).width());
            }
        }
        let border =
            r.combo_box_dropdown_border_thickness[0] + r.combo_box_dropdown_border_thickness[2];
        (widest + border).max(popup::popup_min_width(&r, window))
    }

    /// `ArrangePopup` with the box at `target` in a window of `window`.
    fn arrangement(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        target: Rect,
        window: Size,
    ) -> Option<Arrangement> {
        let heights = self.item_heights(app, context);
        let fallback_height = text_item_size(app, context, "").height();
        let widget = self.widget(app).clone();
        let editable = widget.is_editable.then(|| {
            if app.get(self).text_box_enabled {
                target.height()
            } else {
                0.0
            }
        });
        let rtl =
            Directionality::maybe_of(app, context) == Some(inset_embedder::TextDirection::Rtl);
        let input = ArrangeInput {
            target,
            window,
            heights: &heights,
            fallback_height,
            selected: self.selected_index(app),
            max_drop_down_height: widget.max_drop_down_height,
            editable,
            content_width: 0.0,
            last_height: app.get(self).last_popup_height.get(),
            rtl,
        };
        let r = ThemeResources::of(app, context);
        let first_item = popup::arrange(&r, &input)?.first_item;
        let content_width = self.content_width(app, context, first_item, window);
        popup::arrange(
            &r,
            &ArrangeInput {
                content_width,
                ..input
            },
        )
    }

    // MARK: Template

    /// The box's `CommonStates`, `FocusStates` and, editable, `EditableModeStates`, as
    /// `ChangeVisualState` chooses them.
    fn visual_states(
        self: Handle<Self>,
        app: &mut App,
    ) -> (CommonState, ComboBoxFocusState, Option<EditableModeState>) {
        let shows_focus_visual = FocusState::current(app).shows_focus_visual();
        let widget = self.widget(app);
        let state = app.get(self);
        let editable = widget.is_editable.then(|| {
            let text_focus = self.editable_text_has_focus(app);
            match (state.pointer_over_overlay, state.pressed, text_focus) {
                (true, true, true) => EditableModeState::TextBoxFocusedOverlayPressed,
                (true, true, false) => EditableModeState::TextBoxOverlayPressed,
                (true, false, true) => EditableModeState::TextBoxFocusedOverlayPointerOver,
                (true, false, false) => EditableModeState::TextBoxOverlayPointerOver,
                (false, _, true) => EditableModeState::TextBoxFocused,
                (false, _, false) => EditableModeState::TextBoxUnfocused,
            }
        });
        let ignore_pressed = widget.is_editable && state.pointer_over_overlay && state.pressed;
        let common = if !widget.is_enabled {
            CommonState::Disabled
        } else if state.pressed && !ignore_pressed {
            CommonState::Pressed
        } else if state.pointer_over_main || state.pointer_over_popup {
            CommonState::PointerOver
        } else {
            CommonState::Normal
        };
        let box_focused = self.box_focus(app).has_primary_focus(app);
        let focus = if !state.selection_active || !widget.is_enabled {
            ComboBoxFocusState::Unfocused
        } else if state.open {
            ComboBoxFocusState::FocusedDropDown
        } else if !box_focused {
            ComboBoxFocusState::Unfocused
        } else if !shows_focus_visual {
            ComboBoxFocusState::PointerFocused
        } else if state.pressed {
            ComboBoxFocusState::FocusedPressed
        } else {
            ComboBoxFocusState::Focused
        };
        (common, focus, editable)
    }
}

impl ComboBoxState {
    /// When the last key event arrived.
    fn last_key_time(&self) -> Duration {
        self.key_time.unwrap_or_default()
    }
}

impl ComboBoxState {
    /// Keeps the focus of the items in `items`, and checks what the application passes: ids are
    /// unique, and `selected_index` is an index of `items`.
    fn sync_items(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let ids: HashSet<&str> = widget.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            ids.len(),
            widget.items.len(),
            "ComboBox item ids must be unique"
        );
        assert!(
            widget
                .selected_index
                .is_none_or(|index| index < widget.items.len()),
            "ComboBox selected_index must be an index of items"
        );
        let removed: Vec<String> = app
            .get(self)
            .item_focus
            .keys()
            .filter(|id| !ids.contains(id.as_str()))
            .cloned()
            .collect();
        for id in removed {
            let state = app.get_mut(self);
            let node = state.item_focus.remove(&id).unwrap();
            state.heights.remove(&id);
            state.widths.remove(&id);
            // The item's widget may still hold the node until the frame ends.
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

    /// The application has answered the selection the box asked for, by building the box again:
    /// a selection it did not take is undone, which the text box of an editable box shows once
    /// the frame is over.
    fn settle_request(self: Handle<Self>, app: &mut App) {
        let Some(requested) = app.get_mut(self).requested.take() else {
            return;
        };
        let selection = app.get(self).selection.clone();
        if requested != selection {
            self.after_frame(app, move |app| {
                self.selection_did_change(app, &requested, &selection);
                self.set_state(app, |_| {});
            });
        }
    }

    /// An application that owns `is_drop_down_open` and passes another state than the box has,
    /// or did not take the one the box asked for, has the box open or close once the frame is
    /// over.
    fn settle_drop_down_open(self: Handle<Self>, app: &mut App) {
        let Some(open) = self.widget(app).is_drop_down_open else {
            return;
        };
        if open == app.get(self).open || app.get(self).pending_open == Some(open) {
            return;
        }
        app.get_mut(self).pending_open = Some(open);
        self.after_frame(app, move |app| {
            app.get_mut(self).pending_open = None;
            if self.widget(app).is_drop_down_open == Some(open) {
                self.set_drop_down_open(app, open);
                self.set_state(app, |_| {});
            }
        });
    }
}

impl State for ComboBoxState {
    type Widget = ComboBox;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        // Programmatic focus, which opening gives the selected item, shows the focus visual only
        // after keyboard input.
        InputDevice::initialize(app);
        let scope_focus = FocusNode::new(app);
        let focus = FocusNode::new(app);
        focus.add_listener(app, Listener::handle_method(self, Self::box_focus_changed));
        let text = TextEditingController::new(app);
        let text_focus = FocusNode::new(app);
        text_focus.as_node().set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.text_box_key(app, event))),
        );
        text_focus.add_listener(app, Listener::handle_method(self, Self::text_focus_changed));
        let popup = OverlayPortalController::new(app, Some("ComboBox Popup".into()));
        let state = app.get_mut(self);
        state.scope_focus = Some(scope_focus);
        state.focus = Some(focus);
        state.text = Some(text);
        state.text_focus = Some(text_focus);
        state.popup = Some(popup);
        self.sync_items(app);
        if self.is_editable(app) {
            self.setup_editable_mode(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &ComboBox) {
        app.get_mut(self).building = true;
        self.sync_items(app);
        let widget = self.widget(app).clone();
        let passed = Selection::of(&widget);
        let current = self.selected(app);
        let state = app.get_mut(self);
        state.selection = passed.clone();
        state.requested = None;
        if current != passed {
            self.selection_did_change(app, &current, &passed);
        }
        // `OnPropertyChanged2` for `IsEditable`.
        if old.is_editable != widget.is_editable {
            if widget.is_editable {
                self.setup_editable_mode(app);
            } else {
                self.disable_editable_mode(app);
            }
        }
        // `OnIsEnabledChanged`'s `ClearStateFlags`.
        if old.is_enabled && !widget.is_enabled {
            let state = app.get_mut(self);
            state.pointer_over_main = false;
            state.pointer_over_popup = false;
            state.pointer_over_overlay = false;
            state.pressed = false;
            self.after_frame(app, move |app| self.put_is_drop_down_open(app, false));
        }
        app.get_mut(self).building = false;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        // `OnUnloaded` closes the drop-down.
        if app.get(self).open
            && let Some(closed) = self.widget(app).drop_down_closed.clone()
        {
            after_next_frame(
                app,
                None,
                FrameCallback::new(move |app, _| closed.call(app)),
            );
        }
        let state = app.get_mut(self);
        let scope_focus = state.scope_focus.take().expect("an initialized ComboBox");
        let focus = state.focus.take().expect("an initialized ComboBox");
        let text = state.text.take().expect("an initialized ComboBox");
        let text_focus = state.text_focus.take().expect("an initialized ComboBox");
        let popup = state.popup.take().expect("an initialized ComboBox");
        let scroll = state.scroll.take();
        let items: Vec<_> = state.item_focus.drain().map(|(_, node)| node).collect();
        focus.remove_listener(app, &Listener::handle_method(self, Self::box_focus_changed));
        text_focus.remove_listener(
            app,
            &Listener::handle_method(self, Self::text_focus_changed),
        );
        for node in [scope_focus, focus, text_focus].into_iter().chain(items) {
            node.dispose(app);
            app.destroy(node);
        }
        app.get_mut(text).dispose();
        app.destroy(text);
        app.destroy(popup);
        if let Some(scroll) = scroll {
            scroll.dispose(app);
            app.destroy(scroll);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        self.settle_request(app);
        self.settle_drop_down_open(app);
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let (common, _, _) = self.visual_states(app);
        let mut cells = Vec::new();
        if let Some(header) = widget.header.clone() {
            // `HeaderContentPresenter`.
            let foreground = if common == CommonState::Disabled {
                r.combo_box_header_foreground_disabled
            } else {
                r.combo_box_header_foreground
            };
            cells.push(
                GridCell::new(Margin::new(
                    r.combo_box_top_header_margin,
                    DefaultTextStyle::new(
                        control_text_style(
                            &resources,
                            r.control_content_theme_font_size,
                            r.combo_box_header_theme_font_weight,
                            foreground,
                        )
                        .height(HEADER_LINE_HEIGHT / r.control_content_theme_font_size),
                        header,
                    ),
                ))
                .into_widget(),
            );
        }
        cells.push(
            GridCell::new(self.target_area(app, context))
                .row(1)
                .into_widget(),
        );
        // `LayoutRoot`.
        let layout_root = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::STAR),
            ])
            .children(cells);
        let root = Focus::new(layout_root)
            .focus_node(self.scope_focus(app).as_node())
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.on_key(app, event)))
            .on_focus_change(move |app, has_focus| self.scope_focus_changed(app, has_focus));
        // An editable box is one text field to the platform: pressing it or its drop-down leaves
        // the text box focused.
        TextFieldTapRegion::new(root)
            .enabled(widget.is_editable)
            .into_widget()
    }
}

impl ComboBoxState {
    /// The part of the template that takes pointer input, which the source's `IsChildOfTarget`
    /// tells from the header: rows 1 and 2, with the box's own focus.
    fn target_area(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let (common, focus, editable_state) = self.visual_states(app);
        let text_box_enabled = widget.is_editable && app.get(self).text_box_enabled;
        let focused = matches!(
            focus,
            ComboBoxFocusState::Focused | ComboBoxFocusState::FocusedPressed
        );
        let mut cells = Vec::new();
        // `HighlightBackground`, behind `Background`, 4 pixels around it.
        cells.push(
            GridCell::new(
                Opacity::new(if focused { 1.0 } else { 0.0 }).child(Margin::new(
                    HIGHLIGHT_BACKGROUND_MARGIN,
                    ControlBorder::new(
                        Brush::Solid(r.combo_box_background_focused),
                        Brush::Solid(r.combo_box_background_border_brush_focused),
                    )
                    .border_thickness_ltrb(r.combo_box_background_border_thickness_focused)
                    .corner_radius_corners(r.combo_box_higlight_border_corner_radius),
                )),
            )
            .column_span(2)
            .into_widget(),
        );
        // `Background`, where `Popup` opens from.
        let (background, border) = match common {
            CommonState::Normal => (
                Brush::Solid(r.combo_box_background),
                Brush::ControlElevation(r.combo_box_border_brush),
            ),
            CommonState::PointerOver => (
                Brush::Solid(r.combo_box_background_pointer_over),
                Brush::ControlElevation(r.combo_box_border_brush_pointer_over),
            ),
            CommonState::Pressed => (
                Brush::Solid(r.combo_box_background_pressed),
                Brush::Solid(r.combo_box_border_brush_pressed),
            ),
            CommonState::Disabled => (
                Brush::Solid(r.combo_box_background_disabled),
                Brush::Solid(r.combo_box_border_brush_disabled),
            ),
        };
        let background = KeyedSubtree::new(
            ConstrainedBox::new(BoxConstraints::new().min_width(r.combo_box_theme_min_width))
                .child(
                    ControlBorder::new(background, border)
                        .border_thickness_ltrb(r.combo_box_border_theme_thickness)
                        .corner_radius_corners(r.control_corner_radius),
                ),
        )
        .key(Rc::new(app.get(self).background_key.clone()));
        let popup = OverlayPortal::overlay_child_layout_builder(
            self.popup(app),
            move |app, context, info| self.popup_overlay(app, context, info),
        )
        .overlay_location(OverlayChildLocation::RootOverlay)
        .child(background);
        cells.push(GridCell::new(popup).column_span(2).into_widget());
        // `Pill`.
        cells.push(
            GridCell::new(Margin::new(
                PILL_MARGIN,
                Align::new()
                    .alignment(Alignment::CENTER_LEFT.into())
                    .width_factor(1.0)
                    .height_factor(1.0)
                    .child(
                        Opacity::new(if focused { 1.0 } else { 0.0 }).child(
                            SizedBox::new()
                                .width(r.combo_box_item_pill_width)
                                .height(r.combo_box_item_pill_height)
                                .child(
                                    ControlBorder::new(
                                        Brush::Solid(r.combo_box_item_pill_fill_brush),
                                        Brush::Solid(Color::new(0)),
                                    )
                                    .border_thickness(0.0)
                                    .corner_radius_corners(r.combo_box_item_pill_corner_radius),
                                ),
                        ),
                    ),
            ))
            .into_widget(),
        );
        // `ContentPresenter`, collapsed while the text box shows. Its cell stays, so that the
        // cells after it keep their elements and the text box its input connection.
        let content = if text_box_enabled {
            SizedBox::shrink().into_widget()
        } else {
            self.content_presenter(app, context, common, focus)
        };
        cells.push(GridCell::new(content).into_widget());
        if widget.is_editable {
            // `EditableText`, which has no size while the box shows `ContentPresenter`.
            cells.push(
                GridCell::new(
                    Offstage::new()
                        .offstage(!text_box_enabled)
                        .child(self.editable_text(app, context)),
                )
                .column_span(2)
                .into_widget(),
            );
            cells.push(
                GridCell::new(self.drop_down_overlay(
                    app,
                    context,
                    editable_state,
                    text_box_enabled,
                ))
                .column(1)
                .into_widget(),
            );
        }
        // `DropDownGlyph`, which takes no pointer input.
        let glyph = if common == CommonState::Disabled {
            r.combo_box_drop_down_glyph_foreground_disabled
        } else {
            match (focus, editable_state) {
                (ComboBoxFocusState::Focused, _) => r.combo_box_drop_down_glyph_foreground_focused,
                (ComboBoxFocusState::FocusedPressed, _) => {
                    r.combo_box_drop_down_glyph_foreground_focused_pressed
                }
                (
                    _,
                    Some(
                        EditableModeState::TextBoxFocused
                        | EditableModeState::TextBoxFocusedOverlayPointerOver
                        | EditableModeState::TextBoxFocusedOverlayPressed,
                    ),
                ) => r.combo_box_editable_drop_down_glyph_foreground,
                _ => r.combo_box_drop_down_glyph_foreground,
            }
        };
        cells.push(
            GridCell::new(
                IgnorePointer::new().child(
                    Align::new()
                        .alignment(Alignment::CENTER_RIGHT.into())
                        .width_factor(1.0)
                        .height_factor(1.0)
                        .child(Margin::new(
                            GLYPH_MARGIN,
                            SizedBox::new()
                                .width(GLYPH_SIZE)
                                .height(r.combo_box_min_height)
                                .child(
                                    Center::new().child(
                                        FontIcon::symbol(FluentSymbol::ChevronDown)
                                            .font_size(GLYPH_SIZE)
                                            .foreground(glyph),
                                    ),
                                ),
                        )),
                ),
            )
            .column(1)
            .into_widget(),
        );
        let row_grid = Grid::new()
            .column_definitions([
                ColumnDefinition::new(GridLength::STAR),
                ColumnDefinition::new(GridLength::pixel(GLYPH_COLUMN_WIDTH)),
            ])
            .children(cells);
        // Pressing the box opens it; with an editable box's text box shown, only presses on
        // `DropDownOverlay` reach the box, the rest being the text box's.
        let row = self.pointer_target(app, row_grid.into_widget(), !text_box_enabled);
        let mut rows = vec![GridCell::new(row).into_widget()];
        if let Some(description) = widget.description.clone() {
            // `DescriptionPresenter`.
            rows.push(
                GridCell::new(
                    self.pointer_target(
                        app,
                        DefaultTextStyle::new(
                            control_text_style(
                                &resources,
                                r.control_content_theme_font_size,
                                FontWeight::NORMAL,
                                r.system_control_description_text_foreground_brush,
                            ),
                            description,
                        )
                        .into_widget(),
                        true,
                    ),
                )
                .row(1)
                .into_widget(),
            );
        }
        let target = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::STAR),
                RowDefinition::new(GridLength::AUTO),
            ])
            .children(rows);
        let target = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                // `OnPointerEntered`.
                self.set_state(app, |state| {
                    state.pointer_over_main = true;
                    state.pressed = false;
                })
            }))
            .on_exit(Rc::new(move |app, _| {
                // `OnPointerExited`.
                self.set_state(app, |state| {
                    state.pointer_over_main = false;
                    state.pressed = false;
                })
            }))
            .child(
                PointerListener::new()
                    .on_pointer_signal(Rc::new(move |app, event| self.pointer_signal(app, &event)))
                    .child(target),
            );
        Focus::new(target)
            .focus_node(self.box_focus(app).as_node())
            .can_request_focus(widget.is_enabled)
            // An editable box is reached by Tab in its text box.
            .skip_traversal(widget.is_editable || !widget.is_enabled)
            .into_widget()
    }

    /// Takes the box's presses while `takes_presses` and the box is enabled: `OnPointerPressed`
    /// and `OnPointerReleased`. The widget stays the same either way, so that what it holds keeps
    /// its state.
    fn pointer_target(
        self: Handle<Self>,
        app: &App,
        child: WidgetRef,
        takes_presses: bool,
    ) -> WidgetRef {
        let gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .child(child);
        if !takes_presses || !self.widget(app).is_enabled {
            return gesture.into_widget();
        }
        gesture
            .on_tap_down(Rc::new(move |app, _| self.pointer_pressed(app)))
            .on_tap_up(Rc::new(
                move |app, details: inset_gestures::TapUpDetails| {
                    let on_overlay = self.is_on_overlay(app, details.global_position);
                    self.pointer_released(app, on_overlay);
                },
            ))
            .on_tap_cancel(Listener::new(move |app| {
                self.set_state(app, |state| state.pressed = false)
            }))
            .into_widget()
    }

    /// Whether `position` is on `DropDownOverlay`: `IsDropDownOverlay`.
    fn is_on_overlay(self: Handle<Self>, app: &mut App, position: Offset) -> bool {
        let Some(render) = app
            .get(self)
            .overlay_key
            .clone()
            .current_context(app)
            .and_then(|context| context.find_render_object(app))
            .and_then(|object| object.as_box())
        else {
            return false;
        };
        let origin = render.local_to_global(app, Offset::ZERO, None);
        let bounds = origin & render.size(app);
        bounds.contains(position)
    }

    /// `ContentPresenter`: the selected item's content at `Padding`, or for an editable box's
    /// custom value its text, or `PlaceholderTextBlock`, hidden while the drop-down is open or an
    /// item is selected.
    fn content_presenter(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        common: CommonState,
        focus: ComboBoxFocusState,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let r = ThemeResources::of(app, context);
        let foreground = match (focus, common) {
            (ComboBoxFocusState::Focused, _) => r.combo_box_foreground_focused,
            (ComboBoxFocusState::FocusedPressed, _) => r.combo_box_foreground_focused_pressed,
            (_, CommonState::Normal) => r.combo_box_foreground,
            (_, CommonState::PointerOver) => r.combo_box_foreground_pointer_over,
            (_, CommonState::Pressed) => r.combo_box_foreground_pressed,
            (_, CommonState::Disabled) => r.combo_box_foreground_disabled,
        };
        let placeholder = widget
            .placeholder_foreground
            .unwrap_or(match (focus, common) {
                (ComboBoxFocusState::Focused, _) => r.combo_box_place_holder_foreground_focused,
                (ComboBoxFocusState::FocusedPressed, _) => {
                    r.combo_box_place_holder_foreground_focused_pressed
                }
                (_, CommonState::Normal) => r.combo_box_place_holder_foreground,
                (_, CommonState::PointerOver) => r.combo_box_place_holder_foreground_pointer_over,
                (_, CommonState::Pressed) => r.combo_box_place_holder_foreground_pressed,
                (_, CommonState::Disabled) => r.combo_box_place_holder_foreground_disabled,
            });
        let open = app.get(self).open;
        let content = match self.selected(app) {
            Selection::Index(index) if !open => widget.items[index].content.clone(),
            Selection::Custom(text) if !open && widget.is_editable => {
                Text::new(text).soft_wrap(false).into_widget()
            }
            selected => {
                // `PlaceholderTextBlock`, the content `SetContentPresenter(-1)` puts back.
                let visible = !open && selected.index().is_none();
                Opacity::new(if visible { 1.0 } else { 0.0 })
                    .child(
                        Text::new(widget.placeholder_text.clone())
                            .style(control_text_style(
                                &r,
                                r.control_content_theme_font_size,
                                FontWeight::NORMAL,
                                placeholder,
                            ))
                            .soft_wrap(false),
                    )
                    .into_widget()
            }
        };
        let content = DefaultTextStyle::new(
            control_text_style(
                &r,
                r.control_content_theme_font_size,
                FontWeight::NORMAL,
                foreground,
            ),
            content,
        );
        // `HorizontalContentAlignment` is Stretch, and `VerticalContentAlignment` Center,
        // `Control`'s default: the content takes the row's width and its own height, centred.
        let aligned = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::STAR),
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::STAR),
            ])
            .children([GridCell::new(content).row(1).into_widget()]);
        Margin::new(r.combo_box_padding, aligned).into_widget()
    }

    /// `EditableText`: a TextBox in `ComboBoxTextBoxStyle` at `ComboBoxEditableTextPadding`, its
    /// border transparent until hovered or focused, with the corners next to an open drop-down
    /// square.
    fn editable_text(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let r = ThemeResources::of(app, context);
        let mut config = TextBox::new(self.text(app).as_controller())
            .focus_node(self.text_focus(app))
            .placeholder_text(widget.placeholder_text.clone())
            .is_enabled(widget.is_enabled)
            .padding(r.combo_box_editable_text_padding)
            .border_brush(Brush::Solid(Color::new(0)))
            .foreground(
                widget
                    .placeholder_foreground
                    .unwrap_or(r.combo_box_place_holder_foreground),
            )
            .text_changed(move |app, _| self.text_edited(app));
        if let Some(color) = widget.placeholder_foreground {
            config = config.placeholder_foreground(color);
        }
        let radius = r.control_corner_radius;
        let corner_radius = match (app.get(self).open, app.get(self).opened_up) {
            (false, _) => radius,
            (true, false) => [radius[0], radius[1], 0.0, 0.0],
            (true, true) => [0.0, 0.0, radius[2], radius[3]],
        };
        TextControl::new(config, None)
            .combo_box_style(ComboBoxTextBoxStyle { corner_radius })
            .into_widget()
    }

    /// `DropDownOverlay`: the area over the glyph of an editable box, which opens and closes the
    /// drop-down and shows the pointer over it.
    fn drop_down_overlay(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        state: Option<EditableModeState>,
        text_box_enabled: bool,
    ) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let background = match state {
            Some(
                EditableModeState::TextBoxFocusedOverlayPointerOver
                | EditableModeState::TextBoxOverlayPointerOver,
            ) => r.combo_box_drop_down_background_pointer_over,
            Some(EditableModeState::TextBoxFocusedOverlayPressed) => {
                r.combo_box_focused_drop_down_background_pointer_pressed
            }
            Some(EditableModeState::TextBoxOverlayPressed) => {
                r.combo_box_drop_down_background_pointer_pressed
            }
            _ => Color::new(0),
        };
        let overlay = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                self.set_state(app, |state| state.pointer_over_overlay = true)
            }))
            .on_exit(Rc::new(move |app, _| {
                self.set_state(app, |state| state.pointer_over_overlay = false)
            }))
            .child(
                KeyedSubtree::new(
                    ControlBorder::new(Brush::Solid(background), Brush::Solid(Color::new(0)))
                        .border_thickness(0.0)
                        .corner_radius_corners(
                            r.combo_box_drop_down_button_background_corner_radius,
                        ),
                )
                .key(Rc::new(app.get(self).overlay_key.clone())),
            );
        let overlay = self.pointer_target(app, overlay.into_widget(), text_box_enabled);
        // Right-aligned and stretched to the row, inside its margin; it adds nothing to the
        // row's size.
        let [left, top, right, bottom] = DROP_DOWN_OVERLAY_MARGIN;
        debug_assert!(left >= 0.0);
        Stack::new()
            .children([
                // What the stack sizes to: nothing of its own, the whole cell once arranged.
                SizedBox::shrink().into_widget(),
                Positioned::new(overlay)
                    .top(top)
                    .bottom(bottom)
                    .right(right)
                    .width(DROP_DOWN_OVERLAY_WIDTH)
                    .into_widget(),
            ])
            .into_widget()
    }

    /// The child of `Popup` in the root overlay: the light-dismiss canvas over the window and
    /// `PopupBorder` where `ArrangePopup` puts it. A change of the window's size closes it, as
    /// `Popup`'s own light dismiss does.
    fn popup_overlay(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        info: OverlayChildLayoutInfo,
    ) -> WidgetRef {
        let window = info.overlay_size();
        if app
            .get(self)
            .window_at_open
            .is_some_and(|at_open| at_open != window)
        {
            app.get_mut(self).window_at_open = None;
            self.after_frame(app, move |app| {
                if app.get(self).open {
                    self.put_is_drop_down_open(app, false);
                }
            });
        }
        let target = transform_rect(
            &info.child_paint_transform(),
            Offset::ZERO & info.child_size(),
        );
        let Some(arrangement) = self.arrangement(app, context, target, window) else {
            return SizedBox::shrink().into_widget();
        };
        if arrangement.opened_up != app.get(self).opened_up {
            // `ComboBoxHelper::UpdateCornerRadius`, after the drop-down opened.
            self.after_frame(app, move |app| {
                self.set_state(app, |state| state.opened_up = arrangement.opened_up)
            });
        }
        let barrier = PointerListener::new()
            .behavior(HitTestBehavior::Opaque)
            .on_pointer_down(Rc::new(move |app, event| {
                // `OnElementOutsidePopupPointerPressed`.
                if event.buttons & K_PRIMARY_BUTTON != 0 {
                    self.light_dismiss(app);
                }
            }))
            .child(SizedBox::expand());
        let presenter = self.popup_border(app, context, arrangement);
        let overlay = Stack::new()
            .fit(inset_rendering::StackFit::Expand)
            .children([
                barrier.into_widget(),
                CustomSingleChildLayout::new(Rc::new(PopupPosition {
                    arrangement,
                    laid_out_height: app.get(self).last_popup_height.clone(),
                }))
                .child(presenter)
                .into_widget(),
            ]);
        TextFieldTapRegion::new(overlay)
            .enabled(self.is_editable(app))
            .into_widget()
    }

    /// `PopupBorder`: the lazy list of the items in `ScrollViewer`, inside the items' margin, on
    /// acrylic, with ElevationHelper's shadow.
    fn popup_border(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        arrangement: Arrangement,
    ) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let Some(scroll) = app.get(self).scroll else {
            return SizedBox::shrink().into_widget();
        };
        let overlay = r.overlay_corner_radius;
        let corner_radius = if !self.is_editable(app) {
            overlay
        } else if arrangement.opened_up {
            [overlay[0], overlay[1], 0.0, 0.0]
        } else {
            [0.0, 0.0, overlay[2], overlay[3]]
        };
        let ids: Vec<String> = self
            .widget(app)
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect();
        let list = ListView::builder(move |app, _, index| self.item_container(app, index as usize))
            .item_count(ids.len() as i32)
            .find_child_index_callback(Rc::new(move |key| {
                (key.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<ValueKey<String>>()
                    .and_then(|key| ids.iter().position(|id| *id == key.value))
                    .map(|index| index as i32)
            }))
            .build()
            .controller(scroll.native_controller(app))
            .shrink_wrap(true)
            .padding(EdgeInsetsGeometry::ZERO);
        // `ItemsPresenter`'s margin stays while the items scroll.
        let scroll_viewer = ScrollBarViewport::scroller(
            scroll,
            Padding::new(insets(r.combo_box_dropdown_content_margin)).child(list),
        );
        let border = ControlBorder::new(
            Brush::Acrylic(r.combo_box_drop_down_background),
            Brush::Solid(r.combo_box_drop_down_border_brush),
        )
        .border_thickness_ltrb(r.combo_box_dropdown_border_thickness)
        .corner_radius_corners(corner_radius)
        .background_sizing(BackgroundSizing::InnerBorderEdge)
        .padding(r.combo_box_dropdown_border_padding)
        .child(DefaultTextStyle::new(
            control_text_style(
                &resources,
                r.control_content_theme_font_size,
                FontWeight::NORMAL,
                r.combo_box_drop_down_foreground,
            ),
            scroll_viewer,
        ));
        MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                // `OnElementPopupChildPointerEntered`.
                self.set_state(app, |state| state.pointer_over_popup = true)
            }))
            .on_exit(Rc::new(move |app, _| {
                self.set_state(app, |state| state.pointer_over_popup = false)
            }))
            .child(
                CustomPaint::new()
                    .painter(PopupShadow::for_combo_box_popup(
                        resources.theme,
                        corner_radius,
                    ))
                    .child(border),
            )
            .into_widget()
    }

    /// The container of the item at `index`, measured where the list lays it out.
    fn item_container(self: Handle<Self>, app: &mut App, index: usize) -> Option<WidgetRef> {
        let item = self.widget(app).items.get(index)?.clone();
        let id = item.id.clone();
        let focus_node = self.item_focus_node(app, &id);
        let shows_focus_visual = FocusState::current(app).shows_focus_visual();
        let state = app.get(self);
        let container = ComboBoxItemContainer {
            appears_selected: self.appears_selected(app, index),
            selection_active: state.selection_active,
            shows_focus_visual,
            focus_node,
            clicked: Rc::new(move |app, index| self.item_clicked(app, index)),
            focus_changed: Rc::new(move |app, index, focused| {
                self.item_focus_changed(app, index, focused)
            }),
            item,
            index,
        };
        let measured = id.clone();
        Some(
            KeyedSubtree::new(ItemProbe::new(
                container,
                Rc::new(move |app, size| self.item_measured(app, &measured, size)),
            ))
            .key(Rc::new(ValueKey::new(id)))
            .into_widget(),
        )
    }
}

/// The keyboard modifiers `Input_GetKeyboardModifiers` reports.
#[derive(Clone, Copy, Debug)]
struct Modifiers {
    alt: bool,
    control: bool,
    shift: bool,
    meta: bool,
}

impl Modifiers {
    fn of(app: &mut App) -> Modifiers {
        let keyboard = HardwareKeyboard::instance(app);
        Modifiers {
            alt: keyboard.is_alt_pressed(app),
            control: keyboard.is_control_pressed(app),
            shift: keyboard.is_shift_pressed(app),
            meta: keyboard.is_meta_pressed(app),
        }
    }

    /// `VirtualKeyModifiers_None`.
    fn none(self) -> bool {
        !(self.alt || self.control || self.shift || self.meta)
    }
}

/// Keys the box handles that WinUI's TextBox takes first, so that they never reach an editable
/// box's key handling while its text box has the focus. `CTextBoxBase::OnKeyDown` passes on Page
/// Up and Page Down when they move no caret, as in one line, and Backspace always.
fn is_text_box_key(key: LogicalKeyboardKey) -> bool {
    matches!(
        key,
        LogicalKeyboardKey::ARROW_LEFT
            | LogicalKeyboardKey::ARROW_RIGHT
            | LogicalKeyboardKey::HOME
            | LogicalKeyboardKey::END
            | LogicalKeyboardKey::DELETE
    )
}

/// The size an item container takes for one line of `text`: its content's, `Padding`, and
/// `LayoutRoot`'s margin, at least `Pill`'s height.
fn text_item_size(app: &mut App, context: BuildContext, text: &str) -> Size {
    let r = ThemeResources::of(app, context);
    let fonts = PaintingBinding::instance(app).fonts(app);
    let mut painter = TextPainter::new();
    painter.set_text(Some(Rc::new(TextSpan::new().text(text.to_owned()).style(
        control_text_style(
            &r,
            r.control_content_theme_font_size,
            FontWeight::NORMAL,
            Color::new(0),
        ),
    ))));
    painter.set_text_direction(Some(Directionality::of(app, context)));
    painter.set_text_scaler(MediaQuery::text_scaler_of(app, context));
    painter.layout(app.get_mut(fonts), 0.0, f64::INFINITY);
    let size = Size::new(painter.width(), painter.height());
    painter.dispose();
    let padding = r.combo_box_item_theme_padding;
    Size::new(
        size.width() + padding[0] + padding[2] + ITEM_MARGIN[0] + ITEM_MARGIN[2],
        (size.height() + padding[1] + padding[3]).max(r.combo_box_item_pill_height)
            + ITEM_MARGIN[1]
            + ITEM_MARGIN[3],
    )
}

/// Converts XAML thickness order into padding.
fn insets(value: [f64; 4]) -> EdgeInsetsGeometry {
    EdgeInsetsGeometry::from_ltrb(value[0], value[1], value[2], value[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selection_changed_arguments_name_the_new_selection() {
        let args = ComboBoxSelectionChangedEventArgs {
            removed_item: Some(ComboBoxSelectedItem::CustomValue("13".to_owned())),
            added_item: Some(ComboBoxSelectedItem::Item {
                index: 2,
                id: "b".to_owned(),
            }),
        };
        assert_eq!(args.selected_index(), Some(2));
        assert_eq!(args.custom_value(), None);
    }

    #[test]
    fn a_widget_passes_an_index_before_a_custom_value() {
        let widget =
            ComboBox::new(vec![ComboBoxItem::text("a", "A")], Some(0), |_, _| {}).custom_value("x");
        assert_eq!(Selection::of(&widget), Selection::Index(0));
        let widget = ComboBox::new(Vec::new(), None, |_, _| {}).custom_value("x");
        assert_eq!(Selection::of(&widget), Selection::Custom("x".to_owned()));
    }
}
