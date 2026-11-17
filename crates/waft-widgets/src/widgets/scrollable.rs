//! Flutter counterpart: `widgets/scrollable.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::Curve;
use waft_embedder::{Clip, Offset, Rect};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, CompleterFuture, Handle, HandleId, Listenable,
    Listener, OptionRetained, PRECISION_ERROR_TOLERANCE, Ref, wait_all,
};
use waft_gestures::{
    DeviceGestureSettings, Drag, DragDownDetails, DragEndDetails, DragGestureRecognizer,
    DragStartBehavior, DragStartDetails, DragUpdateDetails, GestureBinding,
    HorizontalDragGestureRecognizer, PointerEvent, VerticalDragGestureRecognizer,
};
use waft_painting::{
    Axis, AxisDirection, axis_direction_is_reversed, axis_direction_to_axis, flip_axis,
    transform_point,
};
use waft_rendering::{
    AnyRenderObject, AnySelectable, HitTestBehavior, RenderIgnorePointer, SelectedContentRange,
    SelectionEvent, SelectionEventType, SelectionGeometry, SelectionHandler, SelectionRegistrar,
    SelectionRegistrarRef, SelectionResult, ViewportOffsetRef,
};
use waft_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProvider, TickerProviderObject,
};
use waft_services::{HardwareKeyboard, RestorationBucket, RestorationData, RestorationManager};

use crate::framework::{
    BuildContext, GlobalKey, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    WidgetRef,
};
use crate::view::View;
use crate::widgets::basic::{IgnorePointer, Listener as PointerListener};
use crate::widgets::gesture_detector::{
    GestureRecognizerFactories, GestureRecognizerFactory, GestureRecognizerFactoryWithHandlers,
    RawGestureDetector, RawGestureDetectorState,
};
use crate::widgets::media_query::MediaQuery;
use crate::widgets::restoration::{
    RestorableProperty, RestorablePropertyData, RestorationMixin, RestorationMixinData,
};
use crate::widgets::restoration_properties::{RestorableValue, RestorableValueData};
use crate::widgets::scroll_activity::ScrollHoldController;
use crate::widgets::scroll_configuration::{ScrollBehaviorRef, ScrollConfiguration};
use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_controller::{
    ScrollController, ScrollControllerLeaf, ScrollControllerRef,
};
use crate::widgets::scroll_physics::ScrollPhysicsRef;
use crate::widgets::scroll_position::{
    AnyScrollPosition, ScrollPositionAlignmentPolicy, ScrollPositionRef,
};
use crate::widgets::scrollable_helpers::{
    EdgeDraggingAutoScroller, ScrollIncrementCalculator, ScrollableDetails,
};
use crate::widgets::selectable_region::{
    MultiSelectableSelectionContainerDelegate, MultiSelectableSelectionContainerDelegateBase,
    MultiSelectableSelectionContainerDelegateData,
};
use crate::widgets::selection_container::{
    SelectionContainer, SelectionContainerDelegate, SelectionContainerDelegateData,
};
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};

/// Signature used by [`Scrollable`] to build the viewport through which the
/// scrollable content is displayed.
pub type ViewportBuilder = Rc<dyn Fn(&mut App, BuildContext, ViewportOffsetRef) -> WidgetRef>;

// The return type of `perform_ensure_visible`.
//
// The list of futures represents each pending ScrollPosition call to
// ensure_visible. The returned ScrollableState's context is used to find the
// next potential ancestor Scrollable.
type EnsureVisibleResults = (Vec<CompleterFuture<()>>, Handle<ScrollableState>);

/// A widget that manages scrolling in one dimension and informs the `Viewport`
/// through which the content is viewed.
///
/// [`Scrollable`] implements the interaction model for a scrollable widget,
/// including gesture recognition, but does not have an opinion about how the
/// viewport, which actually displays the children, is constructed.
///
/// It's rare to construct a [`Scrollable`] directly. Instead, consider `ListView`
/// or `GridView`, which combine scrolling, viewporting, and a layout model.
///
/// The static [`Scrollable::of`] and [`Scrollable::ensure_visible`] functions are
/// often used to interact with the [`Scrollable`] widget inside a `ListView` or
/// a `GridView`.
///
/// To further customize scrolling behavior with a [`Scrollable`]:
///
/// 1. You can provide a [`viewport_builder`](Self::viewport_builder) to customize the child
///    model.
///
/// 2. You can provide a custom `ScrollController` that creates a custom
///    [`ScrollPosition`](crate::ScrollPosition).
///
/// ## Persisting the scroll position during a session
///
/// Scrollables attempt to persist their scroll position using `PageStorage`.
/// This can be disabled by setting `ScrollController.keepScrollOffset` to false
/// on the [`controller`](Self::controller).
pub struct Scrollable {
    pub key: Option<KeyRef>,

    /// The direction in which this widget scrolls.
    ///
    /// For example, if the [`axis_direction`](Self::axis_direction) is
    /// [`AxisDirection::Down`], increasing the scroll position will cause content below the
    /// bottom of the viewport to become visible through the viewport. Similarly, if the axis
    /// direction is [`AxisDirection::Right`], increasing the scroll position will cause
    /// content beyond the right edge of the viewport to become visible through the viewport.
    ///
    /// Defaults to [`AxisDirection::Down`].
    pub axis_direction: AxisDirection,

    /// An object that can be used to control the position to which this widget is
    /// scrolled.
    ///
    /// A `ScrollController` serves several purposes. It can be used to control
    /// the initial scroll position, whether the scroll view should automatically
    /// save and restore its scroll position in the `PageStorage`, to read the current
    /// scroll position, or change it.
    ///
    /// If `None`, a `ScrollController` will be created internally by [`Scrollable`]
    /// in order to create and manage the [`ScrollPosition`](crate::ScrollPosition).
    pub controller: Option<ScrollControllerRef>,

    /// How the widgets should respond to user input.
    ///
    /// For example, determines how the widget continues to animate after the
    /// user stops dragging the scroll view.
    ///
    /// Defaults to matching platform conventions via the physics provided from
    /// the ambient [`ScrollConfiguration`].
    ///
    /// If an explicit [`ScrollBehaviorRef`] is provided to
    /// [`scroll_behavior`](Self::scroll_behavior), the [`ScrollPhysics`](crate::ScrollPhysics) provided by that
    /// behavior will take precedence after this one.
    ///
    /// The physics can be changed dynamically, but new physics will only take
    /// effect if the _type_ of the provided object changes.
    pub physics: Option<ScrollPhysicsRef>,

    /// Builds the viewport through which the scrollable content is displayed.
    ///
    /// A typical viewport uses the given [`AnyViewportOffset`] to determine which part
    /// of its content is actually visible through the viewport.
    pub viewport_builder: ViewportBuilder,

    /// An optional function that will be called to calculate the distance to
    /// scroll when the scrollable is asked to scroll via the keyboard using a
    /// [`crate::ScrollAction`].
    ///
    /// If `None`, the default for [`crate::ScrollIncrementType::Page`] is 80% of the size of
    /// the scroll window, and for [`crate::ScrollIncrementType::Line`], 50 logical pixels.
    pub increment_calculator: Option<ScrollIncrementCalculator>,

    /// Defines the behavior of the gesture detector used in this [`Scrollable`].
    ///
    /// This defaults to [`HitTestBehavior::Opaque`] which means it prevents targets
    /// behind this [`Scrollable`] from receiving events.
    pub hit_test_behavior: HitTestBehavior,

    /// Determines the way that drag start behavior is handled.
    ///
    /// If set to [`DragStartBehavior::Start`], scrolling drag behavior will
    /// begin at the position where the drag gesture won the arena. If set to
    /// [`DragStartBehavior::Down`] it will begin at the position where a down
    /// event is first detected.
    ///
    /// By default, the drag start behavior is [`DragStartBehavior::Start`].
    pub drag_start_behavior: DragStartBehavior,

    /// Restoration ID to save and restore the scroll offset of the scrollable.
    ///
    /// If a restoration id is provided, the scrollable will persist its current
    /// scroll offset and restore it during state restoration.
    ///
    /// The scroll offset is persisted in a [`RestorationBucket`] claimed from
    /// the surrounding `RestorationScope` using the provided restoration ID.
    pub restoration_id: Option<String>,

    /// A [`ScrollBehaviorRef`] that will be applied to this widget individually.
    ///
    /// Defaults to `None`, wherein the inherited behavior is copied and
    /// modified to alter the viewport decoration.
    ///
    /// A behavior also provides [`ScrollPhysics`](crate::ScrollPhysics). If an explicit physics is provided in
    /// [`physics`](Self::physics), it will take precedence, followed by this behavior, and
    /// then the inherited ancestor behavior.
    pub scroll_behavior: Option<ScrollBehaviorRef>,

    /// The clip a decorator should honour.
    ///
    /// Defaults to [`Clip::HardEdge`].
    ///
    /// This is passed to decorators in [`ScrollableDetails`], and does not directly affect
    /// clipping of the [`Scrollable`].
    pub clip_behavior: Clip,
}

impl Scrollable {
    /// Creates a widget that scrolls.
    pub fn new(viewport_builder: ViewportBuilder) -> Scrollable {
        Scrollable {
            key: None,
            axis_direction: AxisDirection::Down,
            controller: None,
            physics: None,
            viewport_builder,
            increment_calculator: None,
            hit_test_behavior: HitTestBehavior::Opaque,
            drag_start_behavior: DragStartBehavior::Start,
            restoration_id: None,
            scroll_behavior: None,
            clip_behavior: Clip::HardEdge,
        }
    }

    /// Dart `Scrollable(key:)`.
    pub fn key(mut self, key: KeyRef) -> Scrollable {
        self.key = Some(key);
        self
    }

    /// Dart `Scrollable(axisDirection:)`.
    pub fn axis_direction(mut self, axis_direction: AxisDirection) -> Scrollable {
        self.axis_direction = axis_direction;
        self
    }

    /// Dart `Scrollable(controller:)`.
    pub fn controller(mut self, controller: ScrollControllerRef) -> Scrollable {
        self.controller = Some(controller);
        self
    }

