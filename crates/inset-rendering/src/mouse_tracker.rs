//! Flutter counterpart: `rendering/mouse_tracker.dart`.

use std::any::Any;
use std::rc::Rc;

use indexmap::IndexMap;
use inset_embedder::{Matrix4, Offset, PointerDeviceKind, ViewId};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, RetainedHandle};
use inset_gestures::{
    HitTestEntry, HitTestResult, PointerEnterEvent, PointerEvent, PointerExitEvent,
};
use inset_services::{MouseCursorManager, MouseCursorRef, SystemMouseCursors};

use crate::box_::BoxHitTestEntry;
use crate::object::AnyRenderObject;

/// Signature for hit testing at the given offset for the specified view.
///
/// It is used by the [`MouseTracker`] to fetch annotations for the mouse
/// position.
pub type MouseTrackerHitTest = Rc<dyn Fn(&mut App, Offset, ViewId) -> HitTestResult>;

/// The annotations under a mouse, front to back, each with the transform of its hit test
/// entry.
///
/// Dart keys this map by the annotation object. Here the key is the render object that
/// implements the annotation (see [`crate::RenderObject::mouse_tracker_annotation`]), and the
/// annotation itself is read from it when needed.
type Annotations = IndexMap<AnyRenderObject, Matrix4>;

// Various states of a connected mouse device used by [MouseTracker].
struct MouseState {
    /// The retained handles that keep the annotations' render objects past a rebuild.
    retained: Vec<RetainedHandle<AnyRenderObject>>,
    // The list of annotations that contains this device.
    //
    // It uses [IndexMap] to keep the insertion order.
    annotations: Annotations,
    // The most recently processed mouse event observed from this device.
    latest_event: PointerEvent,
}

impl MouseState {
    fn new(initial_event: PointerEvent) -> MouseState {
        MouseState {
            retained: Vec::new(),
            annotations: Annotations::new(),
            latest_event: initial_event,
        }
    }

    fn replace_annotations(
        &mut self,
        value: Annotations,
        retained: Vec<RetainedHandle<AnyRenderObject>>,
    ) -> (Annotations, Vec<RetainedHandle<AnyRenderObject>>) {
        (
            std::mem::replace(&mut self.annotations, value),
            std::mem::replace(&mut self.retained, retained),
        )
    }

    fn replace_latest_event(&mut self, value: PointerEvent) -> PointerEvent {
        debug_assert_eq!(value.device(), self.latest_event.device());
        std::mem::replace(&mut self.latest_event, value)
    }
}

// The details of an update of a mouse device.
//
// This class contains the information needed to handle the update that might
// change the state of a mouse device, or the [MouseTrackerAnnotation]s that
// the mouse device is hovering.
struct MouseTrackerUpdateDetails {
    // The annotations that the device is hovering before the update.
    //
    // It is never null.
    last_annotations: Annotations,
    // The annotations that the device is hovering after the update.
    //
    // It is never null.
    next_annotations: Annotations,
    // The last event that the device observed before the update.
    //
    // If the update is triggered by a frame, the [previous_event] is never null,
    // since the pointer must have been added before it can move.
    //
    // If the update is triggered by a pointer event, the [previous_event] is not
    // null except for cases where the event is the first event observed by the
    // pointer (which is not necessarily a [PointerEvent::Added]).
    previous_event: Option<PointerEvent>,
    // The event that triggered this update.
    //
    // It is non-null if and only if the update is triggered by a pointer event.
    triggering_event: Option<PointerEvent>,
}

impl MouseTrackerUpdateDetails {
    // When device update is triggered by a new frame.
    //
    // All parameters are required.
    fn by_new_frame(
        last_annotations: Annotations,
        next_annotations: Annotations,
        previous_event: PointerEvent,
    ) -> MouseTrackerUpdateDetails {
        MouseTrackerUpdateDetails {
            last_annotations,
            next_annotations,
            previous_event: Some(previous_event),
            triggering_event: None,
        }
    }

    // When device update is triggered by a pointer event.
    //
    // The [last_annotations], [next_annotations], and [triggering_event] are
    // required.
    fn by_pointer_event(
        last_annotations: Annotations,
        next_annotations: Annotations,
        previous_event: Option<PointerEvent>,
        triggering_event: PointerEvent,
    ) -> MouseTrackerUpdateDetails {
        MouseTrackerUpdateDetails {
            last_annotations,
            next_annotations,
            previous_event,
            triggering_event: Some(triggering_event),
        }
    }

