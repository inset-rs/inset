//! Flutter counterpart: `widgets/drag_target.dart`.

use std::any::Any;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{Offset, ViewId};
use inset_foundation::{App, Handle, HandleId, Listener, RetainedHandleId};
use inset_gestures::*;
use inset_painting::Axis;
use inset_rendering::{BoxHitTestEntry, HitTestBehavior, RenderMetaData};
use inset_services::HapticFeedback;

use crate::*;

/// Signature for determining whether a drag target will accept the given data.
pub type DragTargetWillAccept<T> = Rc<dyn Fn(&mut App, Option<Rc<T>>) -> bool>;

/// Signature for determining whether a drag target will accept the given drag details.
pub type DragTargetWillAcceptWithDetails<T> = Rc<dyn Fn(&mut App, DragTargetDetails<T>) -> bool>;

/// Signature for causing a drag target to accept the given data.
pub type DragTargetAccept<T> = Rc<dyn Fn(&mut App, Rc<T>)>;

/// Signature for causing a drag target to accept the given drag details.
pub type DragTargetAcceptWithDetails<T> = Rc<dyn Fn(&mut App, DragTargetDetails<T>)>;

/// Signature for building a drag target from its candidate and rejected data.
pub type DragTargetBuilder<T> =
    Rc<dyn Fn(&mut App, BuildContext, Vec<Option<Rc<T>>>, Vec<Option<Rc<dyn Any>>>) -> WidgetRef>;

/// Signature for when a Draggable is dragged across the screen.
pub type DragUpdateCallback = Rc<dyn Fn(&mut App, DragUpdateDetails)>;

/// Signature for a draggable dropped without being accepted by a target.
pub type DraggableCanceledCallback = Rc<dyn Fn(&mut App, Velocity, Offset)>;

/// Signature for when the draggable is dropped.
pub type DragEndCallback = Rc<dyn Fn(&mut App, DraggableDetails)>;

/// Signature for when a Draggable leaves a DragTarget.
pub type DragTargetLeave<T> = Rc<dyn Fn(&mut App, Option<Rc<T>>)>;

/// Signature for when a Draggable moves within a DragTarget.
pub type DragTargetMove<T> = Rc<dyn Fn(&mut App, DragTargetDetails<T>)>;

/// Signature for the strategy that determines the drag start point.
pub type DragAnchorStrategy<T> =
    Rc<dyn Fn(&mut App, &Draggable<T>, BuildContext, Offset) -> Offset>;

/// Display the feedback anchored at the position of the original child.
pub fn child_drag_anchor_strategy<T: 'static>(
    app: &mut App,
    _draggable: &Draggable<T>,
    context: BuildContext,
    position: Offset,
) -> Offset {
    context
        .find_render_object(app)
        .unwrap()
        .as_box()
        .unwrap()
        .global_to_local(app, position, None)
}

/// Display the feedback anchored at the position of the touch that started the drag.
pub fn pointer_drag_anchor_strategy<T: 'static>(
    _app: &mut App,
    _draggable: &Draggable<T>,
    _context: BuildContext,
    _position: Offset,
) -> Offset {
    Offset::ZERO
}

