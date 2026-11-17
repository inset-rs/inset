//! Flutter counterpart: `rendering/selection.dart`.
//!
//! The types a selection stack builds on: what can be selected ([`Selectable`]), what keeps
//! track of it ([`SelectionRegistrar`]), the events sent to it, and the geometry and content read
//! back from it. A widget's side of this (`SelectionContainer`) lives in `waft-widgets`.
//!
//! [`AnySelectable`] and [`AnySelectionRegistrar`] are Dart's abstract classes used as types: an
//! identity plus a vtable, so a registrar can hold selectables of more than one kind and a widget
//! can hold a registrar without naming its concrete type.

use waft_embedder::{Matrix4, Offset, Rect, Size, TextDirection};
use waft_foundation::{
    App, Handle, HandleId, ListenableHandle, ListenableObject, Listener, Ref, Retained,
};

use crate::layer::LayerLink;
use crate::object::AnyRenderObject;

/// The result after handling a [`SelectionEvent`].
///
/// A [`SelectionContainer`](https://api.flutter.dev/flutter/widgets/SelectionContainer-class.html)
/// uses it to decide how a selection expands across its selectables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionResult {
    /// There is nothing left to select forward in this selectable, and further selection should
    /// extend to the next selectable in screen order.
    Next,
    /// Selection does not reach this selectable and is located before it in screen order.
    Previous,
    /// Selection ends in this selectable.
    End,
    /// The result can't be determined in this frame; the subtree is scrolling to reveal content.
    Pending,
    /// There is no result for the event, as for select-all, clear and select-word.
    None,
}

/// The selected range of one selectable or handler, in content offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SelectedContentRange {
    /// The start of the selection relative to the start of the content.
    pub start_offset: i32,
    /// The end of the selection relative to the start of the content.
    pub end_offset: i32,
}

impl SelectedContentRange {
    /// Creates a range; both offsets are non-negative, as Dart asserts.
    pub fn new(start_offset: i32, end_offset: i32) -> SelectedContentRange {
        debug_assert!(start_offset >= 0 && end_offset >= 0);
        SelectedContentRange {
            start_offset,
            end_offset,
        }
    }
}

/// The selected content in a selectable or handler; only plain text so far.
#[derive(Clone, Debug)]
pub struct SelectedContent {
    /// The selected content in plain text format.
    pub plain_text: String,
}

impl SelectedContent {
    /// Creates selected content of plain text.
    pub fn new(plain_text: impl Into<String>) -> SelectedContent {
        SelectedContent {
            plain_text: plain_text.into(),
        }
    }
}

/// The abstract interface to handle [`SelectionEvent`]s.
///
/// Extended by [`Selectable`] and by a widget's selection container delegate. This class returns
/// a [`SelectionGeometry`] as its [`value`](Self::value) and notifies its listeners when that
/// geometry changes, as Dart's `ValueListenable<SelectionGeometry>`.
pub trait SelectionHandler: ListenableObject {
    /// The geometry of the current selection.
    fn value(self: Handle<Self>, app: &App) -> SelectionGeometry;

    /// Marks this handler responsible for pushing [`LeaderLayer`](crate::LeaderLayer)s for the
    /// selection handles. `None` cleans up a layer it pushed before.
    fn push_handle_layers(
        self: Handle<Self>,
        app: &mut App,
        start_handle: Option<Ref<LayerLink>>,
        end_handle: Option<Ref<LayerLink>>,
    );

    /// The selected content in this object, if anything is selected.
    fn selected_content(self: Handle<Self>, app: &App) -> Option<SelectedContent>;

    /// The selected range in this object; `None` when nothing is selected.
    fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange>;

    /// Handles a selection event sent to this object.
    fn dispatch_selection_event(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult;

    /// The length of the content in this object.
    fn content_length(self: Handle<Self>, app: &App) -> usize;
}

/// A mixin that can be selected by users under a `SelectionArea`.
///
/// Receives selection events; [`value`](SelectionHandler::value) must reflect the current
/// selection, and listeners are notified when it changes. The object draws the selection
/// highlight itself. It registers with a [`SelectionRegistrar`] through [`SelectionRegistrant`].
pub trait Selectable: SelectionHandler {
    /// Erases this selectable for a registrar, as Dart's implicit upcast.
    fn as_selectable(self: Handle<Self>) -> AnySelectable {
        AnySelectable {
            id: self.id(),
            vtable: const { &SelectableVTable::of::<Self>() },
        }
    }