    // The pointing device of this update.
    fn device(&self) -> i64 {
        self.previous_event
            .as_ref()
            .or(self.triggering_event.as_ref())
            .expect("an update has an event")
            .device()
    }

    // The last event that the device observed after the update.
    //
    // The [latest_event] is never null.
    fn latest_event(&self) -> &PointerEvent {
        self.triggering_event
            .as_ref()
            .or(self.previous_event.as_ref())
            .expect("an update has an event")
    }
}

/// Tracks the relationship between mouse devices and annotations, and
/// triggers mouse events and cursor changes accordingly.
///
/// The [`MouseTracker`] tracks the relationship between mouse devices and
/// [`MouseTrackerAnnotation`](inset_services::MouseTrackerAnnotation), notified by [`update_with_event`] and
/// [`update_all_devices`]. At every update, [`MouseTracker`] triggers the
/// following changes if applicable:
///
///  * Dispatches mouse-related pointer events (pointer enter, hover, and exit).
///  * Changes mouse cursors.
///  * Notifies when [`mouse_is_connected`] changes.
///
/// This class is a `ChangeNotifier` that notifies its listeners if the value of
/// [`mouse_is_connected`] changes.
///
/// An instance of [`MouseTracker`] is owned by the global singleton
/// `RendererBinding`.
///
/// [`update_with_event`]: MouseTracker::update_with_event
/// [`update_all_devices`]: MouseTracker::update_all_devices
/// [`mouse_is_connected`]: MouseTracker::mouse_is_connected
pub struct MouseTracker {
    change_notifier: ChangeNotifierData,
    hit_test_in_view: MouseTrackerHitTest,
    mouse_cursor_mixin: MouseCursorManager,
    mouse_states: IndexMap<i64, MouseState>,
    debug_during_device_update: bool,
}

impl ChangeNotifier for MouseTracker {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl MouseTracker {
    /// Create a mouse tracker.
    ///
    /// The `hit_test_in_view` is used to find the render objects on a given
    /// position in the specific view. It is typically provided by the
    /// `RendererBinding`.
    pub fn new(app: &mut App, hit_test_in_view: MouseTrackerHitTest) -> Handle<MouseTracker> {
        app.create(MouseTracker {
            change_notifier: ChangeNotifierData::new(),
            hit_test_in_view,
            mouse_cursor_mixin: MouseCursorManager::new(SystemMouseCursors::BASIC.into()),
            mouse_states: IndexMap::new(),
            debug_during_device_update: false,
        })
    }

    /// Discards any resources used by the object. After this is called, the
    /// object is not in a usable state and should be discarded.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let states = std::mem::take(&mut app.get_mut(self).mouse_states);
        for state in states.into_values() {
            MouseTracker::release_annotations(app, state.retained);
        }
        app.get_mut(self).change_notifier.dispose();
    }

    /// The remembered annotations keep their render objects past a rebuild, as Dart's map holds
    /// the annotation objects: an exit is still delivered to a region that just left the tree.
    fn retain_annotations(
        app: &mut App,
        annotations: &Annotations,
    ) -> Vec<RetainedHandle<AnyRenderObject>> {
        annotations
            .keys()
            .copied()
            .map(|target| app.retain(target))
            .collect()
    }

    fn release_annotations(app: &mut App, retained: Vec<RetainedHandle<AnyRenderObject>>) {
        for handle in retained {
            app.release(handle);
        }
    }

    // Whether an observed event might update a device.
    //
    // For example, a [PointerEvent::Move] would always update a device, while
    // a [PointerEvent::Scroll] would not.
    fn should_mark_state_dirty(state: Option<&MouseState>, event: &PointerEvent) -> bool {
        let Some(state) = state else {
            return true;
        };
        let last_event = &state.latest_event;
        debug_assert_eq!(event.device(), last_event.device());
        // An Added can only follow a Removed, and a Removed can only follow an Added.
        debug_assert_eq!(
            matches!(event, PointerEvent::Added(_)),
            matches!(last_event, PointerEvent::Removed(_))
        );
        // Ignore events that are unrelated to mouse tracking.
        if is_signal_event(event) {
            return false;
        }
        matches!(last_event, PointerEvent::Added(_))
            || matches!(event, PointerEvent::Removed(_))
            || last_event.position() != event.position()
    }

