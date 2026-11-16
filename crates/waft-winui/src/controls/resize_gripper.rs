//! `ResizeGripper` (`controls/dev/ResizeGripper`: `ResizeGripper.xaml` and `ResizeGripper.cpp`): a drag handle that owns its gesture, its presentation and its keyboard step, and owns no value and no bounds. It reports how far the user has dragged; its host clamps that and applies it.
//!
//! The gesture lives in a [`ResizeGripperDrag`] the host makes and keeps, so the host can end a drag in flight (Escape) and step it from a key the host's own element received. A later editor must keep [`ResizeGripperDrag::end_drag`] reachable from every way a gesture can stop, or `is_dragging` stays true and every later `begin_drag` is refused.

use crate::*;
use std::rc::Rc;
use waft_embedder::Color;
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, ListenableObject, Listener, Ref,
};
use waft_painting::Axis;
use waft_rendering::HitTestBehavior;
use waft_services::LogicalKeyboardKey;
use waft_widgets::*;

/// `c_dragDeadband`: sub-pixel jitter does not reach the host. DIPs.
const DRAG_DEADBAND: f64 = 0.5;

/// `c_largeIncrementMultiplier`: Shift takes a coarser keyboard step.
const LARGE_INCREMENT_MULTIPLIER: f64 = 4.0;

/// `c_defaultKeyboardIncrement`.
pub(crate) const DEFAULT_KEYBOARD_INCREMENT: f64 = 8.0;

/// `DragDelta`'s handler: the change since the last raise, and the offset from where the drag began.
pub(crate) type ResizeGripperDragDelta = Rc<dyn Fn(&mut App, f64, f64)>;

/// `DragCompleted`'s handler: the offset, and whether the gesture was torn down rather than released.
pub(crate) type ResizeGripperDragCompleted = Rc<dyn Fn(&mut App, f64, bool)>;

/// What a gripper raises: `DragStarted`, `DragDelta` (the change since the last raise and the offset from where the drag began) and `DragCompleted` (the offset, and whether the gesture was torn down rather than released).
pub(crate) struct ResizeGripperEvents {
    pub(crate) drag_started: Rc<dyn Fn(&mut App)>,
    pub(crate) drag_delta: ResizeGripperDragDelta,
    pub(crate) drag_completed: ResizeGripperDragCompleted,
}

/// One gripper's gesture: `IsDragging`, the last offset raised, and the events its host handles.
pub(crate) struct ResizeGripperDrag {
    notifier: ChangeNotifierData,
    /// `IsDragging`.
    is_dragging: bool,
    /// `m_lastRaisedDelta`.
    last_raised_delta: f64,
    /// `DragOrientation`: the axis the drag is measured along.
    drag_orientation: Axis,
    /// `KeyboardIncrement`.
    keyboard_increment: f64,
    events: ResizeGripperEvents,
}

impl ResizeGripperDrag {
    pub(crate) fn new(
        app: &mut App,
        drag_orientation: Axis,
        events: ResizeGripperEvents,
    ) -> Ref<Self> {
        app.create(Self {
            notifier: ChangeNotifierData::new(),
            is_dragging: false,
            last_raised_delta: 0.0,
            drag_orientation,
            keyboard_increment: DEFAULT_KEYBOARD_INCREMENT,
            events,
        })
    }

    pub(crate) fn dispose(self: Handle<Self>, app: &mut App) {
        ChangeNotifierData::dispose(app, self);
    }

    /// `IsDragging`.
    pub(crate) fn is_dragging(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_dragging
    }

    fn is_horizontal_drag(self: Handle<Self>, app: &App) -> bool {
        app.get(self).drag_orientation == Axis::Horizontal
    }

    /// `BeginDrag`.
    pub(crate) fn begin_drag(self: Handle<Self>, app: &mut App) {
        if app.get(self).is_dragging {
            return;
        }
        let state = app.get_mut(self);
        state.last_raised_delta = 0.0;
        state.is_dragging = true;
        self.notify_listeners(app);
        let started = app.get(self).events.drag_started.clone();
        started(app);
    }

