//! Flutter counterpart: `widgets/selectable_region.dart` (the selection container delegates).
//!
//! A delegate owns the [`AnySelectable`]s a [`SelectionContainer`](crate::SelectionContainer)
//! collects from its subtree and folds their geometries into one, so a region can select across
//! several texts, or across a scrollable.
//!
//! [`MultiSelectableSelectionContainerDelegate`] is Dart's abstract class: a trait with the
//! class's bodies, over [`MultiSelectableSelectionContainerDelegateData`]. The bodies an override
//! reaches with `super` live on [`MultiSelectableSelectionContainerDelegateBase`], which every
//! delegate implements. [`StaticSelectionContainerDelegate`] is the concrete class that remembers
//! which selectables received the start and end edges and synthesizes the missing one; its
//! bodies live on [`StaticSelectionContainerDelegateBase`], and
//! [`PlainStaticSelectionContainerDelegate`] is Dart's instantiable `StaticSelectionContainerDelegate()`.
//! A subclass implements the trait and writes [`static_selection_container_delegate_overrides!`]
//! where Dart writes `extends StaticSelectionContainerDelegate`.
//!
//! The text and scrollable delegates, and `SelectableRegion` itself, wait for later.

use std::cmp::Ordering;

use waft_embedder::{Offset, Rect, clamp_double};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, Listener, PRECISION_ERROR_TOLERANCE, Ref,
};
use waft_painting::{transform_point, transform_rect};
use waft_rendering::{
    AnySelectable, LayerLink, SelectedContent, SelectedContentRange, SelectionEvent,
    SelectionEventType, SelectionExtendDirection, SelectionGeometry, SelectionHandler,
    SelectionPoint, SelectionRegistrar, SelectionResult, SelectionStatus,
};
use waft_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};

use crate::widgets::selection_container::{
    SelectionContainerDelegate, SelectionContainerDelegateData,
};

/// In practice some selectables, such as widget spans, shift several pixels. When the vertical
/// position difference is within this threshold, the screen order compares horizontally too.
const K_SELECTABLE_VERTICAL_COMPARING_THRESHOLD: f64 = 3.0;

/// The padding around the [`SelectionContainer`](crate::SelectionContainer) added to the
/// selection handle drawable area; a handle outside it is hidden.
const K_SELECTION_HANDLE_DRAWABLE_AREA_PADDING: f64 = 5.0;

/// Dart's `Set<Selectable>`: whether `set` holds `selectable`.
fn set_contains(set: &[AnySelectable], selectable: AnySelectable) -> bool {
    set.contains(&selectable)
}

/// Dart's `Set<Selectable>.add`: whether `selectable` was new.
fn set_add(set: &mut Vec<AnySelectable>, selectable: AnySelectable) -> bool {
    if set_contains(set, selectable) {
        return false;
    }
    set.push(selectable);
    true
}

/// Dart's `Set<Selectable>.remove`: whether `selectable` was there.
fn set_remove(set: &mut Vec<AnySelectable>, selectable: AnySelectable) -> bool {
    let Some(index) = set.iter().position(|held| *held == selectable) else {
        return false;
    };
    set.remove(index);
    true
}

/// The listener a delegate registers with each selectable, for Dart's
/// `_handleSelectableGeometryChange` tear-off.
fn selectable_geometry_change_listener<T: MultiSelectableSelectionContainerDelegate>(
    this: Handle<T>,
) -> Listener {
    Listener::handle_method(this, notify_selectable_geometry_change::<T>)
}

/// The body of [`selectable_geometry_change_listener`], as a function item so it keeps the
/// identity a tear-off has.
fn notify_selectable_geometry_change<T: MultiSelectableSelectionContainerDelegate>(
    this: Handle<T>,
    app: &mut App,
) {
    MultiSelectableSelectionContainerDelegate::handle_selectable_geometry_change(this, app);
}

/// The microtask that folds this frame's added selectables in, for Dart's `runScheduledTask`
/// closure.
fn scheduled_update_listener<T: MultiSelectableSelectionContainerDelegate>(
    this: Handle<T>,
) -> Listener {
    Listener::handle_method(this, run_scheduled_task_tearoff::<T>)
}

/// The frame callback that folds this frame's added selectables in.
fn scheduled_update_frame_callback<T: MultiSelectableSelectionContainerDelegate>(
    this: Handle<T>,
) -> FrameCallback {
    FrameCallback::handle_method(this, post_frame_run_scheduled_task_tearoff::<T>)
}

/// The body of [`scheduled_update_listener`].
fn run_scheduled_task_tearoff<T: MultiSelectableSelectionContainerDelegate>(
    this: Handle<T>,
    app: &mut App,
) {
    MultiSelectableSelectionContainerDelegate::run_scheduled_task(this, app);
}

/// The body of [`scheduled_update_frame_callback`].
fn post_frame_run_scheduled_task_tearoff<T: MultiSelectableSelectionContainerDelegate>(
    this: Handle<T>,
    app: &mut App,
    _time_stamp: std::time::Duration,
) {
    MultiSelectableSelectionContainerDelegate::run_scheduled_task(this, app);
}

// ---------------------------------------------------------------------------------------------
// MultiSelectableSelectionContainerDelegate

/// The fields of Dart's `MultiSelectableSelectionContainerDelegate`.
pub struct MultiSelectableSelectionContainerDelegateData {
    /// The `ChangeNotifier` side of the delegate; `notify_listeners` runs it.
    pub change_notifier: ChangeNotifierData,

    /// Gets the list of [`AnySelectable`]s this delegate is managing.
    pub selectables: Vec<AnySelectable>,

    /// The current selectable that contains the selection end edge, or `-1`.
    pub current_selection_end_index: i32,

    /// The current selectable that contains the selection start edge, or `-1`.
    pub current_selection_start_index: i32,

    start_handle_layer: Option<Ref<LayerLink>>,
    start_handle_layer_owner: Option<AnySelectable>,
    end_handle_layer: Option<Ref<LayerLink>>,
    end_handle_layer_owner: Option<AnySelectable>,

    /// Whether a selection event is being handled, so a selectable's own change is not folded
    /// back in before the event has finished.
    is_handling_selection_event: bool,

    /// Whether an update of the selectables is already scheduled for after this frame.
    scheduled_selectable_update: bool,

    /// Whether a selection is in progress, which keeps the selectables sorted for the gesture.
    selection_in_progress: bool,

    /// The selectables added since the last update, in the order they arrived.
    additions: Vec<AnySelectable>,

    /// Whether a granular or directional extension is in progress, which keeps the selection
    /// indices where the gesture put them, collapsed selectables included.
    extend_selection_in_progress: bool,

    selection_geometry: SelectionGeometry,
}

impl MultiSelectableSelectionContainerDelegateData {
    /// Creates the delegate's fields, with no selectables and no selection.
    pub fn new() -> MultiSelectableSelectionContainerDelegateData {
        MultiSelectableSelectionContainerDelegateData {
            change_notifier: ChangeNotifierData::new(),
            selectables: Vec::new(),
            current_selection_end_index: -1,
            current_selection_start_index: -1,
            start_handle_layer: None,
            start_handle_layer_owner: None,
            end_handle_layer: None,
            end_handle_layer_owner: None,
            is_handling_selection_event: false,
            scheduled_selectable_update: false,
            selection_in_progress: false,
            additions: Vec::new(),
            extend_selection_in_progress: false,
            selection_geometry: SelectionGeometry::new(SelectionStatus::None, false),
        }
    }
}

impl Default for MultiSelectableSelectionContainerDelegateData {
    fn default() -> Self {
        Self::new()
    }
}

/// The accessors [`MultiSelectableSelectionContainerDelegate`] asks for, for a struct whose bag
/// is the field `multi_selectable_selection_container_delegate`.
#[macro_export]
macro_rules! multi_selectable_selection_container_delegate_accessors {
    () => {
        fn multi_selectable_selection_container_delegate_data(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> &$crate::MultiSelectableSelectionContainerDelegateData {
            &app.get(self).multi_selectable_selection_container_delegate
        }

        fn multi_selectable_selection_container_delegate_data_mut(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> &mut $crate::MultiSelectableSelectionContainerDelegateData {
            &mut app
                .get_mut(self)
                .multi_selectable_selection_container_delegate
        }
    };
}

/// A delegate that handles events and updates for multiple [`AnySelectable`] children.
///
/// Updates are optimized by tracking which selectables reside on the edges of a selection.
/// A subclass implements [`ensure_child_updated`](Self::ensure_child_updated) to describe how a
/// selectable behaves when it is added to a selection.
pub trait MultiSelectableSelectionContainerDelegate:
    SelectionContainerDelegate + ChangeNotifier + Sized + 'static
{
    /// Dart's `MultiSelectableSelectionContainerDelegate` fields, held under the field
    /// `multi_selectable_selection_container_delegate`
    /// ([`multi_selectable_selection_container_delegate_accessors!`](crate::multi_selectable_selection_container_delegate_accessors)).
    fn multi_selectable_selection_container_delegate_data(
        self: Handle<Self>,
        app: &App,
    ) -> &MultiSelectableSelectionContainerDelegateData;

    /// See [`multi_selectable_selection_container_delegate_data`](Self::multi_selectable_selection_container_delegate_data).
    fn multi_selectable_selection_container_delegate_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut MultiSelectableSelectionContainerDelegateData;

    /// Notifies this delegate that layout of the container has changed.
    fn layout_did_change(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegate::update_selection_geometry(self, app);
    }

    /// Called when this delegate finishes updating the selectables.
    fn did_change_selectables(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegateBase::did_change_selectables(self, app);
    }

    /// The combined [`SelectionGeometry`] for child selectables.
    fn get_selection_geometry(self: Handle<Self>, app: &mut App) -> SelectionGeometry {
        MultiSelectableSelectionContainerDelegateBase::get_selection_geometry(self, app)
    }

    /// The compare function this delegate uses for the selection order of the selectables.
    ///
    /// Defaults to screen order.
    fn compare_order(
        self: Handle<Self>,
        app: &mut App,
        a: AnySelectable,
        b: AnySelectable,
    ) -> Ordering {
        MultiSelectableSelectionContainerDelegateBase::compare_order(self, app, a, b)
    }

    /// Selects all contents of all selectables.
    fn handle_select_all(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::handle_select_all(self, app, event)
    }

    /// Selects a word in a selectable at the location the event names.
    fn handle_select_word(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::handle_select_word(self, app, event)
    }

    /// Selects a paragraph in a selectable at the location the event names.
    fn handle_select_paragraph(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::handle_select_paragraph(self, app, event)
    }

    /// Removes the selection of all selectables this delegate manages.
    fn handle_clear_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::handle_clear_selection(self, app, event)
    }

    /// Extends the current selection by a [`waft_rendering::TextGranularity`].
    fn handle_granularly_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::handle_granularly_extend_selection(
            self, app, event,
        )
    }

    /// Extends the current selection in a [`SelectionExtendDirection`].
    fn handle_directionally_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::handle_directionally_extend_selection(
            self, app, event,
        )
    }

    /// Updates the selection edges.
    fn handle_selection_edge_update(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::handle_selection_edge_update(
            self, app, event,
        )
    }

    /// Discards the resources the delegate holds; Dart's `dispose`, as `ChangeNotifier.dispose`
    /// takes it.
    fn dispose(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegateBase::dispose(self, app);
    }

    /// Ensures the selectable child has received an up to date selection event.
    ///
    /// This is called when a new selectable is added to the delegate and its screen location
    /// falls into the previous selection. Subclasses describe how the newly added selectable is
    /// updated.
    fn ensure_child_updated(self: Handle<Self>, app: &mut App, selectable: AnySelectable);

    /// Dispatches a selection event to one selectable.
    ///
    /// Override this method when a subclass needs to generate additional events or treatments
    /// before sending the [`SelectionEvent`].
    fn dispatch_selection_event_to_child(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::dispatch_selection_event_to_child(
            self, app, selectable, event,
        )
    }

    /// Dart's `_scheduleSelectableUpdate`.
    fn schedule_selectable_update(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegateBase::schedule_selectable_update(self, app);
    }

    /// Dart's `_updateSelectables`.
    fn update_selectables(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegateBase::update_selectables(self, app);
    }

    /// Dart's `_updateSelectionGeometry`.
    fn update_selection_geometry(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegateBase::update_selection_geometry(self, app);
    }

    /// Dart's `_handleSelectableGeometryChange`.
    fn handle_selectable_geometry_change(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegateBase::handle_selectable_geometry_change(self, app);
    }

    /// Dart's `runScheduledTask`, as the microtask and the frame callback both run it.
    fn run_scheduled_task(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegateBase::run_scheduled_task(self, app);
    }
}