    fn hit_test_in_view_result_to_annotations(app: &App, result: &HitTestResult) -> Annotations {
        let mut annotations = Annotations::new();
        for entry in result.path() {
            if let Some(target) = annotation_target(app, entry) {
                annotations.insert(target, entry.transform().expect("an added entry is placed"));
            }
        }
        annotations
    }

    // Find the annotations that is hovered by the device of the `state`.
    //
    // If the device is not connected or not a mouse, an empty map is returned
    // without calling `hit_test_in_view`.
    fn find_annotations(
        self: Handle<Self>,
        app: &mut App,
        latest_event: &PointerEvent,
    ) -> Annotations {
        let global_position = latest_event.position();
        let device = latest_event.device();
        let view_id = latest_event.view_id();
        if !app.get(self).mouse_states.contains_key(&device) {
            return Annotations::new();
        }
        let result = self.run_hit_test(app, global_position, view_id);
        MouseTracker::hit_test_in_view_result_to_annotations(app, &result)
    }

    fn run_hit_test(
        self: Handle<Self>,
        app: &mut App,
        position: Offset,
        view_id: ViewId,
    ) -> HitTestResult {
        let hit_test_in_view = Rc::clone(&app.get(self).hit_test_in_view);
        hit_test_in_view(app, position, view_id)
    }

    // A callback that is called on the update of a device.
    //
    // An event (not necessarily a pointer event) that might change the
    // relationship between mouse devices and [MouseTrackerAnnotation]s is called
    // a _device update_. This method should be called at each such update.
    //
    // The update can be caused by a pointer event, in which case
    // `triggering_event` should not be null, or by other changes, such as when a
    // widget has moved under a still mouse, which is detected after the current
    // frame is complete.
    fn handle_device_update(self: Handle<Self>, app: &mut App, details: MouseTrackerUpdateDetails) {
        debug_assert!(app.get(self).debug_during_device_update);
        MouseTracker::handle_device_update_mouse_events(app, &details);
        let cursors: Vec<MouseCursorRef> = details
            .next_annotations
            .keys()
            .filter_map(|target| target.mouse_tracker_annotation(app))
            .map(|annotation| annotation.cursor)
            .collect();
        let platform = app.platform();
        app.get_mut(self)
            .mouse_cursor_mixin
            .handle_device_cursor_update(
                &*platform,
                details.device(),
                details.triggering_event.as_ref(),
                cursors,
            );
    }

    /// Whether or not at least one mouse is connected and has produced events.
    pub fn mouse_is_connected(self: Handle<Self>, app: &App) -> bool {
        !app.get(self).mouse_states.is_empty()
    }

    /// Perform a device update for one device according to the given new event.
    ///
    /// The [`update_with_event`] is typically called by the `RendererBinding` during
    /// the handler of a pointer event. All pointer events should call this
    /// method, and let [`MouseTracker`] filter which to react to.
    ///
    /// The `hit_test_result` serves as an optional optimization, and is the hit
    /// test result already performed by the `RendererBinding` for other gestures.
    /// It can be `None`, but when it's not `None`, it should be identical to the
    /// result from directly calling `hit_test_in_view` given in the constructor
    /// (which means that it must not use the cached result for
    /// [`PointerEvent::Move`]).
    ///
    /// The `update_with_event` is one of the two ways of updating mouse
    /// states, the other one being [`update_all_devices`].
    ///
    /// [`update_with_event`]: MouseTracker::update_with_event
    /// [`update_all_devices`]: MouseTracker::update_all_devices
    pub fn update_with_event(
        self: Handle<Self>,
        app: &mut App,
        event: &PointerEvent,
        hit_test_result: Option<&HitTestResult>,
    ) {
        if event.kind() != PointerDeviceKind::Mouse && event.kind() != PointerDeviceKind::Stylus {
            // Only track mouse and stylus events.
            return;
        }
        if is_signal_event(event) {
            // Ignore signal events, which are not tracked by the mouse tracker.
            return;
        }
        let is_removed = matches!(event, PointerEvent::Removed(_));
        let owned_result;
        let result: Option<&HitTestResult> = if is_removed {
            None
        } else if let Some(result) = hit_test_result {
            Some(result)
        } else {
            owned_result = self.run_hit_test(app, event.position(), event.view_id());
            Some(&owned_result)
        };
        let device = event.device();
        if !MouseTracker::should_mark_state_dirty(app.get(self).mouse_states.get(&device), event) {
            return;
        }

        self.monitor_mouse_connection(app, |app| {
            self.device_update_phase(app, |app| {
                // Update mouse_states
                if !app.get(self).mouse_states.contains_key(&device) {
                    if is_removed {
                        return;
                    }
                    app.get_mut(self)
                        .mouse_states
                        .insert(device, MouseState::new(event.clone()));
                } else {
                    debug_assert!(!matches!(event, PointerEvent::Added(_)));
                }
                let next_annotations = match result {
                    Some(result) => {
                        MouseTracker::hit_test_in_view_result_to_annotations(app, result)
                    }
                    None => Annotations::new(),
                };
                let target_state = app
                    .get_mut(self)
                    .mouse_states
                    .get_mut(&device)
                    .expect("the state was just found or created");
                let last_event = target_state.replace_latest_event(event.clone());
                let retained = MouseTracker::retain_annotations(app, &next_annotations);
                let target_state = app
                    .get_mut(self)
                    .mouse_states
                    .get_mut(&device)
                    .expect("the state was just found or created");
                let (last_annotations, last_retained) =
                    target_state.replace_annotations(next_annotations.clone(), retained);
                if is_removed {
                    app.get_mut(self).mouse_states.shift_remove(&device);
                }
                self.handle_device_update(
                    app,
                    MouseTrackerUpdateDetails::by_pointer_event(
                        last_annotations,
                        next_annotations,
                        Some(last_event),
                        event.clone(),
                    ),
                );
                MouseTracker::release_annotations(app, last_retained);
            });
        });
    }