    /// Dart `Scrollable(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> Scrollable {
        self.physics = Some(physics);
        self
    }

    /// Dart `Scrollable(incrementCalculator:)`.
    pub fn increment_calculator(
        mut self,
        increment_calculator: ScrollIncrementCalculator,
    ) -> Scrollable {
        self.increment_calculator = Some(increment_calculator);
        self
    }

    /// Dart `Scrollable(hitTestBehavior:)`.
    pub fn hit_test_behavior(mut self, hit_test_behavior: HitTestBehavior) -> Scrollable {
        self.hit_test_behavior = hit_test_behavior;
        self
    }

    /// Dart `Scrollable(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> Scrollable {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `Scrollable(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> Scrollable {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `Scrollable(scrollBehavior:)`.
    pub fn scroll_behavior(mut self, scroll_behavior: ScrollBehaviorRef) -> Scrollable {
        self.scroll_behavior = Some(scroll_behavior);
        self
    }

    /// Dart `Scrollable(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Scrollable {
        self.clip_behavior = clip_behavior;
        self
    }

    /// The axis along which the scroll view scrolls.
    ///
    /// Determined by the [`axis_direction`](Self::axis_direction).
    pub fn axis(&self) -> Axis {
        axis_direction_to_axis(self.axis_direction)
    }

    /// The state from the closest instance of this class that encloses the given
    /// context, or `None` if none is found.
    ///
    /// Calling this method will create a dependency on the [`ScrollableState`]
    /// that is returned, if there is one. This is typically the closest
    /// [`Scrollable`], but may be a more distant ancestor if `axis` is used to
    /// target a specific [`Scrollable`].
    ///
    /// Using the optional [`Axis`] is useful when scrollables are nested and the
    /// target [`Scrollable`] is not the closest instance.
    ///
    /// This finds the nearest _ancestor_ [`Scrollable`] of the `context`. This
    /// means that if the `context` is that of a [`Scrollable`], it will _not_ find
    /// _that_ [`Scrollable`].
    ///
    /// See also:
    ///
    /// * [`Scrollable::of`], which is similar to this method, but panics
    ///   if no [`Scrollable`] ancestor is found.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
        axis: Option<Axis>,
    ) -> Option<Handle<ScrollableState>> {
        // This is the context that will need to establish the dependency.
        let original_context = context;
        let mut context = context;
        let mut element =
            context.get_element_for_inherited_widget_of_exact_type::<ScrollableScope>(app);
        while let Some(current) = element {
            let scrollable = ScrollableScope::of_element(app, current).scrollable;
            if axis
                .is_none_or(|axis| axis_direction_to_axis(scrollable.axis_direction(app)) == axis)
            {
                // Establish the dependency on the correct context.
                original_context.depend_on_inherited_element(app, current, None);
                return Some(scrollable);
            }
            context = scrollable.context(app);
            element =
                context.get_element_for_inherited_widget_of_exact_type::<ScrollableScope>(app);
        }
        None
    }

    /// The state from the closest instance of this class that encloses the given
    /// context.
    ///
    /// Calling this method will create a dependency on the [`ScrollableState`]
    /// that is returned.
    ///
    /// # Panics
    ///
    /// If no [`Scrollable`] ancestor is found.
    ///
    /// See also:
    ///
    /// * [`Scrollable::maybe_of`], which is similar to this method, but returns `None`
    ///   if no [`Scrollable`] ancestor is found.
    pub fn of(app: &mut App, context: BuildContext, axis: Option<Axis>) -> Handle<ScrollableState> {
        Scrollable::maybe_of(app, context, axis).expect(
            "Scrollable.of() was called with a context that does not contain a Scrollable \
             widget.\n\
             No Scrollable widget ancestor could be found starting from the context that was \
             passed to Scrollable.of(). This can happen because you are using a widget that \
             looks for a Scrollable ancestor, but no such ancestor exists. When specifying an \
             axis, this method will only look for a Scrollable that matches the given Axis.",
        )
    }

    /// Provides a heuristic to determine if expensive frame-bound tasks should be
    /// deferred for the `context` at a specific point in time.
    ///
    /// Calling this method does _not_ create a dependency on any other widget.
    /// This also means that the value returned is only good for the point in time
    /// when it is called, and callers will not get updated if the value changes.
    ///
    /// The heuristic used is determined by the [`physics`](Self::physics) of this
    /// [`Scrollable`] via [`ScrollPhysics::recommend_deferred_loading`](crate::ScrollPhysics::recommend_deferred_loading).
    ///
    /// If there is no [`Scrollable`] in the widget tree above the `context`, this
    /// method returns false.
    pub fn recommend_deferred_loading_for_context(
        app: &mut App,
        context: BuildContext,
        axis: Option<Axis>,
    ) -> bool {
        let mut context = context;
        loop {
            let Some(widget) = context.get_inherited_widget_of_exact_type::<ScrollableScope>(app)
            else {
                return false;
            };
            let (scrollable, position) = (widget.scrollable, widget.position.handle());
            if axis
                .is_none_or(|axis| axis_direction_to_axis(scrollable.axis_direction(app)) == axis)
            {
                return position.recommend_deferred_loading(app, context);
            }
            context = scrollable.context(app);
        }
    }

    /// Scrolls all scrollables that enclose the given context so as to make the
    /// given context visible.
    ///
    /// The returned future completes once every enclosing scrollable's animation has ended.
    pub fn ensure_visible(
        app: &mut App,
        context: BuildContext,
        alignment: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        alignment_policy: ScrollPositionAlignmentPolicy,
    ) -> CompleterFuture<()> {
        let mut futures: Vec<CompleterFuture<()>> = Vec::new();

        // The `target_render_object` is used to record the first target render object.
        // If there are multiple scrollable widgets nested, it is made to be as visible as
        // possible to improve the user experience. If it is already visible, then let the
        // outer render object be as visible as possible.
        let mut target_render_object: Option<AnyRenderObject> = None;
        let mut context = context;
        let mut scrollable = Scrollable::maybe_of(app, context, None);
        while let Some(current) = scrollable {
            let (new_futures, current) = current.perform_ensure_visible(
                app,
                context.find_render_object(app).expect("a render object"),
                alignment,
                duration,
                curve.clone(),
                alignment_policy,
                target_render_object,
            );
            futures.extend(new_futures);

            if target_render_object.is_none() {
                target_render_object = context.find_render_object(app);
            }
            context = current.context(app);
            scrollable = Scrollable::maybe_of(app, context, None);
        }

        if futures.is_empty() || duration.is_zero() {
            return CompleterFuture::ready(());
        }
        if futures.len() == 1 {
            return futures.pop().expect("the single future");
        }
        CompleterFuture::spawn(app, async move {
            wait_all(futures).await;
        })
    }
}

impl StatefulWidget for Scrollable {
    type State = ScrollableState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ScrollableState {
        ScrollableState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            restoration: RestorationMixinData::new(),
            position: None,
            physics: None,
            persisted_scroll_offset: None,
            configuration: None,
            fallback_scroll_controller: None,
            media_query_gesture_settings: None,
            device_pixel_ratio: 1.0,
            gesture_detector_key: GlobalKey::new(),
            ignore_pointer_key: GlobalKey::new(),
            gesture_recognizers: Vec::new(),
            should_ignore_pointer: false,
            last_can_drag: None,
            last_axis_direction: None,
            drag: None,
            hold: None,
            scroll_context: None,
        }
    }
}

impl Debug for Scrollable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scrollable")
            .field("axisDirection", &self.axis_direction)
            .field("physics", &self.physics)
            .field("restorationId", &self.restoration_id)
            .finish()
    }
}

/// Enables [`Scrollable::of`] to work as if [`ScrollableState`] was an inherited widget.
///
/// [`ScrollableState::build`] always rebuilds its scope; Dart's `_ScrollableScope`.
#[derive(Debug)]
struct ScrollableScope {
    scrollable: Handle<ScrollableState>,
    position: ScrollPositionRef,
    child: WidgetRef,
}

impl ScrollableScope {
    /// The scope an inherited element holds.
    fn of_element(app: &App, element: crate::framework::AnyElement) -> &ScrollableScope {
        crate::framework::downcast_widget::<ScrollableScope>(&**element.widget(app))
            .expect("a _ScrollableScope element holds a _ScrollableScope")
    }
}

impl InheritedWidget for ScrollableScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &ScrollableScope) -> bool {
        self.position != old.position
    }
}

/// State object for a [`Scrollable`] widget.
///
/// To manipulate a [`Scrollable`] widget's scroll position, use the object
/// obtained from the [`position`](Self::position) property.
///
/// To be informed of when a [`Scrollable`] widget is scrolling, use a
/// `NotificationListener` to listen for [`crate::ScrollNotification`] notifications.
pub struct ScrollableState {
    state: StateData<Scrollable>,
    ticker_provider: TickerProviderStateMixinData,
    restoration: RestorationMixinData,
    position: Option<ScrollPositionRef>,
    physics: Option<ScrollPhysicsRef>,
    persisted_scroll_offset: Option<Ref<RestorableScrollOffset>>,
    configuration: Option<ScrollBehaviorRef>,
    fallback_scroll_controller: Option<Ref<ScrollController>>,
    media_query_gesture_settings: Option<DeviceGestureSettings>,
    device_pixel_ratio: f64,
    gesture_detector_key: GlobalKey,
    ignore_pointer_key: GlobalKey,
    // This field is set during layout, and then reused until the next time it is set.
    gesture_recognizers: GestureRecognizerFactories,
    should_ignore_pointer: bool,
    last_can_drag: Option<bool>,
    last_axis_direction: Option<Axis>,
    drag: Option<Rc<dyn Drag>>,
    hold: Option<Rc<dyn ScrollHoldController>>,
    /// This state as the `ScrollContext` its position holds, minted once in `init_state`: a
    /// position compares contexts by identity, and Dart passes `this`.
    scroll_context: Option<Rc<dyn ScrollContext>>,
}

impl ScrollableState {
    /// The manager for this [`Scrollable`] widget's viewport position.
    ///
    /// To control what kind of [`ScrollPosition`](crate::ScrollPosition) is created for a [`Scrollable`],
    /// provide it with a custom `ScrollController` that creates the appropriate
    /// [`ScrollPosition`](crate::ScrollPosition) in its
    /// [`create_scroll_position`](ScrollControllerLeaf::create_scroll_position) method.
    pub fn position(self: Handle<Self>, app: &App) -> AnyScrollPosition {
        app.get(self)
            .position
            .handle()
            .expect("the position is created in didChangeDependencies")
    }

    /// The resolved [`ScrollPhysics`](crate::ScrollPhysics) of the [`ScrollableState`].
    pub fn resolved_physics(self: Handle<Self>, app: &App) -> Option<ScrollPhysicsRef> {
        app.get(self).physics.clone()
    }

    /// An [`Offset`] that represents the absolute distance from the origin, or 0,
    /// of the [`ScrollPosition`](crate::ScrollPosition) expressed in the associated [`Axis`].
    ///
    /// Used by an auto scroller to progress the position forward when a drag gesture reaches
    /// the edge of the viewport.
    pub fn delta_to_scroll_origin(self: Handle<Self>, app: &App) -> Offset {
        let pixels = self.position(app).pixels(app);
        match self.axis_direction(app) {
            AxisDirection::Up => Offset::new(0.0, -pixels),
            AxisDirection::Down => Offset::new(0.0, pixels),
            AxisDirection::Left => Offset::new(-pixels, 0.0),
            AxisDirection::Right => Offset::new(pixels, 0.0),
        }
    }

    /// The controller this scrollable's position is attached to; Dart's
    /// `_effectiveScrollController`.
    fn effective_scroll_controller(self: Handle<Self>, app: &App) -> ScrollControllerRef {
        self.widget(app).controller.clone().unwrap_or_else(|| {
            app.get(self)
                .fallback_scroll_controller
                .as_ref()
                .expect("a scrollable without a controller creates a fallback")
                .erase()
        })
    }

    /// The [`position`](Self::position) as a reference, for a caller that keeps it: a field
    /// Dart types as the `ScrollPosition` it reads from a `ScrollableState`.
    pub fn kept_position(self: Handle<Self>, app: &App) -> ScrollPositionRef {
        app.get(self)
            .position
            .clone()
            .expect("the position is created in didChangeDependencies")
    }

    fn persisted_scroll_offset(self: Handle<Self>, app: &App) -> Handle<RestorableScrollOffset> {
        app.get(self)
            .persisted_scroll_offset
            .handle()
            .expect("the property is created in initState")
    }

    fn configuration(self: Handle<Self>, app: &App) -> ScrollBehaviorRef {
        app.get(self)
            .configuration
            .clone()
            .expect("the configuration is read in didChangeDependencies")
    }

    // Only call this from places that will definitely trigger a rebuild.
    fn scroll_context(self: Handle<Self>, app: &App) -> Rc<dyn ScrollContext> {
        Rc::clone(
            app.get(self)
                .scroll_context
                .as_ref()
                .expect("init_state has run"),
        )
    }

    fn update_position(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let widget_behavior = self.widget(app).scroll_behavior.clone();
        let configuration = match widget_behavior.clone() {
            Some(behavior) => behavior,
            None => ScrollConfiguration::of(app, context),
        };
        app.get_mut(self).configuration = Some(configuration.clone());
        let physics_from_widget =
            self.widget(app).physics.clone().or_else(|| {
                widget_behavior.map(|behavior| behavior.get_scroll_physics(app, context))
            });
        let mut physics = configuration.get_scroll_physics(app, context);
        physics = physics_from_widget.map_or(physics.clone(), |from_widget| {
            from_widget.apply_to(Some(physics))
        });
        app.get_mut(self).physics = Some(physics.clone());

        let old_position = app.get_mut(self).position.take();
        if let Some(old_position) = old_position.clone() {
            self.effective_scroll_controller(app)
                .detach(app, old_position.handle());
            // It's important that we not dispose the old position until after the viewport has
            // had a chance to unregister its listeners from the old position. So, schedule a
            // microtask to do it.
            app.schedule_microtask(Listener::new(move |app| old_position.dispose(app)));
        }

        let scroll_context = self.scroll_context(app);
        let position = self
            .effective_scroll_controller(app)
            .create_scroll_position(app, physics, scroll_context, old_position.handle());
        app.get_mut(self).position = Some(position.clone());
        self.effective_scroll_controller(app).attach(app, position);
    }

    fn should_update_position(self: Handle<Self>, app: &mut App, old_widget: &Scrollable) -> bool {
        let scroll_behavior = self.widget(app).scroll_behavior.clone();
        if scroll_behavior.is_none() != old_widget.scroll_behavior.is_none() {
            return true;
        }
        if let (Some(behavior), Some(old_behavior)) =
            (&scroll_behavior, &old_widget.scroll_behavior)
            && behavior.should_notify(&**old_behavior)
        {
            return true;
        }
        let context = self.context(app);
        let mut new_physics = self.widget(app).physics.clone().or_else(|| {
            scroll_behavior
                .as_ref()
                .map(|behavior| behavior.get_scroll_physics(app, context))
        });
        let mut old_physics = old_widget.physics.clone().or_else(|| {
            old_widget
                .scroll_behavior
                .as_ref()
                .map(|behavior| behavior.get_scroll_physics(app, context))
        });
        loop {
            let new_type = new_physics
                .as_ref()
                .map(|physics| physics.as_any().type_id());
            let old_type = old_physics
                .as_ref()
                .map(|physics| physics.as_any().type_id());
            if new_type != old_type {
                return true;
            }
            new_physics = new_physics.and_then(|physics| physics.parent().cloned());
            old_physics = old_physics.and_then(|physics| physics.parent().cloned());
            if new_physics.is_none() && old_physics.is_none() {
                break;
            }
        }

        self.widget(app)
            .controller
            .handle()
            .map(|controller| controller.type_id())
            != old_widget
                .controller
                .handle()
                .map(|controller| controller.type_id())
    }