/// A widget that can be dragged from to a DragTarget.
///
/// Active drags and their feedback survive removal of the source widget.
pub struct Draggable<T: 'static> {
    /// Controls how one widget replaces another widget in the tree.
    pub key: Option<KeyRef>,

    /// The data that was dropped onto this `DragTarget`.
    pub data: Option<Rc<T>>,

    /// The `Axis` to restrict this draggable's movement, if specified.
    ///
    /// When axis is set to `Axis.horizontal`, this widget can only be dragged
    /// horizontally. Behavior is similar for `Axis.vertical`.
    ///
    /// Defaults to allowing drag on both `Axis.horizontal` and `Axis.vertical`.
    ///
    /// When null, allows drag on both `Axis.horizontal` and `Axis.vertical`.
    ///
    /// For the direction of gestures this widget competes with to start a drag
    /// event, see `Draggable.affinity`.
    pub axis: Option<Axis>,

    /// The widget below this widget in the tree.
    ///
    /// This widget displays `child` when zero drags are under way. If
    /// `childWhenDragging` is non-null, this widget instead displays
    /// `childWhenDragging` when one or more drags are underway. Otherwise, this
    /// widget always displays `child`.
    ///
    /// The `feedback` widget is shown under the pointer when a drag is under way.
    ///
    /// To limit the number of simultaneous drags on multitouch devices, see
    /// `maxSimultaneousDrags`.
    ///
    /// {@macro flutter.widgets.ProxyWidget.child}
    pub child: WidgetRef,

    /// The widget to display instead of `child` when one or more drags are under way.
    ///
    /// If this is null, then this widget will always display `child` (and so the
    /// drag source representation will not change while a drag is under
    /// way).
    ///
    /// The `feedback` widget is shown under the pointer when a drag is under way.
    ///
    /// To limit the number of simultaneous drags on multitouch devices, see
    /// `maxSimultaneousDrags`.
    pub child_when_dragging: Option<WidgetRef>,

    /// The widget to show under the pointer when a drag is under way.
    ///
    /// See `child` and `childWhenDragging` for information about what is shown
    /// at the location of the `Draggable` itself when a drag is under way.
    pub feedback: WidgetRef,

    /// The feedbackOffset can be used to set the hit test target point for the
    /// purposes of finding a drag target. It is especially useful if the feedback
    /// is transformed compared to the child.
    pub feedback_offset: Offset,

    /// A strategy that is used by this draggable to get the anchor offset when it
    /// is dragged.
    ///
    /// The anchor offset refers to the distance between the users' fingers and
    /// the `feedback` widget when this draggable is dragged.
    ///
    /// This property's value is a function that implements `DragAnchorStrategy`.
    /// There are two built-in functions that can be used:
    ///
    ///  * `childDragAnchorStrategy`, which displays the feedback anchored at the
    ///    position of the original child.
    ///
    ///  * `pointerDragAnchorStrategy`, which displays the feedback anchored at the
    ///    position of the touch that started the drag.
    ///
    /// Defaults to `childDragAnchorStrategy`.
    pub drag_anchor_strategy: DragAnchorStrategy<T>,

    /// Whether the `feedback` widget is ignored during hit testing.
    ///
    /// Regardless of whether this widget is ignored during hit testing, it will
    /// still consume space during layout and be visible during painting.
    ///
    /// Defaults to true.
    pub ignoring_feedback_pointer: bool,

    /// Controls how this widget competes with other gestures to initiate a drag.
    ///
    /// If affinity is null, this widget initiates a drag as soon as it recognizes
    /// a tap down gesture, regardless of any directionality. If affinity is
    /// horizontal (or vertical), then this widget will compete with other
    /// horizontal (or vertical, respectively) gestures.
    ///
    /// For example, if this widget is placed in a vertically scrolling region and
    /// has horizontal affinity, pointer motion in the vertical direction will
    /// result in a scroll and pointer motion in the horizontal direction will
    /// result in a drag. Conversely, if the widget has a null or vertical
    /// affinity, pointer motion in any direction will result in a drag rather
    /// than in a scroll because the draggable widget, being the more specific
    /// widget, will out-compete the `Scrollable` for vertical gestures.
    ///
    /// For the directions this widget can be dragged in after the drag event
    /// starts, see `Draggable.axis`.
    pub affinity: Option<Axis>,

    /// How many simultaneous drags to support.
    ///
    /// When null, no limit is applied. Set this to 1 if you want to only allow
    /// the drag source to have one item dragged at a time. Set this to 0 if you
    /// want to prevent the draggable from actually being dragged.
    ///
    /// If you set this property to 1, consider supplying an "empty" widget for
    /// `childWhenDragging` to create the illusion of actually moving `child`.
    pub max_simultaneous_drags: Option<usize>,

    /// Called when the draggable starts being dragged.
    pub on_drag_started: Option<Listener>,

    /// Called when the draggable is dragged.
    ///
    /// This function will only be called while this widget is still mounted to
    /// the tree (i.e. `State.mounted` is true), and if this widget has actually moved.
    pub on_drag_update: Option<DragUpdateCallback>,

    /// Called when the draggable is dropped without being accepted by a `DragTarget`.
    ///
    /// This function might be called after this widget has been removed from the
    /// tree. For example, if a drag was in progress when this widget was removed
    /// from the tree and the drag ended up being canceled, this callback will
    /// still be called. For this reason, implementations of this callback might
    /// need to check `State.mounted` to check whether the state receiving the
    /// callback is still in the tree.
    pub on_draggable_canceled: Option<DraggableCanceledCallback>,

    /// Called when the draggable is dropped and accepted by a `DragTarget`.
    ///
    /// This function might be called after this widget has been removed from the
    /// tree. For example, if a drag was in progress when this widget was removed
    /// from the tree and the drag ended up completing, this callback will
    /// still be called. For this reason, implementations of this callback might
    /// need to check `State.mounted` to check whether the state receiving the
    /// callback is still in the tree.
    pub on_drag_completed: Option<Listener>,

    /// Called when the draggable is dropped.
    ///
    /// The velocity and offset at which the pointer was moving when it was
    /// dropped is available in the `DraggableDetails`. Also included in the
    /// `details` is whether the draggable's `DragTarget` accepted it.
    ///
    /// This function will only be called while this widget is still mounted to
    /// the tree (i.e. `State.mounted` is true).
    pub on_drag_end: Option<DragEndCallback>,

    /// Whether the feedback widget will be put on the root `Overlay`.
    ///
    /// When false, the feedback widget will be put on the closest `Overlay`. When
    /// true, the `feedback` widget will be put on the farthest (aka root)
    /// `Overlay`.
    ///
    /// Defaults to false.
    pub root_overlay: bool,

    /// How to behave during hit testing.
    ///
    /// Defaults to `HitTestBehavior.translucent`.
    pub hit_test_behavior: HitTestBehavior,

    /// {@macro flutter.gestures.multidrag._allowedButtonsFilter}
    pub allowed_buttons_filter: Option<AllowedButtonsFilter>,
}

impl<T: 'static> Clone for Draggable<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            data: self.data.clone(),
            axis: self.axis,
            child: self.child.clone(),
            child_when_dragging: self.child_when_dragging.clone(),
            feedback: self.feedback.clone(),
            feedback_offset: self.feedback_offset,
            drag_anchor_strategy: self.drag_anchor_strategy.clone(),
            ignoring_feedback_pointer: self.ignoring_feedback_pointer,
            affinity: self.affinity,
            max_simultaneous_drags: self.max_simultaneous_drags,
            on_drag_started: self.on_drag_started.clone(),
            on_drag_update: self.on_drag_update.clone(),
            on_draggable_canceled: self.on_draggable_canceled.clone(),
            on_drag_completed: self.on_drag_completed.clone(),
            on_drag_end: self.on_drag_end.clone(),
            root_overlay: self.root_overlay,
            hit_test_behavior: self.hit_test_behavior,
            allowed_buttons_filter: self.allowed_buttons_filter.clone(),
        }
    }
}

impl<T: 'static> fmt::Debug for Draggable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Draggable").finish_non_exhaustive()
    }
}