    /// {@macro flutter.rendering.RenderObject.getTransformTo}
    fn get_transform_to(
        self: Handle<Self>,
        app: &App,
        ancestor: Option<AnyRenderObject>,
    ) -> Matrix4;

    /// The size of this selectable.
    fn size(self: Handle<Self>, app: &mut App) -> Size;

    /// The bounding boxes of this selectable in its local coordinates.
    fn bounding_boxes(self: Handle<Self>, app: &mut App) -> Vec<Rect>;

    /// Disposes resources held by the mixin.
    fn dispose_selectable(self: Handle<Self>, app: &mut App);
}

/// Erased [`Selectable`]: one identity and a static vtable, as a registrar holds selectables of
/// more than one concrete type.
#[derive(Clone, Copy)]
pub struct AnySelectable {
    id: HandleId,
    vtable: &'static SelectableVTable,
}

/// The erased `pushHandleLayers`: the object, its two links, and the app whose paint it marks.
pub type PushHandleLayers = fn(&mut App, HandleId, Option<Ref<LayerLink>>, Option<Ref<LayerLink>>);

struct SelectableVTable {
    value: fn(&App, HandleId) -> SelectionGeometry,
    push_handle_layers: PushHandleLayers,
    selected_content: fn(&App, HandleId) -> Option<SelectedContent>,
    selection: fn(&App, HandleId) -> Option<SelectedContentRange>,
    dispatch_selection_event: fn(&mut App, HandleId, &SelectionEvent) -> SelectionResult,
    content_length: fn(&App, HandleId) -> usize,
    get_transform_to: fn(&App, HandleId, Option<AnyRenderObject>) -> Matrix4,
    size: fn(&mut App, HandleId) -> Size,
    bounding_boxes: fn(&mut App, HandleId) -> Vec<Rect>,
    dispose_selectable: fn(&mut App, HandleId),
}

impl SelectableVTable {
    const fn of<A: Selectable>() -> SelectableVTable {
        SelectableVTable {
            value: |app, id| A::value(resolve(id), app),
            push_handle_layers: |app, id, start, end| {
                A::push_handle_layers(resolve(id), app, start, end)
            },
            selected_content: |app, id| A::selected_content(resolve(id), app),
            selection: |app, id| A::selection(resolve(id), app),
            dispatch_selection_event: |app, id, event| {
                A::dispatch_selection_event(resolve(id), app, event)
            },
            content_length: |app, id| A::content_length(resolve(id), app),
            get_transform_to: |app, id, ancestor| A::get_transform_to(resolve(id), app, ancestor),
            size: |app, id| A::size(resolve(id), app),
            bounding_boxes: |app, id| A::bounding_boxes(resolve(id), app),
            dispose_selectable: |app, id| A::dispose_selectable(resolve(id), app),
        }
    }
}

fn resolve<A: 'static>(id: HandleId) -> Handle<A> {
    Handle::from_id(id)
}

impl AnySelectable {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// See [`SelectionHandler::value`].
    pub fn value(self, app: &App) -> SelectionGeometry {
        (self.vtable.value)(app, self.id)
    }

    /// See [`SelectionHandler::push_handle_layers`].
    pub fn push_handle_layers(
        self,
        app: &mut App,
        start_handle: Option<Ref<LayerLink>>,
        end_handle: Option<Ref<LayerLink>>,
    ) {
        (self.vtable.push_handle_layers)(app, self.id, start_handle, end_handle)
    }

    /// See [`SelectionHandler::selected_content`].
    pub fn selected_content(self, app: &App) -> Option<SelectedContent> {
        (self.vtable.selected_content)(app, self.id)
    }

    /// See [`SelectionHandler::selection`].
    pub fn selection(self, app: &App) -> Option<SelectedContentRange> {
        (self.vtable.selection)(app, self.id)
    }

    /// See [`SelectionHandler::dispatch_selection_event`].
    pub fn dispatch_selection_event(
        self,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        (self.vtable.dispatch_selection_event)(app, self.id, event)
    }

    /// See [`SelectionHandler::content_length`].
    pub fn content_length(self, app: &App) -> usize {
        (self.vtable.content_length)(app, self.id)
    }