    // TOUCH HANDLERS

    fn handle_drag_down(self: Handle<Self>, app: &mut App, _details: DragDownDetails) {
        debug_assert!(app.get(self).drag.is_none());
        debug_assert!(app.get(self).hold.is_none());
        let hold = self
            .position(app)
            .hold(app, Listener::handle_method(self, Self::dispose_hold));
        app.get_mut(self).hold = Some(hold);
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, details: DragStartDetails) {
        // It's possible for the hold to become `None` between `handle_drag_down` and
        // `handle_drag_start`, for example if some user code calls jumpTo or otherwise
        // triggers a new activity to begin.
        debug_assert!(app.get(self).drag.is_none());
        let drag = self.position(app).drag(
            app,
            details,
            Listener::handle_method(self, Self::dispose_drag),
        );
        app.get_mut(self).drag = Some(drag);
        // The hold might be non-`None` if the scroll position is currently animating.
        if app.get(self).hold.is_some() {
            self.dispose_hold(app);
        }
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        // The drag might be `None` if the drag activity ended and called `dispose_drag`.
        debug_assert!(app.get(self).hold.is_none() || app.get(self).drag.is_none());
        if let Some(drag) = app.get(self).drag.clone() {
            drag.update(app, details);
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        // The drag might be `None` if the drag activity ended and called `dispose_drag`.
        debug_assert!(app.get(self).hold.is_none() || app.get(self).drag.is_none());
        if let Some(drag) = app.get(self).drag.clone() {
            drag.end(app, details);
        }
        debug_assert!(app.get(self).drag.is_none());
    }

    fn handle_drag_cancel(self: Handle<Self>, app: &mut App) {
        if self
            .gesture_detector_key(app)
            .current_context(app)
            .is_none()
        {
            // The cancel was caused by the gesture detector getting disposed, which means we
            // will get disposed momentarily as well and shouldn't do any work.
            return;
        }
        // The hold might be `None` if the drag started.
        // The drag might be `None` if the drag activity ended and called `dispose_drag`.
        debug_assert!(app.get(self).hold.is_none() || app.get(self).drag.is_none());
        if let Some(hold) = app.get(self).hold.clone() {
            hold.cancel(app);
        }
        if let Some(drag) = app.get(self).drag.clone() {
            drag.cancel(app);
        }
        debug_assert!(app.get(self).hold.is_none());
        debug_assert!(app.get(self).drag.is_none());
    }

    fn dispose_hold(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).hold = None;
    }

    fn dispose_drag(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).drag = None;
    }

    fn gesture_detector_key(self: Handle<Self>, app: &App) -> GlobalKey {
        app.get(self).gesture_detector_key.clone()
    }

    // SCROLL WHEEL

    /// Returns the offset that should result from applying `delta` to the current position,
    /// taking min/max scroll extent into account.
    fn target_scroll_offset_for_pointer_scroll(self: Handle<Self>, app: &App, delta: f64) -> f64 {
        let position = self.position(app);
        (position.pixels(app) + delta)
            .max(position.min_scroll_extent(app))
            .min(position.max_scroll_extent(app))
    }

    /// Returns the delta that should result from applying `event` with axis,
    /// direction, and any modifiers specified by the scroll behavior taken into account.
    fn pointer_signal_event_delta(self: Handle<Self>, app: &mut App, event: &PointerEvent) -> f64 {
        let PointerEvent::Scroll(scroll) = event else {
            unreachable!("a pointer signal delta is only read for a scroll event");
        };
        let pressed = HardwareKeyboard::instance(app).logical_keys_pressed(app);
        let modifiers = self.configuration(app).pointer_axis_modifiers();
        let flip_axes = pressed.iter().any(|key| modifiers.contains(key))
            // Axes are only flipped for physical mouse wheel input.
            // On some platforms, like web, trackpad input is handled through pointer
            // signals, but should not be included in this axis modifying behavior.
            // This is because on a trackpad, all directional axes are available to
            // the user, while mouse scroll wheels typically are restricted to one axis.
            && event.kind() == waft_embedder::PointerDeviceKind::Mouse;

        let widget_axis = self.widget(app).axis();
        let axis = if flip_axes {
            flip_axis(widget_axis)
        } else {
            widget_axis
        };
        let delta = match axis {
            Axis::Horizontal => scroll.scroll_delta.dx(),
            Axis::Vertical => scroll.scroll_delta.dy(),
        };

        if axis_direction_is_reversed(self.widget(app).axis_direction) {
            -delta
        } else {
            delta
        }
    }

    fn received_pointer_signal(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        match event {
            PointerEvent::Scroll(_) if app.get(self).position.is_some() => {
                if let Some(physics) = self.resolved_physics(app) {
                    let metrics = self.position(app).copy_with(app);
                    if !physics.should_accept_user_offset(&*metrics) {
                        return;
                    }
                }
                let delta = self.pointer_signal_event_delta(app, event);
                let target_scroll_offset = self.target_scroll_offset_for_pointer_scroll(app, delta);
                // Only express interest in the event if it would actually result in a scroll.
                if delta != 0.0 && target_scroll_offset != self.position(app).pixels(app) {
                    let resolver = GestureBinding::instance(app).pointer_signal_resolver(app);
                    resolver.register(
                        app,
                        event,
                        Rc::new(move |app, event| self.handle_pointer_scroll(app, event)),
                    );
                }
            }
            PointerEvent::ScrollInertiaCancel(_) => {
                // Don't use the pointer signal resolver, all hit-tested scrollables should stop.
                self.position(app).pointer_scroll(app, 0.0);
            }
            _ => {}
        }
    }

    fn handle_pointer_scroll(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        debug_assert!(matches!(event, PointerEvent::Scroll(_)));
        let delta = self.pointer_signal_event_delta(app, event);
        let target_scroll_offset = self.target_scroll_offset_for_pointer_scroll(app, delta);
        if delta != 0.0 && target_scroll_offset != self.position(app).pixels(app) {
            self.position(app).pointer_scroll(app, delta);
        }
    }

    fn build_chrome(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
    ) -> WidgetRef {
        let (axis_direction, clip_behavior) = {
            let widget = self.widget(app);
            (widget.axis_direction, widget.clip_behavior)
        };
        let details = ScrollableDetails::new(axis_direction)
            .controller(self.effective_scroll_controller(app))
            .decoration_clip_behavior(clip_behavior);

        let configuration = self.configuration(app);
        let decorated = configuration.build_overscroll_indicator(app, context, child, &details);
        configuration.build_scrollbar(app, context, decorated, &details)
    }

    /// Runs `ensure_visible` for the scroll position, so its context can be used to
    /// check for other ancestor scrollables in executing
    /// [`Scrollable::ensure_visible`]; Dart's `_performEnsureVisible`.
    #[allow(clippy::too_many_arguments)]
    fn perform_ensure_visible(
        self: Handle<Self>,
        app: &mut App,
        object: AnyRenderObject,
        alignment: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        alignment_policy: ScrollPositionAlignmentPolicy,
        target_render_object: Option<AnyRenderObject>,
    ) -> EnsureVisibleResults {
        let ensure_visible_future = self.position(app).ensure_visible(
            app,
            object,
            alignment,
            duration,
            curve,
            alignment_policy,
            target_render_object,
        );
        (vec![ensure_visible_future], self)
    }
}

impl TickerProviderObject for ScrollableState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl TickerProviderStateMixin for ScrollableState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl RestorationMixin for ScrollableState {
    crate::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<RestorationBucket>>,
        initial_restore: bool,
    ) {
        let property = self.persisted_scroll_offset(app);
        self.register_for_restoration(app, property.as_property(), "offset");
        debug_assert!(app.get(self).position.is_some());
        if let Some(offset) = *RestorableValue::value(property, app) {
            self.position(app)
                .restore_offset(app, offset, initial_restore);
        }
    }
}

impl ScrollContext for Handle<ScrollableState> {
    fn type_name(&self) -> &'static str {
        "ScrollableState"
    }

    fn handle_id(&self) -> HandleId {
        self.id()
    }

    fn vsync(&self) -> Rc<dyn TickerProvider> {
        Rc::new(*self)
    }

    fn notification_context(&self, app: &mut App) -> Option<BuildContext> {
        let this = *self;
        app.get(this)
            .gesture_detector_key
            .clone()
            .current_context(app)
    }

    fn storage_context(&self, app: &App) -> BuildContext {
        let this = *self;
        this.context(app)
    }

    fn axis_direction(&self, app: &App) -> AxisDirection {
        let this = *self;
        this.widget(app).axis_direction
    }

    fn device_pixel_ratio(&self, app: &App) -> f64 {
        let this = *self;
        app.get(this).device_pixel_ratio
    }

    fn set_ignore_pointer(&self, app: &mut App, value: bool) {
        let this = *self;
        if app.get(this).should_ignore_pointer == value {
            return;
        }
        app.get_mut(this).should_ignore_pointer = value;
        let ignore_pointer_key = app.get(this).ignore_pointer_key.clone();
        if let Some(context) = ignore_pointer_key.current_context(app) {
            let render_box = context
                .find_render_object(app)
                .expect("a mounted IgnorePointer has a render object")
                .downcast::<RenderIgnorePointer>(app)
                .expect("the IgnorePointer's render object");
            render_box.set_ignoring(app, value);
        }
    }

    fn set_can_drag(&self, app: &mut App, value: bool) {
        let this = *self;
        let widget_axis = this.widget(app).axis();
        if Some(value) == app.get(this).last_can_drag
            && (!value || Some(widget_axis) == app.get(this).last_axis_direction)
        {
            return;
        }
        if value {
            let configuration = this.configuration(app);
            let context = this.context(app);
            let drag_devices = configuration.drag_devices();
            let velocity_tracker_builder = configuration.velocity_tracker_builder(app, context);
            let multitouch_drag_strategy = configuration.get_multitouch_drag_strategy(app, context);
            let gesture_settings = app.get(this).media_query_gesture_settings;
            let drag_start_behavior = this.widget(app).drag_start_behavior;
            let physics = this.resolved_physics(app);
            let (min_fling_distance, min_fling_velocity, max_fling_velocity) = (
                physics.as_ref().map(|physics| physics.min_fling_distance()),
                physics.as_ref().map(|physics| physics.min_fling_velocity()),
                physics.as_ref().map(|physics| physics.max_fling_velocity()),
            );
            let constructed_devices = drag_devices.clone();
            macro_rules! drag_factory {
                ($recognizer:ty) => {{
                    let devices = drag_devices.clone();
                    let velocity_tracker_builder = velocity_tracker_builder.clone();
                    GestureRecognizerFactoryWithHandlers::<$recognizer>::new(
                        move |app| {
                            let instance = <$recognizer>::new(app);
                            instance.set_supported_devices(app, Some(constructed_devices.clone()));
                            instance
                        },
                        move |app, instance| {
                            instance.set_on_down(
                                app,
                                Some(Rc::new(move |app: &mut App, details| {
                                    this.handle_drag_down(app, details);
                                })),
                            );
                            instance.set_on_start(
                                app,
                                Some(Rc::new(move |app: &mut App, details| {
                                    this.handle_drag_start(app, details);
                                })),
                            );
                            instance.set_on_update(
                                app,
                                Some(Rc::new(move |app: &mut App, details| {
                                    this.handle_drag_update(app, details);
                                })),
                            );
                            instance.set_on_end(
                                app,
                                Some(Rc::new(move |app: &mut App, details| {
                                    this.handle_drag_end(app, details);
                                })),
                            );
                            instance.set_on_cancel(
                                app,
                                Some(Listener::handle_method(
                                    this,
                                    ScrollableState::handle_drag_cancel,
                                )),
                            );
                            instance.set_min_fling_distance(app, min_fling_distance);
                            instance.set_min_fling_velocity(app, min_fling_velocity);
                            instance.set_max_fling_velocity(app, max_fling_velocity);
                            instance.set_velocity_tracker_builder(
                                app,
                                velocity_tracker_builder.clone(),
                            );
                            instance.set_drag_start_behavior(app, drag_start_behavior);
                            instance.set_multitouch_drag_strategy(app, multitouch_drag_strategy);
                            instance.set_gesture_settings(app, gesture_settings);
                            instance.set_supported_devices(app, Some(devices.clone()));
                        },
                    )
                    .into_factory()
                }};
            }
            let recognizers: GestureRecognizerFactories = match widget_axis {
                Axis::Vertical => vec![(
                    TypeId::of::<VerticalDragGestureRecognizer>(),
                    drag_factory!(VerticalDragGestureRecognizer),
                )],
                Axis::Horizontal => vec![(
                    TypeId::of::<HorizontalDragGestureRecognizer>(),
                    drag_factory!(HorizontalDragGestureRecognizer),
                )],
            };
            app.get_mut(this).gesture_recognizers = recognizers;
        } else {
            app.get_mut(this).gesture_recognizers = Vec::new();
            // Cancel the active hold/drag (if any) because the gesture recognizers
            // will soon be disposed by our RawGestureDetector, and we won't be
            // receiving pointer up events to cancel the hold/drag.
            this.handle_drag_cancel(app);
        }
        app.get_mut(this).last_can_drag = Some(value);
        app.get_mut(this).last_axis_direction = Some(widget_axis);
        let gesture_detector_key = app.get(this).gesture_detector_key.clone();
        if let Some(detector) = gesture_detector_key.current_state::<RawGestureDetectorState>(app) {
            let recognizers = app.get(this).gesture_recognizers.clone();
            detector.replace_gesture_recognizers(app, recognizers);
        }
    }

    fn save_offset(&self, app: &mut App, offset: f64) {
        let this = *self;
        let property = this.persisted_scroll_offset(app);
        property.set_value(app, Some(offset));
        // `save_offset` is called after a scrolling ends and it is usually not
        // followed by a frame. Therefore, manually flush restoration data.
        RestorationManager::instance(app).flush_data(app);
    }
}