    /// `TryDrag`: raises `DragDelta` when the offset moved past the dead band.
    pub(crate) fn try_drag(self: Handle<Self>, app: &mut App, total_delta: f64) -> bool {
        let state = app.get(self);
        if !state.is_dragging || !total_delta.is_finite() {
            return false;
        }
        if (total_delta - state.last_raised_delta).abs() < DRAG_DEADBAND {
            return false;
        }
        let delta = total_delta - state.last_raised_delta;
        app.get_mut(self).last_raised_delta = total_delta;
        let raise = app.get(self).events.drag_delta.clone();
        raise(app, delta, total_delta);
        true
    }

    /// `EndDrag`: the flag is cleared before `DragCompleted` is raised, so a handler cannot wedge it.
    pub(crate) fn end_drag(self: Handle<Self>, app: &mut App, canceled: bool) {
        if !app.get(self).is_dragging {
            return;
        }
        let total_delta = app.get(self).last_raised_delta;
        app.get_mut(self).is_dragging = false;
        self.notify_listeners(app);
        let completed = app.get(self).events.drag_completed.clone();
        completed(app, total_delta, canceled);
    }

    /// `EffectiveKeyboardIncrement`.
    fn effective_keyboard_increment(self: Handle<Self>, app: &App) -> f64 {
        let raw = app.get(self).keyboard_increment.abs();
        if !raw.is_finite() || raw == 0.0 {
            return DEFAULT_KEYBOARD_INCREMENT;
        }
        raw.max(DRAG_DEADBAND)
    }

    /// `TryKeyboardStep`: one arrow key on the drag's axis as a complete one-step drag, mirrored in right-to-left layout. False when the key is not on this axis.
    pub(crate) fn try_keyboard_step(
        self: Handle<Self>,
        app: &mut App,
        key: LogicalKeyboardKey,
        right_to_left: bool,
        shift: bool,
    ) -> bool {
        let horizontal = self.is_horizontal_drag(app);
        let mut direction = match key {
            LogicalKeyboardKey::ARROW_LEFT if horizontal => -1.0,
            LogicalKeyboardKey::ARROW_RIGHT if horizontal => 1.0,
            LogicalKeyboardKey::ARROW_UP if !horizontal => -1.0,
            LogicalKeyboardKey::ARROW_DOWN if !horizontal => 1.0,
            _ => return false,
        };
        // A pointer drag in flight keeps its anchor; the key is still reported as handled.
        if self.is_dragging(app) {
            return true;
        }
        if horizontal && right_to_left {
            direction = -direction;
        }
        let increment = self.effective_keyboard_increment(app);
        let step = if shift {
            increment * LARGE_INCREMENT_MULTIPLIER
        } else {
            increment
        };
        self.begin_drag(app);
        self.try_drag(app, direction * step);
        self.end_drag(app, false);
        true
    }
}

impl std::fmt::Debug for ResizeGripperDrag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResizeGripperDrag")
            .field("is_dragging", &self.is_dragging)
            .field("drag_orientation", &self.drag_orientation)
            .finish_non_exhaustive()
    }
}

impl ChangeNotifier for ResizeGripperDrag {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}

/// XAML `ResizeGripper`: a transparent hit target whose `Separator` shows while the pointer is over it or a drag is in flight. The host sizes it.
#[derive(Clone, Debug)]
pub(crate) struct ResizeGripper {
    /// The gesture, kept by the host.
    pub(crate) drag: Ref<ResizeGripperDrag>,
}

impl ResizeGripper {
    pub(crate) fn new(drag: Ref<ResizeGripperDrag>) -> Self {
        Self { drag }
    }
}

impl StatefulWidget for ResizeGripper {
    type State = ResizeGripperState;

    fn create_state(&self) -> ResizeGripperState {
        ResizeGripperState {
            state: StateData::new(),
            is_pointer_over: false,
            drag_start: 0.0,
            listening: None,
        }
    }
}

pub(crate) struct ResizeGripperState {
    state: StateData<ResizeGripper>,
    /// `m_isPointerOver`.
    is_pointer_over: bool,
    /// Where along the drag's axis, in global coordinates, the drag began: the manipulation container's frame, which does not move while the drag resizes the column.
    drag_start: f64,
    listening: Option<(Ref<ResizeGripperDrag>, Listener)>,
}

impl ResizeGripperState {
    fn drag(self: Handle<Self>, app: &App) -> Handle<ResizeGripperDrag> {
        self.widget(app).drag.handle()
    }