impl<T: 'static> Draggable<T> {
    /// Creates a draggable with its normal child and drag feedback.
    pub fn new<C, F>(child: impl IntoWidget<C>, feedback: impl IntoWidget<F>) -> Self {
        Self {
            key: None,
            data: None,
            axis: None,
            child: child.into_widget(),
            child_when_dragging: None,
            feedback: feedback.into_widget(),
            feedback_offset: Offset::ZERO,
            drag_anchor_strategy: Rc::new(child_drag_anchor_strategy::<T>),
            ignoring_feedback_pointer: true,
            affinity: None,
            max_simultaneous_drags: None,
            on_drag_started: None,
            on_drag_update: None,
            on_draggable_canceled: None,
            on_drag_completed: None,
            on_drag_end: None,
            root_overlay: false,
            hit_test_behavior: HitTestBehavior::DeferToChild,
            allowed_buttons_filter: None,
        }
    }

    /// Sets `key` for this draggable.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }

    /// Sets `data` for this draggable.
    pub fn data(mut self, value: T) -> Self {
        self.data = Some(Rc::new(value));
        self
    }

    /// Sets `axis` for this draggable.
    pub fn axis(mut self, value: Axis) -> Self {
        self.axis = Some(value);
        self
    }

    /// Sets `child_when_dragging` for this draggable.
    pub fn child_when_dragging<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.child_when_dragging = Some(value.into_widget());
        self
    }

    /// Sets `feedback_offset` for this draggable.
    pub fn feedback_offset(mut self, value: Offset) -> Self {
        self.feedback_offset = value;
        self
    }

    /// Sets `drag_anchor_strategy` for this draggable.
    pub fn drag_anchor_strategy(mut self, value: DragAnchorStrategy<T>) -> Self {
        self.drag_anchor_strategy = value;
        self
    }

    /// Sets `ignoring_feedback_pointer` for this draggable.
    pub fn ignoring_feedback_pointer(mut self, value: bool) -> Self {
        self.ignoring_feedback_pointer = value;
        self
    }

    /// Sets `affinity` for this draggable.
    pub fn affinity(mut self, value: Axis) -> Self {
        self.affinity = Some(value);
        self
    }

    /// Sets `max_simultaneous_drags` for this draggable.
    pub fn max_simultaneous_drags(mut self, value: usize) -> Self {
        self.max_simultaneous_drags = Some(value);
        self
    }

    /// Sets `on_drag_started` for this draggable.
    pub fn on_drag_started(mut self, value: Listener) -> Self {
        self.on_drag_started = Some(value);
        self
    }

    /// Sets `on_drag_update` for this draggable.
    pub fn on_drag_update(mut self, value: DragUpdateCallback) -> Self {
        self.on_drag_update = Some(value);
        self
    }

    /// Sets `on_draggable_canceled` for this draggable.
    pub fn on_draggable_canceled(mut self, value: DraggableCanceledCallback) -> Self {
        self.on_draggable_canceled = Some(value);
        self
    }

    /// Sets `on_drag_completed` for this draggable.
    pub fn on_drag_completed(mut self, value: Listener) -> Self {
        self.on_drag_completed = Some(value);
        self
    }

    /// Sets `on_drag_end` for this draggable.
    pub fn on_drag_end(mut self, value: DragEndCallback) -> Self {
        self.on_drag_end = Some(value);
        self
    }

    /// Sets `root_overlay` for this draggable.
    pub fn root_overlay(mut self, value: bool) -> Self {
        self.root_overlay = value;
        self
    }

    /// Sets `hit_test_behavior` for this draggable.
    pub fn hit_test_behavior(mut self, value: HitTestBehavior) -> Self {
        self.hit_test_behavior = value;
        self
    }

    /// Sets `allowed_buttons_filter` for this draggable.
    pub fn allowed_buttons_filter(mut self, value: AllowedButtonsFilter) -> Self {
        self.allowed_buttons_filter = Some(value);
        self
    }
}

/// The Draggable superclass interface; a subclass overrides `create_recognizer`.
pub trait DraggableWidget: StatefulWidget<State = DraggableState<Self>> + Clone {
    type Data: 'static;

    fn draggable(&self) -> &Draggable<Self::Data>;

    fn create_recognizer(
        &self,
        app: &mut App,
        on_start: GestureMultiDragStartCallback,
    ) -> AnyGestureRecognizer {
        let draggable = self.draggable();
        macro_rules! create {
            ($ty:ty) => {{
                let recognizer = <$ty>::new(app);
                if let Some(filter) = draggable.allowed_buttons_filter.clone() {
                    recognizer.set_allowed_buttons_filter(app, filter);
                }
                recognizer.set_on_start(app, Some(on_start));
                recognizer.as_recognizer()
            }};
        }
        match draggable.affinity {
            Some(Axis::Horizontal) => create!(HorizontalMultiDragGestureRecognizer),
            Some(Axis::Vertical) => create!(VerticalMultiDragGestureRecognizer),
            None => create!(ImmediateMultiDragGestureRecognizer),
        }
    }
}

impl<T: 'static> DraggableWidget for Draggable<T> {
    type Data = T;

    fn draggable(&self) -> &Self {
        self
    }
}

impl<T: 'static> StatefulWidget for Draggable<T> {
    type State = DraggableState<Self>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        DraggableState::new()
    }
}

/// Makes its child draggable starting from long press.
pub struct LongPressDraggable<T: 'static> {
    /// The Draggable superclass configuration.
    pub draggable: Draggable<T>,

    /// Whether haptic feedback should be triggered on drag start.
    pub haptic_feedback_on_start: bool,

    /// The duration that a user has to press down before a long press is registered.
    ///
    /// Defaults to `kLongPressTimeout`.
    pub delay: Duration,
}