impl State for ScrollableState {
    type Widget = Scrollable;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).scroll_context = Some(Rc::new(self));
        let property = RestorableScrollOffset::new(app);
        app.get_mut(self).persisted_scroll_offset = Some(property);
        if self.widget(app).controller.is_none() {
            let fallback = ScrollController::default(app);
            app.get_mut(self).fallback_scroll_controller = Some(fallback);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        app.get_mut(self).media_query_gesture_settings =
            MediaQuery::maybe_gesture_settings_of(app, context);
        let device_pixel_ratio = MediaQuery::maybe_device_pixel_ratio_of(app, context)
            .unwrap_or_else(|| View::of(app, context).metrics().device_pixel_ratio);
        app.get_mut(self).device_pixel_ratio = device_pixel_ratio;
        self.update_position(app);
        self.did_change_dependencies_restoration(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Scrollable) {
        self.did_update_restoration_id(app);

        let controller = self.widget(app).controller.handle();
        if controller != old_widget.controller.handle() {
            let position = self.position(app);
            match old_widget.controller.handle() {
                None => {
                    // The old controller was `None`, meaning the fallback cannot be `None`.
                    // Dispose of the fallback.
                    let fallback = app
                        .get(self)
                        .fallback_scroll_controller
                        .handle()
                        .expect("a scrollable without a controller has a fallback");
                    debug_assert!(controller.is_some());
                    fallback.detach(app, position);
                    ScrollControllerLeaf::dispose(fallback, app);
                    app.get_mut(self).fallback_scroll_controller = None;
                }
                Some(old_controller) => {
                    // The old controller was not `None`, detach.
                    old_controller.detach(app, position);
                    if controller.is_none() {
                        // If the new controller is `None`, we need to set up the fallback.
                        let fallback = ScrollController::default(app);
                        app.get_mut(self).fallback_scroll_controller = Some(fallback);
                    }
                }
            }
            // Attach the updated effective scroll controller.
            let position = self.kept_position(app);
            self.effective_scroll_controller(app).attach(app, position);
        }

        if self.should_update_position(app, old_widget) {
            self.update_position(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let position = self.position(app);
        match self.widget(app).controller.handle() {
            Some(controller) => controller.detach(app, position),
            None => {
                if let Some(fallback) = app.get(self).fallback_scroll_controller.handle() {
                    fallback.detach(app, position);
                    ScrollControllerLeaf::dispose(fallback, app);
                }
            }
        }

        position.dispose(app);
        RestorableProperty::dispose(self.persisted_scroll_offset(app), app);
        self.dispose_restoration(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(app.get(self).position.is_some());
        // The scope must be placed above the build context returned by
        // `notification_context` so that we can get this state by doing the following:
        //
        //     Scrollable::of(app, notification.context())
        //
        // Since `notification_context` points to the gesture detector's context, the scope
        // must be placed above the widget using it: `RawGestureDetector`.
        let position = self.kept_position(app);
        let viewport_builder = self.widget(app).viewport_builder.clone();
        let viewport = viewport_builder(app, context, position.erase());
        let hit_test_behavior = self.widget(app).hit_test_behavior;
        let state = app.get(self);
        let result = ScrollableScope {
            scrollable: self,
            position: position.clone(),
            child: PointerListener::new()
                .on_pointer_signal(Rc::new(move |app: &mut App, event| {
                    self.received_pointer_signal(app, &event);
                }))
                .child(
                    RawGestureDetector::new()
                        .key(state.gesture_detector_key.clone().into())
                        .gestures(state.gesture_recognizers.clone())
                        .behavior(hit_test_behavior)
                        .child(
                            IgnorePointer::new()
                                .key(state.ignore_pointer_key.clone().into())
                                .ignoring(state.should_ignore_pointer)
                                .child(viewport),
                        ),
                )
                .into_widget(),
        }
        .into_widget();

        let result = self.build_chrome(app, context, result);
        // Selection is only enabled when there is a parent registrar.
        match SelectionContainer::maybe_of(app, context) {
            Some(registrar) => ScrollableSelectionHandler {
                state: self,
                position,
                registrar,
                child: result,
            }
            .into_widget(),
            None => result,
        }
    }
}

impl Debug for ScrollableState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScrollableState")
            .field("position", &self.position)
            .field("effective physics", &self.physics)
            .finish()
    }
}

// ---------------------------------------------------------------------------------------------
// ScrollableSelectionHandler

/// Dart's `_ScrollableSelectionHandler`: the [`SelectionContainer`] a [`Scrollable`] builds when
/// there is a [`SelectionRegistrar`] above it, so the subtree's selectables are its delegate's.
#[derive(Debug)]
struct ScrollableSelectionHandler {
    state: Handle<ScrollableState>,
    position: ScrollPositionRef,
    registrar: SelectionRegistrarRef,
    child: WidgetRef,
}

impl StatefulWidget for ScrollableSelectionHandler {
    type State = ScrollableSelectionHandlerState;

    fn create_state(&self) -> ScrollableSelectionHandlerState {
        ScrollableSelectionHandlerState {
            state: StateData::new(),
            selection_delegate: None,
        }
    }
}

/// The state of a [`ScrollableSelectionHandler`].
struct ScrollableSelectionHandlerState {
    state: StateData<ScrollableSelectionHandler>,
    selection_delegate: Option<Ref<ScrollableSelectionContainerDelegate>>,
}

impl ScrollableSelectionHandlerState {
    fn selection_delegate(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<ScrollableSelectionContainerDelegate> {
        app.get(self)
            .selection_delegate
            .as_ref()
            .expect("the delegate is created in init_state")
            .handle()
    }
}

impl State for ScrollableSelectionHandlerState {
    type Widget = ScrollableSelectionHandler;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let (state, position) = (widget.state, widget.position.clone());
        let delegate = ScrollableSelectionContainerDelegate::new(app, state, position);
        app.get_mut(self).selection_delegate = Some(delegate);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &ScrollableSelectionHandler,
    ) {
        let position = self.widget(app).position.clone();
        if old_widget.position != position {
            self.selection_delegate(app).set_position(app, position);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let delegate = self.selection_delegate(app);
        MultiSelectableSelectionContainerDelegate::dispose(delegate, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let delegate = self.selection_delegate(app).keep(app).into_erased();
        let widget = self.widget(app);
        SelectionContainer::new(delegate, widget.child.clone())
            .registrar(widget.registrar.clone())
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// ScrollableSelectionContainerDelegate

/// The zero width and height of the drag target a selection edge makes: a pointer drag is a
/// single point.
const K_DEFAULT_DRAG_TARGET_SIZE: f64 = 0.0;

/// An eyeballed value for a smooth scrolling speed.
const K_DEFAULT_SELECT_TO_SCROLL_VELOCITY_SCALAR: f64 = 30.0;

/// Dart's `_ScrollableSelectionContainerDelegate`: it keeps each selectable's recorded scroll
/// offset, so a selectable that scrolled into the selection is brought up to date with the edge
/// events it missed, and it scrolls the viewport when the selection is dragged past its edge.
struct ScrollableSelectionContainerDelegate {
    selection_container_delegate: SelectionContainerDelegateData,
    multi_selectable_selection_container_delegate: MultiSelectableSelectionContainerDelegateData,
    /// The scrollable whose viewport the selection scrolls.
    state: Handle<ScrollableState>,
    /// The position the delegate listens to and jumps; Dart's `_position`.
    position: ScrollPositionRef,
    auto_scroller: Ref<EdgeDraggingAutoScroller>,
    scheduled_layout_change: bool,
    current_drag_start_related_to_origin: Option<Offset>,
    current_drag_end_related_to_origin: Option<Offset>,
    /// The scrollable only auto scrolls if the selection starts in the scrollable.
    selection_starts_in_scrollable: bool,
    /// The scroll offset each selectable last received a start edge at.
    selectable_start_edge_update_records: Vec<(AnySelectable, f64)>,
    /// The scroll offset each selectable last received an end edge at.
    selectable_end_edge_update_records: Vec<(AnySelectable, f64)>,
}

impl ScrollableSelectionContainerDelegate {
    fn new(
        app: &mut App,
        state: Handle<ScrollableState>,
        position: ScrollPositionRef,
    ) -> Ref<ScrollableSelectionContainerDelegate> {
        let auto_scroller = EdgeDraggingAutoScroller::new(
            app,
            state,
            None,
            K_DEFAULT_SELECT_TO_SCROLL_VELOCITY_SCALAR,
        );
        let delegate = app.create(ScrollableSelectionContainerDelegate {
            selection_container_delegate: SelectionContainerDelegateData::new(),
            multi_selectable_selection_container_delegate:
                MultiSelectableSelectionContainerDelegateData::new(),
            state,
            position,
            auto_scroller,
            scheduled_layout_change: false,
            current_drag_start_related_to_origin: None,
            current_drag_end_related_to_origin: None,
            selection_starts_in_scrollable: false,
            selectable_start_edge_update_records: Vec::new(),
            selectable_end_edge_update_records: Vec::new(),
        });
        delegate
            .position(app)
            .add_listener(app, schedule_layout_change_listener(delegate.handle()));
        delegate
    }

    fn position(self: Handle<Self>, app: &App) -> ScrollPositionRef {
        app.get(self).position.clone()
    }

    /// Dart's `position =` setter: listen to the new position instead.
    fn set_position(self: Handle<Self>, app: &mut App, other: ScrollPositionRef) {
        if app.get(self).position == other {
            return;
        }
        let old = self.position(app);
        old.remove_listener(app, &schedule_layout_change_listener(self));
        app.get_mut(self).position = other;
        self.position(app)
            .add_listener(app, schedule_layout_change_listener(self));
    }

    fn auto_scroller(self: Handle<Self>, app: &App) -> Handle<EdgeDraggingAutoScroller> {
        app.get(self).auto_scroller.handle()
    }

    /// Dart's `_scheduleLayoutChange`: the layout is only accurate a frame after a scroll.
    fn schedule_layout_change(self: Handle<Self>, app: &mut App) {
        if app.get(self).scheduled_layout_change {
            return;
        }
        app.get_mut(self).scheduled_layout_change = true;
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::handle_method(self, run_scheduled_layout_change_callback),
            None,
        );
    }

    /// Dart's `layoutDidChange` branch of the scheduled post-frame callback.
    fn run_scheduled_layout_change(self: Handle<Self>, app: &mut App) {
        if !app.get(self).scheduled_layout_change {
            return;
        }
        app.get_mut(self).scheduled_layout_change = false;
        MultiSelectableSelectionContainerDelegate::layout_did_change(self, app);
    }

    fn scrollable_render_box(self: Handle<Self>, app: &App) -> waft_rendering::AnyRenderBox {
        let state = app.get(self).state;
        state
            .context(app)
            .find_render_object(app)
            .and_then(|object| object.as_box())
            .expect("a Scrollable's render object is a RenderBox")
    }

    fn delta_to_scroll_origin(self: Handle<Self>, app: &App) -> Offset {
        app.get(self).state.delta_to_scroll_origin(app)
    }

    /// Dart's `_inferPositionRelatedToOrigin`.
    fn infer_position_related_to_origin(
        self: Handle<Self>,
        app: &App,
        global_position: Offset,
    ) -> Offset {
        let scrollable_box = self.scrollable_render_box(app);
        let local_position = scrollable_box.global_to_local(app, global_position, None);
        if !app.get(self).selection_starts_in_scrollable {
            // If the selection starts outside the scrollable, selecting across its boundary
            // selects the whole content in it: a point outside moves to the origin or to
            // infinity.
            if local_position.dy() < 0.0 || local_position.dx() < 0.0 {
                return scrollable_box.local_to_global(app, Offset::ZERO, None);
            }
            let size = scrollable_box.size(app);
            if local_position.dy() > size.height() || local_position.dx() > size.width() {
                return Offset::INFINITE;
            }
        }
        let delta_to_origin = self.delta_to_scroll_origin(app);
        scrollable_box.local_to_global(
            app,
            local_position.translate(delta_to_origin.dx(), delta_to_origin.dy()),
            None,
        )
    }

    /// Dart's `_updateDragLocationsFromGeometries`: infer the drag start and end from the
    /// selected edges' geometries, after a select-word or select-all that no drag made.
    fn update_drag_locations_from_geometries(
        self: Handle<Self>,
        app: &mut App,
        force_update_start: bool,
        force_update_end: bool,
    ) {
        let delta_to_origin = self.delta_to_scroll_origin(app);
        let scrollable_box = self.scrollable_render_box(app);
        let transform = scrollable_box.as_object().get_transform_to(app, None);
        let data = self.multi_selectable_selection_container_delegate_data(app);
        let start_index = data.current_selection_start_index;
        let end_index = data.current_selection_end_index;
        let selectables = data.selectables.clone();
        let current_drag_start = app.get(self).current_drag_start_related_to_origin;
        let current_drag_end = app.get(self).current_drag_end_related_to_origin;
        if start_index != -1 && (current_drag_start.is_none() || force_update_start) {
            let selectable = selectables[start_index as usize];
            let geometry = selectable.value(app);
            debug_assert!(geometry.has_selection());
            let start = geometry
                .start_selection_point
                .expect("a selectable with a selection has a start point");
            let child_transform =
                selectable.get_transform_to(app, Some(scrollable_box.as_object()));
            let local_drag_start = transform_point(
                &child_transform,
                start.local_position + Offset::new(0.0, -start.line_height / 2.0),
            );
            let drag_start = transform_point(&transform, local_drag_start + delta_to_origin);
            app.get_mut(self).current_drag_start_related_to_origin = Some(drag_start);
        }
        if end_index != -1 && (current_drag_end.is_none() || force_update_end) {
            let selectable = selectables[end_index as usize];
            let geometry = selectable.value(app);
            debug_assert!(geometry.has_selection());
            let end = geometry
                .end_selection_point
                .expect("a selectable with a selection has an end point");
            let child_transform =
                selectable.get_transform_to(app, Some(scrollable_box.as_object()));
            let local_drag_end = transform_point(
                &child_transform,
                end.local_position + Offset::new(0.0, -end.line_height / 2.0),
            );
            let drag_end = transform_point(&transform, local_drag_end + delta_to_origin);
            app.get_mut(self).current_drag_end_related_to_origin = Some(drag_end);
        }
    }

    /// Dart's `_jumpToEdge`: scroll the edge the event moved back into the viewport.
    fn jump_to_edge(self: Handle<Self>, app: &mut App, is_extent: bool) {
        let data = self.multi_selectable_selection_container_delegate_data(app);
        let selectable = if is_extent {
            data.selectables[data.current_selection_end_index as usize]
        } else {
            data.selectables[data.current_selection_start_index as usize]
        };
        let geometry = selectable.value(app);
        let (edge, line_height) = if is_extent {
            (
                geometry.end_selection_point,
                geometry.end_selection_point.map(|point| point.line_height),
            )
        } else {
            (
                geometry.start_selection_point,
                geometry
                    .start_selection_point
                    .map(|point| point.line_height),
            )
        };
        let (Some(edge), Some(line_height)) = (edge, line_height) else {
            return;
        };
        let scrollable_box = self.scrollable_render_box(app);
        let transform = selectable.get_transform_to(app, Some(scrollable_box.as_object()));
        let edge_offset = transform_point(&transform, edge.local_position);
        let size = scrollable_box.size(app);
        let scrollable_rect = Rect::from_ltwh(0.0, 0.0, size.width(), size.height());
        let state = app.get(self).state;
        let position = self.position(app);
        match state.axis_direction(app) {
            AxisDirection::Up => {
                let edge_bottom = edge_offset.dy();
                let edge_top = edge_offset.dy() - line_height;
                if edge_bottom >= scrollable_rect.bottom && edge_top <= scrollable_rect.top {
                    return;
                }
                if edge_bottom > scrollable_rect.bottom {
                    position.jump_to(
                        app,
                        position.pixels(app) + scrollable_rect.bottom - edge_bottom,
                    );
                    return;
                }
                if edge_top < scrollable_rect.top {
                    position.jump_to(app, position.pixels(app) + scrollable_rect.top - edge_top);
                }
            }
            AxisDirection::Right => {
                let edge = edge_offset.dx();
                if edge >= scrollable_rect.right && edge <= scrollable_rect.left {
                    return;
                }
                if edge > scrollable_rect.right {
                    position.jump_to(app, position.pixels(app) + edge - scrollable_rect.right);
                    return;
                }
                if edge < scrollable_rect.left {
                    position.jump_to(app, position.pixels(app) + edge - scrollable_rect.left);
                }
            }
            AxisDirection::Down => {
                let edge_bottom = edge_offset.dy();
                let edge_top = edge_offset.dy() - line_height;
                if edge_bottom >= scrollable_rect.bottom && edge_top <= scrollable_rect.top {
                    return;
                }
                if edge_bottom > scrollable_rect.bottom {
                    position.jump_to(
                        app,
                        position.pixels(app) + edge_bottom - scrollable_rect.bottom,
                    );
                    return;
                }
                if edge_top < scrollable_rect.top {
                    position.jump_to(app, position.pixels(app) + edge_top - scrollable_rect.top);
                }
            }
            AxisDirection::Left => {
                let edge = edge_offset.dx();
                if edge >= scrollable_rect.right && edge <= scrollable_rect.left {
                    return;
                }
                if edge > scrollable_rect.right {
                    position.jump_to(app, position.pixels(app) + scrollable_rect.right - edge);
                    return;
                }
                if edge < scrollable_rect.left {
                    position.jump_to(app, position.pixels(app) + scrollable_rect.left - edge);
                }
            }
        }
    }

    /// Dart's `_globalPositionInScrollable`.
    fn global_position_in_scrollable(
        self: Handle<Self>,
        app: &App,
        global_position: Offset,
    ) -> bool {
        let scrollable_box = self.scrollable_render_box(app);
        let local_position = scrollable_box.global_to_local(app, global_position, None);
        let size = scrollable_box.size(app);
        Rect::from_ltwh(0.0, 0.0, size.width(), size.height()).contains(local_position)
    }

    /// Dart's `_dragTargetFromEvent`.
    fn drag_target_from_event(self: Handle<Self>, event: &SelectionEvent) -> Rect {
        let global_position = match event {
            SelectionEvent::StartEdgeUpdate {
                global_position, ..
            }
            | SelectionEvent::EndEdgeUpdate {
                global_position, ..
            } => *global_position,
            _ => panic!("a selection edge update names an edge"),
        };
        Rect::from_center(
            global_position,
            K_DEFAULT_DRAG_TARGET_SIZE,
            K_DEFAULT_DRAG_TARGET_SIZE,
        )
    }

    fn record_start(self: Handle<Self>, app: &mut App, selectable: AnySelectable, offset: f64) {
        record(
            &mut app.get_mut(self).selectable_start_edge_update_records,
            selectable,
            offset,
        );
    }

    fn record_end(self: Handle<Self>, app: &mut App, selectable: AnySelectable, offset: f64) {
        record(
            &mut app.get_mut(self).selectable_end_edge_update_records,
            selectable,
            offset,
        );
    }
}

fn record(records: &mut Vec<(AnySelectable, f64)>, selectable: AnySelectable, offset: f64) {
    match records.iter_mut().find(|(held, _)| *held == selectable) {
        Some((_, held_offset)) => *held_offset = offset,
        None => records.push((selectable, offset)),
    }
}

fn recorded_offset(records: &[(AnySelectable, f64)], selectable: AnySelectable) -> Option<f64> {
    records
        .iter()
        .find_map(|(held, offset)| (*held == selectable).then_some(*offset))
}

fn remove_record(records: &mut Vec<(AnySelectable, f64)>, selectable: AnySelectable) {
    records.retain(|(held, _)| *held != selectable);
}

fn schedule_layout_change_listener(this: Handle<ScrollableSelectionContainerDelegate>) -> Listener {
    Listener::handle_method(this, schedule_layout_change_listener_body)
}

fn schedule_layout_change_listener_body(
    this: Handle<ScrollableSelectionContainerDelegate>,
    app: &mut App,
) {
    this.schedule_layout_change(app);
}

fn run_scheduled_layout_change_callback(
    this: Handle<ScrollableSelectionContainerDelegate>,
    app: &mut App,
    _time_stamp: Duration,
) {
    this.run_scheduled_layout_change(app);
}

impl SelectionContainerDelegate for ScrollableSelectionContainerDelegate {
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

impl MultiSelectableSelectionContainerDelegate for ScrollableSelectionContainerDelegate {
    crate::multi_selectable_selection_container_delegate_accessors!();

    fn did_change_selectables(self: Handle<Self>, app: &mut App) {
        let selectables = self
            .multi_selectable_selection_container_delegate_data(app)
            .selectables
            .clone();
        app.get_mut(self)
            .selectable_start_edge_update_records
            .retain(|(selectable, _)| selectables.contains(selectable));
        app.get_mut(self)
            .selectable_end_edge_update_records
            .retain(|(selectable, _)| selectables.contains(selectable));
        MultiSelectableSelectionContainerDelegateBase::did_change_selectables(self, app);
    }

    fn handle_clear_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let data = app.get_mut(self);
        data.selectable_start_edge_update_records.clear();
        data.selectable_end_edge_update_records.clear();
        data.current_drag_start_related_to_origin = None;
        data.current_drag_end_related_to_origin = None;
        data.selection_starts_in_scrollable = false;
        MultiSelectableSelectionContainerDelegateBase::handle_clear_selection(self, app, event)
    }

    fn handle_selection_edge_update(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let (global_position, is_end, granularity) = match event {
            SelectionEvent::StartEdgeUpdate {
                global_position,
                granularity,
            } => (*global_position, false, *granularity),
            SelectionEvent::EndEdgeUpdate {
                global_position,
                granularity,
            } => (*global_position, true, *granularity),
            _ => panic!("a selection edge update names an edge"),
        };
        if app.get(self).current_drag_end_related_to_origin.is_none()
            && app.get(self).current_drag_start_related_to_origin.is_none()
        {
            debug_assert!(!app.get(self).selection_starts_in_scrollable);
            let starts = self.global_position_in_scrollable(app, global_position);
            app.get_mut(self).selection_starts_in_scrollable = starts;
        }
        let delta_to_origin = self.delta_to_scroll_origin(app);
        let related_to_origin = self.infer_position_related_to_origin(app, global_position);
        if is_end {
            app.get_mut(self).current_drag_end_related_to_origin = Some(related_to_origin);
        } else {
            app.get_mut(self).current_drag_start_related_to_origin = Some(related_to_origin);
        }
        let offset = related_to_origin.translate(-delta_to_origin.dx(), -delta_to_origin.dy());
        let event = if is_end {
            SelectionEvent::for_end(offset, Some(granularity))
        } else {
            SelectionEvent::for_start(offset, Some(granularity))
        };
        let result = MultiSelectableSelectionContainerDelegateBase::handle_selection_edge_update(
            self, app, &event,
        );

        // The result may be pending if one of the selectable children is also a scrollable; the
        // parent scrollable then waits for the child to finish scrolling.
        if result == SelectionResult::Pending {
            self.auto_scroller(app).stop_auto_scroll(app);
            return result;
        }
        if app.get(self).selection_starts_in_scrollable {
            // Paged scrollables (like a PageView) disallow this; edge scrolling there flips to
            // an adjacent page instead of revealing more content.
            let allow = app
                .get(self)
                .state
                .resolved_physics(app)
                .is_none_or(|physics| physics.allow_selection_edge_scrolling());
            if !allow {
                self.auto_scroller(app).stop_auto_scroll(app);
                return result;
            }
            let drag_target = self.drag_target_from_event(&event);
            self.auto_scroller(app)
                .start_auto_scroll_if_necessary(app, drag_target);
            if self.auto_scroller(app).scrolling(app) {
                return SelectionResult::Pending;
            }
        }
        result
    }

    fn handle_select_all(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        debug_assert!(!app.get(self).selection_starts_in_scrollable);
        let result =
            MultiSelectableSelectionContainerDelegateBase::handle_select_all(self, app, event);
        let data = self.multi_selectable_selection_container_delegate_data(app);
        debug_assert_eq!(
            data.current_selection_start_index == -1,
            data.current_selection_end_index == -1
        );
        if data.current_selection_start_index != -1 {
            self.update_drag_locations_from_geometries(app, true, true);
        }
        result
    }

    fn handle_select_word(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let global_position = match event {
            SelectionEvent::SelectWord { global_position } => *global_position,
            _ => panic!("a select-word event names a position"),
        };
        let starts = self.global_position_in_scrollable(app, global_position);
        app.get_mut(self).selection_starts_in_scrollable = starts;
        let result =
            MultiSelectableSelectionContainerDelegateBase::handle_select_word(self, app, event);
        self.update_drag_locations_from_geometries(app, true, true);
        result
    }

    fn handle_granularly_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let result =
            MultiSelectableSelectionContainerDelegateBase::handle_granularly_extend_selection(
                self, app, event,
            );
        let is_end = matches!(
            event,
            SelectionEvent::GranularlyExtendSelection { is_end: true, .. }
        );
        // The geometry of an edge outside the viewport may not be valid; only the edge this
        // event updated is sure to be accurate.
        self.update_drag_locations_from_geometries(app, !is_end, is_end);
        if app.get(self).selection_starts_in_scrollable {
            self.jump_to_edge(app, is_end);
        }
        result
    }

    fn handle_directionally_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let result =
            MultiSelectableSelectionContainerDelegateBase::handle_directionally_extend_selection(
                self, app, event,
            );
        let is_end = matches!(
            event,
            SelectionEvent::DirectionallyExtendSelection { is_end: true, .. }
        );
        self.update_drag_locations_from_geometries(app, !is_end, is_end);
        if app.get(self).selection_starts_in_scrollable {
            self.jump_to_edge(app, is_end);
        }
        result
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self)
            .selectable_start_edge_update_records
            .clear();
        app.get_mut(self).selectable_end_edge_update_records.clear();
        app.get_mut(self).scheduled_layout_change = false;
        self.auto_scroller(app).stop_auto_scroll(app);
        let position = self.position(app);
        position.remove_listener(app, &schedule_layout_change_listener(self));
        MultiSelectableSelectionContainerDelegateBase::dispose(self, app);
    }

    fn ensure_child_updated(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        let new_record = app.get(self).state.position(app).pixels(app);
        let previous_start_record = recorded_offset(
            &app.get(self).selectable_start_edge_update_records,
            selectable,
        );
        if let Some(current_start) = app.get(self).current_drag_start_related_to_origin
            && (previous_start_record.is_none()
                || (new_record - previous_start_record.expect("checked just now")).abs()
                    > PRECISION_ERROR_TOLERANCE)
        {
            // Make sure the selectable has up to date events.
            let delta_to_origin = self.delta_to_scroll_origin(app);
            let start_offset =
                current_start.translate(-delta_to_origin.dx(), -delta_to_origin.dy());
            selectable
                .dispatch_selection_event(app, &SelectionEvent::for_start(start_offset, None));
            // Track the synthesized start event, so it is not synthesized again.
            self.record_start(
                app,
                selectable,
                app.get(self).state.position(app).pixels(app),
            );
        }
        let previous_end_record = recorded_offset(
            &app.get(self).selectable_end_edge_update_records,
            selectable,
        );
        if let Some(current_end) = app.get(self).current_drag_end_related_to_origin
            && (previous_end_record.is_none()
                || (new_record - previous_end_record.expect("checked just now")).abs()
                    > PRECISION_ERROR_TOLERANCE)
        {
            let delta_to_origin = self.delta_to_scroll_origin(app);
            let end_offset = current_end.translate(-delta_to_origin.dx(), -delta_to_origin.dy());
            selectable.dispatch_selection_event(app, &SelectionEvent::for_end(end_offset, None));
            self.record_end(
                app,
                selectable,
                app.get(self).state.position(app).pixels(app),
            );
        }
    }

    fn dispatch_selection_event_to_child(
        self: Handle<Self>,
        app: &mut App,
        selectable: AnySelectable,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let pixels = app.get(self).state.position(app).pixels(app);
        match event.ty() {
            SelectionEventType::StartEdgeUpdate => {
                self.record_start(app, selectable, pixels);
                MultiSelectableSelectionContainerDelegate::ensure_child_updated(
                    self, app, selectable,
                );
            }
            SelectionEventType::EndEdgeUpdate => {
                self.record_end(app, selectable, pixels);
                MultiSelectableSelectionContainerDelegate::ensure_child_updated(
                    self, app, selectable,
                );
            }
            SelectionEventType::GranularlyExtendSelection
            | SelectionEventType::DirectionallyExtendSelection => {
                MultiSelectableSelectionContainerDelegate::ensure_child_updated(
                    self, app, selectable,
                );
                self.record_start(app, selectable, pixels);
                self.record_end(app, selectable, pixels);
            }
            SelectionEventType::Clear => {
                remove_record(
                    &mut app.get_mut(self).selectable_end_edge_update_records,
                    selectable,
                );
                remove_record(
                    &mut app.get_mut(self).selectable_start_edge_update_records,
                    selectable,
                );
            }
            SelectionEventType::SelectAll
            | SelectionEventType::SelectWord
            | SelectionEventType::SelectParagraph => {
                self.record_end(app, selectable, pixels);
                self.record_start(app, selectable, pixels);
            }
        }
        MultiSelectableSelectionContainerDelegateBase::dispatch_selection_event_to_child(
            self, app, selectable, event,
        )
    }
}

impl SelectionHandler for ScrollableSelectionContainerDelegate {
    fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
        MultiSelectableSelectionContainerDelegateBase::value(self, app)
    }

    fn push_handle_layers(
        self: Handle<Self>,
        app: &mut App,
        start_handle: Option<Ref<waft_rendering::LayerLink>>,
        end_handle: Option<Ref<waft_rendering::LayerLink>>,
    ) {
        MultiSelectableSelectionContainerDelegateBase::push_handle_layers(
            self,
            app,
            start_handle,
            end_handle,
        );
    }

    fn selected_content(self: Handle<Self>, app: &App) -> Option<waft_rendering::SelectedContent> {
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
        MultiSelectableSelectionContainerDelegateBase::dispatch_selection_event(self, app, event)
    }

    fn content_length(self: Handle<Self>, app: &App) -> usize {
        MultiSelectableSelectionContainerDelegateBase::content_length(self, app)
    }
}