/// The bodies of Dart's `MultiSelectableSelectionContainerDelegate`, over
/// [`MultiSelectableSelectionContainerDelegateData`].
///
/// Every [`MultiSelectableSelectionContainerDelegate`] implements this, so an override that runs
/// the inherited body calls it trait-qualified,
/// `MultiSelectableSelectionContainerDelegateBase::remove(self, app, selectable)`, where Dart
/// writes `super.remove(selectable)`.
pub trait MultiSelectableSelectionContainerDelegateBase:
    MultiSelectableSelectionContainerDelegate
{
    /// Dart's `MultiSelectableSelectionContainerDelegate.add`: queue the selectable until after
    /// this frame, when the container has laid it out.
    fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        debug_assert!(!data.selectables.contains(&selectable));
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        set_add(&mut data.additions, selectable);
        MultiSelectableSelectionContainerDelegate::schedule_selectable_update(self, app);
    }

    /// Dart's `MultiSelectableSelectionContainerDelegate.remove`.
    fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        if set_remove(
            &mut self
                .multi_selectable_selection_container_delegate_data_mut(app)
                .additions,
            selectable,
        ) {
            // The same selectable was added in the same frame and is not yet incorporated into
            // the selectables; removing it needs no selection geometry update.
            return;
        }
        self.remove_selectable(app, selectable);
        MultiSelectableSelectionContainerDelegate::schedule_selectable_update(self, app);
    }

    /// Dart's `dispatchSelectionEventToChild`: send the event to the selectable.
    fn dispatch_selection_event_to_child(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        event: &SelectionEvent,
    ) -> SelectionResult {
        selectable.dispatch_selection_event(app, event)
    }

    /// Dart's `_scheduleSelectableUpdate`: one update per frame, on the microtask queue when the
    /// frame's own callbacks are running so a nested container is updated in the same frame.
    fn schedule_selectable_update(self: Handle<Self>, app: &mut App) {
        if self
            .multi_selectable_selection_container_delegate_data(app)
            .scheduled_selectable_update
        {
            return;
        }
        self.multi_selectable_selection_container_delegate_data_mut(app)
            .scheduled_selectable_update = true;
        if SchedulerBinding::scheduler_phase(app) == SchedulerPhase::PostFrameCallbacks {
            app.schedule_microtask(scheduled_update_listener(self));
        } else {
            SchedulerBinding::add_post_frame_callback(
                app,
                scheduled_update_frame_callback(self),
                None,
            );
        }
    }

    /// Dart's `runScheduledTask`.
    fn run_scheduled_task(self: Handle<Self>, app: &mut App) {
        if !self
            .multi_selectable_selection_container_delegate_data(app)
            .scheduled_selectable_update
        {
            return;
        }
        self.multi_selectable_selection_container_delegate_data_mut(app)
            .scheduled_selectable_update = false;
        MultiSelectableSelectionContainerDelegate::update_selectables(self, app);
    }

    /// Dart's `_updateSelectables`: fold the selectables added since the last frame in, then let
    /// the geometry catch up with them.
    ///
    /// Dart's first step, removing off-screen selectables, is the registrant's own: a selectable
    /// with no content removes itself from its registrar.
    fn update_selectables(self: Handle<Self>, app: &mut App) {
        if !self
            .multi_selectable_selection_container_delegate_data(app)
            .additions
            .is_empty()
        {
            self.flush_additions(app);
        }
        MultiSelectableSelectionContainerDelegate::did_change_selectables(self, app);
    }

    /// Dart's `_flushAdditions`: merge the new selectables into screen order, keeping the
    /// selection indices on the selectables they named.
    fn flush_additions(self: Handle<Self>, app: &mut App) {
        let mut merging_selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .additions
            .clone();
        merging_selectables.sort_by(|a, b| {
            MultiSelectableSelectionContainerDelegate::compare_order(self, app, *a, *b)
        });
        let existing_selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let current_selection_start_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_start_index;
        let current_selection_end_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_end_index;
        let mut selectables = Vec::new();
        let mut selection_start_index = current_selection_start_index;
        let mut selection_end_index = current_selection_end_index;
        let mut merging_index = 0;
        let mut existing_index = 0;
        // Merge two sorted lists.
        while merging_index < merging_selectables.len()
            || existing_index < existing_selectables.len()
        {
            if merging_index >= merging_selectables.len()
                || (existing_index < existing_selectables.len()
                    && MultiSelectableSelectionContainerDelegate::compare_order(
                        self,
                        app,
                        existing_selectables[existing_index],
                        merging_selectables[merging_index],
                    ) == Ordering::Less)
            {
                if existing_index as i32 == current_selection_start_index {
                    selection_start_index = selectables.len() as i32;
                }
                if existing_index as i32 == current_selection_end_index {
                    selection_end_index = selectables.len() as i32;
                }
                selectables.push(existing_selectables[existing_index]);
                existing_index += 1;
                continue;
            }

            // If the merging selectable falls in the selection range, its selection needs to be
            // updated.
            let merging_selectable = merging_selectables[merging_index];
            if (existing_index as i32)
                < current_selection_start_index.max(current_selection_end_index)
                && (existing_index as i32)
                    > current_selection_start_index.min(current_selection_end_index)
            {
                MultiSelectableSelectionContainerDelegate::ensure_child_updated(
                    self,
                    app,
                    merging_selectable,
                );
            }
            merging_selectable.add_listener(app, selectable_geometry_change_listener(self));
            selectables.push(merging_selectable);
            merging_index += 1;
        }
        debug_assert_eq!(merging_index, merging_selectables.len());
        debug_assert_eq!(existing_index, existing_selectables.len());
        debug_assert_eq!(
            selectables.len(),
            existing_index + merging_index,
            "the merged list holds every selectable once"
        );
        debug_assert!(
            selection_start_index >= -1 || (selection_start_index as usize) < selectables.len()
        );
        debug_assert!(
            selection_end_index >= -1 || (selection_end_index as usize) < selectables.len()
        );
        // Selection indices should not be set to -1 unless they originally were.
        debug_assert_eq!(
            current_selection_start_index == -1,
            selection_start_index == -1
        );
        debug_assert_eq!(current_selection_end_index == -1, selection_end_index == -1);
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        data.selectables = selectables;
        data.current_selection_end_index = selection_end_index;
        data.current_selection_start_index = selection_start_index;
        data.additions = Vec::new();
    }

    /// Dart's `_removeSelectable`.
    fn remove_selectable(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        debug_assert!(
            data.selectables.contains(&selectable),
            "The selectable is not in this registrar."
        );
        let index = data
            .selectables
            .iter()
            .position(|held| *held == selectable)
            .expect("checked just now");
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        data.selectables.remove(index);
        if index as i32 <= data.current_selection_end_index {
            data.current_selection_end_index -= 1;
        }
        if index as i32 <= data.current_selection_start_index {
            data.current_selection_start_index -= 1;
        }
        selectable.remove_listener(app, &selectable_geometry_change_listener(self));
    }

    /// Dart's `didChangeSelectables`.
    fn did_change_selectables(self: Handle<Self>, app: &mut App) {
        MultiSelectableSelectionContainerDelegate::update_selection_geometry(self, app);
    }

    /// Dart's `value`: the combined geometry of the selectables.
    fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
        self.multi_selectable_selection_container_delegate_data(app)
            .selection_geometry
            .clone()
    }

    /// Dart's `_updateSelectionGeometry`: fold the selectables' geometries into `value`, telling
    /// listeners when it changed, and hand the handles to the selectables on the edges.
    fn update_selection_geometry(self: Handle<Self>, app: &mut App) {
        let new_value =
            MultiSelectableSelectionContainerDelegate::get_selection_geometry(self, app);
        if SelectionHandler::value(self, app) != new_value {
            self.multi_selectable_selection_container_delegate_data_mut(app)
                .selection_geometry = new_value;
            self.notify_listeners(app);
        }
        self.update_handle_layers_and_owners(app);
    }

    /// Dart's `_handleSelectableGeometryChange`.
    fn handle_selectable_geometry_change(self: Handle<Self>, app: &mut App) {
        // Geometries of selectable children may change several times while a selection event is
        // handled; this delegate's own geometry is updated once the event has finished.
        if self
            .multi_selectable_selection_container_delegate_data(app)
            .is_handling_selection_event
        {
            return;
        }
        MultiSelectableSelectionContainerDelegate::update_selection_geometry(self, app);
    }

    /// Dart's `getSelectionGeometry`: one geometry covering the selectables from the start edge
    /// to the end edge, in the container's coordinates.
    fn get_selection_geometry(self: Handle<Self>, app: &mut App) -> SelectionGeometry {
        {
            let data = self.multi_selectable_selection_container_delegate_data(app);
            if data.current_selection_end_index == -1
                || data.current_selection_start_index == -1
                || data.selectables.is_empty()
            {
                // There is no valid selection.
                return SelectionGeometry::new(SelectionStatus::None, !data.selectables.is_empty());
            }
            if !data.extend_selection_in_progress {
                let start = self.adjust_selection_index_based_on_selection_geometry(
                    app,
                    data.current_selection_start_index,
                    data.current_selection_end_index,
                );
                let end = self.adjust_selection_index_based_on_selection_geometry(
                    app,
                    data.current_selection_end_index,
                    start,
                );
                let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                data.current_selection_start_index = start;
                data.current_selection_end_index = end;
            }
        }
        let start_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_start_index;
        let end_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_end_index;
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let forward_selection = end_index >= start_index;

        // Need to find a non-null start selection point.
        let mut start_geometry = selectables[start_index as usize].value(app);
        let mut start_index_walker = start_index;
        while start_index_walker != end_index && start_geometry.start_selection_point.is_none() {
            start_index_walker += if forward_selection { 1 } else { -1 };
            start_geometry = selectables[start_index_walker as usize].value(app);
        }
        let start_point = start_geometry.start_selection_point.and_then(|point| {
            let transform = self.get_transform_from(app, selectables[start_index_walker as usize]);
            let start = transform_point(&transform, point.local_position);
            // It can be NaN if it is detached or off-screen.
            start
                .is_finite()
                .then(|| SelectionPoint::new(start, point.line_height, point.handle_type))
        });

        // Need to find a non-null end selection point.
        let mut end_geometry = selectables[end_index as usize].value(app);
        let mut end_index_walker = end_index;
        while end_index_walker != start_index && end_geometry.end_selection_point.is_none() {
            end_index_walker += if forward_selection { -1 } else { 1 };
            end_geometry = selectables[end_index_walker as usize].value(app);
        }
        let end_point = end_geometry.end_selection_point.and_then(|point| {
            let transform = self.get_transform_from(app, selectables[end_index_walker as usize]);
            let end = transform_point(&transform, point.local_position);
            // It can be NaN if it is detached or off-screen.
            end.is_finite()
                .then(|| SelectionPoint::new(end, point.line_height, point.handle_type))
        });

        // Need to collect selection rects from the selectables from the start index to the end
        // index; a reverse selection gives none, as Dart's ascending loop does.
        let mut selection_rects = Vec::new();
        let drawable_area = if self.has_size(app) {
            let size = self.container_size(app);
            Some(Rect::from_ltrb(0.0, 0.0, size.width(), size.height()))
        } else {
            None
        };
        for index in start_index..=end_index {
            let selectable = selectables[index as usize];
            let transform = self.get_transform_from(app, selectable);
            for selection_rect in selectable.value(app).selection_rects {
                let local_rect = transform_rect(&transform, selection_rect);
                let within_drawable_area =
                    drawable_area.map_or(local_rect, |area| area.intersect(local_rect));
                if within_drawable_area.is_finite() && !within_drawable_area.is_empty() {
                    selection_rects.push(within_drawable_area);
                }
            }
        }

        SelectionGeometry {
            start_selection_point: start_point,
            end_selection_point: end_point,
            selection_rects,
            status: if start_geometry != end_geometry {
                SelectionStatus::Uncollapsed
            } else {
                start_geometry.status
            },
            // Would have at least one selectable child.
            has_content: true,
        }
    }

    /// Dart's `_adjustSelectionIndexBasedOnSelectionGeometry`: the current edge index may point
    /// at a selectable whose own selection is collapsed at its edge, so move it toward the
    /// opposite edge to one with a selection.
    fn adjust_selection_index_based_on_selection_geometry(
        self: Handle<Self>,
        app: &App,
        current_index: i32,
        toward_index: i32,
    ) -> i32 {
        let forward = toward_index > current_index;
        let mut index = current_index;
        while index != toward_index {
            let selectable = self
                .multi_selectable_selection_container_delegate_data(app)
                .selectables[index as usize];
            if selectable.value(app).status == SelectionStatus::Uncollapsed {
                break;
            }
            index += if forward { 1 } else { -1 };
        }
        index
    }

    /// Dart's `compareOrder`: the default is screen order.
    fn compare_order(
        self: Handle<Self>,
        app: &mut App,
        a: AnySelectable,
        b: AnySelectable,
    ) -> Ordering {
        let rect_a = transform_rect(
            &a.get_transform_to(app, None),
            self.get_bounding_box(app, a),
        );
        let rect_b = transform_rect(
            &b.get_transform_to(app, None),
            self.get_bounding_box(app, b),
        );
        let result = compare_vertically(rect_a, rect_b);
        if result != Ordering::Equal {
            return result;
        }
        compare_horizontally(rect_a, rect_b)
    }

    /// Dart's `_getBoundingBox`: every bounding box of the selectable, unioned.
    fn get_bounding_box(self: Handle<Self>, app: &mut App, selectable: AnySelectable) -> Rect {
        let mut boxes = selectable.bounding_boxes(app).into_iter();
        let mut result = boxes.next().expect("a selectable has a bounding box");
        for other in boxes {
            result = result.expand_to_include(other);
        }
        result
    }

    /// Dart's `pushHandleLayers`.
    fn push_handle_layers(
        self: Handle<Self>,
        app: &mut App,
        start_handle: Option<Ref<LayerLink>>,
        end_handle: Option<Ref<LayerLink>>,
    ) {
        {
            let data = self.multi_selectable_selection_container_delegate_data(app);
            if data.start_handle_layer == start_handle && data.end_handle_layer == end_handle {
                return;
            }
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        data.start_handle_layer = start_handle;
        data.end_handle_layer = end_handle;
        self.update_handle_layers_and_owners(app);
    }

    /// Dart's `_updateHandleLayersAndOwners`: hand both handle layers to the selectables that
    /// hold the selection edges, hiding a handle outside the container's drawable area.
    fn update_handle_layers_and_owners(self: Handle<Self>, app: &mut App) {
        let start_layer = self
            .multi_selectable_selection_container_delegate_data(app)
            .start_handle_layer
            .clone();
        let end_layer = self
            .multi_selectable_selection_container_delegate_data(app)
            .end_handle_layer
            .clone();
        let mut effective_start_handle = start_layer.clone();
        let mut effective_end_handle = end_layer.clone();
        if effective_start_handle.is_some() || effective_end_handle.is_some() {
            let drawable_area = if self.has_size(app) {
                let size = self.container_size(app);
                Some(
                    Rect::from_ltrb(0.0, 0.0, size.width(), size.height())
                        .inflate(K_SELECTION_HANDLE_DRAWABLE_AREA_PADDING),
                )
            } else {
                None
            };
            let value = SelectionHandler::value(self, app);
            let hide_start_handle = value.start_selection_point.is_none()
                || drawable_area.is_none()
                || !drawable_area.expect("checked just now").contains(
                    value
                        .start_selection_point
                        .expect("checked just now")
                        .local_position,
                );
            let hide_end_handle = value.end_selection_point.is_none()
                || drawable_area.is_none()
                || !drawable_area.expect("checked just now").contains(
                    value
                        .end_selection_point
                        .expect("checked just now")
                        .local_position,
                );
            effective_start_handle = if hide_start_handle { None } else { start_layer };
            effective_end_handle = if hide_end_handle { None } else { end_layer };
        }
        let start_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_start_index;
        let end_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_end_index;
        if start_index == -1 || end_index == -1 {
            // No valid selection.
            let start_owner = self
                .multi_selectable_selection_container_delegate_data_mut(app)
                .start_handle_layer_owner
                .take();
            if let Some(owner) = start_owner {
                owner.push_handle_layers(app, None, None);
            }
            let end_owner = self
                .multi_selectable_selection_container_delegate_data_mut(app)
                .end_handle_layer_owner
                .take();
            if let Some(owner) = end_owner {
                owner.push_handle_layers(app, None, None);
            }
            return;
        }
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let start_selectable = selectables[start_index as usize];
        let end_selectable = selectables[end_index as usize];
        if self
            .multi_selectable_selection_container_delegate_data(app)
            .start_handle_layer_owner
            != Some(start_selectable)
            && let Some(owner) = self
                .multi_selectable_selection_container_delegate_data(app)
                .start_handle_layer_owner
        {
            owner.push_handle_layers(app, None, None);
        }
        if self
            .multi_selectable_selection_container_delegate_data(app)
            .end_handle_layer_owner
            != Some(end_selectable)
            && let Some(owner) = self
                .multi_selectable_selection_container_delegate_data(app)
                .end_handle_layer_owner
        {
            owner.push_handle_layers(app, None, None);
        }
        {
            let data = self.multi_selectable_selection_container_delegate_data_mut(app);
            data.start_handle_layer_owner = Some(start_selectable);
            if start_index == end_index {
                // The selection edges are on the same selectable.
                data.end_handle_layer_owner = Some(start_selectable);
            } else {
                data.end_handle_layer_owner = Some(end_selectable);
            }
        }
        if start_index == end_index {
            start_selectable.push_handle_layers(app, effective_start_handle, effective_end_handle);
            return;
        }
        start_selectable.push_handle_layers(app, effective_start_handle, None);
        end_selectable.push_handle_layers(app, None, effective_end_handle);
    }

    /// Dart's `getSelectedContent`: the selected contents of all selectables, concatenated.
    fn selected_content(self: Handle<Self>, app: &App) -> Option<SelectedContent> {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let selections: Vec<SelectedContent> = selectables
            .iter()
            .filter_map(|selectable| selectable.selected_content(app))
            .collect();
        if selections.is_empty() {
            return None;
        }
        let mut plain_text = String::new();
        for selection in selections {
            plain_text.push_str(&selection.plain_text);
        }
        Some(SelectedContent::new(plain_text))
    }

    /// Dart's `contentLength`: the sum of every selectable's content length.
    fn content_length(self: Handle<Self>, app: &App) -> usize {
        self.multi_selectable_selection_container_delegate_data(app)
            .selectables
            .iter()
            .map(|selectable| selectable.content_length(app))
            .sum()
    }

    /// Dart's `getSelection`: the selection range across the selectables, in content offsets.
    fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let selections: Vec<(usize, Option<SelectedContentRange>)> = selectables
            .iter()
            .map(|selectable| (selectable.content_length(app), selectable.selection(app)))
            .collect();
        self.calculate_local_range(app, &selections)
    }

    /// Dart's `_calculateLocalRange`: the range the selectables' own ranges make up, offset by
    /// the content before them.
    fn calculate_local_range(
        self: Handle<Self>,
        app: &App,
        selections: &[(usize, Option<SelectedContentRange>)],
    ) -> Option<SelectedContentRange> {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        if data.current_selection_start_index == -1 || data.current_selection_end_index == -1 {
            return None;
        }
        let mut start_offset = 0i32;
        let mut end_offset = 0i32;
        let mut found_start = false;
        let mut forward_selection =
            data.current_selection_end_index >= data.current_selection_start_index;
        if data.current_selection_end_index == data.current_selection_start_index {
            // Determining the selection direction is inaccurate when the two edges share a
            // selectable; the range within it is the source of truth.
            let range_at_selectable_in_selection = data.selectables
                [data.current_selection_start_index as usize]
                .selection(app)
                .expect("the selectable holding both edges has a selection");
            forward_selection = range_at_selectable_in_selection.end_offset
                >= range_at_selectable_in_selection.start_offset;
        }
        for (content_length, range) in selections {
            let Some(range) = range else {
                if found_start {
                    return Some(SelectedContentRange::new(
                        if forward_selection {
                            start_offset
                        } else {
                            end_offset
                        },
                        if forward_selection {
                            end_offset
                        } else {
                            start_offset
                        },
                    ));
                }
                start_offset += *content_length as i32;
                end_offset = start_offset;
                continue;
            };
            let selection_start_normalized = range.start_offset.min(range.end_offset);
            let selection_end_normalized = range.start_offset.max(range.end_offset);
            if !found_start {
                start_offset += selection_start_normalized;
                end_offset =
                    start_offset + (selection_end_normalized - selection_start_normalized).abs();
                found_start = true;
            } else {
                end_offset += (selection_end_normalized - selection_start_normalized).abs();
            }
        }
        debug_assert!(
            found_start,
            "The start of the selection has not been found despite this selection delegate \
             having an existing currentSelectionStartIndex and currentSelectionEndIndex."
        );
        Some(SelectedContentRange::new(
            if forward_selection {
                start_offset
            } else {
                end_offset
            },
            if forward_selection {
                end_offset
            } else {
                start_offset
            },
        ))
    }

    /// Dart's `_flushInactiveSelections`: clear the selection of every selectable outside the
    /// edges, or of every selectable but the one edge that exists.
    fn flush_inactive_selections(self: Handle<Self>, app: &mut App) {
        let start_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_start_index;
        let end_index = self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_end_index;
        if start_index == -1 && end_index == -1 {
            return;
        }
        if start_index == -1 || end_index == -1 {
            let skip_index = if start_index == -1 {
                end_index
            } else {
                start_index
            };
            self.clear_selectables(app, Some(skip_index));
            return;
        }
        let skip_start = start_index.min(end_index);
        let skip_end = start_index.max(end_index);
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        for (index, selectable) in selectables.iter().enumerate() {
            if (index as i32) >= skip_start && (index as i32) <= skip_end {
                continue;
            }
            MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                self,
                app,
                *selectable,
                &SelectionEvent::Clear,
            );
        }
    }

    /// Dart's `handleSelectAll`: select every selectable.
    fn handle_select_all(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        for selectable in &selectables {
            MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                self,
                app,
                *selectable,
                event,
            );
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        data.current_selection_start_index = 0;
        data.current_selection_end_index = selectables.len() as i32 - 1;
        SelectionResult::None
    }

    /// Dart's `_clearSelectables`: clear the selection of every selectable but `skip_index`.
    fn clear_selectables(self: Handle<Self>, app: &mut App, skip_index: Option<i32>) {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        for (index, selectable) in selectables.iter().enumerate() {
            if Some(index as i32) == skip_index {
                continue;
            }
            MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                self,
                app,
                *selectable,
                &SelectionEvent::Clear,
            );
        }
    }

    /// Dart's `_handleSelectBoundary`: the word or paragraph at the event's position, in the
    /// selectable under it, or the nearest one when no bounding box contains it.
    fn handle_select_boundary(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        debug_assert!(
            matches!(
                event,
                SelectionEvent::SelectWord { .. } | SelectionEvent::SelectParagraph { .. }
            ),
            "This method should only be given selection events that select text boundaries."
        );
        let effective_global_position = match event {
            SelectionEvent::SelectWord { global_position } => *global_position,
            SelectionEvent::SelectParagraph {
                global_position, ..
            } => *global_position,
            _ => panic!("Unsupported selection event"),
        };
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let mut last_selection_result: Option<SelectionResult> = None;
        // For tracking the nearest index if no bounding box contains the position.
        let mut min_distance_squared = f64::INFINITY;
        let mut nearest_index = 0usize;
        for index in 0..selectables.len() {
            let mut global_rects_contain_position = false;
            let transform = selectables[index].get_transform_to(app, None);
            for rect in selectables[index].bounding_boxes(app) {
                let global_rect = transform_rect(&transform, rect);
                if global_rect.contains(effective_global_position) {
                    global_rects_contain_position = true;
                    break;
                }
                let dx = effective_global_position.dx()
                    - clamp_double(
                        effective_global_position.dx(),
                        global_rect.left,
                        global_rect.right,
                    );
                let dy = effective_global_position.dy()
                    - clamp_double(
                        effective_global_position.dy(),
                        global_rect.top,
                        global_rect.bottom,
                    );
                let distance_squared = dx * dx + dy * dy;
                if distance_squared < min_distance_squared {
                    min_distance_squared = distance_squared;
                    nearest_index = index;
                }
            }
            if global_rects_contain_position {
                let existing_geometry = selectables[index].value(app);
                last_selection_result = Some(
                    MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                        self,
                        app,
                        selectables[index],
                        event,
                    ),
                );
                let result = last_selection_result.expect("assigned just now");
                if index == selectables.len() - 1 && result == SelectionResult::Next {
                    return SelectionResult::Next;
                }
                if result == SelectionResult::Next {
                    continue;
                }
                if index == 0 && result == SelectionResult::Previous {
                    return SelectionResult::Previous;
                }
                if selectables[index].value(app) != existing_geometry {
                    // The geometry has changed as a result of selecting a boundary; clear the
                    // other selectables to keep the selection in sync.
                    self.clear_selectables(app, Some(index as i32));
                    let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                    data.current_selection_start_index = index as i32;
                    data.current_selection_end_index = index as i32;
                }
                return SelectionResult::End;
            } else if last_selection_result == Some(SelectionResult::Next) {
                let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                data.current_selection_start_index = index as i32 - 1;
                data.current_selection_end_index = index as i32 - 1;
                return SelectionResult::End;
            }
        }
        debug_assert!(last_selection_result.is_none());
        // No selectable's bounding box contained the position. Clamp to the nearest selectable
        // so that the boundary selection event always produces a valid selection.
        if !selectables.is_empty() {
            let existing_geometry = selectables[nearest_index].value(app);
            MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                self,
                app,
                selectables[nearest_index],
                event,
            );
            if selectables[nearest_index].value(app) != existing_geometry {
                // The geometry has changed as a result of selecting a boundary; clear the other
                // selectables to keep the selection in sync.
                self.clear_selectables(app, Some(nearest_index as i32));
                let data = self.multi_selectable_selection_container_delegate_data_mut(app);
                data.current_selection_start_index = nearest_index as i32;
                data.current_selection_end_index = nearest_index as i32;
            }
        }
        SelectionResult::End
    }

    /// Dart's `handleSelectWord`.
    fn handle_select_word(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        self.handle_select_boundary(app, event)
    }

    /// Dart's `handleSelectParagraph`.
    fn handle_select_paragraph(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        self.handle_select_boundary(app, event)
    }

    /// Dart's `handleClearSelection`: clear the selection of every selectable.
    fn handle_clear_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        for selectable in &selectables {
            MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                self,
                app,
                *selectable,
                event,
            );
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        data.current_selection_end_index = -1;
        data.current_selection_start_index = -1;
        SelectionResult::None
    }

    /// Dart's `handleGranularlyExtendSelection`.
    fn handle_granularly_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let SelectionEvent::GranularlyExtendSelection {
            forward, is_end, ..
        } = event
        else {
            panic!("a granular extension event updates the selection");
        };
        let (forward, is_end) = (*forward, *is_end);
        self.assert_selection_indices_agree(app);
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        if self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_start_index
            == -1
        {
            let index = if forward {
                0
            } else {
                selectables.len() as i32 - 1
            };
            let data = self.multi_selectable_selection_container_delegate_data_mut(app);
            data.current_selection_start_index = index;
            data.current_selection_end_index = index;
        }
        let mut target_index = {
            let data = self.multi_selectable_selection_container_delegate_data(app);
            if is_end {
                data.current_selection_end_index
            } else {
                data.current_selection_start_index
            }
        };
        let mut result =
            MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                self,
                app,
                selectables[target_index as usize],
                event,
            );
        if forward {
            debug_assert_ne!(result, SelectionResult::Previous);
            while (target_index as usize) < selectables.len() - 1 && result == SelectionResult::Next
            {
                target_index += 1;
                result =
                    MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                        self,
                        app,
                        selectables[target_index as usize],
                        event,
                    );
                debug_assert_ne!(result, SelectionResult::Previous);
            }
        } else {
            debug_assert_ne!(result, SelectionResult::Next);
            while target_index > 0 && result == SelectionResult::Previous {
                target_index -= 1;
                result =
                    MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                        self,
                        app,
                        selectables[target_index as usize],
                        event,
                    );
                debug_assert_ne!(result, SelectionResult::Next);
            }
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        if is_end {
            data.current_selection_end_index = target_index;
        } else {
            data.current_selection_start_index = target_index;
        }
        result
    }

    /// Dart's `handleDirectionallyExtendSelection`.
    fn handle_directionally_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let SelectionEvent::DirectionallyExtendSelection {
            is_end, direction, ..
        } = event
        else {
            panic!("a directional extension event updates the selection");
        };
        let (is_end, direction) = (*is_end, *direction);
        self.assert_selection_indices_agree(app);
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        if self
            .multi_selectable_selection_container_delegate_data(app)
            .current_selection_start_index
            == -1
        {
            let index = match direction {
                SelectionExtendDirection::PreviousLine | SelectionExtendDirection::Backward => {
                    selectables.len() as i32 - 1
                }
                SelectionExtendDirection::NextLine | SelectionExtendDirection::Forward => 0,
            };
            let data = self.multi_selectable_selection_container_delegate_data_mut(app);
            data.current_selection_start_index = index;
            data.current_selection_end_index = index;
        }
        let mut target_index = {
            let data = self.multi_selectable_selection_container_delegate_data(app);
            if is_end {
                data.current_selection_end_index
            } else {
                data.current_selection_start_index
            }
        };
        let mut result =
            MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                self,
                app,
                selectables[target_index as usize],
                event,
            );
        match direction {
            SelectionExtendDirection::PreviousLine => {
                debug_assert!(
                    result == SelectionResult::End || result == SelectionResult::Previous
                );
                if result == SelectionResult::Previous && target_index > 0 {
                    target_index -= 1;
                    result = MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(self,
                        app,
                        selectables[target_index as usize],
                        &event.copy_with(None, None, Some(SelectionExtendDirection::Backward)),
                    );
                    debug_assert_eq!(result, SelectionResult::End);
                }
            }
            SelectionExtendDirection::NextLine => {
                debug_assert!(result == SelectionResult::End || result == SelectionResult::Next);
                if result == SelectionResult::Next
                    && (target_index as usize) < selectables.len() - 1
                {
                    target_index += 1;
                    result = MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(self,
                        app,
                        selectables[target_index as usize],
                        &event.copy_with(None, None, Some(SelectionExtendDirection::Forward)),
                    );
                    debug_assert_eq!(result, SelectionResult::End);
                }
            }
            SelectionExtendDirection::Forward | SelectionExtendDirection::Backward => {
                debug_assert_eq!(result, SelectionResult::End);
            }
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        if is_end {
            data.current_selection_end_index = target_index;
        } else {
            data.current_selection_start_index = target_index;
        }
        result
    }

    /// Dart's `assert((currentSelectionStartIndex == -1) == (currentSelectionEndIndex == -1))`,
    /// as a debug check.
    fn assert_selection_indices_agree(self: Handle<Self>, app: &App) {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        debug_assert_eq!(
            data.current_selection_start_index == -1,
            data.current_selection_end_index == -1,
            "both selection edges exist, or neither does"
        );
    }

    /// Dart's `handleSelectionEdgeUpdate`: start the selection when the edge is new, and move it
    /// across the selectables otherwise.
    fn handle_selection_edge_update(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let is_end = event.ty() == SelectionEventType::EndEdgeUpdate;
        let edge_index = {
            let data = self.multi_selectable_selection_container_delegate_data(app);
            if is_end {
                data.current_selection_end_index
            } else {
                data.current_selection_start_index
            }
        };
        if edge_index == -1 {
            self.init_selection(app, event, is_end)
        } else {
            self.adjust_selection(app, event, is_end)
        }
    }

    /// Dart's `_initSelection`: find the selectable that contains the new edge, sweeping from
    /// the opposite edge when it exists.
    fn init_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
        is_end: bool,
    ) -> SelectionResult {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let opposite_edge_index = {
            let data = self.multi_selectable_selection_container_delegate_data(app);
            if is_end {
                data.current_selection_start_index
            } else {
                data.current_selection_end_index
            }
        };
        let mut new_index: i32 = -1;
        let mut has_found_edge_index = false;
        let mut result: Option<SelectionResult> = None;
        let mut forward: Option<bool> = None;
        let mut index = opposite_edge_index.max(0);
        while index >= 0 && (index as usize) < selectables.len() {
            let child = selectables[index as usize];
            let child_result =
                MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                    self, app, child, event,
                );
            match child_result {
                SelectionResult::Next => {
                    if forward == Some(false) {
                        has_found_edge_index = true;
                        result = Some(SelectionResult::End);
                    } else {
                        forward = Some(true);
                        new_index = index;
                    }
                }
                SelectionResult::None => new_index = index,
                SelectionResult::End => {
                    new_index = index;
                    result = Some(SelectionResult::End);
                    has_found_edge_index = true;
                }
                SelectionResult::Previous => {
                    if index == 0 {
                        has_found_edge_index = true;
                        new_index = 0;
                        result = Some(SelectionResult::Previous);
                    } else if forward == Some(true) {
                        has_found_edge_index = true;
                        result = Some(SelectionResult::End);
                    } else {
                        forward = Some(false);
                        new_index = index;
                    }
                }
                SelectionResult::Pending => {
                    new_index = index;
                    result = Some(SelectionResult::Pending);
                    has_found_edge_index = true;
                }
            }
            if has_found_edge_index {
                break;
            }
            index += if forward.unwrap_or(true) { 1 } else { -1 };
        }

        if new_index == -1 {
            debug_assert!(selectables.is_empty());
            return SelectionResult::None;
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        if is_end {
            data.current_selection_end_index = new_index;
        } else {
            data.current_selection_start_index = new_index;
        }
        self.flush_inactive_selections(app);
        // The result can only be absent if the loop went through the whole list without any
        // selectable answering end or previous; the caller then finds the next selectable.
        result.unwrap_or(SelectionResult::Next)
    }

    /// Dart's `_adjustSelection`: move an edge that already has a selectable, searching from the
    /// opposite edge when the current one is out of the viewport.
    fn adjust_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
        is_end: bool,
    ) -> SelectionResult {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let value = self
            .multi_selectable_selection_container_delegate_data(app)
            .selection_geometry
            .clone();
        let is_current_edge_within_viewport = if is_end {
            value.end_selection_point.is_some()
        } else {
            value.start_selection_point.is_some()
        };
        let is_opposite_edge_within_viewport = if is_end {
            value.start_selection_point.is_some()
        } else {
            value.end_selection_point.is_some()
        };
        let data = self.multi_selectable_selection_container_delegate_data(app);
        let mut new_index: i32 = if is_end {
            if is_current_edge_within_viewport {
                data.current_selection_end_index
            } else if is_opposite_edge_within_viewport {
                data.current_selection_start_index
            } else {
                0
            }
        } else if is_current_edge_within_viewport {
            data.current_selection_start_index
        } else if is_opposite_edge_within_viewport {
            data.current_selection_end_index
        } else {
            0
        };
        debug_assert!(new_index < selectables.len() as i32 && new_index >= 0);
        let mut final_result: Option<SelectionResult> = None;
        let mut forward: Option<bool> = None;
        // This loop sends the event to one of the following to determine the direction of the
        // search: the current edge's selectable if it is in the viewport, the opposite edge's
        // when it is not, and the first one when neither is.
        while (new_index as usize) < selectables.len() && new_index >= 0 && final_result.is_none() {
            let current_selectable_result =
                MultiSelectableSelectionContainerDelegate::dispatch_selection_event_to_child(
                    self,
                    app,
                    selectables[new_index as usize],
                    event,
                );
            match current_selectable_result {
                SelectionResult::End | SelectionResult::Pending | SelectionResult::None => {
                    final_result = Some(current_selectable_result);
                }
                SelectionResult::Next => {
                    if forward == Some(false) {
                        new_index += 1;
                        final_result = Some(SelectionResult::End);
                    } else if new_index as usize == selectables.len() - 1 {
                        final_result = Some(current_selectable_result);
                    } else {
                        forward = Some(true);
                        new_index += 1;
                    }
                }
                SelectionResult::Previous => {
                    if forward == Some(true) {
                        new_index -= 1;
                        final_result = Some(SelectionResult::End);
                    } else if new_index == 0 {
                        final_result = Some(current_selectable_result);
                    } else {
                        forward = Some(false);
                        new_index -= 1;
                    }
                }
            }
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        if is_end {
            data.current_selection_end_index = new_index;
        } else {
            data.current_selection_start_index = new_index;
        }
        self.flush_inactive_selections(app);
        final_result.expect("the loop answered a result")
    }

    /// Dart's `dispatchSelectionEvent`: hand the event to the handler its type names, keeping
    /// the selectables in screen order while a selection is in progress.
    fn dispatch_selection_event(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let selection_will_be_in_progress = event.ty() != SelectionEventType::Clear;
        let was_in_progress = self
            .multi_selectable_selection_container_delegate_data(app)
            .selection_in_progress;
        if !was_in_progress && selection_will_be_in_progress {
            // Sort the selectables every time a selection starts.
            let mut selectables = self
                .multi_selectable_selection_container_delegate_data(app)
                .selectables
                .clone();
            selectables.sort_by(|a, b| {
                MultiSelectableSelectionContainerDelegate::compare_order(self, app, *a, *b)
            });
            self.multi_selectable_selection_container_delegate_data_mut(app)
                .selectables = selectables;
        }
        self.multi_selectable_selection_container_delegate_data_mut(app)
            .selection_in_progress = selection_will_be_in_progress;
        self.multi_selectable_selection_container_delegate_data_mut(app)
            .is_handling_selection_event = true;
        let result = match event.ty() {
            SelectionEventType::StartEdgeUpdate | SelectionEventType::EndEdgeUpdate => {
                self.set_extend_selection_in_progress(app, false);
                MultiSelectableSelectionContainerDelegate::handle_selection_edge_update(
                    self, app, event,
                )
            }
            SelectionEventType::Clear => {
                self.set_extend_selection_in_progress(app, false);
                MultiSelectableSelectionContainerDelegate::handle_clear_selection(self, app, event)
            }
            SelectionEventType::SelectAll => {
                self.set_extend_selection_in_progress(app, false);
                MultiSelectableSelectionContainerDelegate::handle_select_all(self, app, event)
            }
            SelectionEventType::SelectWord => {
                self.set_extend_selection_in_progress(app, false);
                MultiSelectableSelectionContainerDelegate::handle_select_word(self, app, event)
            }
            SelectionEventType::SelectParagraph => {
                self.set_extend_selection_in_progress(app, false);
                MultiSelectableSelectionContainerDelegate::handle_select_paragraph(self, app, event)
            }
            SelectionEventType::GranularlyExtendSelection => {
                self.set_extend_selection_in_progress(app, true);
                MultiSelectableSelectionContainerDelegate::handle_granularly_extend_selection(
                    self, app, event,
                )
            }
            SelectionEventType::DirectionallyExtendSelection => {
                self.set_extend_selection_in_progress(app, true);
                MultiSelectableSelectionContainerDelegate::handle_directionally_extend_selection(
                    self, app, event,
                )
            }
        };
        self.multi_selectable_selection_container_delegate_data_mut(app)
            .is_handling_selection_event = false;
        MultiSelectableSelectionContainerDelegate::update_selection_geometry(self, app);
        result
    }

    /// Dart's `_extendSelectionInProgress =`.
    fn set_extend_selection_in_progress(self: Handle<Self>, app: &mut App, value: bool) {
        self.multi_selectable_selection_container_delegate_data_mut(app)
            .extend_selection_in_progress = value;
    }

    /// Dart's `dispose`: stop listening to the selectables, forget them, and dispose the
    /// `ChangeNotifier`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        for selectable in &selectables {
            selectable.remove_listener(app, &selectable_geometry_change_listener(self));
        }
        let data = self.multi_selectable_selection_container_delegate_data_mut(app);
        data.selectables = Vec::new();
        data.scheduled_selectable_update = false;
        ChangeNotifierData::dispose(app, self);
    }
}