    /// See [`Selectable::get_transform_to`].
    pub fn get_transform_to(self, app: &App, ancestor: Option<AnyRenderObject>) -> Matrix4 {
        (self.vtable.get_transform_to)(app, self.id, ancestor)
    }

    /// See [`Selectable::size`].
    pub fn size(self, app: &mut App) -> Size {
        (self.vtable.size)(app, self.id)
    }

    /// See [`Selectable::bounding_boxes`].
    pub fn bounding_boxes(self, app: &mut App) -> Vec<Rect> {
        (self.vtable.bounding_boxes)(app, self.id)
    }

    /// See [`Selectable::dispose_selectable`].
    pub fn dispose_selectable(self, app: &mut App) {
        (self.vtable.dispose_selectable)(app, self.id)
    }
}

impl std::fmt::Debug for AnySelectable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnySelectable")
            .field("id", &self.id)
            .finish()
    }
}

waft_foundation::erased_handle!(AnySelectable: Selectable => Selectable::as_selectable);

/// The fields of Dart's `SelectionRegistrant` mixin.
#[derive(Debug, Default)]
pub struct SelectionRegistrantData {
    registrar: Option<SelectionRegistrarRef>,
    subscribed_to_selection_registrar: bool,
}

impl SelectionRegistrantData {
    /// Creates registrant fields with no registrar.
    pub fn new() -> SelectionRegistrantData {
        SelectionRegistrantData::default()
    }
}

/// A mixin that auto-registers the mixer with its registrar.
///
/// Set [`registrar`](SelectionRegistrant::registrar) to the registrar to register to. The mixin
/// only registers while the [`SelectionGeometry::has_content`] returned by the selectable is true.
pub trait SelectionRegistrant: Selectable {
    /// The fields of this mixin.
    fn selection_registrant_data(self: Handle<Self>, app: &App) -> &SelectionRegistrantData;

    /// The fields of this mixin, mutably.
    fn selection_registrant_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SelectionRegistrantData;

    /// The registrar this mixin will be or is registered to.
    fn registrar(self: Handle<Self>, app: &App) -> Option<AnySelectionRegistrar> {
        self.selection_registrant_data(app)
            .registrar
            .as_ref()
            .map(Retained::handle)
    }

    /// Dart `SelectionRegistrant.registrar =`.
    fn set_registrar(self: Handle<Self>, app: &mut App, value: Option<SelectionRegistrarRef>) {
        let registrar = self.registrar(app);
        if value.as_ref().map(Retained::handle) == registrar {
            return;
        }
        let listener = Listener::handle_method(self, Self::update_selection_registrar_subscription);
        if value.is_none() {
            // When registrar goes from non-null to null.
            self.remove_listener_from(app, &listener);
        } else if registrar.is_none() {
            // When registrar goes from null to non-null.
            self.add_listener_to(app, listener);
        }
        self.remove_selection_registrar_subscription(app);
        self.selection_registrant_data_mut(app).registrar = value;
        self.update_selection_registrar_subscription(app);
    }

    /// Dart's `_updateSelectionRegistrarSubscription`.
    fn update_selection_registrar_subscription(self: Handle<Self>, app: &mut App) {
        let Some(registrar) = self.registrar(app) else {
            self.selection_registrant_data_mut(app)
                .subscribed_to_selection_registrar = false;
            return;
        };
        let has_content = self.value(app).has_content;
        let subscribed = self
            .selection_registrant_data(app)
            .subscribed_to_selection_registrar;
        if subscribed && !has_content {
            registrar.remove(app, self.as_selectable());
            self.selection_registrant_data_mut(app)
                .subscribed_to_selection_registrar = false;
        } else if !subscribed && has_content {
            registrar.add(app, self.as_selectable());
            self.selection_registrant_data_mut(app)
                .subscribed_to_selection_registrar = true;
        }
    }

    /// Dart's `_removeSelectionRegistrarSubscription`.
    fn remove_selection_registrar_subscription(self: Handle<Self>, app: &mut App) {
        if self
            .selection_registrant_data(app)
            .subscribed_to_selection_registrar
        {
            let registrar = self
                .registrar(app)
                .expect("a subscribed registrant has a registrar");
            registrar.remove(app, self.as_selectable());
            self.selection_registrant_data_mut(app)
                .subscribed_to_selection_registrar = false;
        }
    }

