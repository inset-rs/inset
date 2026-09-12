//! Flutter counterpart: `widgets/scrollable.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use inset_animation::Curve;
use inset_embedder::{Clip, Offset};
use inset_foundation::{App, CompleterFuture, Handle, Listener, wait_all};
use inset_gestures::{
    DeviceGestureSettings, Drag, DragDownDetails, DragEndDetails, DragGestureRecognizer,
    DragStartBehavior, DragStartDetails, DragUpdateDetails, GestureBinding,
    HorizontalDragGestureRecognizer, PointerEvent, VerticalDragGestureRecognizer,
};
use inset_painting::{
    Axis, AxisDirection, axis_direction_is_reversed, axis_direction_to_axis, flip_axis,
};
use inset_rendering::{AnyRenderObject, AnyViewportOffset, HitTestBehavior, RenderIgnorePointer};
use inset_scheduler::{Ticker, TickerCallback, TickerProvider, TickerProviderObject};
use inset_services::{HardwareKeyboard, RestorationBucket, RestorationData, RestorationManager};

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
    AnyScrollController, ScrollController, ScrollControllerLeaf,
};
use crate::widgets::scroll_physics::ScrollPhysicsRef;
use crate::widgets::scroll_position::{AnyScrollPosition, ScrollPositionAlignmentPolicy};
use crate::widgets::scrollable_helpers::{ScrollIncrementCalculator, ScrollableDetails};
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};