/// Dart's `_compareVertically`: two rectangles in screen order by their vertical positions only,
/// equal when one cannot be placed above the other within the threshold.
pub(crate) fn compare_vertically(a: Rect, b: Rect) -> Ordering {
    // A selectable a sliver no longer paints, such as a kept-alive child with a selection, has
    // a zero transform. Dart's `transformRect` turns that into NaN, whose comparisons make it
    // sort before the others; `transform_rect` here keeps the infinities, which would sort it
    // last, so it is put before them here.
    if !a.is_finite() || !b.is_finite() {
        return Ordering::Less;
    }
    if (a.top - b.top < K_SELECTABLE_VERTICAL_COMPARING_THRESHOLD
        && a.bottom - b.bottom > -K_SELECTABLE_VERTICAL_COMPARING_THRESHOLD)
        || (b.top - a.top < K_SELECTABLE_VERTICAL_COMPARING_THRESHOLD
            && b.bottom - a.bottom > -K_SELECTABLE_VERTICAL_COMPARING_THRESHOLD)
    {
        return Ordering::Equal;
    }
    if (a.top - b.top).abs() > K_SELECTABLE_VERTICAL_COMPARING_THRESHOLD {
        return if a.top > b.top {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }
    if a.bottom > b.bottom {
        Ordering::Greater
    } else {
        Ordering::Less
    }
}

/// Dart's `_compareHorizontally`: two rectangles in screen order by their horizontal positions,
/// assuming one encloses the other vertically.
pub(crate) fn compare_horizontally(a: Rect, b: Rect) -> Ordering {
    // a encloses b.
    if a.left - b.left < PRECISION_ERROR_TOLERANCE && a.right - b.right > -PRECISION_ERROR_TOLERANCE
    {
        return Ordering::Less;
    }
    // b encloses a.
    if b.left - a.left < PRECISION_ERROR_TOLERANCE && b.right - a.right > -PRECISION_ERROR_TOLERANCE
    {
        return Ordering::Greater;
    }
    if (a.left - b.left).abs() > PRECISION_ERROR_TOLERANCE {
        return if a.left > b.left {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }
    if a.right > b.right {
        Ordering::Greater
    } else {
        Ordering::Less
    }
}

// ---------------------------------------------------------------------------------------------
// StaticSelectionContainerDelegate

/// The fields of Dart's `StaticSelectionContainerDelegate`.
pub struct StaticSelectionContainerDelegateData {
    /// The selectables that have received start events.
    has_received_start_event: Vec<AnySelectable>,

    /// The selectables that have received end events.
    has_received_end_event: Vec<AnySelectable>,

    /// The global position of the last selection start edge update.
    last_start_edge_update_global_position: Option<Offset>,

    /// The global position of the last selection end edge update.
    last_end_edge_update_global_position: Option<Offset>,
}

impl StaticSelectionContainerDelegateData {
    /// Creates the delegate's fields, with no selectable marked and no edge update seen.
    pub fn new() -> StaticSelectionContainerDelegateData {
        StaticSelectionContainerDelegateData {
            has_received_start_event: Vec::new(),
            has_received_end_event: Vec::new(),
            last_start_edge_update_global_position: None,
            last_end_edge_update_global_position: None,
        }
    }
}

impl Default for StaticSelectionContainerDelegateData {
    fn default() -> Self {
        Self::new()
    }
}

/// The accessors [`StaticSelectionContainerDelegate`] asks for, for a struct whose bag is the
/// field `static_selection_container_delegate`.
#[macro_export]
macro_rules! static_selection_container_delegate_accessors {
    () => {
        fn static_selection_container_delegate_data(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> &$crate::StaticSelectionContainerDelegateData {
            &app.get(self).static_selection_container_delegate
        }

        fn static_selection_container_delegate_data_mut(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> &mut $crate::StaticSelectionContainerDelegateData {
            &mut app.get_mut(self).static_selection_container_delegate
        }
    };
}

/// A delegate that handles selection events for selectables of more than one kind, remembering
/// which edge each one has already received so a child that comes into the selection is brought
/// up to date with a synthesized event.
pub trait StaticSelectionContainerDelegate: MultiSelectableSelectionContainerDelegate {
    /// Dart's `StaticSelectionContainerDelegate` fields, held under the field
    /// `static_selection_container_delegate`
    /// ([`static_selection_container_delegate_accessors!`](crate::static_selection_container_delegate_accessors)).
    fn static_selection_container_delegate_data(
        self: Handle<Self>,
        app: &App,
    ) -> &StaticSelectionContainerDelegateData;

    /// See [`static_selection_container_delegate_data`](Self::static_selection_container_delegate_data).
    fn static_selection_container_delegate_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut StaticSelectionContainerDelegateData;

    /// Tracks whether a selection edge update event for `selectable` was received.
    ///
    /// With `for_end` `Some(true)` the selectable is registered as having received an end event,
    /// with `Some(false)` a start event, and with `None` both.
    ///
    /// Call this when a [`SelectionEvent`] is dispatched to a child selectable managed by this
    /// delegate.
    fn did_receive_selection_event_for(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        for_end: Option<bool>,
    ) {
        StaticSelectionContainerDelegateBase::did_receive_selection_event_for(
            self, app, selectable, for_end,
        );
    }

    /// Updates the internal selection state after a [`SelectionEvent`] that selects a boundary,
    /// such as select-word, select-paragraph and select-all.
    fn did_receive_selection_boundary_events(self: Handle<Self>, app: &mut App) {
        StaticSelectionContainerDelegateBase::did_receive_selection_boundary_events(self, app);
    }

    /// Updates the last selection edge location of the edge `for_end` names.
    fn update_last_selection_edge_location(
        self: Handle<Self>,
        app: &mut App,
        global_selection_edge_location: Offset,
        for_end: bool,
    ) {
        StaticSelectionContainerDelegateBase::update_last_selection_edge_location(
            self,
            app,
            global_selection_edge_location,
            for_end,
        );
    }

    /// Clears the internal selection state: no selectable under this delegate has received a
    /// start or end event, and no edge location is remembered.
    fn clear_internal_selection_state(self: Handle<Self>, app: &mut App) {
        StaticSelectionContainerDelegateBase::clear_internal_selection_state(self, app);
    }

    /// Clears the internal selection state of `selectable`.
    fn clear_internal_selection_state_for_selectable(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
    ) {
        StaticSelectionContainerDelegateBase::clear_internal_selection_state_for_selectable(
            self, app, selectable,
        );
    }
}

/// The `SelectionHandler` a leaf of [`StaticSelectionContainerDelegate`] that keeps Static's
/// selection handling writes inside `impl SelectionHandler for MyDelegate`.
#[macro_export]
macro_rules! static_selection_container_delegate_selection_handler {
    () => {
        fn value(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::waft_rendering::SelectionGeometry {
            $crate::StaticSelectionContainerDelegateBase::value(self, app)
        }

        fn push_handle_layers(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            start_handle: ::std::option::Option<
                ::waft_foundation::Ref<::waft_rendering::LayerLink>,
            >,
            end_handle: ::std::option::Option<::waft_foundation::Ref<::waft_rendering::LayerLink>>,
        ) {
            $crate::StaticSelectionContainerDelegateBase::push_handle_layers(
                self,
                app,
                start_handle,
                end_handle,
            )
        }

        fn selected_content(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::std::option::Option<::waft_rendering::SelectedContent> {
            $crate::StaticSelectionContainerDelegateBase::selected_content(self, app)
        }

        fn selection(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::std::option::Option<::waft_rendering::SelectedContentRange> {
            $crate::StaticSelectionContainerDelegateBase::selection(self, app)
        }

        fn dispatch_selection_event(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            event: &::waft_rendering::SelectionEvent,
        ) -> ::waft_rendering::SelectionResult {
            $crate::StaticSelectionContainerDelegateBase::dispatch_selection_event(self, app, event)
        }

        fn content_length(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> usize {
            $crate::StaticSelectionContainerDelegateBase::content_length(self, app)
        }
    };
}

/// The `SelectionRegistrar` a leaf of [`StaticSelectionContainerDelegate`] writes inside
/// `impl SelectionRegistrar for MyDelegate`.
#[macro_export]
macro_rules! static_selection_container_delegate_selection_registrar {
    () => {
        fn add(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            selectable: ::waft_rendering::AnySelectable,
        ) {
            $crate::StaticSelectionContainerDelegateBase::add(self, app, selectable)
        }

        fn remove(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            selectable: ::waft_rendering::AnySelectable,
        ) {
            $crate::StaticSelectionContainerDelegateBase::remove(self, app, selectable)
        }
    };
}

/// The overrides a [`StaticSelectionContainerDelegate`] leaf keeps from the class: the members
/// Static handles itself. Write it inside
/// `impl MultiSelectableSelectionContainerDelegate for MyDelegate { .. }`, beside
/// [`multi_selectable_selection_container_delegate_accessors!`](crate::multi_selectable_selection_container_delegate_accessors).
///
/// A subclass that changes one of them writes its own method instead and calls
/// [`StaticSelectionContainerDelegateBase`] for the inherited body.
#[macro_export]
macro_rules! static_selection_container_delegate_overrides {
    () => {
        fn did_change_selectables(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) {
            $crate::StaticSelectionContainerDelegateBase::did_change_selectables(self, app)
        }

        fn handle_select_all(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            event: &::waft_rendering::SelectionEvent,
        ) -> ::waft_rendering::SelectionResult {
            $crate::StaticSelectionContainerDelegateBase::handle_select_all(self, app, event)
        }

        fn handle_select_word(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            event: &::waft_rendering::SelectionEvent,
        ) -> ::waft_rendering::SelectionResult {
            $crate::StaticSelectionContainerDelegateBase::handle_select_word(self, app, event)
        }

        fn handle_select_paragraph(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            event: &::waft_rendering::SelectionEvent,
        ) -> ::waft_rendering::SelectionResult {
            $crate::StaticSelectionContainerDelegateBase::handle_select_paragraph(self, app, event)
        }

        fn handle_clear_selection(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            event: &::waft_rendering::SelectionEvent,
        ) -> ::waft_rendering::SelectionResult {
            $crate::StaticSelectionContainerDelegateBase::handle_clear_selection(self, app, event)
        }

        fn handle_selection_edge_update(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            event: &::waft_rendering::SelectionEvent,
        ) -> ::waft_rendering::SelectionResult {
            $crate::StaticSelectionContainerDelegateBase::handle_selection_edge_update(
                self, app, event,
            )
        }

        fn dispose(self: ::waft_foundation::Handle<Self>, app: &mut ::waft_foundation::App) {
            $crate::StaticSelectionContainerDelegateBase::dispose(self, app)
        }

        fn ensure_child_updated(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            selectable: ::waft_rendering::AnySelectable,
        ) {
            $crate::StaticSelectionContainerDelegateBase::ensure_child_updated(
                self, app, selectable,
            )
        }

        fn dispatch_selection_event_to_child(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            selectable: ::waft_rendering::AnySelectable,
            event: &::waft_rendering::SelectionEvent,
        ) -> ::waft_rendering::SelectionResult {
            $crate::StaticSelectionContainerDelegateBase::dispatch_selection_event_to_child(
                self, app, selectable, event,
            )
        }
    };
}

/// The bodies of Dart's `StaticSelectionContainerDelegate`.
///
/// Every [`StaticSelectionContainerDelegate`] implements this, so an override that runs the
/// inherited body calls it trait-qualified,
/// `StaticSelectionContainerDelegateBase::handle_select_all(self, app, event)`, where Dart
/// writes `super.handleSelectAll(event)`.
pub trait StaticSelectionContainerDelegateBase: StaticSelectionContainerDelegate {
    /// Dart's `getSelectedContent`, inherited.
    fn selected_content(self: Handle<Self>, app: &App) -> Option<SelectedContent> {
        MultiSelectableSelectionContainerDelegateBase::selected_content(self, app)
    }

    /// Dart's `value`, inherited.
    fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
        MultiSelectableSelectionContainerDelegateBase::value(self, app)
    }

    /// Dart's `pushHandleLayers`, inherited.
    fn push_handle_layers(
        self: Handle<Self>,
        app: &mut App,
        start_handle: Option<Ref<LayerLink>>,
        end_handle: Option<Ref<LayerLink>>,
    ) {
        MultiSelectableSelectionContainerDelegateBase::push_handle_layers(
            self,
            app,
            start_handle,
            end_handle,
        );
    }

    /// Dart's `getSelection`, inherited.
    fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
        MultiSelectableSelectionContainerDelegateBase::selection(self, app)
    }

    /// Dart's `dispatchSelectionEvent`, inherited.
    fn dispatch_selection_event(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        MultiSelectableSelectionContainerDelegateBase::dispatch_selection_event(self, app, event)
    }

    /// Dart's `contentLength`, inherited.
    fn content_length(self: Handle<Self>, app: &App) -> usize {
        MultiSelectableSelectionContainerDelegateBase::content_length(self, app)
    }

    /// Dart's `add`, inherited.
    fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        MultiSelectableSelectionContainerDelegateBase::add(self, app, selectable);
    }

    /// Dart's `StaticSelectionContainerDelegate.remove`.
    fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        StaticSelectionContainerDelegate::clear_internal_selection_state_for_selectable(
            self, app, selectable,
        );
        MultiSelectableSelectionContainerDelegateBase::remove(self, app, selectable);
    }

    /// Dart's `didReceiveSelectionEventFor`.
    fn did_receive_selection_event_for(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        for_end: Option<bool>,
    ) {
        let data = self.static_selection_container_delegate_data_mut(app);
        match for_end {
            Some(true) => {
                set_add(&mut data.has_received_end_event, selectable);
            }
            Some(false) => {
                set_add(&mut data.has_received_start_event, selectable);
            }
            None => {
                set_add(&mut data.has_received_start_event, selectable);
                set_add(&mut data.has_received_end_event, selectable);
            }
        }
    }

    /// Dart's `didReceiveSelectionBoundaryEvents`.
    fn did_receive_selection_boundary_events(self: Handle<Self>, app: &mut App) {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        if data.current_selection_start_index == -1 || data.current_selection_end_index == -1 {
            return;
        }
        let start = data
            .current_selection_start_index
            .min(data.current_selection_end_index);
        let end = data
            .current_selection_start_index
            .max(data.current_selection_end_index);
        let selectables = data.selectables.clone();
        for index in start..=end {
            StaticSelectionContainerDelegate::did_receive_selection_event_for(
                self,
                app,
                selectables[index as usize],
                None,
            );
        }
        self.update_last_selection_edge_locations_from_geometries(app);
    }

    /// Dart's `updateLastSelectionEdgeLocation`.
    fn update_last_selection_edge_location(
        self: Handle<Self>,
        app: &mut App,
        global_selection_edge_location: Offset,
        for_end: bool,
    ) {
        let data = self.static_selection_container_delegate_data_mut(app);
        if for_end {
            data.last_end_edge_update_global_position = Some(global_selection_edge_location);
        } else {
            data.last_start_edge_update_global_position = Some(global_selection_edge_location);
        }
    }

    /// Dart's `_updateLastSelectionEdgeLocationsFromGeometries`.
    fn update_last_selection_edge_locations_from_geometries(self: Handle<Self>, app: &mut App) {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        let start_index = data.current_selection_start_index;
        let end_index = data.current_selection_end_index;
        let selectables = data.selectables.clone();
        if start_index != -1 && selectables[start_index as usize].value(app).has_selection() {
            let start = selectables[start_index as usize];
            let point = start
                .value(app)
                .start_selection_point
                .expect("a selectable with a selection has a start point");
            let local_start_edge =
                point.local_position + Offset::new(0.0, -point.line_height / 2.0);
            let global_start_edge =
                transform_point(&start.get_transform_to(app, None), local_start_edge);
            StaticSelectionContainerDelegate::update_last_selection_edge_location(
                self,
                app,
                global_start_edge,
                false,
            );
        }
        if end_index != -1 && selectables[end_index as usize].value(app).has_selection() {
            let end = selectables[end_index as usize];
            let point = end
                .value(app)
                .end_selection_point
                .expect("a selectable with a selection has an end point");
            let local_end_edge = point.local_position + Offset::new(0.0, -point.line_height / 2.0);
            let global_end_edge = transform_point(&end.get_transform_to(app, None), local_end_edge);
            StaticSelectionContainerDelegate::update_last_selection_edge_location(
                self,
                app,
                global_end_edge,
                true,
            );
        }
    }

    /// Dart's `clearInternalSelectionState`.
    fn clear_internal_selection_state(self: Handle<Self>, app: &mut App) {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        for selectable in &selectables {
            StaticSelectionContainerDelegate::clear_internal_selection_state_for_selectable(
                self,
                app,
                *selectable,
            );
        }
        let data = self.static_selection_container_delegate_data_mut(app);
        data.last_start_edge_update_global_position = None;
        data.last_end_edge_update_global_position = None;
    }

    /// Dart's `clearInternalSelectionStateForSelectable`.
    fn clear_internal_selection_state_for_selectable(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
    ) {
        let data = self.static_selection_container_delegate_data_mut(app);
        set_remove(&mut data.has_received_start_event, selectable);
        set_remove(&mut data.has_received_end_event, selectable);
    }

    /// Dart's `handleSelectAll`.
    fn handle_select_all(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let result =
            MultiSelectableSelectionContainerDelegateBase::handle_select_all(self, app, event);
        StaticSelectionContainerDelegate::did_receive_selection_boundary_events(self, app);
        result
    }

    /// Dart's `handleSelectWord`.
    fn handle_select_word(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let result =
            MultiSelectableSelectionContainerDelegateBase::handle_select_word(self, app, event);
        StaticSelectionContainerDelegate::did_receive_selection_boundary_events(self, app);
        result
    }

    /// Dart's `handleSelectParagraph`.
    fn handle_select_paragraph(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let result = MultiSelectableSelectionContainerDelegateBase::handle_select_paragraph(
            self, app, event,
        );
        StaticSelectionContainerDelegate::did_receive_selection_boundary_events(self, app);
        result
    }

    /// Dart's `handleClearSelection`.
    fn handle_clear_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let result =
            MultiSelectableSelectionContainerDelegateBase::handle_clear_selection(self, app, event);
        StaticSelectionContainerDelegate::clear_internal_selection_state(self, app);
        result
    }

    /// Dart's `handleSelectionEdgeUpdate`.
    fn handle_selection_edge_update(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let (global_position, for_end) = match event {
            SelectionEvent::StartEdgeUpdate {
                global_position, ..
            } => (*global_position, false),
            SelectionEvent::EndEdgeUpdate {
                global_position, ..
            } => (*global_position, true),
            _ => panic!("a selection edge update names an edge"),
        };
        StaticSelectionContainerDelegate::update_last_selection_edge_location(
            self,
            app,
            global_position,
            for_end,
        );
        MultiSelectableSelectionContainerDelegateBase::handle_selection_edge_update(
            self, app, event,
        )
    }

    /// Dart's `dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        StaticSelectionContainerDelegate::clear_internal_selection_state(self, app);
        MultiSelectableSelectionContainerDelegateBase::dispose(self, app);
    }

    /// Dart's `dispatchSelectionEventToChild`: mark the edge the child is about to receive, and
    /// bring it up to date, before sending the event on.
    fn dispatch_selection_event_to_child(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        event: &SelectionEvent,
    ) -> SelectionResult {
        match event.ty() {
            SelectionEventType::StartEdgeUpdate => {
                StaticSelectionContainerDelegate::did_receive_selection_event_for(
                    self,
                    app,
                    selectable,
                    Some(false),
                );
                MultiSelectableSelectionContainerDelegate::ensure_child_updated(
                    self, app, selectable,
                );
            }
            SelectionEventType::EndEdgeUpdate => {
                StaticSelectionContainerDelegate::did_receive_selection_event_for(
                    self,
                    app,
                    selectable,
                    Some(true),
                );
                MultiSelectableSelectionContainerDelegate::ensure_child_updated(
                    self, app, selectable,
                );
            }
            SelectionEventType::Clear => {
                StaticSelectionContainerDelegate::clear_internal_selection_state_for_selectable(
                    self, app, selectable,
                );
            }
            SelectionEventType::SelectAll
            | SelectionEventType::SelectWord
            | SelectionEventType::SelectParagraph => {}
            SelectionEventType::GranularlyExtendSelection
            | SelectionEventType::DirectionallyExtendSelection => {
                StaticSelectionContainerDelegate::did_receive_selection_event_for(
                    self, app, selectable, None,
                );
                MultiSelectableSelectionContainerDelegate::ensure_child_updated(
                    self, app, selectable,
                );
            }
        }
        MultiSelectableSelectionContainerDelegateBase::dispatch_selection_event_to_child(
            self, app, selectable, event,
        )
    }

    /// Dart's `ensureChildUpdated`: synthesize the edge event the child has not seen, from the
    /// last edge location, so a selectable added inside the selection is selected at once.
    fn ensure_child_updated(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        let last_end = self
            .static_selection_container_delegate_data(app)
            .last_end_edge_update_global_position;
        if let Some(global_position) = last_end
            && set_add(
                &mut self
                    .static_selection_container_delegate_data_mut(app)
                    .has_received_end_event,
                selectable,
            )
        {
            let synthesized_event = SelectionEvent::for_end(global_position, None);
            if self
                .multi_selectable_selection_container_delegate_data(app)
                .current_selection_end_index
                == -1
            {
                MultiSelectableSelectionContainerDelegate::handle_selection_edge_update(
                    self,
                    app,
                    &synthesized_event,
                );
            }
            selectable.dispatch_selection_event(app, &synthesized_event);
        }
        let last_start = self
            .static_selection_container_delegate_data(app)
            .last_start_edge_update_global_position;
        if let Some(global_position) = last_start
            && set_add(
                &mut self
                    .static_selection_container_delegate_data_mut(app)
                    .has_received_start_event,
                selectable,
            )
        {
            let synthesized_event = SelectionEvent::for_start(global_position, None);
            if self
                .multi_selectable_selection_container_delegate_data(app)
                .current_selection_start_index
                == -1
            {
                MultiSelectableSelectionContainerDelegate::handle_selection_edge_update(
                    self,
                    app,
                    &synthesized_event,
                );
            }
            selectable.dispatch_selection_event(app, &synthesized_event);
        }
    }

    /// Dart's `didChangeSelectables`: replay the last edge updates over the new list, then drop
    /// the selectables that are gone from the edge bookkeeping.
    fn did_change_selectables(self: Handle<Self>, app: &mut App) {
        let last_end = self
            .static_selection_container_delegate_data(app)
            .last_end_edge_update_global_position;
        if let Some(global_position) = last_end {
            MultiSelectableSelectionContainerDelegate::handle_selection_edge_update(
                self,
                app,
                &SelectionEvent::for_end(global_position, None),
            );
        }
        let last_start = self
            .static_selection_container_delegate_data(app)
            .last_start_edge_update_global_position;
        if let Some(global_position) = last_start {
            MultiSelectableSelectionContainerDelegate::handle_selection_edge_update(
                self,
                app,
                &SelectionEvent::for_start(global_position, None),
            );
        }
        let selectable_set = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        let data = self.static_selection_container_delegate_data_mut(app);
        data.has_received_end_event
            .retain(|selectable| set_contains(&selectable_set, *selectable));
        data.has_received_start_event
            .retain(|selectable| set_contains(&selectable_set, *selectable));
        MultiSelectableSelectionContainerDelegateBase::did_change_selectables(self, app);
    }
}

impl<T: StaticSelectionContainerDelegate> StaticSelectionContainerDelegateBase for T {}

/// Dart's instantiable `StaticSelectionContainerDelegate()`.
///
/// A delegate that handles selection events for the selectables in a `SelectionContainer`, which
/// `SelectableRegion` uses. A Waft app that needs another behavior writes its own struct of
/// [`StaticSelectionContainerDelegate`] and overrides the members it changes.
pub struct PlainStaticSelectionContainerDelegate {
    selection_container_delegate: SelectionContainerDelegateData,
    multi_selectable_selection_container_delegate: MultiSelectableSelectionContainerDelegateData,
    static_selection_container_delegate: StaticSelectionContainerDelegateData,
}

impl PlainStaticSelectionContainerDelegate {
    /// Creates a delegate over no selectables and no selection.
    pub fn new(app: &mut App) -> Ref<PlainStaticSelectionContainerDelegate> {
        app.create(PlainStaticSelectionContainerDelegate {
            selection_container_delegate: SelectionContainerDelegateData::new(),
            multi_selectable_selection_container_delegate:
                MultiSelectableSelectionContainerDelegateData::new(),
            static_selection_container_delegate: StaticSelectionContainerDelegateData::new(),
        })
    }
}

impl SelectionContainerDelegate for PlainStaticSelectionContainerDelegate {
    fn selection_container_delegate_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SelectionContainerDelegateData {
        &app.get(self).selection_container_delegate
    }

    fn selection_container_delegate_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SelectionContainerDelegateData {
        &mut app.get_mut(self).selection_container_delegate
    }
}

impl MultiSelectableSelectionContainerDelegate for PlainStaticSelectionContainerDelegate {
    crate::multi_selectable_selection_container_delegate_accessors!();
    crate::static_selection_container_delegate_overrides!();
}

impl StaticSelectionContainerDelegate for PlainStaticSelectionContainerDelegate {
    crate::static_selection_container_delegate_accessors!();
}

impl SelectionHandler for PlainStaticSelectionContainerDelegate {
    crate::static_selection_container_delegate_selection_handler!();
}

impl SelectionRegistrar for PlainStaticSelectionContainerDelegate {
    crate::static_selection_container_delegate_selection_registrar!();
}

impl ChangeNotifier for PlainStaticSelectionContainerDelegate {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self
            .multi_selectable_selection_container_delegate
            .change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self
            .multi_selectable_selection_container_delegate
            .change_notifier
    }
}

impl std::fmt::Debug for PlainStaticSelectionContainerDelegate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticSelectionContainerDelegate")
            .field(
                "selectables",
                &self
                    .multi_selectable_selection_container_delegate
                    .selectables,
            )
            .finish_non_exhaustive()
    }
}