    /// Perform a device update for all devices.
    ///
    /// This is called by the `RendererBinding` after a frame, since the render
    /// tree might have changed under a still mouse.
    ///
    /// The `update_all_devices` is one of the two ways of updating mouse
    /// states, the other one being [`update_with_event`].
    ///
    /// [`update_with_event`]: MouseTracker::update_with_event
    pub fn update_all_devices(self: Handle<Self>, app: &mut App) {
        self.device_update_phase(app, |app| {
            let devices: Vec<i64> = app.get(self).mouse_states.keys().copied().collect();
            for device in devices {
                let Some(last_event) = app
                    .get(self)
                    .mouse_states
                    .get(&device)
                    .map(|state| state.latest_event.clone())
                else {
                    continue;
                };
                let next_annotations = self.find_annotations(app, &last_event);
                if !app.get(self).mouse_states.contains_key(&device) {
                    continue;
                }
                let retained = MouseTracker::retain_annotations(app, &next_annotations);
                let Some(dirty_state) = app.get_mut(self).mouse_states.get_mut(&device) else {
                    continue;
                };
                let (last_annotations, last_retained) =
                    dirty_state.replace_annotations(next_annotations.clone(), retained);
                self.handle_device_update(
                    app,
                    MouseTrackerUpdateDetails::by_new_frame(
                        last_annotations,
                        next_annotations,
                        last_event,
                    ),
                );
                MouseTracker::release_annotations(app, last_retained);
            }
        });
    }

    /// Returns the active mouse cursor for a device.
    ///
    /// The return value is the last [`MouseCursorRef`] activated onto this
    /// device, even if the activation failed.
    ///
    /// This function is only active in debug mode: in release it returns `None`.
    pub fn debug_device_active_cursor(
        self: Handle<Self>,
        app: &App,
        device: i64,
    ) -> Option<MouseCursorRef> {
        app.get(self)
            .mouse_cursor_mixin
            .debug_device_active_cursor(device)
    }

    // Used to wrap any procedures that might change `mouse_is_connected`.
    //
    // This method records `mouse_is_connected` before and after the `task`, and
    // notifies listeners if the value changed.
    fn monitor_mouse_connection(self: Handle<Self>, app: &mut App, task: impl FnOnce(&mut App)) {
        let mouse_was_connected = self.mouse_is_connected(app);
        task(app);
        if mouse_was_connected != self.mouse_is_connected(app) {
            self.notify_listeners(app);
        }
    }