impl SelectionRegistrar for ScrollableSelectionContainerDelegate {
    fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        MultiSelectableSelectionContainerDelegateBase::add(self, app, selectable);
    }

    fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
        MultiSelectableSelectionContainerDelegateBase::remove(self, app, selectable);
    }
}

impl ChangeNotifier for ScrollableSelectionContainerDelegate {
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

/// Dart's `_RestorableScrollOffset`: the scroll offset a [`ScrollableState`] persists.
pub struct RestorableScrollOffset {
    change_notifier: waft_foundation::ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<Option<f64>>,
}

impl RestorableScrollOffset {
    /// Creates a property whose default value is "no stored offset".
    pub fn new(app: &mut App) -> Ref<RestorableScrollOffset> {
        app.create(RestorableScrollOffset {
            change_notifier: waft_foundation::ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
        })
    }
}

impl waft_foundation::ChangeNotifier for RestorableScrollOffset {
    fn change_notifier_data(&self) -> &waft_foundation::ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut waft_foundation::ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableScrollOffset {
    type Value = Option<f64>;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, _app: &mut App) -> Option<f64> {
        None
    }

    fn from_primitives(self: Handle<Self>, _app: &mut App, data: &RestorationData) -> Option<f64> {
        Some(
            data.as_double()
                .expect("a stored scroll offset is a double"),
        )
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        match RestorableValue::value(self, app) {
            Some(offset) => RestorationData::Double(*offset),
            None => RestorationData::Null,
        }
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        RestorableValue::value(self, app).is_some()
    }
}

impl RestorableValue for RestorableScrollOffset {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, _old_value: Option<Option<f64>>) {
        self.notify_listeners(app);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;
    use waft_foundation::AppCell;