impl<T: MultiSelectableSelectionContainerDelegate> MultiSelectableSelectionContainerDelegateBase
    for T
{
}

#[cfg(test)]
mod tests {
    //! `selection_container_test.dart`'s cases, which build a
    //! [`MultiSelectableSelectionContainerDelegate`], and `selectable_region_test.dart`'s
    //! numbers for the delegate itself.
    //!
    //! Flutter's cases put `Text` children in the container, and Waft's `Text` does not select
    //! yet, so a case here puts a `RichText` wired to the enclosing container where Flutter
    //! writes `Text`: one paragraph, and so one selectable, either way.

    use waft_embedder::{Offset, Size, TextDirection, TextPosition};
    use waft_foundation::{App, Handle, Ref};
    use waft_rendering::{RenderBox, RenderParagraph};

    use super::*;
    use crate::Placeholder;
    use crate::framework::{IntoWidget, WidgetRef};
    use waft_scheduler::{FrameTarget, SchedulerBinding};

    use crate::testing::{WidgetTester, find, test_widgets};
    use crate::widgets::basic::{Builder, Center, Column, SizedBox};
    use crate::widgets::selection_container::{SelectionContainer, SelectionRegistrarScope};
    use crate::widgets::text::Text;

    /// A [`Text`] under the enclosing [`SelectionContainer`], as Flutter's is under a
    /// `SelectableRegion`; under a disabled container it is simply not selectable.
    fn selectable_text(text: &'static str) -> WidgetRef {
        Text::new(text)
            .text_direction(TextDirection::Ltr)
            .into_widget()
    }