    // Used to wrap any procedures that might change the state of a device.
    //
    // A device update, i.e. a change that might affect which annotations a
    // device hovers, must not happen during another device update.
    fn device_update_phase(self: Handle<Self>, app: &mut App, task: impl FnOnce(&mut App)) {
        debug_assert!(!app.get(self).debug_during_device_update);
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_during_device_update = true;
        }
        task(app);
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_during_device_update = false;
        }
    }

    // Handles device update and dispatches mouse event callbacks.
    fn handle_device_update_mouse_events(app: &mut App, details: &MouseTrackerUpdateDetails) {
        let latest_event = details.latest_event();
        let last_annotations = &details.last_annotations;
        let next_annotations = &details.next_annotations;

        // Order is important for mouse event callbacks. The
        // `hit_test_in_view_result_to_annotations` returns annotations in the visual order
        // from front to back, called the "hit-test order". The algorithm here is
        // explained in https://github.com/flutter/flutter/issues/41420

        // Send exit events to annotations that are in last but not in next, in
        // hit-test order.
        let base_exit_event = PointerExitEvent::from_mouse_event(latest_event);
        for (target, transform) in last_annotations {
            if next_annotations.contains_key(target) {
                continue;
            }
            let Some(annotation) = target.mouse_tracker_annotation(app) else {
                continue;
            };
            if annotation.valid_for_mouse_tracker
                && let Some(on_exit) = annotation.on_exit
            {
                on_exit(app, base_exit_event.transformed(Some(*transform)));
            }
        }

        // Send enter events to annotations that are not in last but in next, in
        // reverse hit-test order.
        let entering_annotations: Vec<(AnyRenderObject, Matrix4)> = next_annotations
            .iter()
            .filter(|(target, _)| !last_annotations.contains_key(*target))
            .map(|(target, transform)| (*target, *transform))
            .collect();
        let base_enter_event = PointerEnterEvent::from_mouse_event(latest_event);
        for (target, transform) in entering_annotations.into_iter().rev() {
            let Some(annotation) = target.mouse_tracker_annotation(app) else {
                continue;
            };
            if annotation.valid_for_mouse_tracker
                && let Some(on_enter) = annotation.on_enter
            {
                on_enter(app, base_enter_event.transformed(Some(transform)));
            }
        }
    }
}

/// Dart's `event is PointerSignalEvent`.
fn is_signal_event(event: &PointerEvent) -> bool {
    matches!(
        event,
        PointerEvent::Scroll(_) | PointerEvent::ScrollInertiaCancel(_) | PointerEvent::Scale(_)
    )
}