    /// Dart `SelectionRegistrant.dispose`'s own part; the mixer then runs the rest of its
    /// `dispose`, as Dart's `super.dispose()`.
    fn dispose_selection_registrant(self: Handle<Self>, app: &mut App) {
        self.remove_selection_registrar_subscription(app);
    }
}

/// A registrar that keeps track of the [`Selectable`]s in a subtree.
pub trait SelectionRegistrar: Sized + 'static {
    /// Erases this registrar, as Dart's implicit upcast.
    fn as_selection_registrar(self: Handle<Self>) -> AnySelectionRegistrar {
        AnySelectionRegistrar {
            id: self.id(),
            vtable: const { &SelectionRegistrarVTable::of::<Self>() },
        }
    }

    /// Adds `selectable` to the registrar.
    fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable);

    /// Removes `selectable` from the registrar.
    fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable);
}

/// Erased [`SelectionRegistrar`], for a registrant that does not name the concrete registrar.
#[derive(Clone, Copy)]
pub struct AnySelectionRegistrar {
    id: HandleId,
    vtable: &'static SelectionRegistrarVTable,
}

struct SelectionRegistrarVTable {
    add: fn(&mut App, HandleId, AnySelectable),
    remove: fn(&mut App, HandleId, AnySelectable),
}

impl SelectionRegistrarVTable {
    const fn of<A: SelectionRegistrar>() -> SelectionRegistrarVTable {
        SelectionRegistrarVTable {
            add: |app, id, selectable| A::add(resolve(id), app, selectable),
            remove: |app, id, selectable| A::remove(resolve(id), app, selectable),
        }
    }
}

impl AnySelectionRegistrar {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// See [`SelectionRegistrar::add`].
    pub fn add(self, app: &mut App, selectable: AnySelectable) {
        (self.vtable.add)(app, self.id, selectable)
    }

    /// See [`SelectionRegistrar::remove`].
    pub fn remove(self, app: &mut App, selectable: AnySelectable) {
        (self.vtable.remove)(app, self.id, selectable)
    }
}

impl std::fmt::Debug for AnySelectionRegistrar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnySelectionRegistrar")
            .field("id", &self.id)
            .finish()
    }
}

/// A field that keeps a registrar: a paragraph's, a registrant's, a widget's.
///
/// A registrar holds its selectables as bare [`AnySelectable`]s, never kept, so a selectable that
/// keeps its registrar makes no cycle; a selectable removes itself before it is freed.
pub type SelectionRegistrarRef = Retained<AnySelectionRegistrar>;

waft_foundation::erased_handle!(AnySelectionRegistrar: SelectionRegistrar => SelectionRegistrar::as_selection_registrar);

/// The status of a selection: whether there is one and whether it is collapsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionStatus {
    /// The selection is not collapsed.
    Uncollapsed,
    /// The selection starts and ends at the same location.
    Collapsed,
    /// No selection.
    None,
}

/// The geometry of the current selection, in the local coordinates of the selectable.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionGeometry {
    /// The geometry at the selection start, `None` when it is offstage.
    pub start_selection_point: Option<SelectionPoint>,
    /// The geometry at the selection end, `None` when it is offstage.
    pub end_selection_point: Option<SelectionPoint>,
    /// The status of the ongoing selection.
    pub status: SelectionStatus,
    /// The rects that represent the selection, if there is any.
    pub selection_rects: Vec<Rect>,
    /// Whether there is any selectable content.
    pub has_content: bool,
}

impl SelectionGeometry {
    /// Creates a selection geometry with no selection points and no rects; the setters give
    /// them, as Dart's optional arguments.
    pub fn new(status: SelectionStatus, has_content: bool) -> SelectionGeometry {
        SelectionGeometry {
            start_selection_point: None,
            end_selection_point: None,
            status,
            selection_rects: Vec::new(),
            has_content,
        }
    }

    /// Dart `SelectionGeometry(startSelectionPoint:)`.
    pub fn start_selection_point(mut self, value: Option<SelectionPoint>) -> SelectionGeometry {
        self.start_selection_point = value;
        self.debug_assert_points_have_selection();
        self
    }