    /// The offset in the view of the caret at `offset` in `paragraph`, five pixels down from the
    /// top of the line, as flutter_test's `textOffsetToPosition` and Flutter's selection tests
    /// place a point inside a line.
    fn global_offset(
        app: &mut App,
        paragraph: waft_rendering::RenderHandle<RenderParagraph>,
        offset: i32,
    ) -> Offset {
        let local = paragraph.get_offset_for_caret(app, TextPosition::new(offset), Rect::ZERO)
            + Offset::new(0.0, 5.0);
        transform_point(&paragraph.as_object().get_transform_to(app, None), local)
    }

    /// `paragraph_test.dart`'s and `selectable_region_test.dart`'s `TestSelectionRegistrar`: it
    /// holds what registers with it.
    struct TestSelectionRegistrar {
        selectables: Vec<AnySelectable>,
    }

    impl SelectionRegistrar for TestSelectionRegistrar {
        fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
            set_add(&mut app.get_mut(self).selectables, selectable);
        }

        fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
            set_remove(&mut app.get_mut(self).selectables, selectable);
        }
    }

    /// `selection_container_test.dart`'s `TestContainerDelegate`: the two members the container
    /// would ask for throw, since the cases here never send it a selection event.
    struct TestContainerDelegate {
        selection_container_delegate: SelectionContainerDelegateData,
        multi_selectable_selection_container_delegate:
            MultiSelectableSelectionContainerDelegateData,
    }

    impl TestContainerDelegate {
        fn new(app: &mut App) -> Ref<TestContainerDelegate> {
            app.create(TestContainerDelegate {
                selection_container_delegate: SelectionContainerDelegateData::new(),
                multi_selectable_selection_container_delegate:
                    MultiSelectableSelectionContainerDelegateData::new(),
            })
        }
    }

    impl SelectionContainerDelegate for TestContainerDelegate {
        fn selection_container_delegate_data(
            self: Handle<Self>,
            app: &App,
        ) -> &SelectionContainerDelegateData {
            &app.get(self).selection_container_delegate
        }

        fn selection_container_delegate_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut SelectionContainerDelegateData {
            &mut app.get_mut(self).selection_container_delegate
        }
    }

    impl MultiSelectableSelectionContainerDelegate for TestContainerDelegate {
        crate::multi_selectable_selection_container_delegate_accessors!();

        fn ensure_child_updated(self: Handle<Self>, _app: &mut App, _selectable: AnySelectable) {
            panic!("UnimplementedError");
        }

        fn dispatch_selection_event_to_child(
            self: Handle<Self>,
            _app: &mut App,
            _selectable: AnySelectable,
            _event: &SelectionEvent,
        ) -> SelectionResult {
            panic!("UnimplementedError");
        }
    }

    impl SelectionHandler for TestContainerDelegate {
        fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
            MultiSelectableSelectionContainerDelegateBase::value(self, app)
        }

        fn push_handle_layers(
            self: Handle<Self>,
            app: &mut App,
            start_handle: Option<Ref<LayerLink>>,
            end_handle: Option<Ref<LayerLink>>,
        ) {
            MultiSelectableSelectionContainerDelegateBase::push_handle_layers(
                self,
                app,
                start_handle,
                end_handle,
            );
        }

        fn selected_content(self: Handle<Self>, app: &App) -> Option<SelectedContent> {
            MultiSelectableSelectionContainerDelegateBase::selected_content(self, app)
        }

        fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
            MultiSelectableSelectionContainerDelegateBase::selection(self, app)
        }

        fn dispatch_selection_event(
            self: Handle<Self>,
            app: &mut App,
            event: &SelectionEvent,
        ) -> SelectionResult {
            MultiSelectableSelectionContainerDelegateBase::dispatch_selection_event(
                self, app, event,
            )
        }

        fn content_length(self: Handle<Self>, app: &App) -> usize {
            MultiSelectableSelectionContainerDelegateBase::content_length(self, app)
        }
    }

    impl SelectionRegistrar for TestContainerDelegate {
        fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
            MultiSelectableSelectionContainerDelegateBase::add(self, app, selectable);
        }

        fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
            MultiSelectableSelectionContainerDelegateBase::remove(self, app, selectable);
        }
    }

    impl ChangeNotifier for TestContainerDelegate {
        fn change_notifier_data(&self) -> &ChangeNotifierData {
            &self
                .multi_selectable_selection_container_delegate
                .change_notifier
        }

        fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
            &mut self
                .multi_selectable_selection_container_delegate
                .change_notifier
        }
    }

    /// `selectable_region_test.dart`'s `ColumnSelectionContainerDelegate`: the selected
    /// contents of all selectables, separated by a new line.
    struct ColumnSelectionContainerDelegate {
        selection_container_delegate: SelectionContainerDelegateData,
        multi_selectable_selection_container_delegate:
            MultiSelectableSelectionContainerDelegateData,
        static_selection_container_delegate: StaticSelectionContainerDelegateData,
    }

    impl ColumnSelectionContainerDelegate {
        fn new(app: &mut App) -> Ref<ColumnSelectionContainerDelegate> {
            app.create(ColumnSelectionContainerDelegate {
                selection_container_delegate: SelectionContainerDelegateData::new(),
                multi_selectable_selection_container_delegate:
                    MultiSelectableSelectionContainerDelegateData::new(),
                static_selection_container_delegate: StaticSelectionContainerDelegateData::new(),
            })
        }
    }

    impl SelectionContainerDelegate for ColumnSelectionContainerDelegate {
        fn selection_container_delegate_data(
            self: Handle<Self>,
            app: &App,
        ) -> &SelectionContainerDelegateData {
            &app.get(self).selection_container_delegate
        }

        fn selection_container_delegate_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut SelectionContainerDelegateData {
            &mut app.get_mut(self).selection_container_delegate
        }
    }

    impl MultiSelectableSelectionContainerDelegate for ColumnSelectionContainerDelegate {
        crate::multi_selectable_selection_container_delegate_accessors!();
        crate::static_selection_container_delegate_overrides!();
    }

    impl StaticSelectionContainerDelegate for ColumnSelectionContainerDelegate {
        crate::static_selection_container_delegate_accessors!();
    }

    impl SelectionHandler for ColumnSelectionContainerDelegate {
        fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
            StaticSelectionContainerDelegateBase::value(self, app)
        }

        fn push_handle_layers(
            self: Handle<Self>,
            app: &mut App,
            start_handle: Option<Ref<LayerLink>>,
            end_handle: Option<Ref<LayerLink>>,
        ) {
            StaticSelectionContainerDelegateBase::push_handle_layers(
                self,
                app,
                start_handle,
                end_handle,
            );
        }

        fn selected_content(self: Handle<Self>, app: &App) -> Option<SelectedContent> {
            let selectables = self
                .multi_selectable_selection_container_delegate_data(app)
                .selectables
                .clone();
            let selections: Vec<SelectedContent> = selectables
                .iter()
                .filter_map(|selectable| selectable.selected_content(app))
                .collect();
            if selections.is_empty() {
                return None;
            }
            let plain_text = selections
                .iter()
                .map(|selection| selection.plain_text.clone())
                .collect::<Vec<String>>()
                .join("\n");
            Some(SelectedContent::new(plain_text))
        }

        fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
            StaticSelectionContainerDelegateBase::selection(self, app)
        }

        fn dispatch_selection_event(
            self: Handle<Self>,
            app: &mut App,
            event: &SelectionEvent,
        ) -> SelectionResult {
            StaticSelectionContainerDelegateBase::dispatch_selection_event(self, app, event)
        }

        fn content_length(self: Handle<Self>, app: &App) -> usize {
            StaticSelectionContainerDelegateBase::content_length(self, app)
        }
    }

    impl SelectionRegistrar for ColumnSelectionContainerDelegate {
        crate::static_selection_container_delegate_selection_registrar!();
    }

    impl ChangeNotifier for ColumnSelectionContainerDelegate {
        fn change_notifier_data(&self) -> &ChangeNotifierData {
            &self
                .multi_selectable_selection_container_delegate
                .change_notifier
        }

        fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
            &mut self
                .multi_selectable_selection_container_delegate
                .change_notifier
        }
    }

    // selection_container_test.dart 'updates its registrar and delegate based on the number of
    // selectables'.
    #[test]
    fn updates_its_registrar_and_delegate_based_on_the_number_of_selectables() {
        test_widgets(|tester| {
            let registrar = tester.update(|app| {
                app.create(TestSelectionRegistrar {
                    selectables: Vec::new(),
                })
            });
            let delegate = tester.update(TestContainerDelegate::new);

            tester.pump_widget(
                SelectionContainer::new(
                    delegate.erase(),
                    Column::new().children([
                        selectable_text("column1"),
                        selectable_text("column2"),
                        selectable_text("column3"),
                    ]),
                )
                .registrar(registrar.erase()),
            );
            tester.pump_and_settle();
            tester.update(|app| {
                assert_eq!(app.get(&registrar).selectables.len(), 1);
                assert_eq!(
                    app.get(&delegate)
                        .multi_selectable_selection_container_delegate
                        .selectables
                        .len(),
                    3
                );
            });
        });
    }

    // selection_container_test.dart 'disabled container'.
    #[test]
    fn a_disabled_container_registers_neither_itself_nor_its_children() {
        test_widgets(|tester| {
            let registrar = tester.update(|app| {
                app.create(TestSelectionRegistrar {
                    selectables: Vec::new(),
                })
            });
            let delegate = tester.update(TestContainerDelegate::new);

            tester.pump_widget(
                SelectionContainer::new(
                    delegate.erase(),
                    SelectionContainer::disabled(Column::new().children([
                        selectable_text("column1"),
                        selectable_text("column2"),
                        selectable_text("column3"),
                    ])),
                )
                .registrar(registrar.erase()),
            );

            tester.update(|app| {
                assert_eq!(app.get(&registrar).selectables.len(), 0);
                assert_eq!(
                    app.get(&delegate)
                        .multi_selectable_selection_container_delegate
                        .selectables
                        .len(),
                    0
                );
            });
        });
    }

    // selection_container_test.dart 'Swapping out container delegate does not crash'.
    #[test]
    fn swapping_out_the_container_delegate_does_not_crash() {
        test_widgets(|tester| {
            let registrar = tester.update(|app| {
                app.create(TestSelectionRegistrar {
                    selectables: Vec::new(),
                })
            });
            let delegate = tester.update(TestContainerDelegate::new);
            let child_delegate = tester.update(TestContainerDelegate::new);

            let pump_inner = |tester: &mut WidgetTester, inner: &Ref<TestContainerDelegate>| {
                let inner = inner.clone();
                tester.pump_widget(
                    SelectionContainer::new(
                        delegate.erase(),
                        Builder::new(move |app, context| {
                            let registrar = SelectionContainer::maybe_of(app, context)
                                .expect("the outer container hosts a registrar");
                            SelectionContainer::new(inner.erase(), selectable_text("dummy"))
                                .registrar(registrar)
                                .into_widget()
                        }),
                    )
                    .registrar(registrar.erase()),
                );
                tester.pump_and_settle();
                next_frame(tester);
            };
            pump_inner(tester, &child_delegate);
            tester.update(|app| {
                assert_eq!(app.get(&registrar).selectables.len(), 1);
                assert!(SelectionHandler::value(delegate.handle(), app).has_content);
            });

            let new_delegate = tester.update(TestContainerDelegate::new);
            pump_inner(tester, &new_delegate);
            tester.update(|app| {
                assert_eq!(app.get(&registrar).selectables.len(), 1);
                assert!(SelectionHandler::value(delegate.handle(), app).has_content);
            });
        });
    }

    // selection_container_test.dart 'Can update within one frame'.
    #[test]
    fn the_container_updates_within_one_frame() {
        test_widgets(|tester| {
            let registrar = tester.update(|app| {
                app.create(TestSelectionRegistrar {
                    selectables: Vec::new(),
                })
            });
            let delegate = tester.update(TestContainerDelegate::new);
            let child_delegate = tester.update(TestContainerDelegate::new);

            tester.pump_widget(
                SelectionContainer::new(
                    delegate.erase(),
                    Builder::new(move |app, context| {
                        let registrar = SelectionContainer::maybe_of(app, context)
                            .expect("the outer container hosts a registrar");
                        SelectionContainer::new(child_delegate.erase(), selectable_text("dummy"))
                            .registrar(registrar)
                            .into_widget()
                    }),
                )
                .registrar(registrar.erase()),
            );
            // The nested delegate's update is a post-frame callback, and its microtask stands in
            // for `tester.idle()`; the next frame, which Flutter's `pump` always produces, folds
            // the nested container's selectable into the outer delegate.
            next_frame(tester);

            tester.update(|app| {
                assert_eq!(app.get(&registrar).selectables.len(), 1);
                assert!(SelectionHandler::value(delegate.handle(), app).has_content);
            });
        });
    }

    // selection_container_test.dart 'selection container registers itself if there is a
    // selectable child'.
    #[test]
    fn a_selection_container_registers_itself_if_there_is_a_selectable_child() {
        test_widgets(|tester| {
            let registrar = tester.update(|app| {
                app.create(TestSelectionRegistrar {
                    selectables: Vec::new(),
                })
            });
            let delegate = tester.update(TestContainerDelegate::new);

            tester.pump_widget(
                SelectionContainer::new(delegate.erase(), Column::new())
                    .registrar(registrar.erase()),
            );
            tester.update(|app| assert_eq!(app.get(&registrar).selectables.len(), 0));

            tester.pump_widget(
                SelectionContainer::new(
                    delegate.erase(),
                    Column::new().children([selectable_text("column1")]),
                )
                .registrar(registrar.erase()),
            );
            tester.pump_and_settle();
            tester.update(|app| assert_eq!(app.get(&registrar).selectables.len(), 1));

            tester.pump_widget(
                SelectionContainer::new(delegate.erase(), Column::new())
                    .registrar(registrar.erase()),
            );
            tester.pump_and_settle();
            tester.update(|app| assert_eq!(app.get(&registrar).selectables.len(), 0));
        });
    }

    // selection_container_test.dart 'selection container gets registrar from context if not
    // provided'.
    #[test]
    fn a_selection_container_gets_its_registrar_from_the_context() {
        test_widgets(|tester| {
            let registrar = tester.update(|app| {
                app.create(TestSelectionRegistrar {
                    selectables: Vec::new(),
                })
            });
            let delegate = tester.update(TestContainerDelegate::new);

            tester.pump_widget(SelectionRegistrarScope::new(
                registrar.erase(),
                SelectionContainer::new(
                    delegate.erase(),
                    Column::new().children([selectable_text("column1")]),
                ),
            ));
            tester.pump_and_settle();

            tester.update(|app| assert_eq!(app.get(&registrar).selectables.len(), 1));
        });
    }

    // selection_container_test.dart 'SelectionContainer does not crash at zero area'.
    #[test]
    fn a_selection_container_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let delegate = tester.update(TestContainerDelegate::new);
            tester.pump_widget(Center::new().child(SizedBox::shrink().child(
                SelectionContainer::new(delegate.erase(), Placeholder::new()),
            )));
            assert_eq!(
                tester.get_size(find::by_type::<SelectionContainer>()),
                Size::ZERO
            );
        });
    }

    /// Runs one more frame, for an update a delegate scheduled after the last one: a nested
    /// container's selectable reaches its delegate a frame after the container below it, as in
    /// Flutter, where `pump` always produces a frame.
    fn next_frame(tester: &mut WidgetTester) {
        tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::EveryView));
        tester.pump_and_settle();
    }

    /// Sends `event` to `handle`, as a gesture or a keyboard action would.
    fn dispatch<D: SelectionHandler>(
        tester: &mut WidgetTester,
        handle: Handle<D>,
        event: SelectionEvent,
    ) {
        tester.update(|app| {
            SelectionHandler::dispatch_selection_event(handle, app, &event);
        });
    }

    /// The selected range and status of `handle`.
    fn assert_selection<D: SelectionHandler>(
        tester: &WidgetTester,
        handle: Handle<D>,
        expected_range: Option<(i32, i32)>,
        expected_status: SelectionStatus,
    ) {
        tester.update(|app| {
            let range = SelectionHandler::selection(handle, app);
            assert_eq!(
                range.map(|range| (range.start_offset, range.end_offset)),
                expected_range
            );
            let value = SelectionHandler::value(handle, app);
            assert_eq!(value.status, expected_status);
            assert!(value.has_content);
        });
    }

    /// The plain text `handle` has selected, or `None`.
    fn selected_text<D: SelectionHandler>(
        tester: &WidgetTester,
        handle: Handle<D>,
    ) -> Option<String> {
        tester.update(|app| {
            SelectionHandler::selected_content(handle, app).map(|content| content.plain_text)
        })
    }

    /// The global position of the caret at `offset` in the `which`th paragraph.
    fn position_of(
        tester: &mut WidgetTester,
        paragraphs: &[waft_rendering::RenderHandle<RenderParagraph>],
        which: usize,
        offset: i32,
    ) -> Offset {
        tester.update(|app| global_offset(app, paragraphs[which], offset))
    }

    // selectable_region_test.dart 'Can extend StaticSelectionContainerDelegate': a delegate that
    // puts a new line between the selectables' selected contents.
    #[test]
    fn a_subclass_joins_the_selectable_contents_its_own_way() {
        test_widgets(|tester| {
            let delegate = tester.update(ColumnSelectionContainerDelegate::new);
            tester.pump_widget(SelectionContainer::new(
                delegate.erase(),
                Column::new().children([
                    selectable_text("Hello World!"),
                    selectable_text("How are you!"),
                ]),
            ));
            tester.pump_and_settle();

            let paragraphs = tester.all::<RenderParagraph>();
            assert_eq!(paragraphs.len(), 2);
            let start = position_of(tester, &paragraphs, 0, 4);
            let end = position_of(tester, &paragraphs, 1, 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );

            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("o World!\nHow are yo".to_string())
            );
            tester.update(|app| {
                assert_eq!(SelectionHandler::content_length(delegate.handle(), app), 24);
            });
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 22)),
                SelectionStatus::Uncollapsed,
            );
        });
    }

    // The same selection through the class itself: the default joins the selectables' contents
    // with nothing between them.
    #[test]
    fn the_delegate_joins_the_selectable_contents_without_a_separator() {
        test_widgets(|tester| {
            let delegate = tester.update(PlainStaticSelectionContainerDelegate::new);
            tester.pump_widget(SelectionContainer::new(
                delegate.erase(),
                Column::new().children([
                    selectable_text("Hello World!"),
                    selectable_text("How are you!"),
                ]),
            ));
            tester.pump_and_settle();

            let paragraphs = tester.all::<RenderParagraph>();
            assert_eq!(paragraphs.len(), 2);
            let start = position_of(tester, &paragraphs, 0, 4);
            let end = position_of(tester, &paragraphs, 1, 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );

            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("o World!How are yo".to_string())
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 22)),
                SelectionStatus::Uncollapsed,
            );
        });
    }

    // selectable_region_test.dart 'onSelectionChanged SelectedContentRange is accurate' numbers,
    // driven through the delegate rather than a mouse. The third text is one more character
    // than the second, as in the test: 36 is 12 + 14 + 10, and 30 is 12 + 14 + 4.
    #[test]
    fn the_selection_range_covers_the_selectables_between_the_edges() {
        test_widgets(|tester| {
            let delegate = tester.update(PlainStaticSelectionContainerDelegate::new);
            tester.pump_widget(SelectionContainer::new(
                delegate.erase(),
                Column::new().children([
                    selectable_text("How are you?"),
                    selectable_text("Good, and you?"),
                    selectable_text("Fine, thank you."),
                ]),
            ));
            tester.pump_and_settle();

            let paragraphs = tester.all::<RenderParagraph>();
            assert_eq!(paragraphs.len(), 3);
            let start = position_of(tester, &paragraphs, 0, 4);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );

            // Selection on paragraph 1.
            let end = position_of(tester, &paragraphs, 0, 7);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 7)),
                SelectionStatus::Uncollapsed,
            );

            // Selection on paragraph 1.
            let end = position_of(tester, &paragraphs, 0, 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 10)),
                SelectionStatus::Uncollapsed,
            );

            // Selection on paragraphs 1 and 2.
            let end = position_of(tester, &paragraphs, 1, 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 22)),
                SelectionStatus::Uncollapsed,
            );

            // Selection on paragraphs 1, 2 and 3.
            let end = position_of(tester, &paragraphs, 2, 10);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((4, 36)),
                SelectionStatus::Uncollapsed,
            );

            // Collapsed selection.
            dispatch(tester, delegate.handle(), SelectionEvent::Clear);
            let point = position_of(tester, &paragraphs, 0, 3);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(point, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(point, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((3, 3)),
                SelectionStatus::Collapsed,
            );

            // Backwards selection.
            dispatch(tester, delegate.handle(), SelectionEvent::Clear);
            let start = position_of(tester, &paragraphs, 2, 4);
            let end = position_of(tester, &paragraphs, 0, 0);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((30, 0)),
                SelectionStatus::Uncollapsed,
            );

            // Collapsed selection at the first character.
            dispatch(tester, delegate.handle(), SelectionEvent::Clear);
            let point = position_of(tester, &paragraphs, 0, 0);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(point, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(point, None),
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((0, 0)),
                SelectionStatus::Collapsed,
            );
        });
    }

    // The class's own behavior: a selectable added inside the selection is brought up to date
    // with the last edge updates, so it is selected without the user dragging again.
    #[test]
    fn a_selectable_added_inside_the_selection_is_selected_at_once() {
        test_widgets(|tester| {
            let delegate = tester.update(PlainStaticSelectionContainerDelegate::new);
            let pump = |tester: &mut WidgetTester, texts: &[&'static str]| {
                tester.pump_widget(SelectionContainer::new(
                    delegate.erase(),
                    Column::new().children(
                        texts
                            .iter()
                            .map(|text| selectable_text(text))
                            .collect::<Vec<_>>(),
                    ),
                ));
                tester.pump_and_settle();
            };

            pump(tester, &["AAA", "CCC"]);
            let paragraphs = tester.all::<RenderParagraph>();
            let start = position_of(tester, &paragraphs, 0, 0);
            let end = position_of(tester, &paragraphs, 1, 2);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(end, None),
            );
            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("AAACC".to_string())
            );

            // `BBB` takes the second line: the end edge's old global position lands inside it,
            // at the same offset, while `CCC` moves down out of the selection.
            pump(tester, &["AAA", "BBB", "CCC"]);
            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("AAABB".to_string())
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((0, 5)),
                SelectionStatus::Uncollapsed,
            );
        });
    }

    /// Select-all, select-word, select-paragraph and clear, as the region's actions send them.
    #[test]
    fn select_all_a_word_a_paragraph_and_clear_act_on_the_selectables() {
        test_widgets(|tester| {
            let delegate = tester.update(PlainStaticSelectionContainerDelegate::new);
            tester.pump_widget(SelectionContainer::new(
                delegate.erase(),
                Column::new().children([
                    selectable_text("Hello World!"),
                    selectable_text("How are you!"),
                ]),
            ));
            tester.pump_and_settle();

            dispatch(tester, delegate.handle(), SelectionEvent::SelectAll);
            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("Hello World!How are you!".to_string())
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((0, 24)),
                SelectionStatus::Uncollapsed,
            );

            dispatch(tester, delegate.handle(), SelectionEvent::Clear);
            assert_eq!(selected_text(tester, delegate.handle()), None);
            assert_selection(tester, delegate.handle(), None, SelectionStatus::None);
            tester.update(|app| {
                assert_eq!(SelectionHandler::content_length(delegate.handle(), app), 24);
            });

            let paragraphs = tester.all::<RenderParagraph>();
            let word = position_of(tester, &paragraphs, 1, 5);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::SelectWord {
                    global_position: word,
                },
            );
            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("are".to_string())
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((16, 19)),
                SelectionStatus::Uncollapsed,
            );

            let paragraph = position_of(tester, &paragraphs, 1, 5);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::SelectParagraph {
                    global_position: paragraph,
                    absorb: false,
                },
            );
            assert_eq!(
                selected_text(tester, delegate.handle()),
                Some("How are you!".to_string())
            );
            assert_selection(
                tester,
                delegate.handle(),
                Some((12, 24)),
                SelectionStatus::Uncollapsed,
            );
        });
    }
}