impl<T: 'static> Clone for LongPressDraggable<T> {
    fn clone(&self) -> Self {
        Self {
            draggable: self.draggable.clone(),
            haptic_feedback_on_start: self.haptic_feedback_on_start,
            delay: self.delay,
        }
    }
}

impl<T: 'static> fmt::Debug for LongPressDraggable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LongPressDraggable").finish_non_exhaustive()
    }
}

impl<T: 'static> LongPressDraggable<T> {
    /// Creates a draggable with its normal child and drag feedback.
    pub fn new<C, F>(child: impl IntoWidget<C>, feedback: impl IntoWidget<F>) -> Self {
        Self {
            draggable: Draggable::new(child, feedback),
            haptic_feedback_on_start: true,
            delay: K_LONG_PRESS_TIMEOUT,
        }
    }

    /// Sets `delay` for this draggable.
    pub fn delay(mut self, value: Duration) -> Self {
        self.delay = value;
        self
    }

    /// Sets `haptic_feedback_on_start` for this draggable.
    pub fn haptic_feedback_on_start(mut self, value: bool) -> Self {
        self.haptic_feedback_on_start = value;
        self
    }

    /// Sets `key` for this draggable.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.draggable.key = Some(value);
        self
    }

    /// Sets `data` for this draggable.
    pub fn data(mut self, value: T) -> Self {
        self.draggable.data = Some(Rc::new(value));
        self
    }

    /// Sets `axis` for this draggable.
    pub fn axis(mut self, value: Axis) -> Self {
        self.draggable.axis = Some(value);
        self
    }

    /// Sets `child_when_dragging` for this draggable.
    pub fn child_when_dragging<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.draggable.child_when_dragging = Some(value.into_widget());
        self
    }

    /// Sets `feedback_offset` for this draggable.
    pub fn feedback_offset(mut self, value: Offset) -> Self {
        self.draggable.feedback_offset = value;
        self
    }

    /// Sets `drag_anchor_strategy` for this draggable.
    pub fn drag_anchor_strategy(mut self, value: DragAnchorStrategy<T>) -> Self {
        self.draggable.drag_anchor_strategy = value;
        self
    }

    /// Sets `ignoring_feedback_pointer` for this draggable.
    pub fn ignoring_feedback_pointer(mut self, value: bool) -> Self {
        self.draggable.ignoring_feedback_pointer = value;
        self
    }

    /// Sets `max_simultaneous_drags` for this draggable.
    pub fn max_simultaneous_drags(mut self, value: usize) -> Self {
        self.draggable.max_simultaneous_drags = Some(value);
        self
    }

    /// Sets `on_drag_started` for this draggable.
    pub fn on_drag_started(mut self, value: Listener) -> Self {
        self.draggable.on_drag_started = Some(value);
        self
    }

    /// Sets `on_drag_update` for this draggable.
    pub fn on_drag_update(mut self, value: DragUpdateCallback) -> Self {
        self.draggable.on_drag_update = Some(value);
        self
    }

    /// Sets `on_draggable_canceled` for this draggable.
    pub fn on_draggable_canceled(mut self, value: DraggableCanceledCallback) -> Self {
        self.draggable.on_draggable_canceled = Some(value);
        self
    }

    /// Sets `on_drag_completed` for this draggable.
    pub fn on_drag_completed(mut self, value: Listener) -> Self {
        self.draggable.on_drag_completed = Some(value);
        self
    }

    /// Sets `on_drag_end` for this draggable.
    pub fn on_drag_end(mut self, value: DragEndCallback) -> Self {
        self.draggable.on_drag_end = Some(value);
        self
    }

    /// Sets `root_overlay` for this draggable.
    pub fn root_overlay(mut self, value: bool) -> Self {
        self.draggable.root_overlay = value;
        self
    }

    /// Sets `hit_test_behavior` for this draggable.
    pub fn hit_test_behavior(mut self, value: HitTestBehavior) -> Self {
        self.draggable.hit_test_behavior = value;
        self
    }

    /// Sets `allowed_buttons_filter` for this draggable.
    pub fn allowed_buttons_filter(mut self, value: AllowedButtonsFilter) -> Self {
        self.draggable.allowed_buttons_filter = Some(value);
        self
    }
}

impl<T: 'static> DraggableWidget for LongPressDraggable<T> {
    type Data = T;

    fn draggable(&self) -> &Draggable<T> {
        &self.draggable
    }

    fn create_recognizer(
        &self,
        app: &mut App,
        on_start: GestureMultiDragStartCallback,
    ) -> AnyGestureRecognizer {
        let recognizer = DelayedMultiDragGestureRecognizer::new(app);
        recognizer.set_delay(app, self.delay);
        if let Some(filter) = self.draggable.allowed_buttons_filter.clone() {
            recognizer.set_allowed_buttons_filter(app, filter);
        }
        let haptic = self.haptic_feedback_on_start;
        recognizer.set_on_start(
            app,
            Some(Rc::new(move |app, position| {
                let result = on_start(app, position);
                if result.is_some() && haptic {
                    HapticFeedback::selection_click(app);
                }
                result
            })),
        );
        recognizer.as_recognizer()
    }
}

impl<T: 'static> StatefulWidget for LongPressDraggable<T> {
    type State = DraggableState<Self>;

    fn key(&self) -> Option<&KeyRef> {
        self.draggable.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        DraggableState::new()
    }
}

/// Retains the recognizer until every active drag has ended, even after unmount.
pub struct DraggableState<W: DraggableWidget> {
    /// Native widget lifecycle and element association.
    state: StateData<W>,

    /// Stays alive after unmount until every drag listening to pointer events has ended.
    recognizer: Option<AnyGestureRecognizer>,