    use waft_embedder::{PointerDeviceKind, Restoration, TargetPlatform, TextDirection};
    use waft_services::{RestorationMap, RestorationUpdate};
    use waft_test::TestPlatform;

    use super::*;
    use crate::framework::Element;
    use crate::test_harness::Harness;
    use crate::test_harness::pump_every_view;
    use crate::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, find, test_widgets, test_widgets_variant,
    };
    use crate::widgets::basic::{Builder, Directionality, SizedBox, SliverToBoxAdapter};
    use crate::widgets::media_query::{MediaQuery, MediaQueryData};
    use crate::widgets::restoration::RootRestorationScope;
    use crate::widgets::scroll_configuration::ScrollBehavior;
    use crate::widgets::scroll_controller::ScrollControllerLeaf;
    use crate::widgets::scroll_view::CustomScrollView;
    use waft_rendering::{RenderBox, RenderSliver};

    /// A host that answers `Restoration::get` with what it was handed and records every
    /// `Restoration::put`.
    #[derive(Default)]
    struct RecordingRestoration {
        stored: RefCell<Option<RestorationUpdate>>,
        puts: RefCell<Vec<RestorationMap>>,
    }

    impl Restoration for RecordingRestoration {
        fn get(&self) -> Option<RestorationUpdate> {
            self.stored.borrow().clone()
        }

        fn put(&self, data: RestorationMap) {
            self.puts.borrow_mut().push(data);
        }
    }

    fn app_restoring(data: Option<RestorationMap>) -> (Rc<AppCell>, Rc<RecordingRestoration>) {
        let restoration = Rc::new(RecordingRestoration::default());
        *restoration.stored.borrow_mut() = Some(RestorationUpdate {
            enabled: true,
            data,
        });
        let platform = TestPlatform::new().with_restoration(restoration.clone());
        (AppCell::with_platform(Rc::new(platform)), restoration)
    }

    fn map<const N: usize>(entries: [(&str, RestorationData); N]) -> RestorationMap {
        entries
            .into_iter()
            .map(|(key, value)| (RestorationData::from(key), value))
            .collect()
    }

    fn values<const N: usize>(entries: [(&str, RestorationData); N]) -> RestorationMap {
        map([("v", map(entries).into())])
    }

    fn child(restoration_id: &str, data: RestorationMap) -> RestorationMap {
        map([("c", map([(restoration_id, data.into())]).into())])
    }