/// Dart's `entry.target is MouseTrackerAnnotation`: the render object behind a hit test entry,
/// when it implements the annotation.
fn annotation_target(app: &App, entry: &HitTestEntry) -> Option<AnyRenderObject> {
    let target: &dyn Any = entry.target();
    let target = target
        .downcast_ref::<BoxHitTestEntry>()?
        .target()
        .as_object();
    target.mouse_tracker_annotation(app).map(|_| target)
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::{Cell, RefCell};

    use inset_embedder::{
        InertPlatform, MouseCursor, Platform, PlatformRef, Size, SystemMouseCursorKind,
        TargetPlatform, ViewRef,
    };
    use inset_gestures::{
        PointerAddedEvent, PointerHoverEvent, PointerRemovedEvent, PointerScrollEvent,
    };

    use inset_foundation::{Listenable, Listener};

    use super::*;
    use crate::box_::{BoxConstraints, BoxHitTestResult, RenderBox};
    use crate::object::RenderObject;
    use crate::object::RenderObjectWithChildMixin;
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::{RenderConstrainedBox, RenderMouseRegion, RenderRepaintBoundary};
    use crate::shifted_box::RenderPadding;

    /// A host that records the cursor requests it receives.
    struct CursorRecordingPlatform {
        activated: RefCell<Vec<(i64, SystemMouseCursorKind)>>,
    }

    impl Platform for CursorRecordingPlatform {
        fn target_platform(&self) -> TargetPlatform {
            InertPlatform.target_platform()
        }

        fn request_frame(&self) {}

        fn now(&self) -> std::time::Instant {
            InertPlatform.now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }

        fn mouse_cursor(&self) -> Option<&dyn MouseCursor> {
            Some(self)
        }
    }

    impl MouseCursor for CursorRecordingPlatform {
        fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind) {
            self.activated.borrow_mut().push((device, kind));
        }
    }

    fn app_with_cursor_host() -> (Rc<AppCell>, Rc<CursorRecordingPlatform>) {
        let platform = Rc::new(CursorRecordingPlatform {
            activated: RefCell::new(Vec::new()),
        });
        let platform_ref: PlatformRef = Rc::clone(&platform) as PlatformRef;
        (AppCell::with_platform(platform_ref), platform)
    }

    /// A laid-out tree: a 200×200 root holding a mouse region padded 50 on each side, so the
    /// region covers (50..150, 50..150). Returns the root and the region.
    struct RegionTree {
        owner: Handle<PipelineOwner>,
        root: crate::RenderHandle<RenderRepaintBoundary>,
        padding: crate::RenderHandle<RenderPadding>,
        region: crate::RenderHandle<RenderMouseRegion>,
    }

    fn tree_with_region(app: &mut App) -> RegionTree {
        let region = RenderMouseRegion::new(app, true, None);
        let padding = RenderPadding::new(
            app,
            inset_painting::EdgeInsets::all(50.0).into(),
            None,
            Some(region.as_box()),
        );
        let root = RenderRepaintBoundary::new(app, Some(padding.as_box()));
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.schedule_initial_layout(app);
        root.layout(app, BoxConstraints::tight(Size::new(200.0, 200.0)), false);
        RegionTree {
            owner,
            root,
            padding,
            region,
        }
    }

    fn tracker_for(
        app: &mut App,
        root: crate::RenderHandle<RenderRepaintBoundary>,
    ) -> Handle<MouseTracker> {
        MouseTracker::new(
            app,
            Rc::new(move |app: &mut App, position: Offset, _view_id: ViewId| {
                let mut result = HitTestResult::new();
                root.hit_test(app, &mut BoxHitTestResult::wrap(&mut result), position);
                result
            }),
        )
    }

    fn mouse_hover(position: Offset) -> PointerEvent {
        PointerEvent::Hover(PointerHoverEvent {
            kind: PointerDeviceKind::Mouse,
            position,
            ..PointerHoverEvent::default()
        })
    }

    fn mouse_added(position: Offset) -> PointerEvent {
        PointerEvent::Added(PointerAddedEvent {
            kind: PointerDeviceKind::Mouse,
            position,
            ..PointerAddedEvent::default()
        })
    }

    fn mouse_removed(position: Offset) -> PointerEvent {
        PointerEvent::Removed(PointerRemovedEvent {
            kind: PointerDeviceKind::Mouse,
            position,
            ..PointerRemovedEvent::default()
        })
    }

    /// Records the events a region's callbacks receive.
    fn record_events(
        app: &mut App,
        region: crate::RenderHandle<RenderMouseRegion>,
    ) -> Rc<RefCell<Vec<String>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let enter_log = Rc::clone(&log);
        region.set_on_enter(
            app,
            Some(Rc::new(move |_app: &mut App, event: PointerEnterEvent| {
                enter_log
                    .borrow_mut()
                    .push(format!("enter {:?}", event.local_position()));
            })),
        );
        let hover_log = Rc::clone(&log);
        region.set_on_hover(
            app,
            Some(Rc::new(move |_app: &mut App, event: PointerHoverEvent| {
                hover_log
                    .borrow_mut()
                    .push(format!("hover {:?}", event.local_position()));
            })),
        );
        let exit_log = Rc::clone(&log);
        region.set_on_exit(
            app,
            Some(Rc::new(move |_app: &mut App, event: PointerExitEvent| {
                exit_log
                    .borrow_mut()
                    .push(format!("exit {:?}", event.local_position()));
            })),
        );
        log
    }

    #[test]
    fn entering_and_leaving_a_region_sends_enter_then_exit_in_local_coordinates() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let RegionTree { root, region, .. } = tree_with_region(&mut app);
        let tracker = tracker_for(&mut app, root);
        let log = record_events(&mut app, region);

        tracker.update_with_event(&mut app, &mouse_added(Offset::new(10.0, 10.0)), None);
        assert!(log.borrow().is_empty(), "outside the region: nothing");

        tracker.update_with_event(&mut app, &mouse_hover(Offset::new(60.0, 70.0)), None);
        assert_eq!(*log.borrow(), ["enter Offset(10.0, 20.0)"]);

        tracker.update_with_event(&mut app, &mouse_hover(Offset::new(160.0, 70.0)), None);
        assert_eq!(
            *log.borrow(),
            ["enter Offset(10.0, 20.0)", "exit Offset(110.0, 20.0)"]
        );
    }

    #[test]
    fn a_still_mouse_notices_the_tree_moving_under_it_after_a_frame() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let RegionTree {
            owner,
            root,
            padding,
            region,
        } = tree_with_region(&mut app);
        let tracker = tracker_for(&mut app, root);
        let log = record_events(&mut app, region);
        tracker.update_with_event(&mut app, &mouse_added(Offset::new(100.0, 100.0)), None);
        assert_eq!(*log.borrow(), ["enter Offset(50.0, 50.0)"]);

        // Move the region to the right of the still mouse, then run the frame's update.
        padding.set_padding(
            &mut app,
            inset_painting::EdgeInsetsGeometry::only(150.0, 0.0, 0.0, 0.0),
        );
        owner.flush_layout(&mut app);
        tracker.update_all_devices(&mut app);
        assert_eq!(
            *log.borrow(),
            ["enter Offset(50.0, 50.0)", "exit Offset(50.0, 50.0)"],
            "the exit is placed with the transform recorded when the region was entered"
        );
    }

    #[test]
    fn a_region_removed_from_the_tree_sends_no_exit() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let RegionTree {
            owner,
            root,
            padding,
            region,
        } = tree_with_region(&mut app);
        let tracker = tracker_for(&mut app, root);
        let log = record_events(&mut app, region);
        tracker.update_with_event(&mut app, &mouse_added(Offset::new(100.0, 100.0)), None);
        assert_eq!(*log.borrow(), ["enter Offset(50.0, 50.0)"]);

        padding.set_child(&mut app, None);
        owner.flush_layout(&mut app);
        tracker.update_all_devices(&mut app);
        assert_eq!(
            *log.borrow(),
            ["enter Offset(50.0, 50.0)"],
            "Flutter: a detached region is no longer valid for the tracker, so no exit is sent"
        );
    }

    #[test]
    fn a_detached_region_is_invalid_for_the_tracker() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let region = RenderMouseRegion::new(&mut app, true, None);
        assert!(region.valid_for_mouse_tracker(&app));
        let owner = PipelineOwner::new(&mut app, None);
        region.as_object().attach(&mut app, owner);
        assert!(region.valid_for_mouse_tracker(&app));
        region.as_object().detach(&mut app);
        assert!(!region.valid_for_mouse_tracker(&app));
        assert!(
            !region
                .mouse_tracker_annotation(&app)
                .unwrap()
                .valid_for_mouse_tracker
        );
    }

    #[test]
    fn a_removed_mouse_exits_and_disconnects() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let RegionTree { root, region, .. } = tree_with_region(&mut app);
        let tracker = tracker_for(&mut app, root);
        let log = record_events(&mut app, region);
        let notifications = Rc::new(Cell::new(0));
        let counted = Rc::clone(&notifications);
        tracker.add_listener(
            &mut app,
            Listener::new(move |_app| counted.set(counted.get() + 1)),
        );

        assert!(!tracker.mouse_is_connected(&app));
        tracker.update_with_event(&mut app, &mouse_added(Offset::new(100.0, 100.0)), None);
        assert!(tracker.mouse_is_connected(&app));
        assert_eq!(notifications.get(), 1);

        tracker.update_with_event(&mut app, &mouse_removed(Offset::new(100.0, 100.0)), None);
        assert!(!tracker.mouse_is_connected(&app));
        assert_eq!(notifications.get(), 2);
        assert_eq!(
            *log.borrow(),
            ["enter Offset(50.0, 50.0)", "exit Offset(50.0, 50.0)"]
        );
    }

    #[test]
    fn touch_and_signal_events_are_ignored() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let RegionTree { root, region, .. } = tree_with_region(&mut app);
        let tracker = tracker_for(&mut app, root);
        let log = record_events(&mut app, region);

        tracker.update_with_event(
            &mut app,
            &PointerEvent::Hover(PointerHoverEvent {
                position: Offset::new(100.0, 100.0),
                ..PointerHoverEvent::default()
            }),
            None,
        );
        assert!(!tracker.mouse_is_connected(&app), "touch: not tracked");

        tracker.update_with_event(
            &mut app,
            &PointerEvent::Scroll(PointerScrollEvent {
                kind: PointerDeviceKind::Mouse,
                position: Offset::new(100.0, 100.0),
                ..PointerScrollEvent::default()
            }),
            None,
        );
        assert!(!tracker.mouse_is_connected(&app), "a signal: not tracked");
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn the_front_regions_cursor_wins_and_deferring_regions_let_the_one_behind_decide() {
        let (cell, platform) = app_with_cursor_host();
        let mut app = cell.borrow_mut();
        let RegionTree {
            owner,
            root,
            region,
            ..
        } = tree_with_region(&mut app);
        region.set_cursor(&mut app, SystemMouseCursors::TEXT.into());
        // A deferring region in front of the text region, sharing its bounds.
        let front = RenderMouseRegion::new(&mut app, true, None);
        region.set_child(&mut app, Some(front.as_box()));
        owner.flush_layout(&mut app);
        let tracker = tracker_for(&mut app, root);

        tracker.update_with_event(&mut app, &mouse_added(Offset::new(100.0, 100.0)), None);
        assert_eq!(
            *platform.activated.borrow(),
            [(0, SystemMouseCursorKind::Text)]
        );

        front.set_cursor(&mut app, SystemMouseCursors::CLICK.into());
        tracker.update_all_devices(&mut app);
        assert_eq!(
            *platform.activated.borrow(),
            [
                (0, SystemMouseCursorKind::Text),
                (0, SystemMouseCursorKind::Click)
            ]
        );
        let click: MouseCursorRef = SystemMouseCursors::CLICK.into();
        assert!(*tracker.debug_device_active_cursor(&app, 0).unwrap() == *click);

        tracker.update_with_event(&mut app, &mouse_hover(Offset::new(10.0, 10.0)), None);
        assert_eq!(
            platform.activated.borrow().last().unwrap().1,
            SystemMouseCursorKind::Basic,
            "over nothing, the fallback cursor"
        );
    }

    #[test]
    fn an_opaque_region_hides_the_region_behind_it_and_a_transparent_one_does_not() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let back = RenderMouseRegion::new(&mut app, true, None);
        let front = RenderMouseRegion::new(&mut app, true, Some(back.as_box()));
        let root = RenderRepaintBoundary::new(&mut app, Some(front.as_box()));
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(root.as_object()));
        root.schedule_initial_layout(&mut app);
        root.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        let tracker = tracker_for(&mut app, root);
        let back_log = record_events(&mut app, back);
        let front_log = record_events(&mut app, front);

        tracker.update_with_event(&mut app, &mouse_added(Offset::new(10.0, 10.0)), None);
        assert_eq!(*front_log.borrow(), ["enter Offset(10.0, 10.0)"]);
        assert_eq!(
            *back_log.borrow(),
            ["enter Offset(10.0, 10.0)"],
            "the front region's hit test still visits its child; opacity only stops what is behind the region in the parent's hit test"
        );
    }

    #[test]
    fn hover_reaches_the_region_through_handle_event() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let region = RenderMouseRegion::new(&mut app, true, None);
        let log = record_events(&mut app, region);
        let event = mouse_hover(Offset::new(3.0, 4.0));
        let entry = BoxHitTestEntry::new(region.as_box(), Offset::new(3.0, 4.0));
        region.handle_event(&mut app, &event, &entry);
        assert_eq!(*log.borrow(), ["hover Offset(3.0, 4.0)"]);
    }

    #[test]
    fn an_opaque_region_absorbs_the_hit_test_for_what_is_behind_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let region = RenderMouseRegion::new(&mut app, true, None);
        let root = RenderRepaintBoundary::new(&mut app, Some(region.as_box()));
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(root.as_object()));
        root.schedule_initial_layout(&mut app);
        root.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );

        let mut result = HitTestResult::new();
        assert!(region.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0)
        ));
        region.set_opaque(&mut app, false);
        let mut result = HitTestResult::new();
        assert!(!region.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0)
        ));
        assert_eq!(
            result.path().len(),
            1,
            "the region is still on the path; it just does not claim the hit"
        );
    }

    #[test]
    fn a_region_with_no_annotation_neighbours_still_uses_the_fallback_cursor() {
        let (cell, platform) = app_with_cursor_host();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        let root = RenderRepaintBoundary::new(&mut app, Some(child.as_box()));
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(root.as_object()));
        root.schedule_initial_layout(&mut app);
        root.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        let tracker = tracker_for(&mut app, root);
        tracker.update_with_event(&mut app, &mouse_added(Offset::new(5.0, 5.0)), None);
        assert_eq!(
            *platform.activated.borrow(),
            [(0, SystemMouseCursorKind::Basic)]
        );
    }
}