    /// Number of active drag avatars retaining this state and recognizer.
    active_count: usize,
}

impl<W: DraggableWidget> Default for DraggableState<W> {
    /// Creates the same inactive state as `new`.
    fn default() -> Self {
        Self::new()
    }
}

impl<W: DraggableWidget> DraggableState<W> {
    /// Creates state for Draggable and widgets overriding its recognizer factory.
    pub fn new() -> Self {
        Self {
            state: StateData::new(),
            recognizer: None,
            active_count: 0,
        }
    }

    fn dispose_recognizer_if_inactive(self: Handle<Self>, app: &mut App) {
        if app.get(self).active_count > 0 {
            return;
        }
        if let Some(recognizer) = app.get_mut(self).recognizer.take() {
            recognizer.dispose(app);
        }
    }

    fn route_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        if self
            .widget(app)
            .draggable()
            .max_simultaneous_drags
            .is_some_and(|max| app.get(self).active_count >= max)
        {
            return;
        }
        app.get(self).recognizer.unwrap().add_pointer(app, &event);
    }

    fn start_drag(self: Handle<Self>, app: &mut App, position: Offset) -> Option<Rc<dyn Drag>> {
        let widget = self.widget(app).draggable().clone();
        if widget
            .max_simultaneous_drags
            .is_some_and(|max| app.get(self).active_count >= max)
        {
            return None;
        }
        let context = self.context(app);
        let start = (widget.drag_anchor_strategy)(app, &widget, context, position);
        self.set_state(app, |state| state.active_count += 1);
        let retained = app.retain(self);
        let overlay = Overlay::of(app, context, widget.root_overlay);
        let view_id = View::of(app, context).id();
        let avatar = DragAvatar::new(
            app,
            overlay,
            widget.data.clone().map(|data| data as Rc<dyn Any>),
            widget.axis,
            position,
            start,
            widget.feedback.clone(),
            widget.feedback_offset,
            widget.ignoring_feedback_pointer,
            view_id,
            Rc::new(move |app, details| {
                if self.mounted(app)
                    && let Some(callback) = self.widget(app).draggable().on_drag_update.clone()
                {
                    callback(app, details);
                }
            }),
            Rc::new(move |app, velocity, offset, was_accepted| {
                let _keep_alive = &retained;
                if self.mounted(app) {
                    self.set_state(app, |state| state.active_count -= 1);
                } else {
                    app.get_mut(self).active_count -= 1;
                    self.dispose_recognizer_if_inactive(app);
                }
                let widget = self.widget(app).draggable().clone();
                if self.mounted(app)
                    && let Some(callback) = widget.on_drag_end
                {
                    callback(
                        app,
                        DraggableDetails {
                            was_accepted,
                            velocity,
                            offset,
                        },
                    );
                }
                if was_accepted
                    && let Some(callback) = self.widget(app).draggable().on_drag_completed.clone()
                {
                    callback.call(app);
                }
                if !was_accepted
                    && let Some(callback) =
                        self.widget(app).draggable().on_draggable_canceled.clone()
                {
                    callback(app, velocity, offset);
                }
            }),
        );
        if let Some(callback) = widget.on_drag_started {
            callback.call(app);
        }
        Some(Rc::new(avatar))
    }
}

impl<W: DraggableWidget> State for DraggableState<W> {
    type Widget = W;
    crate::state_accessors!();
    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        let recognizer = widget.create_recognizer(
            app,
            Rc::new(move |app, position| self.start_drag(app, position)),
        );
        app.get_mut(self).recognizer = Some(recognizer);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_recognizer_if_inactive(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let settings = MediaQuery::maybe_gesture_settings_of(app, context);
        app.get(self)
            .recognizer
            .unwrap()
            .set_gesture_settings(app, settings);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).draggable();
        let can_drag = widget
            .max_simultaneous_drags
            .is_none_or(|max| app.get(self).active_count < max);
        let show_child = app.get(self).active_count == 0 || widget.child_when_dragging.is_none();
        let mut listener = crate::Listener::new()
            .behavior(widget.hit_test_behavior)
            .child(if show_child {
                widget.child.clone()
            } else {
                widget.child_when_dragging.clone().unwrap()
            });
        if can_drag {
            listener =
                listener.on_pointer_down(Rc::new(move |app, event| self.route_pointer(app, event)));
        }
        listener.into_widget()
    }
}

/// Details when a draggable is dropped, including acceptance, velocity, and offset.
#[derive(Clone, Copy, Debug)]
pub struct DraggableDetails {
    /// Determines whether the `DragTarget` accepted this draggable.
    pub was_accepted: bool,

    /// The velocity at which the pointer was moving when the specific pointer
    /// event occurred on the draggable.
    pub velocity: Velocity,

    /// The global position when the specific pointer event occurred on
    /// the draggable.
    pub offset: Offset,
}

/// The data and global position at which a pointer event occurred on a drag target.
pub struct DragTargetDetails<T> {
    /// The data that was dropped onto this `DragTarget`.
    pub data: Rc<T>,

    /// The global position when the specific pointer event occurred on
    /// the draggable.
    pub offset: Offset,
}

impl<T> Clone for DragTargetDetails<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            offset: self.offset,
        }
    }
}