    fn at<'a>(data: &'a RestorationMap, path: &[&str]) -> Option<&'a RestorationData> {
        let (last, parents) = path.split_last()?;
        let mut map = data;
        for key in parents {
            map = map.get(&RestorationData::from(*key))?.as_map()?;
        }
        map.get(&RestorationData::from(*last))
    }

    /// A viewport builder that records the offset it is handed and builds a leaf box.
    fn box_viewport() -> ViewportBuilder {
        Rc::new(|_app, _context, _offset| SizedBox::shrink().into_widget())
    }

    /// Mounts `scrollable` under a `MediaQuery` and hands back the state.
    fn mount_scrollable(
        app: &mut App,
        scrollable: Scrollable,
    ) -> (Harness, Handle<ScrollableState>) {
        let harness = Harness::mount(
            app,
            MediaQuery::new(MediaQueryData::new(), scrollable).into_widget(),
        );
        harness.pump(app);
        // MediaQuery, its inherited model scope, then the Scrollable's stateful element.
        let mut element = harness.root.as_element();
        let state = loop {
            element = element.children(app)[0];
            if let Some(state) = element.state_handle::<ScrollableState>(app) {
                break state;
            }
        };
        (harness, state)
    }

    /// A leaf widget whose render object is a `RenderViewport` over 800 logical pixels of
    /// content: what the `Viewport` widget builds, in miniature.
    #[derive(Debug)]
    struct TestViewport {
        offset: ViewportOffsetRef,
    }

    impl crate::framework::RenderObjectWidget for TestViewport {
        type RenderObject = waft_rendering::RenderViewport;

        fn create_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
        ) -> waft_rendering::AnyRenderObject {
            let content = waft_rendering::RenderConstrainedBox::new(
                app,
                waft_rendering::BoxConstraints::tight(waft_embedder::Size::new(300.0, 800.0)),
                None,
            );
            let sliver = waft_rendering::RenderSliverToBoxAdapter::new(
                app,
                Some(waft_rendering::RenderBox::as_box(content)),
            );
            waft_rendering::RenderViewport::new(
                app,
                AxisDirection::Right,
                self.offset.clone(),
                Some(vec![RenderSliver::as_sliver(sliver)]),
                None,
            )
            .as_object()
        }

        fn update_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
            render_object: waft_rendering::RenderHandle<waft_rendering::RenderViewport>,
        ) {
            waft_rendering::RenderViewportBase::set_offset(render_object, app, self.offset.clone());
        }
    }

    impl crate::framework::LeafRenderObjectWidget for TestViewport {}

    #[test]
    fn a_laid_out_scrollable_takes_its_dimensions_from_its_viewport() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let viewport: ViewportBuilder =
            Rc::new(|_app, _context, offset| TestViewport { offset }.into_widget());
        let (_harness, state) = mount_scrollable(&mut app, Scrollable::new(viewport));

        let position = state.position(&app);
        assert!(position.have_dimensions(&app));
        assert_eq!(
            position.viewport_dimension(&app),
            crate::test_harness::VIEW_HEIGHT
        );
        assert_eq!(
            position.max_scroll_extent(&app),
            800.0 - crate::test_harness::VIEW_HEIGHT
        );
        assert_eq!(position.min_scroll_extent(&app), 0.0);
        assert_eq!(position.pixels(&app), 0.0);
        // The physics accept a user offset, so the drag recognizer factory is in place.
        assert_eq!(app.get(state).gesture_recognizers.len(), 1);
    }

    #[test]
    fn a_scrollable_creates_a_position_on_its_effective_controller() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::new(&mut app, 17.0, true, None, None, None);
        let (_harness, state) = mount_scrollable(
            &mut app,
            Scrollable::new(box_viewport()).controller(controller.erase()),
        );

        assert_eq!(state.position(&app).pixels(&app), 17.0);
        assert_eq!(controller.positions(&app), vec![state.position(&app)]);
        assert_eq!(state.axis_direction(&app), AxisDirection::Down);
        assert!(state.resolved_physics(&app).is_some());
        assert_eq!(state.delta_to_scroll_origin(&app), Offset::new(0.0, 17.0));
    }

    #[test]
    fn a_scrollable_without_a_controller_uses_a_fallback_one() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (_harness, state) = mount_scrollable(&mut app, Scrollable::new(box_viewport()));

        assert_eq!(state.position(&app).pixels(&app), 0.0);
        assert!(app.get(state).fallback_scroll_controller.is_some());
    }

    #[test]
    fn of_finds_the_state_from_a_descendant_of_the_viewport() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen: Rc<Cell<Option<BuildContext>>> = Rc::new(Cell::new(None));
        let captured = seen.clone();
        let viewport: ViewportBuilder = Rc::new(move |_app, _context, _offset| {
            let captured = captured.clone();
            Builder::new(move |_app, context| {
                captured.set(Some(context));
                SizedBox::shrink().into_widget()
            })
            .into_widget()
        });
        let (_harness, state) = mount_scrollable(&mut app, Scrollable::new(viewport));

        let inner = seen.get().expect("the viewport built");
        assert_eq!(Scrollable::maybe_of(&mut app, inner, None), Some(state));
        assert_eq!(
            Scrollable::maybe_of(&mut app, inner, Some(Axis::Vertical)),
            Some(state)
        );
        assert_eq!(
            Scrollable::maybe_of(&mut app, inner, Some(Axis::Horizontal)),
            None
        );
        assert_eq!(Scrollable::of(&mut app, inner, None), state);
        // No scrollable in the horizontal axis encloses the context, so the walk ends there.
        assert!(!Scrollable::recommend_deferred_loading_for_context(
            &mut app,
            inner,
            Some(Axis::Horizontal)
        ));
    }

    #[test]
    fn a_scrollable_restores_its_offset_through_the_restoration_mixin() {
        let (cell, _restoration) = app_restoring(Some(child(
            "app",
            child(
                "scroll",
                values([("offset", RestorationData::Double(42.0))]),
            ),
        )));
        let mut app = cell.borrow_mut();
        let scrollable = Scrollable::new(box_viewport()).restoration_id("scroll");
        let harness = Harness::mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                MediaQuery::new(MediaQueryData::new(), scrollable).into_widget(),
            )
            .into_widget(),
        );
        harness.pump(&mut app);

        let mut element = harness.root.as_element();
        let state = loop {
            element = element.children(&app)[0];
            if let Some(state) = element.state_handle::<ScrollableState>(&app) {
                break state;
            }
        };
        assert_eq!(state.position(&app).pixels(&app), 42.0);
    }

    #[test]
    fn saving_an_offset_writes_it_into_the_restoration_data() {
        let (cell, restoration) = app_restoring(None);
        let mut app = cell.borrow_mut();
        let scrollable = Scrollable::new(box_viewport()).restoration_id("scroll");
        let harness = Harness::mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                MediaQuery::new(MediaQueryData::new(), scrollable).into_widget(),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let mut element = harness.root.as_element();
        let state = loop {
            element = element.children(&app)[0];
            if let Some(state) = element.state_handle::<ScrollableState>(&app) {
                break state;
            }
        };

        ScrollContext::save_offset(&state, &mut app, 55.0);
        pump_every_view(&mut app, Duration::ZERO);

        let stored = restoration.puts.borrow();
        let last = stored.last().expect("the manager sent the data");
        assert_eq!(
            at(last, &["c", "app", "c", "scroll", "v", "offset"]),
            Some(&RestorationData::Double(55.0))
        );
    }

    #[test]
    fn the_scroll_context_reports_the_media_querys_device_pixel_ratio() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            MediaQuery::new(
                MediaQueryData::new().device_pixel_ratio(3.0),
                Scrollable::new(box_viewport()),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let mut element = harness.root.as_element();
        let state = loop {
            element = element.children(&app)[0];
            if let Some(state) = element.state_handle::<ScrollableState>(&app) {
                break state;
            }
        };

        assert_eq!(ScrollContext::device_pixel_ratio(&state, &app), 3.0);
        assert_eq!(state.position(&app).device_pixel_ratio(&app), 3.0);
    }

    // ---- scrollable_test.dart: holds, flings and carried momentum ----

    /// `scrollable_test.dart`'s `dragOffset`.
    const DRAG_OFFSET: f64 = 200.0;

    /// The center of the `Scrollable` [`pump_test`] mounts, which fills the 800 x 600 view:
    /// where Flutter's `tester.getCenter(find.byType(Scrollable))` puts a gesture.
    const SCROLLABLE_CENTER: Offset = Offset::new(400.0, 300.0);

    /// `scrollable_test.dart`'s `NoScrollbarBehavior`.
    #[derive(Debug)]
    struct NoScrollbarBehavior;

    impl ScrollBehavior for NoScrollbarBehavior {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn build_scrollbar(
            &self,
            _app: &mut App,
            _context: BuildContext,
            child: WidgetRef,
            _details: &ScrollableDetails,
        ) -> WidgetRef {
            child
        }
    }

    /// `tester.startGesture(location)` for a touch numbered `pointer`.
    fn start_gesture(tester: &mut WidgetTester, pointer: i64, location: Offset) -> TestPointer {
        let mut gesture = TestPointer::touch(pointer);
        tester.down(&mut gesture, location);
        gesture
    }

    /// `tester.drag(find.byType(Scrollable), offset, touchSlopY: 0.0)` for a vertical
    /// `offset`: with no slop to break the drag at, one move by all of it.
    fn drag_without_touch_slop(tester: &mut WidgetTester, pointer: i64, offset: Offset) {
        assert_eq!(offset.dx(), 0.0, "a horizontal drag is split at the slop");
        let mut gesture = start_gesture(tester, pointer, SCROLLABLE_CENTER);
        tester.move_to(&mut gesture, SCROLLABLE_CENTER + offset, true);
        tester.up(&gesture);
    }

    /// `tester.fling(find.byType(Scrollable), offset, speed)`.
    fn fling(tester: &mut WidgetTester, pointer: i64, offset: Offset, speed: f64) {
        tester.fling_from(SCROLLABLE_CENTER, offset, speed, pointer);
    }

    fn scrollable(tester: &WidgetTester) -> Handle<ScrollableState> {
        let scrollable = tester.element(find::by_type::<Scrollable>());
        scrollable
            .state_handle::<ScrollableState>(&tester.cell.borrow())
            .expect("a Scrollable has a ScrollableState")
    }

    /// `scrollable_test.dart`'s `getScrollOffset`.
    fn scroll_offset(tester: &WidgetTester) -> f64 {
        let position = scrollable(tester).position(&tester.cell.borrow());
        position.pixels(&tester.cell.borrow())
    }

    /// `scrollable_test.dart`'s `getScrollVelocity`.
    fn scroll_velocity(tester: &WidgetTester) -> f64 {
        let position = scrollable(tester).position(&tester.cell.borrow());
        tester.update(|app| {
            let activity = position.activity(app).expect("a position has an activity");
            activity.velocity(app)
        })
    }

    /// `scrollable_test.dart`'s `pumpTest` with its defaults: a vertical `CustomScrollView`
    /// over 2000 pixels that every device kind drags, with no scrollbar.
    fn pump_test(tester: &mut WidgetTester) {
        let behavior = ScrollBehaviorRef::new(NoScrollbarBehavior)
            .copy_with()
            .drag_devices(HashSet::from([
                PointerDeviceKind::Touch,
                PointerDeviceKind::Mouse,
                PointerDeviceKind::Stylus,
                PointerDeviceKind::InvertedStylus,
                PointerDeviceKind::Trackpad,
                PointerDeviceKind::Unknown,
            ]));
        let scroll_view = CustomScrollView::new().slivers([SliverToBoxAdapter::new()
            .child(SizedBox::new().height(2000.0))
            .into_widget()]);
        tester.pump_widget(Directionality::new(
            TextDirection::Ltr,
            ScrollConfiguration::new(ScrollBehaviorRef::new(behavior), scroll_view),
        ));
    }

    /// `TargetPlatformVariant(<TargetPlatform>{TargetPlatform.iOS, TargetPlatform.macOS})`.
    fn ios_and_macos() -> TargetPlatformVariant {
        TargetPlatformVariant::new([TargetPlatform::IOS, TargetPlatform::MacOS])
    }

    /// `scrollable_test.dart`: "Holding scroll".
    #[test]
    #[ignore = "settles in 2 pumps, not 3: Flutter's third is the semantics update a ScrollMetricsNotification asks for, and accessibility is deferred"]
    fn a_hold_in_overscroll_stops_the_ballistic_and_its_release_snaps_back() {
        test_widgets_variant(ios_and_macos(), |tester| {
            pump_test(tester);
            drag_without_touch_slop(tester, 1, Offset::new(0.0, 200.0));
            assert_eq!(scroll_offset(tester), -200.0);
            tester.pump(); // trigger ballistic
            tester.pump_by(Duration::from_millis(10));
            assert!(scroll_offset(tester) > -200.0);
            assert!(scroll_offset(tester) < 0.0);
            let held_position = scroll_offset(tester);
            // Hold and let go while in overscroll.
            let gesture = start_gesture(tester, 2, SCROLLABLE_CENTER);
            assert_eq!(tester.pump_and_settle(), 1);
            assert_eq!(scroll_offset(tester), held_position);
            tester.up(&gesture);
            // Once the hold is let go, it should still snap back to origin.
            assert_eq!(tester.pump_and_settle_by(Duration::from_secs(60)), 3);
            assert_eq!(scroll_offset(tester), 0.0);
        });
    }

    /// `scrollable_test.dart`: "Repeated flings builds momentum".
    #[test]
    fn repeated_flings_build_momentum_on_ios_and_macos() {
        test_widgets_variant(ios_and_macos(), |tester| {
            pump_test(tester);
            fling(tester, 1, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump(); // trigger fling
            tester.pump_by(Duration::from_millis(10));
            // Repeat the exact same motion.
            fling(tester, 2, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump();
            // On iOS, the velocity will be larger than the velocity of the last fling by a
            // non-trivial amount.
            assert!(scroll_velocity(tester) > 1100.0);
        });
    }

    /// `scrollable_test.dart`: "Repeated flings do not build momentum on Android".
    #[test]
    fn repeated_flings_do_not_build_momentum_on_android() {
        test_widgets(|tester| {
            pump_test(tester);
            fling(tester, 1, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump(); // trigger fling
            tester.pump_by(Duration::from_millis(10));
            // Repeat the exact same motion.
            fling(tester, 2, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump();
            // On Android, there is no momentum build. The final velocity is the same as the
            // velocity of the last fling.
            let velocity = scroll_velocity(tester);
            assert!((velocity - 1000.0).abs() <= 1e-10, "{velocity}");
        });
    }

    /// `scrollable_test.dart`: "A slower final fling does not apply carried momentum".
    #[test]
    fn a_slower_final_fling_does_not_apply_carried_momentum() {
        test_widgets_variant(ios_and_macos(), |tester| {
            pump_test(tester);
            fling(tester, 1, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump(); // trigger fling
            tester.pump_by(Duration::from_millis(10));
            // Repeat the exact same motion to build momentum.
            fling(tester, 2, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump(); // trigger the second fling
            tester.pump_by(Duration::from_millis(10));
            // Make a final fling that is much slower.
            fling(tester, 3, Offset::new(0.0, -DRAG_OFFSET), 200.0);
            tester.pump(); // trigger the third fling
            tester.pump_by(Duration::from_millis(10));
            // expect that there is no carried velocity
            assert!(scroll_velocity(tester) < 200.0);
        });
    }

    /// `scrollable_test.dart`: "No iOS/macOS momentum build with flings in opposite
    /// directions".
    #[test]
    fn flings_in_opposite_directions_build_no_momentum_on_ios_and_macos() {
        test_widgets_variant(ios_and_macos(), |tester| {
            pump_test(tester);
            fling(tester, 1, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump(); // trigger fling
            tester.pump_by(Duration::from_millis(10));
            // Repeat the exact same motion in the opposite direction.
            fling(tester, 2, Offset::new(0.0, DRAG_OFFSET), 1000.0);
            tester.pump();
            // The only applied velocity to the scrollable is the second fling that was in the
            // opposite direction.
            assert_eq!(scroll_velocity(tester), -1000.0);
        });
    }

    /// `scrollable_test.dart`: "No iOS/macOS momentum kept on hold gestures".
    #[test]
    fn a_hold_keeps_no_momentum_on_ios_and_macos() {
        test_widgets_variant(ios_and_macos(), |tester| {
            pump_test(tester);
            fling(tester, 1, Offset::new(0.0, -DRAG_OFFSET), 1000.0);
            tester.pump(); // trigger fling
            tester.pump_by(Duration::from_millis(10));
            assert!(scroll_velocity(tester) > 0.0);
            let gesture = start_gesture(tester, 2, SCROLLABLE_CENTER);
            tester.pump_by(Duration::from_millis(40));
            tester.up(&gesture);
            // After a hold longer than 2 frames, previous velocity is lost.
            assert_eq!(scroll_velocity(tester), 0.0);
        });
    }

    // ---- selection ----

    use waft_embedder::TextPosition;
    use waft_rendering::{RenderHandle, RenderParagraph, SelectionResult};
    use waft_scheduler::{FrameTarget, SchedulerBinding};

    use crate::widgets::scroll_physics::NeverScrollableScrollPhysics;
    use crate::widgets::scroll_view::ListView;
    use crate::widgets::selectable_region::PlainStaticSelectionContainerDelegate;
    use crate::widgets::selection_container::SelectionContainer;
    use crate::widgets::text::Text;

    /// A list of `Item N` texts under a selection container, as under a `SelectableRegion`, and
    /// the container's delegate.
    fn pump_selection_list(
        tester: &mut WidgetTester,
        controller: &Ref<ScrollController>,
        physics: Option<ScrollPhysicsRef>,
    ) -> Ref<PlainStaticSelectionContainerDelegate> {
        let delegate = tester.update(PlainStaticSelectionContainerDelegate::new);
        let mut list = ListView::builder(|_app, _context, index| {
            Some(
                Text::new(format!("Item {index}"))
                    .text_direction(TextDirection::Ltr)
                    .into_widget(),
            )
        })
        .item_count(100)
        .build()
        .controller(controller.erase());
        if let Some(physics) = physics {
            list = list.physics(physics);
        }
        tester.pump_widget(Directionality::new(
            TextDirection::Ltr,
            SelectionContainer::new(delegate.erase(), list),
        ));
        // Selection reaches each delegate a frame after the one below it, as in Flutter, where
        // `pump` always produces a frame.
        for _ in 0..3 {
            tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::EveryView));
            tester.pump_and_settle();
        }
        delegate
    }

    /// The global position of the caret at `offset` in `paragraph`, five pixels down from the
    /// top of the line.
    fn paragraph_offset(
        tester: &mut WidgetTester,
        paragraph: RenderHandle<RenderParagraph>,
        offset: i32,
    ) -> Offset {
        tester.update(|app| {
            let local = paragraph.get_offset_for_caret(app, TextPosition::new(offset), Rect::ZERO)
                + Offset::new(0.0, 5.0);
            transform_point(&paragraph.as_object().get_transform_to(app, None), local)
        })
    }

    /// Sends `event` to `handle`.
    fn dispatch<D: SelectionHandler>(
        tester: &mut WidgetTester,
        handle: Handle<D>,
        event: SelectionEvent,
    ) {
        tester.update(|app| {
            SelectionHandler::dispatch_selection_event(handle, app, &event);
        });
    }

    /// The paragraph showing `text`.
    fn paragraph_showing(tester: &WidgetTester, text: &str) -> RenderHandle<RenderParagraph> {
        let paragraphs = tester.all::<RenderParagraph>();
        tester.update(|app| {
            *paragraphs
                .iter()
                .find(|paragraph| paragraph.text(app).to_plain_text(true, true) == text)
                .unwrap_or_else(|| panic!("a paragraph showing {text}"))
        })
    }

    /// `paragraph`'s one selection, as its base and extent, which is how Flutter's tests state
    /// one.
    fn paragraph_selection(
        tester: &WidgetTester,
        paragraph: RenderHandle<RenderParagraph>,
    ) -> (i32, i32) {
        tester.update(|app| {
            let selections = paragraph.selections(app);
            assert_eq!(selections.len(), 1, "one selection");
            (selections[0].base_offset, selections[0].extent_offset)
        })
    }

    /// A selection container over a 100-item list, with the events the region sends.
    fn selection_list_controller(tester: &WidgetTester) -> Ref<ScrollController> {
        tester.update(|app| ScrollController::new(app, 0.0, true, None, None, None))
    }

    #[test]
    fn scratch_count_paragraphs_in_a_plain_list() {
        test_widgets(|tester| {
            let controller = selection_list_controller(tester);
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                ListView::builder(|_app, _context, index| {
                    Some(
                        Text::new(format!("Item {index}"))
                            .text_direction(TextDirection::Ltr)
                            .into_widget(),
                    )
                })
                .item_count(100)
                .build()
                .controller(controller.erase()),
            ));
            tester.pump_and_settle();
            eprintln!("PLAIN AT 0: {}", tester.all::<RenderParagraph>().len());
            tester.update(|app| controller.jump_to(app, 800.0));
            tester.pump_and_settle();
            eprintln!("PLAIN AT 800: {}", tester.all::<RenderParagraph>().len());
        });
    }

    /// Sends `event` once per frame while the selection answers that it is pending, as
    /// `SelectableRegion` re-sends an edge update until the subtree stops scrolling.
    fn dispatch_edge_until_settled(
        tester: &mut WidgetTester,
        handle: Handle<PlainStaticSelectionContainerDelegate>,
        event: &SelectionEvent,
    ) {
        for _ in 0..2000 {
            let result =
                tester.update(|app| SelectionHandler::dispatch_selection_event(handle, app, event));
            if result != SelectionResult::Pending {
                tester.pump_and_settle();
                return;
            }
            tester.pump_by(Duration::from_millis(16));
        }
        panic!("the selection edge never settled");
    }

    // scrollable_selection_test.dart 'select to scroll forward': the list scrolls to its end,
    // and the items it reveals are selected.
    #[test]
    fn a_selection_dragged_past_the_end_scrolls_the_list_to_it() {
        test_widgets(|tester| {
            let controller = selection_list_controller(tester);
            let delegate = pump_selection_list(tester, &controller, None);
            let start = paragraph_offset(tester, paragraph_showing(tester, "Item 0"), 2);
            let bottom =
                tester.get_bottom_right(find::by_type::<ListView>()) + Offset::new(0.0, 20.0);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );
            dispatch_edge_until_settled(
                tester,
                delegate.handle(),
                &SelectionEvent::for_end(bottom, None),
            );

            let state = scrollable(tester);
            let (offset, max_extent) = tester.update(|app| {
                let position = state.position(app);
                (position.pixels(app), position.max_scroll_extent(app))
            });
            assert_eq!(offset, max_extent, "the list scrolled to its end");
            for index in 96..100 {
                let paragraph = paragraph_showing(tester, &format!("Item {index}"));
                assert_eq!(paragraph_selection(tester, paragraph), (0, 7));
            }
        });
    }

    // scrollable_selection_test.dart 'select to scroll backward': the list scrolls back to its
    // start, and the items it reveals are selected backwards.
    #[test]
    fn a_selection_dragged_past_the_start_scrolls_the_list_back() {
        test_widgets(|tester| {
            let controller = selection_list_controller(tester);
            let delegate = pump_selection_list(tester, &controller, None);
            let state = scrollable(tester);
            let max_extent = tester.update(|app| state.position(app).max_scroll_extent(app));
            tester.update(|app| controller.jump_to(app, max_extent));
            for _ in 0..3 {
                tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::EveryView));
                tester.pump_and_settle();
            }

            let center = tester.get_center(find::by_type::<ListView>());
            let top = tester.get_top_left(find::by_type::<ListView>()) + Offset::new(0.0, -20.0);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(center, None),
            );
            dispatch_edge_until_settled(
                tester,
                delegate.handle(),
                &SelectionEvent::for_end(top, None),
            );

            assert_eq!(
                tester.update(|app| controller.offset(app)),
                0.0,
                "the list scrolled back to its start"
            );
            for index in 0..4 {
                let paragraph = paragraph_showing(tester, &format!("Item {index}"));
                assert_eq!(paragraph_selection(tester, paragraph), (6, 0));
            }
        });
    }

    // scrollable_selection_test.dart 'automatic edge scrolling respects NeverScrollableScrollPhysics'.
    #[test]
    fn edge_scrolling_respects_physics_that_refuse_it() {
        test_widgets(|tester| {
            let controller = selection_list_controller(tester);
            let delegate = pump_selection_list(
                tester,
                &controller,
                Some(Rc::new(NeverScrollableScrollPhysics::new())),
            );
            let start = paragraph_offset(tester, paragraph_showing(tester, "Item 0"), 2);
            let bottom =
                tester.get_bottom_right(find::by_type::<ListView>()) + Offset::new(0.0, 40.0);
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_start(start, None),
            );
            dispatch(
                tester,
                delegate.handle(),
                SelectionEvent::for_end(bottom, None),
            );

            tester.pump_by(Duration::from_secs(2));
            tester.pump_by(Duration::from_secs(2));
            assert_eq!(
                tester.update(|app| controller.offset(app)),
                0.0,
                "the position did not move"
            );
        });
    }

    // scrollable_selection_test.dart 'preserve selection when out of view'.
    #[test]
    fn a_selection_survives_its_item_scrolling_out_of_view() {
        test_widgets(|tester| {
            let controller = selection_list_controller(tester);
            let delegate = pump_selection_list(tester, &controller, None);
            let item_zero = paragraph_showing(tester, "Item 0");
            let item_height = tester.update(|app| item_zero.size(app).height());
            let item_50_offset = item_height * 50.0;
            tester.update(|app| controller.jump_to(app, item_50_offset));
            for _ in 0..3 {
                tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::EveryView));
                tester.pump_and_settle();
            }

            let paragraph = paragraph_showing(tester, "Item 50");
            let start = paragraph_offset(tester, paragraph, 2);
            let end = paragraph_offset(tester, paragraph, 4);
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
            assert_eq!(paragraph_selection(tester, paragraph), (2, 4));

            tester.update(|app| controller.jump_to(app, 0.0));
            for _ in 0..3 {
                tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::EveryView));
                tester.pump_and_settle();
            }
            tester.update(|app| controller.jump_to(app, item_50_offset));
            for _ in 0..3 {
                tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::EveryView));
                tester.pump_and_settle();
            }
            assert_eq!(
                paragraph_selection(tester, paragraph_showing(tester, "Item 50")),
                (2, 4)
            );
        });
    }
}