    /// Dart `SelectionGeometry(endSelectionPoint:)`.
    pub fn end_selection_point(mut self, value: Option<SelectionPoint>) -> SelectionGeometry {
        self.end_selection_point = value;
        self.debug_assert_points_have_selection();
        self
    }

    /// Dart `SelectionGeometry(selectionRects:)`.
    pub fn selection_rects(mut self, value: Vec<Rect>) -> SelectionGeometry {
        self.selection_rects = value;
        self
    }

    /// Dart `SelectionGeometry(status:)`.
    pub fn status(mut self, value: SelectionStatus) -> SelectionGeometry {
        self.status = value;
        self
    }

    /// Dart `SelectionGeometry(hasContent:)`.
    pub fn has_content(mut self, value: bool) -> SelectionGeometry {
        self.has_content = value;
        self
    }

    fn debug_assert_points_have_selection(&self) {
        debug_assert!(
            (self.start_selection_point.is_none() && self.end_selection_point.is_none())
                || self.status != SelectionStatus::None
        );
    }

    /// Whether there is an ongoing selection.
    pub fn has_selection(&self) -> bool {
        self.status != SelectionStatus::None
    }

    /// A copy the setters then change, as Dart's `copyWith`.
    pub fn copy_with(&self) -> SelectionGeometry {
        self.clone()
    }
}

/// The geometry information of one selection point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionPoint {
    /// The position of the selection point in the containing selectable's coordinates.
    pub local_position: Offset,
    /// The line height at the selection point.
    pub line_height: f64,
    /// The selection handle to build at the point.
    pub handle_type: TextSelectionHandleType,
}

impl SelectionPoint {
    /// Creates a selection point.
    pub fn new(
        local_position: Offset,
        line_height: f64,
        handle_type: TextSelectionHandleType,
    ) -> SelectionPoint {
        SelectionPoint {
            local_position,
            line_height,
            handle_type,
        }
    }
}

/// The type of selection handle to display.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextSelectionHandleType {
    /// The selection handle is to the left of the selection end point.
    Left,
    /// The selection handle is to the right of the selection end point.
    Right,
    /// The start and end of the selection are co-incident at this point.
    Collapsed,
}

/// A utility class that provides useful methods for handling selection events.
pub struct SelectionUtils;

impl SelectionUtils {
    /// Determines a [`SelectionResult`] purely from the target rectangle.
    ///
    /// [`SelectionResult::End`] when `point` is inside `target_rect`, [`SelectionResult::Previous`]
    /// when it is above it in screen order, and [`SelectionResult::Next`] when below.
    pub fn get_result_based_on_rect(target_rect: Rect, point: Offset) -> SelectionResult {
        if target_rect.contains(point) {
            return SelectionResult::End;
        }
        if point.dy() < target_rect.top {
            return SelectionResult::Previous;
        }
        if point.dy() > target_rect.bottom {
            return SelectionResult::Next;
        }
        if point.dx() >= target_rect.right {
            SelectionResult::Next
        } else {
            SelectionResult::Previous
        }
    }

    /// Adjusts a dragging offset to lie within `target_rect`.
    ///
    /// A point above the rect (or left of it, when at or above the bottom edge) moves to the top
    /// leading corner; any other outside point moves to the bottom trailing corner. Leading and
    /// trailing follow `direction`, as Dart's default `TextDirection.ltr` does.
    pub fn adjust_drag_offset(
        target_rect: Rect,
        point: Offset,
        direction: TextDirection,
    ) -> Offset {
        if target_rect.contains(point) {
            return point;
        }
        if point.dy() <= target_rect.top
            || point.dy() <= target_rect.bottom && point.dx() <= target_rect.left
        {
            // Area 1.
            if direction == TextDirection::Ltr {
                Offset::new(target_rect.left, target_rect.top)
            } else {
                Offset::new(target_rect.right, target_rect.top)
            }
        } else {
            // Area 2.
            if direction == TextDirection::Ltr {
                Offset::new(target_rect.right, target_rect.bottom)
            } else {
                Offset::new(target_rect.left, target_rect.bottom)
            }
        }
    }
}

/// The unit of how selection handles move in text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextGranularity {
    /// Each character is an atomic unit.
    Character,
    /// A word is an atomic unit.
    Word,
    /// A paragraph is an atomic unit.
    Paragraph,
    /// Each line break is an atomic unit.
    Line,
    /// The entire document is an atomic unit.
    Document,
}