/// A widget that receives data when a Draggable is dropped over it.
pub struct DragTarget<T: 'static> {
    /// Identity of this target in its parent's child list.
    pub key: Option<KeyRef>,

    /// Builds the target using all current accepted and rejected drag data.
    pub builder: DragTargetBuilder<T>,

    /// Legacy data-only acceptance predicate; mutually exclusive with the detailed predicate.
    pub on_will_accept: Option<DragTargetWillAccept<T>>,

    /// Determines whether to accept an entering drag, including its position.
    pub on_will_accept_with_details: Option<DragTargetWillAcceptWithDetails<T>>,

    /// Called when non-null data is dropped on this target.
    pub on_accept: Option<DragTargetAccept<T>>,

    /// Called when non-null data is dropped, including its position.
    pub on_accept_with_details: Option<DragTargetAcceptWithDetails<T>>,

    /// Called when a drag leaves this target.
    pub on_leave: Option<DragTargetLeave<T>>,

    /// Called when non-null drag data moves within this target.
    pub on_move: Option<DragTargetMove<T>>,

    /// Defaults to translucent so targets behind rejected targets can receive a drag.
    pub hit_test_behavior: HitTestBehavior,
}

impl<T: 'static> fmt::Debug for DragTarget<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragTarget").finish_non_exhaustive()
    }
}

impl<T: 'static> DragTarget<T> {
    /// Creates a target whose presentation reflects the current candidate and rejected data.
    pub fn new(
        builder: impl Fn(
            &mut App,
            BuildContext,
            Vec<Option<Rc<T>>>,
            Vec<Option<Rc<dyn Any>>>,
        ) -> WidgetRef
        + 'static,
    ) -> Self {
        Self {
            key: None,
            builder: Rc::new(builder),
            on_will_accept: None,
            on_will_accept_with_details: None,
            on_accept: None,
            on_accept_with_details: None,
            on_leave: None,
            on_move: None,
            hit_test_behavior: HitTestBehavior::Translucent,
        }
    }

    /// Sets the widget identity.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }

    /// Sets the legacy data-only acceptance predicate.
    pub fn on_will_accept(mut self, value: DragTargetWillAccept<T>) -> Self {
        self.on_will_accept = Some(value);
        self
    }

    /// Sets the predicate for accepting an entering drag.
    pub fn on_will_accept_with_details(
        mut self,
        value: DragTargetWillAcceptWithDetails<T>,
    ) -> Self {
        self.on_will_accept_with_details = Some(value);
        self
    }

    /// Sets the callback for dropped non-null data.
    pub fn on_accept(mut self, value: DragTargetAccept<T>) -> Self {
        self.on_accept = Some(value);
        self
    }

    /// Sets the callback for a drop's data and global position.
    pub fn on_accept_with_details(mut self, value: DragTargetAcceptWithDetails<T>) -> Self {
        self.on_accept_with_details = Some(value);
        self
    }

    /// Sets the callback for a drag leaving the target.
    pub fn on_leave(mut self, value: DragTargetLeave<T>) -> Self {
        self.on_leave = Some(value);
        self
    }

    /// Sets the callback for movement within the target.
    pub fn on_move(mut self, value: DragTargetMove<T>) -> Self {
        self.on_move = Some(value);
        self
    }

    /// Sets how the target participates in hit testing.
    pub fn hit_test_behavior(mut self, value: HitTestBehavior) -> Self {
        self.hit_test_behavior = value;
        self
    }
}

impl<T: 'static> StatefulWidget for DragTarget<T> {
    type State = DragTargetState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        debug_assert!(
            self.on_will_accept.is_none() || self.on_will_accept_with_details.is_none(),
            "Don't pass both acceptance callbacks"
        );
        DragTargetState {
            state: StateData::new(),
            candidate_avatars: Vec::new(),
            rejected_avatars: Vec::new(),
        }
    }
}

/// The candidate and rejected avatars currently hovering over one target.
pub struct DragTargetState<T: 'static> {
    /// Native widget lifecycle and element association.
    state: StateData<DragTarget<T>>,

    /// Avatars whose data this target accepted on entry.
    candidate_avatars: Vec<Handle<DragAvatar>>,

    /// Avatars whose data this target rejected on entry.
    rejected_avatars: Vec<Handle<DragAvatar>>,
}

/// Erases the target's data type while retaining its normal mounted-state checks.
trait DragTargetHandler {
    fn id(&self) -> HandleId;

    fn is_expected_data_type(&self, data: &Option<Rc<dyn Any>>) -> bool;

    fn did_enter(&self, app: &mut App, avatar: Handle<DragAvatar>) -> bool;

    fn did_leave(&self, app: &mut App, avatar: Handle<DragAvatar>);

    fn did_drop(&self, app: &mut App, avatar: Handle<DragAvatar>);

    fn did_move(&self, app: &mut App, avatar: Handle<DragAvatar>);
}

/// Keeps a target alive while hit testing and active drags hold its metadata.
struct DragTargetMetadata {
    /// Erased target callbacks stored in the render metadata.
    handler: Rc<dyn DragTargetHandler>,

    /// Keeps the target state alive while metadata is held by a drag.
    _retained: RetainedHandleId,
}