/// Signature used by [`Scrollable`] to build the viewport through which the
/// scrollable content is displayed.
pub type ViewportBuilder = Rc<dyn Fn(&mut App, BuildContext, AnyViewportOffset) -> WidgetRef>;

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
    pub controller: Option<AnyScrollController>,

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
    pub fn controller(mut self, controller: AnyScrollController) -> Scrollable {
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
            let (scrollable, position) = (widget.scrollable, widget.position);
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
            gesture_detector_key: Rc::new(GlobalKey::new()),
            ignore_pointer_key: Rc::new(GlobalKey::new()),
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
    position: AnyScrollPosition,
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
    position: Option<AnyScrollPosition>,
    physics: Option<ScrollPhysicsRef>,
    persisted_scroll_offset: Option<Handle<RestorableScrollOffset>>,
    configuration: Option<ScrollBehaviorRef>,
    fallback_scroll_controller: Option<Handle<ScrollController>>,
    media_query_gesture_settings: Option<DeviceGestureSettings>,
    device_pixel_ratio: f64,
    gesture_detector_key: Rc<GlobalKey>,
    ignore_pointer_key: Rc<GlobalKey>,
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

    fn effective_scroll_controller(self: Handle<Self>, app: &App) -> AnyScrollController {
        self.widget(app).controller.unwrap_or_else(|| {
            app.get(self)
                .fallback_scroll_controller
                .expect("a scrollable without a controller creates a fallback")
                .as_controller()
        })
    }

    fn persisted_scroll_offset(self: Handle<Self>, app: &App) -> Handle<RestorableScrollOffset> {
        app.get(self)
            .persisted_scroll_offset
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

        let old_position = app.get(self).position;
        if let Some(old_position) = old_position {
            self.effective_scroll_controller(app)
                .detach(app, old_position);
            // It's important that we not dispose the old position until after the viewport has
            // had a chance to unregister its listeners from the old position. So, schedule a
            // microtask to do it.
            app.schedule_microtask(Listener::new(move |app| old_position.dispose(app)));
        }

        let scroll_context = self.scroll_context(app);
        let position = self
            .effective_scroll_controller(app)
            .create_scroll_position(app, physics, scroll_context, old_position);
        app.get_mut(self).position = Some(position);
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
            .map(AnyScrollController::type_id)
            != old_widget.controller.map(AnyScrollController::type_id)
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

    fn gesture_detector_key(self: Handle<Self>, app: &App) -> Rc<GlobalKey> {
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
            && event.kind() == inset_embedder::PointerDeviceKind::Mouse;

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
                    if !physics.should_accept_user_offset(&metrics) {
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
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
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

        let controller = self.widget(app).controller;
        if controller != old_widget.controller {
            let position = self.position(app);
            match old_widget.controller {
                None => {
                    // The old controller was `None`, meaning the fallback cannot be `None`.
                    // Dispose of the fallback.
                    let fallback = app
                        .get(self)
                        .fallback_scroll_controller
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
            self.effective_scroll_controller(app).attach(app, position);
        }

        if self.should_update_position(app, old_widget) {
            self.update_position(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let position = self.position(app);
        match self.widget(app).controller {
            Some(controller) => controller.detach(app, position),
            None => {
                if let Some(fallback) = app.get(self).fallback_scroll_controller {
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
        let position = self.position(app);
        let viewport_builder = self.widget(app).viewport_builder.clone();
        let viewport = viewport_builder(app, context, position.as_viewport_offset());
        let hit_test_behavior = self.widget(app).hit_test_behavior;
        let state = app.get(self);
        let result = ScrollableScope {
            scrollable: self,
            position,
            child: PointerListener::new()
                .on_pointer_signal(Rc::new(move |app: &mut App, event| {
                    self.received_pointer_signal(app, &event);
                }))
                .child(
                    RawGestureDetector::new()
                        .key(state.gesture_detector_key.clone())
                        .gestures(state.gesture_recognizers.clone())
                        .behavior(hit_test_behavior)
                        .child(
                            IgnorePointer::new()
                                .key(state.ignore_pointer_key.clone())
                                .ignoring(state.should_ignore_pointer)
                                .child(viewport),
                        ),
                )
                .into_widget(),
        }
        .into_widget();

        self.build_chrome(app, context, result)
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

/// Dart's `_RestorableScrollOffset`: the scroll offset a [`ScrollableState`] persists.
pub struct RestorableScrollOffset {
    change_notifier: inset_foundation::ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<Option<f64>>,
}

impl RestorableScrollOffset {
    /// Creates a property whose default value is "no stored offset".
    pub fn new(app: &mut App) -> Handle<RestorableScrollOffset> {
        app.create(RestorableScrollOffset {
            change_notifier: inset_foundation::ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
        })
    }
}

impl inset_foundation::ChangeNotifier for RestorableScrollOffset {
    fn change_notifier_data(&self) -> &inset_foundation::ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut inset_foundation::ChangeNotifierData {
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
    use inset_foundation::AppCell;
    use std::cell::{Cell, RefCell};
    use std::time::Instant;

    use inset_embedder::{InertPlatform, Platform, PlatformRef, TargetPlatform, ViewId, ViewRef};
    use inset_scheduler::SchedulerBinding;
    use inset_services::{RestorationMap, RestorationUpdate};

    use super::*;
    use crate::framework::Element;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::media_query::{MediaQuery, MediaQueryData};
    use crate::widgets::restoration::RootRestorationScope;
    use crate::widgets::scroll_controller::ScrollControllerLeaf;
    use inset_rendering::{RenderBox, RenderSliver};

    /// A host that answers `restoration_get` with what it was handed and records every
    /// `restoration_put`.
    #[derive(Default)]
    struct RecordingPlatform {
        stored: RefCell<Option<RestorationUpdate>>,
        puts: RefCell<Vec<RestorationMap>>,
    }

    impl Platform for RecordingPlatform {
        fn target_platform(&self) -> TargetPlatform {
            InertPlatform.target_platform()
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            InertPlatform.now()
        }

        fn wake_at(&self, _deadline: Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }

        fn restoration_get(&self) -> Option<RestorationUpdate> {
            self.stored.borrow().clone()
        }

        fn restoration_put(&self, data: RestorationMap) {
            self.puts.borrow_mut().push(data);
        }
    }

    fn app_restoring(data: Option<RestorationMap>) -> (Rc<AppCell>, Rc<RecordingPlatform>) {
        let platform = Rc::new(RecordingPlatform::default());
        *platform.stored.borrow_mut() = Some(RestorationUpdate {
            enabled: true,
            data,
        });
        let cell = AppCell::with_platform(Rc::clone(&platform) as PlatformRef);
        (cell, platform)
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

    fn pump_frame(app: &mut App) {
        SchedulerBinding::handle_begin_frame(app, Some(Duration::ZERO));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    /// A leaf widget whose render object is a `RenderViewport` over 800 logical pixels of
    /// content: what the `Viewport` widget builds, in miniature.
    #[derive(Debug)]
    struct TestViewport {
        offset: AnyViewportOffset,
    }

    impl crate::framework::RenderObjectWidget for TestViewport {
        type RenderObject = inset_rendering::RenderViewport;

        fn create_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
        ) -> inset_rendering::AnyRenderObject {
            let content = inset_rendering::RenderConstrainedBox::new(
                app,
                inset_rendering::BoxConstraints::tight(inset_embedder::Size::new(300.0, 800.0)),
                None,
            );
            let sliver = inset_rendering::RenderSliverToBoxAdapter::new(
                app,
                Some(inset_rendering::RenderBox::as_box(content)),
            );
            inset_rendering::RenderViewport::new(
                app,
                AxisDirection::Right,
                self.offset,
                Some(vec![RenderSliver::as_sliver(sliver)]),
                None,
            )
            .as_object()
        }

        fn update_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
            render_object: inset_rendering::RenderHandle<inset_rendering::RenderViewport>,
        ) {
            inset_rendering::RenderViewportBase::set_offset(render_object, app, self.offset);
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
            Scrollable::new(box_viewport()).controller(controller.as_controller()),
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
        let (cell, _platform) = app_restoring(Some(child(
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
        let (cell, platform) = app_restoring(None);
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
        pump_frame(&mut app);

        let stored = platform.puts.borrow();
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
}