/// The direction to extend a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionExtendDirection {
    /// Move one edge vertically to the previous adjacent line.
    PreviousLine,
    /// Move one edge vertically to the next adjacent line.
    NextLine,
    /// Move the selection edges forward to a horizontal offset in the same line.
    Forward,
    /// Move the selection edges backward to a horizontal offset in the same line.
    Backward,
}

/// A selection event sent to a [`SelectionHandler`].
///
/// Dart's abstract `SelectionEvent` and its subclasses are one enum: the subclass is the variant,
/// and [`SelectionEventType`] follows from it.
#[derive(Clone, Debug)]
pub enum SelectionEvent {
    /// Updates the selection start edge.
    StartEdgeUpdate {
        /// The new location of the edge, in global coordinates.
        global_position: Offset,
        /// The granularity the edge moves by.
        granularity: TextGranularity,
    },
    /// Updates the selection end edge.
    EndEdgeUpdate {
        /// The new location of the edge, in global coordinates.
        global_position: Offset,
        /// The granularity the edge moves by.
        granularity: TextGranularity,
    },
    /// Clears the current selection.
    Clear,
    /// Selects all the available content.
    SelectAll,
    /// Selects the word at `global_position`.
    SelectWord {
        /// The position in global coordinates to select a word at.
        global_position: Offset,
    },
    /// Selects the paragraph at `global_position`.
    SelectParagraph {
        /// The position in global coordinates to select a paragraph at.
        global_position: Offset,
        /// Whether the selectable should be absorbed into an encompassing paragraph.
        absorb: bool,
    },
    /// Extends the start or end of the selection by `granularity`.
    GranularlyExtendSelection {
        /// Whether to extend the selection forward.
        forward: bool,
        /// Whether this event updates the end edge.
        is_end: bool,
        /// The granularity to extend by.
        granularity: TextGranularity,
    },
    /// Extends the current selection with respect to `direction`.
    DirectionallyExtendSelection {
        /// The horizontal offset to move to, in global coordinates.
        dx: f64,
        /// Whether this event updates the end edge.
        is_end: bool,
        /// The direction to move.
        direction: SelectionExtendDirection,
    },
}

impl SelectionEvent {
    /// The type of this event.
    pub fn ty(&self) -> SelectionEventType {
        match self {
            SelectionEvent::StartEdgeUpdate { .. } => SelectionEventType::StartEdgeUpdate,
            SelectionEvent::EndEdgeUpdate { .. } => SelectionEventType::EndEdgeUpdate,
            SelectionEvent::Clear => SelectionEventType::Clear,
            SelectionEvent::SelectAll => SelectionEventType::SelectAll,
            SelectionEvent::SelectWord { .. } => SelectionEventType::SelectWord,
            SelectionEvent::SelectParagraph { .. } => SelectionEventType::SelectParagraph,
            SelectionEvent::GranularlyExtendSelection { .. } => {
                SelectionEventType::GranularlyExtendSelection
            }
            SelectionEvent::DirectionallyExtendSelection { .. } => {
                SelectionEventType::DirectionallyExtendSelection
            }
        }
    }

    /// Creates a start-edge update at `global_position`, at character granularity unless given.
    pub fn for_start(
        global_position: Offset,
        granularity: Option<TextGranularity>,
    ) -> SelectionEvent {
        SelectionEvent::StartEdgeUpdate {
            global_position,
            granularity: granularity.unwrap_or(TextGranularity::Character),
        }
    }

    /// Creates an end-edge update at `global_position`, at character granularity unless given.
    pub fn for_end(
        global_position: Offset,
        granularity: Option<TextGranularity>,
    ) -> SelectionEvent {
        SelectionEvent::EndEdgeUpdate {
            global_position,
            granularity: granularity.unwrap_or(TextGranularity::Character),
        }
    }

    /// A copy with the given values replaced.
    pub fn copy_with(
        &self,
        dx: Option<f64>,
        is_end: Option<bool>,
        direction: Option<SelectionExtendDirection>,
    ) -> SelectionEvent {
        match self {
            SelectionEvent::DirectionallyExtendSelection {
                dx: old_dx,
                is_end: old_is_end,
                direction: old_direction,
            } => SelectionEvent::DirectionallyExtendSelection {
                dx: dx.unwrap_or(*old_dx),
                is_end: is_end.unwrap_or(*old_is_end),
                direction: direction.unwrap_or(*old_direction),
            },
            event => event.clone(),
        }
    }
}