impl<T: 'static> DragTargetHandler for Handle<DragTargetState<T>> {
    fn id(&self) -> HandleId {
        (*self).into()
    }

    fn is_expected_data_type(&self, data: &Option<Rc<dyn Any>>) -> bool {
        data.as_ref().is_none_or(|data| data.is::<T>())
    }

    fn did_enter(&self, app: &mut App, avatar: Handle<DragAvatar>) -> bool {
        debug_assert!(!app.get(*self).candidate_avatars.contains(&avatar));
        debug_assert!(!app.get(*self).rejected_avatars.contains(&avatar));
        let data = app
            .get(avatar)
            .data
            .clone()
            .map(|data| data.downcast::<T>().unwrap());
        let widget = self.widget(app);
        let accepted = match (
            widget.on_will_accept.clone(),
            widget.on_will_accept_with_details.clone(),
        ) {
            (None, None) => true,
            (Some(callback), _) => callback(app, data.clone()),
            (_, Some(callback)) => data.is_some_and(|data| {
                callback(
                    app,
                    DragTargetDetails {
                        data,
                        offset: app.get(avatar).last_offset.unwrap(),
                    },
                )
            }),
        };
        self.set_state(app, |state| {
            if accepted {
                state.candidate_avatars.push(avatar)
            } else {
                state.rejected_avatars.push(avatar)
            }
        });
        accepted
    }

    fn did_leave(&self, app: &mut App, avatar: Handle<DragAvatar>) {
        if !self.mounted(app) {
            return;
        }
        self.set_state(app, |state| {
            state
                .candidate_avatars
                .retain(|candidate| *candidate != avatar);
            state
                .rejected_avatars
                .retain(|candidate| *candidate != avatar);
        });
        if let Some(callback) = self.widget(app).on_leave.clone() {
            let data = app
                .get(avatar)
                .data
                .clone()
                .map(|data| data.downcast::<T>().unwrap());
            callback(app, data);
        }
    }

    fn did_drop(&self, app: &mut App, avatar: Handle<DragAvatar>) {
        debug_assert!(app.get(*self).candidate_avatars.contains(&avatar));
        if !self.mounted(app) {
            return;
        }
        self.set_state(app, |state| {
            state
                .candidate_avatars
                .retain(|candidate| *candidate != avatar)
        });
        if let Some(data) = app.get(avatar).data.clone() {
            let data = data.downcast::<T>().unwrap();
            if let Some(callback) = self.widget(app).on_accept.clone() {
                callback(app, data.clone());
            }
            if let Some(callback) = self.widget(app).on_accept_with_details.clone() {
                callback(
                    app,
                    DragTargetDetails {
                        data,
                        offset: app.get(avatar).last_offset.unwrap(),
                    },
                );
            }
        }
    }

    fn did_move(&self, app: &mut App, avatar: Handle<DragAvatar>) {
        if !self.mounted(app) {
            return;
        }
        if let Some(data) = app.get(avatar).data.clone()
            && let Some(callback) = self.widget(app).on_move.clone()
        {
            callback(
                app,
                DragTargetDetails {
                    data: data.downcast::<T>().unwrap(),
                    offset: app.get(avatar).last_offset.unwrap(),
                },
            );
        }
    }
}

impl<T: 'static> State for DragTargetState<T> {
    type Widget = DragTarget<T>;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let state = app.get(self);
        let candidates = state
            .candidate_avatars
            .iter()
            .map(|avatar| {
                app.get(*avatar)
                    .data
                    .clone()
                    .map(|data| data.downcast::<T>().unwrap())
            })
            .collect();
        let rejected = state
            .rejected_avatars
            .iter()
            .map(|avatar| app.get(*avatar).data.clone())
            .collect();
        let builder = self.widget(app).builder.clone();
        let child = builder(app, context, candidates, rejected);
        MetaData::new()
            .meta_data(Rc::new(DragTargetMetadata {
                handler: Rc::new(self),
                _retained: app.retain_id(self.id()),
            }))
            .behavior(self.widget(app).hit_test_behavior)
            .child(child)
            .into_widget()
    }
}

/// Whether the pointer ended normally or the drag was canceled.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DragEndKind {
    Dropped,
    Canceled,
}

/// Reports the final velocity, feedback offset and target acceptance.
type OnDragEnd = Rc<dyn Fn(&mut App, Velocity, Offset, bool)>;

/// The feedback overlay and target path for one active pointer.
struct DragAvatar {
    /// The data carried by this draggable.
    data: Option<Rc<dyn Any>>,

    /// The axis restricting drag motion, if specified.
    axis: Option<Axis>,

    /// Pickup offset used to anchor feedback to the pointer.
    drag_start_point: Offset,

    /// The widget displayed beneath the pointer during the drag.
    feedback: WidgetRef,

    /// Offset used when hit testing to find drag targets.
    feedback_offset: Offset,

    /// Called after an update changes the drag position.
    on_drag_update: DragUpdateCallback,

    /// Reports the restricted velocity, final offset and acceptance.
    on_drag_end: OnDragEnd,

    /// Overlay into which the feedback entry is inserted.
    overlay_state: Handle<OverlayState>,

    /// Whether feedback is excluded from pointer hit testing.
    ignoring_feedback_pointer: bool,

    /// View used to hit test the global drag position.
    view_id: ViewId,

    /// First entered target currently accepting this avatar.
    active_target: Option<Rc<DragTargetMetadata>>,

    /// Targets whose metadata appeared in the previous hit test.
    entered_targets: Vec<Rc<DragTargetMetadata>>,

    /// Current global pointer position, restricted to the configured axis.
    position: Offset,

    /// Last global feedback offset after subtracting the pickup anchor.
    last_offset: Option<Offset>,

    /// Feedback origin in the overlay’s coordinate system.
    overlay_offset: Offset,

    /// Overlay entry retained until the drag finishes.
    entry: Option<Handle<OverlayEntry>>,
}