    fn listen(self: Handle<Self>, app: &mut App) {
        let drag = self.widget(app).drag.clone();
        if app
            .get(self)
            .listening
            .as_ref()
            .is_some_and(|(listened, _)| listened.handle() == drag.handle())
        {
            return;
        }
        self.stop_listening(app);
        let listener = Listener::new(move |app| {
            if app.contains(self) && self.mounted(app) {
                self.set_state(app, |_| {});
            }
        });
        drag.add_listener(app, listener.clone());
        app.get_mut(self).listening = Some((drag, listener));
    }

    fn stop_listening(self: Handle<Self>, app: &mut App) {
        if let Some((drag, listener)) = app.get_mut(self).listening.take()
            && app.contains(drag.handle())
        {
            drag.remove_listener(app, &listener);
        }
    }

    fn set_pointer_over(self: Handle<Self>, app: &mut App, over: bool) {
        self.set_state(app, |state| state.is_pointer_over = over);
    }

    /// `OnManipulationStarted`.
    fn drag_started(self: Handle<Self>, app: &mut App, global: f64) {
        app.get_mut(self).drag_start = global;
        self.drag(app).begin_drag(app);
    }

    /// `OnManipulationDelta`: the offset from where the drag began, in reading order.
    fn drag_updated(self: Handle<Self>, app: &mut App, global: f64) {
        let total = global - app.get(self).drag_start;
        self.drag(app).try_drag(app, total);
    }
}

impl State for ResizeGripperState {
    type Widget = ResizeGripper;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.listen(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &ResizeGripper) {
        self.listen(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        // `OnUnloaded`: detached mid-gesture, no completion will arrive.
        let drag = self.drag(app);
        self.stop_listening(app);
        if app.contains(drag) {
            drag.end_drag(app, true);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let drag = self.drag(app);
        let horizontal = drag.is_horizontal_drag(app);
        // `UpdateVisualState`: Pressed while dragging, PointerOver while hovered; both show the separator.
        let shows_separator = drag.is_dragging(app) || app.get(self).is_pointer_over;
        // `OrientationStates`.
        let thickness = if horizontal {
            r.resize_gripper_separator_thickness
        } else {
            r.resize_gripper_separator_thickness_vertical
        };
        // Separator
        let separator = IgnorePointer::new().child(
            Opacity::new(if shows_separator { 1.0 } else { 0.0 }).child(
                ControlBorder::new(
                    Brush::Solid(Color::new(0)),
                    Brush::Solid(r.resize_gripper_separator_brush),
                )
                .border_thickness_ltrb(thickness),
            ),
        );
        // LayoutRoot: `Background="Transparent"` so the whole gripper is a pointer target.
        let layout_root = ColoredBox::new(Color::new(0)).child(SizedBox::expand().child(separator));
        let mut gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            // `OnPointerPressed` marks the press handled: a tap on the gripper is the gripper's.
            .on_tap(Listener::new(|_| {}))
            .child(layout_root);
        let along = move |offset: waft_embedder::Offset| {
            if horizontal { offset.dx() } else { offset.dy() }
        };
        let started = Rc::new(move |app: &mut App, global: f64| self.drag_started(app, global));
        let updated = Rc::new(move |app: &mut App, global: f64| self.drag_updated(app, global));
        let completed = Listener::new(move |app| self.drag(app).end_drag(app, false));
        let canceled = Listener::new(move |app| self.drag(app).end_drag(app, true));
        gesture = if horizontal {
            gesture
                .on_horizontal_drag_start(Rc::new(move |app, details| {
                    started(app, along(details.global_position))
                }))
                .on_horizontal_drag_update(Rc::new(move |app, details| {
                    updated(app, along(details.global_position))
                }))
                .on_horizontal_drag_end(Rc::new(move |app, _| completed.call(app)))
                .on_horizontal_drag_cancel(canceled)
        } else {
            gesture
                .on_vertical_drag_start(Rc::new(move |app, details| {
                    started(app, along(details.global_position))
                }))
                .on_vertical_drag_update(Rc::new(move |app, details| {
                    updated(app, along(details.global_position))
                }))
                .on_vertical_drag_end(Rc::new(move |app, _| completed.call(app)))
                .on_vertical_drag_cancel(canceled)
        };
        MouseRegion::new()
            .on_enter(Rc::new(move |app, _| self.set_pointer_over(app, true)))
            .on_exit(Rc::new(move |app, _| self.set_pointer_over(app, false)))
            .child(gesture)
            .into_widget()
    }
}