/// The type of a [`SelectionEvent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionEventType {
    /// An event to update the selection start edge.
    StartEdgeUpdate,
    /// An event to update the selection end edge.
    EndEdgeUpdate,
    /// An event to clear the current selection.
    Clear,
    /// An event to select all the available content.
    SelectAll,
    /// An event to select a word at a location.
    SelectWord,
    /// An event to select a paragraph at a location.
    SelectParagraph,
    /// An event that extends the selection by a [`TextGranularity`].
    GranularlyExtendSelection,
    /// An event that extends the selection in a [`SelectionExtendDirection`].
    DirectionallyExtendSelection,
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect = Rect::from_ltrb(100.0, 100.0, 300.0, 600.0);
    const OUTSIDE_TOP_LEFT: Offset = Offset::new(50.0, 50.0);
    const OUTSIDE_LEFT: Offset = Offset::new(50.0, 200.0);
    const OUTSIDE_BOTTOM_LEFT: Offset = Offset::new(50.0, 700.0);
    const OUTSIDE_TOP: Offset = Offset::new(200.0, 50.0);
    const OUTSIDE_TOP_RIGHT: Offset = Offset::new(350.0, 50.0);
    const OUTSIDE_RIGHT: Offset = Offset::new(350.0, 200.0);
    const OUTSIDE_BOTTOM_RIGHT: Offset = Offset::new(350.0, 700.0);
    const OUTSIDE_BOTTOM: Offset = Offset::new(200.0, 700.0);
    const CENTER: Offset = Offset::new(150.0, 300.0);

    // selection_test.dart 'selectionBasedOnRect works'.
    #[test]
    fn selection_based_on_rect_works() {
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_TOP_LEFT),
            SelectionResult::Previous
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_LEFT),
            SelectionResult::Previous
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_BOTTOM_LEFT),
            SelectionResult::Next
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_TOP),
            SelectionResult::Previous
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_TOP_RIGHT),
            SelectionResult::Previous
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_RIGHT),
            SelectionResult::Next
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_BOTTOM_RIGHT),
            SelectionResult::Next
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, OUTSIDE_BOTTOM),
            SelectionResult::Next
        );
        assert_eq!(
            SelectionUtils::get_result_based_on_rect(RECT, CENTER),
            SelectionResult::End
        );
    }

    // selection_test.dart 'adjustDragOffset works'.
    #[test]
    fn adjust_drag_offset_works() {
        let top_left = Offset::new(RECT.left, RECT.top);
        let top_right = Offset::new(RECT.right, RECT.top);
        let bottom_left = Offset::new(RECT.left, RECT.bottom);
        let bottom_right = Offset::new(RECT.right, RECT.bottom);

        // ltr
        for (point, expected) in [
            (OUTSIDE_TOP_LEFT, top_left),
            (OUTSIDE_LEFT, top_left),
            (OUTSIDE_BOTTOM_LEFT, bottom_right),
            (OUTSIDE_TOP, top_left),
            (OUTSIDE_TOP_RIGHT, top_left),
            (OUTSIDE_RIGHT, bottom_right),
            (OUTSIDE_BOTTOM_RIGHT, bottom_right),
            (OUTSIDE_BOTTOM, bottom_right),
            (CENTER, CENTER),
        ] {
            assert_eq!(
                SelectionUtils::adjust_drag_offset(RECT, point, TextDirection::Ltr),
                expected
            );
        }
        // rtl
        for (point, expected) in [
            (OUTSIDE_TOP_LEFT, top_right),
            (OUTSIDE_LEFT, top_right),
            (OUTSIDE_BOTTOM_LEFT, bottom_left),
            (OUTSIDE_TOP, top_right),
            (OUTSIDE_TOP_RIGHT, top_right),
            (OUTSIDE_RIGHT, bottom_left),
            (OUTSIDE_BOTTOM_RIGHT, bottom_left),
            (OUTSIDE_BOTTOM, bottom_left),
            (CENTER, CENTER),
        ] {
            assert_eq!(
                SelectionUtils::adjust_drag_offset(RECT, point, TextDirection::Rtl),
                expected
            );
        }
    }
}