impl DragAvatar {
    /// Inserts feedback into the overlay and hit-tests the initial pointer position.
    #[allow(clippy::too_many_arguments)]
    fn new(
        app: &mut App,
        overlay_state: Handle<OverlayState>,
        data: Option<Rc<dyn Any>>,
        axis: Option<Axis>,
        initial_position: Offset,
        drag_start_point: Offset,
        feedback: WidgetRef,
        feedback_offset: Offset,
        ignoring_feedback_pointer: bool,
        view_id: ViewId,
        on_drag_update: DragUpdateCallback,
        on_drag_end: OnDragEnd,
    ) -> Handle<Self> {
        let avatar = app.create(Self {
            data,
            axis,
            drag_start_point,
            feedback,
            feedback_offset,
            on_drag_update,
            on_drag_end,
            overlay_state,
            ignoring_feedback_pointer,
            view_id,
            active_target: None,
            entered_targets: Vec::new(),
            position: initial_position,
            last_offset: None,
            overlay_offset: Offset::ZERO,
            entry: None,
        });
        let retained = app.retain(avatar);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |app, context| {
                let _keep_alive = &retained;
                avatar.build(app, context)
            }),
            false,
            false,
            false,
        );
        app.get_mut(avatar).entry = Some(entry);
        overlay_state.insert(app, entry, None, None);
        avatar.update_drag(app, initial_position);
        avatar
    }

    /// Updates overlay coordinates, then enters, leaves and moves targets in hit-test order.
    fn update_drag(self: Handle<Self>, app: &mut App, global_position: Offset) {
        let state = app.get(self);
        let (overlay, anchor, feedback_offset, view_id) = (
            state.overlay_state,
            state.drag_start_point,
            state.feedback_offset,
            state.view_id,
        );
        app.get_mut(self).last_offset = Some(global_position - anchor);
        if overlay.mounted(app) {
            let render_box = overlay
                .context(app)
                .find_render_object(app)
                .unwrap()
                .as_box()
                .unwrap();
            app.get_mut(self).overlay_offset =
                render_box.global_to_local(app, global_position, None) - anchor;
            app.get(self).entry.unwrap().mark_needs_build(app);
        }
        let mut result = HitTestResult::new();
        GestureBinding::instance(app).hit_test_in_view(
            app,
            &mut result,
            global_position + feedback_offset,
            view_id,
        );
        let mut targets = Vec::new();
        for entry in result.path() {
            let Some(hit) = (entry.target() as &dyn Any).downcast_ref::<BoxHitTestEntry>() else {
                continue;
            };
            let Some(render) = hit.target().as_object().downcast::<RenderMetaData>(app) else {
                continue;
            };
            let Some(metadata) = render
                .meta_data(app)
                .and_then(|value| value.downcast::<DragTargetMetadata>().ok())
            else {
                continue;
            };
            if metadata.handler.is_expected_data_type(&app.get(self).data) {
                targets.push(metadata);
            }
        }
        let state = app.get(self);
        let lists_match = !state.entered_targets.is_empty()
            && targets.len() >= state.entered_targets.len()
            && targets
                .iter()
                .zip(&state.entered_targets)
                .all(|(a, b)| a.handler.id() == b.handler.id());
        if lists_match
            && (state.active_target.is_some() || targets.len() == state.entered_targets.len())
        {
            for target in state.entered_targets.clone() {
                target.handler.did_move(app, self);
            }
            return;
        }
        self.leave_all_entered(app);
        let mut active = None;
        for target in targets {
            app.get_mut(self).entered_targets.push(target.clone());
            if target.handler.did_enter(app, self) {
                active = Some(target);
                break;
            }
        }
        for target in app.get(self).entered_targets.clone() {
            target.handler.did_move(app, self);
        }
        app.get_mut(self).active_target = active;
    }

    /// Sends leave to every previously entered target before clearing the path.
    fn leave_all_entered(self: Handle<Self>, app: &mut App) {
        for target in app.get(self).entered_targets.clone() {
            target.handler.did_leave(app, self);
        }
        app.get_mut(self).entered_targets.clear();
    }

    /// Drops on the active target, releases feedback, and reports the final outcome.
    fn finish_drag(self: Handle<Self>, app: &mut App, kind: DragEndKind, velocity: Velocity) {
        let target = app.get(self).active_target.clone();
        let mut accepted = false;
        if kind == DragEndKind::Dropped
            && let Some(target) = target
        {
            target.handler.did_drop(app, self);
            accepted = true;
            app.get_mut(self)
                .entered_targets
                .retain(|entry| entry.handler.id() != target.handler.id());
        }
        self.leave_all_entered(app);
        app.get_mut(self).active_target = None;
        let entry = app.get_mut(self).entry.take().unwrap();
        entry.remove(app);
        entry.dispose(app);
        let callback = app.get(self).on_drag_end.clone();
        callback(app, velocity, app.get(self).last_offset.unwrap(), accepted);
        app.destroy(self);
    }

    /// Positions feedback in overlay coordinates; accessibility is deferred framework-wide.
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let state = app.get(self);
        Positioned::new(
            IgnorePointer::new()
                .ignoring(state.ignoring_feedback_pointer)
                .child(state.feedback.clone()),
        )
        .left(state.overlay_offset.dx())
        .top(state.overlay_offset.dy())
        .into_widget()
    }

    /// Restricts movement to the configured axis without changing hit-test coordinates.
    fn restrict_axis(self: Handle<Self>, app: &App, offset: Offset) -> Offset {
        match app.get(self).axis {
            Some(Axis::Horizontal) => Offset::new(offset.dx(), 0.0),
            Some(Axis::Vertical) => Offset::new(0.0, offset.dy()),
            None => offset,
        }
    }
}

impl DragObject for DragAvatar {
    fn update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        let old_position = app.get(self).position;
        let position = old_position + self.restrict_axis(app, details.delta);
        app.get_mut(self).position = position;
        self.update_drag(app, position);
        if position != old_position {
            let callback = app.get(self).on_drag_update.clone();
            callback(app, details);
        }
    }

    fn end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        let velocity = Velocity::new(self.restrict_axis(app, details.velocity.pixels_per_second));
        self.finish_drag(app, DragEndKind::Dropped, velocity);
    }

    fn cancel(self: Handle<Self>, app: &mut App) {
        self.finish_drag(app, DragEndKind::Canceled, Velocity::ZERO);
    }
}
