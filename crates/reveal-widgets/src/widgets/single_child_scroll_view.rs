//! Flutter counterpart: `widgets/single_child_scroll_view.dart`.
//!
//! Semantics and diagnostics wait; see `PORTING.md`.

use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curve;
use reveal_embedder::{Clip, Matrix4, Offset, Rect, Size};
use reveal_foundation::{App, Handle, Listenable, Listener};
use reveal_gestures::DragStartBehavior;
use reveal_painting::{
    Axis, AxisDirection, EdgeInsetsGeometry, axis_direction_to_axis, transform_rect,
};
use reveal_rendering::{
    AnyRenderAbstractViewport, AnyRenderBox, AnyRenderObject, AnyViewportOffset, BoxConstraints,
    BoxHitTestResult, EmptyParentData, HitTestBehavior, PaintingContext, PipelineOwner,
    RenderAbstractViewport, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RevealedOffset, show_in_viewport,
};

use crate::framework::{
    AnyElement, BuildContext, Element, ElementData, IntoWidget, KeyRef, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget,
    SingleChildRenderObjectElementBase, SingleChildRenderObjectElementData,
    SingleChildRenderObjectWidget, Slot, StatelessWidget, WidgetRef, downcast_widget,
};
use crate::widgets::basic::{Padding, get_axis_direction_from_axis_reverse_and_directionality};
use crate::widgets::focus_manager::{FocusNodeLeaf, UnfocusDisposition, primary_focus};
use crate::widgets::focus_scope::FocusScope;
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::primary_scroll_controller::PrimaryScrollController;
use crate::widgets::scroll_configuration::ScrollConfiguration;
use crate::widgets::scroll_controller::AnyScrollController;
use crate::widgets::scroll_notification::{ScrollUpdateNotification, ViewportElementMixin};
use crate::widgets::scroll_physics::ScrollPhysicsRef;
use crate::widgets::scroll_view::ScrollViewKeyboardDismissBehavior;
use crate::widgets::scrollable::{Scrollable, ViewportBuilder};

/// A box in which a single widget can be scrolled.
///
/// This widget is useful when you have a single box that will normally be
/// entirely visible, for example a clock face in a time picker, but you need to
/// make sure it can be scrolled if the container gets too small in one axis
/// (the scroll direction).
///
/// It is also useful if you need to shrink-wrap in both axes (the main
/// scrolling direction as well as the cross axis), as one might see in a dialog
/// or pop-up menu. In that case, you might pair the [`SingleChildScrollView`]
/// with a `ListBody` child.
///
/// When you have a list of children and do not require cross-axis
/// shrink-wrapping behavior, for example a scrolling list that is always the
/// width of the screen, consider [`ListView`](crate::ListView), which is vastly
/// more efficient than a [`SingleChildScrollView`] containing a `ListBody` or
/// `Column` with many children.
///
/// See also:
///
///  * [`ListView`](crate::ListView), which handles multiple children in a scrolling list.
///  * `Scrollable`, which handles arbitrary scrolling effects.
#[derive(Clone, Debug)]
pub struct SingleChildScrollView {
    pub key: Option<KeyRef>,

    /// The [`Axis`] along which the scroll view's offset increases.
    ///
    /// Defaults to [`Axis::Vertical`].
    pub scroll_direction: Axis,

    /// Whether the scroll view scrolls in the reading direction.
    ///
    /// For example, if the reading direction is left-to-right and
    /// [`scroll_direction`](Self::scroll_direction) is [`Axis::Horizontal`], then the scroll
    /// view scrolls from left to right when [`reverse`](Self::reverse) is false and from right
    /// to left when it is true.
    ///
    /// Defaults to false.
    pub reverse: bool,

    /// The amount of space by which to inset the child.
    pub padding: Option<EdgeInsetsGeometry>,

    /// An object that can be used to control the position to which this scroll
    /// view is scrolled.
    ///
    /// Must be `None` if [`primary`](Self::primary) is true.
    pub controller: Option<AnyScrollController>,

    /// Whether this is the primary scroll view associated with the parent
    /// [`PrimaryScrollController`].
    pub primary: Option<bool>,

    /// How the scroll view should respond to user input.
    ///
    /// For example, determines how the scroll view continues to animate after the
    /// user stops dragging the scroll view.
    ///
    /// Defaults to matching platform conventions.
    pub physics: Option<ScrollPhysicsRef>,

    /// The widget that scrolls.
    pub child: Option<WidgetRef>,

    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,

    /// How to behave during hit testing when deciding how the hit test propagates to children
    /// and whether to consider targets behind this scroll view.
    ///
    /// Defaults to [`HitTestBehavior::Opaque`].
    pub hit_test_behavior: HitTestBehavior,

    /// Restoration ID to save and restore the scroll offset of the scrollable.
    pub restoration_id: Option<String>,

    /// How this scroll view will dismiss the keyboard automatically.
    ///
    /// If `None` then it falls back to the inherited
    /// [`ScrollBehavior::get_keyboard_dismiss_behavior`](crate::ScrollBehavior::get_keyboard_dismiss_behavior).
    pub keyboard_dismiss_behavior: Option<ScrollViewKeyboardDismissBehavior>,
}

impl Default for SingleChildScrollView {
    fn default() -> SingleChildScrollView {
        SingleChildScrollView {
            key: None,
            scroll_direction: Axis::Vertical,
            reverse: false,
            padding: None,
            controller: None,
            primary: None,
            physics: None,
            child: None,
            drag_start_behavior: DragStartBehavior::Start,
            clip_behavior: Clip::HardEdge,
            hit_test_behavior: HitTestBehavior::Opaque,
            restoration_id: None,
            keyboard_dismiss_behavior: None,
        }
    }
}

impl SingleChildScrollView {
    /// Creates a box in which a single widget can be scrolled.
    pub fn new() -> SingleChildScrollView {
        SingleChildScrollView::default()
    }

    /// Dart `SingleChildScrollView(key:)`.
    pub fn key(mut self, key: KeyRef) -> SingleChildScrollView {
        self.key = Some(key);
        self
    }

    /// Dart `SingleChildScrollView(scrollDirection:)`.
    pub fn scroll_direction(mut self, scroll_direction: Axis) -> SingleChildScrollView {
        self.scroll_direction = scroll_direction;
        self
    }

    /// Dart `SingleChildScrollView(reverse:)`.
    pub fn reverse(mut self, reverse: bool) -> SingleChildScrollView {
        self.reverse = reverse;
        self
    }

    /// Dart `SingleChildScrollView(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> SingleChildScrollView {
        self.padding = Some(padding);
        self
    }

    /// Dart `SingleChildScrollView(controller:)`.
    pub fn controller(mut self, controller: AnyScrollController) -> SingleChildScrollView {
        debug_assert!(
            !self.primary.unwrap_or(false),
            "Primary ScrollViews obtain their ScrollController via inheritance from a \
             PrimaryScrollController widget. You cannot both set primary to true and pass an \
             explicit controller."
        );
        self.controller = Some(controller);
        self
    }

    /// Dart `SingleChildScrollView(primary:)`.
    pub fn primary(mut self, primary: bool) -> SingleChildScrollView {
        debug_assert!(
            !(self.controller.is_some() && primary),
            "Primary ScrollViews obtain their ScrollController via inheritance from a \
             PrimaryScrollController widget. You cannot both set primary to true and pass an \
             explicit controller."
        );
        self.primary = Some(primary);
        self
    }

    /// Dart `SingleChildScrollView(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> SingleChildScrollView {
        self.physics = Some(physics);
        self
    }

    /// Dart `SingleChildScrollView(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SingleChildScrollView {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `SingleChildScrollView(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        mut self,
        drag_start_behavior: DragStartBehavior,
    ) -> SingleChildScrollView {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `SingleChildScrollView(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> SingleChildScrollView {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `SingleChildScrollView(hitTestBehavior:)`.
    pub fn hit_test_behavior(
        mut self,
        hit_test_behavior: HitTestBehavior,
    ) -> SingleChildScrollView {
        self.hit_test_behavior = hit_test_behavior;
        self
    }

    /// Dart `SingleChildScrollView(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> SingleChildScrollView {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `SingleChildScrollView(keyboardDismissBehavior:)`.
    pub fn keyboard_dismiss_behavior(
        mut self,
        keyboard_dismiss_behavior: ScrollViewKeyboardDismissBehavior,
    ) -> SingleChildScrollView {
        self.keyboard_dismiss_behavior = Some(keyboard_dismiss_behavior);
        self
    }

    fn get_direction(&self, app: &mut App, context: BuildContext) -> AxisDirection {
        get_axis_direction_from_axis_reverse_and_directionality(
            app,
            context,
            self.scroll_direction,
            self.reverse,
        )
    }

    /// Dart's `contents`: the child, inset by [`padding`](Self::padding) when there is one.
    fn contents(&self) -> Option<WidgetRef> {
        let contents = self.child.clone();
        let Some(padding) = self.padding else {
            return contents;
        };
        let mut inset = Padding::new(padding);
        if let Some(child) = contents {
            inset = inset.child(child);
        }
        Some(inset.into_widget())
    }
}

impl StatelessWidget for SingleChildScrollView {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let axis_direction = self.get_direction(app, context);
        let contents = self.contents();
        let effective_primary = self.primary.unwrap_or_else(|| {
            self.controller.is_none()
                && PrimaryScrollController::should_inherit(app, context, self.scroll_direction)
        });

        let scroll_controller = if effective_primary {
            PrimaryScrollController::maybe_of(app, context)
        } else {
            self.controller
        };

        let clip_behavior = self.clip_behavior;
        let viewport_builder: ViewportBuilder = Rc::new(move |_app, _context, offset| {
            let mut viewport =
                SingleChildViewport::new(offset, clip_behavior).axis_direction(axis_direction);
            if let Some(contents) = contents.clone() {
                viewport = viewport.child(contents);
            }
            viewport.into_widget()
        });

        let mut scrollable = Scrollable::new(viewport_builder)
            .drag_start_behavior(self.drag_start_behavior)
            .axis_direction(axis_direction)
            .clip_behavior(self.clip_behavior)
            .hit_test_behavior(self.hit_test_behavior);
        if let Some(controller) = scroll_controller {
            scrollable = scrollable.controller(controller);
        }
        if let Some(physics) = self.physics.clone() {
            scrollable = scrollable.physics(physics);
        }
        if let Some(restoration_id) = self.restoration_id.clone() {
            scrollable = scrollable.restoration_id(restoration_id);
        }
        let mut scrollable = scrollable.into_widget();

        let effective_keyboard_dismiss_behavior =
            self.keyboard_dismiss_behavior.unwrap_or_else(|| {
                ScrollConfiguration::of(app, context).get_keyboard_dismiss_behavior(app, context)
            });

        if effective_keyboard_dismiss_behavior == ScrollViewKeyboardDismissBehavior::OnDrag {
            scrollable = NotificationListener::<ScrollUpdateNotification>::new(scrollable)
                .on_notification(move |app, notification: &ScrollUpdateNotification| {
                    let current_scope = FocusScope::of(app, context, true).as_node();
                    if notification.drag_details.is_some()
                        && !current_scope.has_primary_focus(app)
                        && current_scope.has_focus(app)
                        && let Some(primary) = primary_focus(app)
                    {
                        primary.unfocus(app, UnfocusDisposition::Scope);
                    }
                    false
                })
                .into_widget();
        }

        if effective_primary && scroll_controller.is_some() {
            // Further descendant scroll views will not inherit the same
            // `PrimaryScrollController`.
            PrimaryScrollController::none(scrollable).into_widget()
        } else {
            scrollable
        }
    }
}

/// Dart's `_SingleChildViewport`: the box viewport a [`SingleChildScrollView`] scrolls.
#[derive(Debug)]
pub struct SingleChildViewport {
    /// The direction in which the [`offset`](Self::offset)'s pixels increase.
    pub axis_direction: AxisDirection,
    /// Which part of the content inside the viewport should be visible.
    pub offset: AnyViewportOffset,
    /// The content will be clipped (or not) according to this option.
    pub clip_behavior: Clip,
    /// The widget that scrolls.
    pub child: Option<WidgetRef>,
}

impl SingleChildViewport {
    /// Creates a viewport for a single box child.
    pub fn new(offset: AnyViewportOffset, clip_behavior: Clip) -> SingleChildViewport {
        SingleChildViewport {
            axis_direction: AxisDirection::Down,
            offset,
            clip_behavior,
            child: None,
        }
    }

    /// Dart `_SingleChildViewport(axisDirection:)`.
    pub fn axis_direction(mut self, axis_direction: AxisDirection) -> SingleChildViewport {
        self.axis_direction = axis_direction;
        self
    }

    /// Dart `_SingleChildViewport(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SingleChildViewport {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for SingleChildViewport {
    type RenderObject = RenderSingleChildViewport;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSingleChildViewport::new(
            app,
            self.axis_direction,
            self.offset,
            None,
            self.clip_behavior,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSingleChildViewport>,
    ) {
        // Order dependency: the offset setter reads the axis direction.
        render_object.set_axis_direction(app, self.axis_direction);
        render_object.set_offset(app, self.offset);
        render_object.set_clip_behavior(app, self.clip_behavior);
    }
}

impl SingleChildRenderObjectWidget for SingleChildViewport {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        SingleChildViewportElement::create(app, this).as_element()
    }
}

/// Dart's `_SingleChildViewportElement`: a single-child render object element that is also a
/// notification node, so a `ScrollNotification` bubbling past it counts one viewport deeper.
pub struct SingleChildViewportElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    single_child: SingleChildRenderObjectElementData,
}

impl SingleChildViewportElement {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> Handle<SingleChildViewportElement> {
        debug_assert!(downcast_widget::<SingleChildViewport>(&*widget).is_some());
        app.create(SingleChildViewportElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            single_child: SingleChildRenderObjectElementData::default(),
        })
    }
}

impl RenderObjectElementWidget for SingleChildViewportElement {
    type Widget = SingleChildViewport;

    fn widget_of(widget: &WidgetRef) -> &SingleChildViewport {
        downcast_widget::<SingleChildViewport>(&**widget)
            .expect("a SingleChildViewportElement holds its SingleChildViewport")
    }
}

impl RenderObjectElement for SingleChildViewportElement {
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl SingleChildRenderObjectElementBase for SingleChildViewportElement {
    fn single_child_data(self: Handle<Self>, app: &App) -> &SingleChildRenderObjectElementData {
        &app.get(self).single_child
    }

    fn single_child_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleChildRenderObjectElementData {
        &mut app.get_mut(self).single_child
    }
}

impl ViewportElementMixin for SingleChildViewportElement {}

impl Element for SingleChildViewportElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        SingleChildRenderObjectElementBase::visit_children(self, app, visitor);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        SingleChildRenderObjectElementBase::forget_child(self, app, child);
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        SingleChildRenderObjectElementBase::update(self, app, new_widget);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::insert_render_object_child(self, app, child, slot);
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::move_render_object_child(
            self, app, child, old_slot, new_slot,
        );
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        SingleChildRenderObjectElementBase::remove_render_object_child(self, app, child, slot);
    }

    fn attach_notification_tree(self: Handle<Self>, app: &mut App) {
        ViewportElementMixin::attach_notification_tree(self, app);
    }

    fn on_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn crate::framework::Notification,
    ) -> bool {
        ViewportElementMixin::on_notification(self, app, notification)
    }
}

/// Dart's `_RenderSingleChildViewport`: a viewport that shows one box child, moved by a
/// `ViewportOffset` and sized to that child in the cross axis.
pub struct RenderSingleChildViewport {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    axis_direction: AxisDirection,
    offset: AnyViewportOffset,
    clip_behavior: Clip,
}

impl RenderSingleChildViewport {
    /// Creates a viewport for a single box child.
    pub fn new(
        app: &mut App,
        axis_direction: AxisDirection,
        offset: AnyViewportOffset,
        child: Option<AnyRenderBox>,
        clip_behavior: Clip,
    ) -> RenderHandle<RenderSingleChildViewport> {
        let this = RenderHandle::new_box(
            app,
            RenderSingleChildViewport {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                axis_direction,
                offset,
                clip_behavior,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The direction in which the [`offset`](Self::offset)'s pixels increase.
    pub fn axis_direction(self: RenderHandle<Self>, app: &App) -> AxisDirection {
        self.get(app).axis_direction
    }

    /// Sets [`axis_direction`](Self::axis_direction).
    pub fn set_axis_direction(self: RenderHandle<Self>, app: &mut App, value: AxisDirection) {
        if value == self.get(app).axis_direction {
            return;
        }
        self.get_mut(app).axis_direction = value;
        self.mark_needs_layout(app);
    }

    /// The axis along which the viewport scrolls.
    pub fn axis(self: RenderHandle<Self>, app: &App) -> Axis {
        axis_direction_to_axis(self.axis_direction(app))
    }

    /// Which part of the content inside the viewport should be visible.
    pub fn offset(self: RenderHandle<Self>, app: &App) -> AnyViewportOffset {
        self.get(app).offset
    }

    /// Sets [`offset`](Self::offset).
    pub fn set_offset(self: RenderHandle<Self>, app: &mut App, value: AnyViewportOffset) {
        let old = self.get(app).offset;
        if value == old {
            return;
        }
        if self.attached(app) {
            old.remove_listener(app, &self.has_scrolled_listener());
        }
        self.get_mut(app).offset = value;
        if self.attached(app) {
            value.add_listener(app, self.has_scrolled_listener());
        }
        self.mark_needs_layout(app);
    }

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::None`].
    pub fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.get(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if value == self.get(app).clip_behavior {
            return;
        }
        self.get_mut(app).clip_behavior = value;
        self.mark_needs_paint(app);
    }

    /// Dart's `_offset.addListener(_hasScrolled)` tear-off.
    fn has_scrolled_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), has_scrolled)
    }

    fn viewport_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        debug_assert!(self.has_size(app));
        match self.axis(app) {
            Axis::Horizontal => self.size(app).width(),
            Axis::Vertical => self.size(app).height(),
        }
    }

    fn min_scroll_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        debug_assert!(self.has_size(app));
        0.0
    }

    fn max_scroll_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        debug_assert!(self.has_size(app));
        let Some(child) = self.child(app) else {
            return 0.0;
        };
        let extent = match self.axis(app) {
            Axis::Horizontal => child.size(app).width() - self.size(app).width(),
            Axis::Vertical => child.size(app).height() - self.size(app).height(),
        };
        extent.max(0.0)
    }

    fn get_inner_constraints(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        match self.axis(app) {
            Axis::Horizontal => constraints.height_constraints(),
            Axis::Vertical => constraints.width_constraints(),
        }
    }

    /// The offset the child is painted at, for the offset's current pixels.
    fn paint_offset(self: RenderHandle<Self>, app: &App) -> Offset {
        self.paint_offset_for_position(app, self.offset(app).pixels(app))
    }

    fn paint_offset_for_position(self: RenderHandle<Self>, app: &App, position: f64) -> Offset {
        let child = self.child(app).expect("a paint offset needs the content");
        let child_size = child.size(app);
        let size = self.size(app);
        match self.axis_direction(app) {
            AxisDirection::Up => Offset::new(0.0, position - child_size.height() + size.height()),
            AxisDirection::Left => Offset::new(position - child_size.width() + size.width(), 0.0),
            AxisDirection::Right => Offset::new(-position, 0.0),
            AxisDirection::Down => Offset::new(0.0, -position),
        }
    }

    fn should_clip_at_paint_offset(
        self: RenderHandle<Self>,
        app: &App,
        paint_offset: Offset,
    ) -> bool {
        let child = self.child(app).expect("the clip test needs the content");
        match self.clip_behavior(app) {
            Clip::None => false,
            Clip::HardEdge | Clip::AntiAlias | Clip::AntiAliasWithSaveLayer => {
                let child_size = child.size(app);
                let size = self.size(app);
                paint_offset.dx() < 0.0
                    || paint_offset.dy() < 0.0
                    || paint_offset.dx() + child_size.width() > size.width()
                    || paint_offset.dy() + child_size.height() > size.height()
            }
        }
    }

    fn paint_contents(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let child = self.child(app).expect("checked by the caller");
        let paint_offset = self.paint_offset(app);
        context.paint_child(app, child.as_object(), offset + paint_offset);
    }
}

/// Dart's `_hasScrolled`: a named function so that the listener added on attach compares equal
/// to the one removed on detach.
fn has_scrolled(this: Handle<RenderSingleChildViewport>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

/// Dart's `transform.translateByDouble(dx, dy, 0, 1)`.
fn translate(transform: &mut Matrix4, offset: Offset) {
    *transform = transform.then(&Matrix4::translation(
        offset.dx() as f32,
        offset.dy() as f32,
    ));
}

impl RenderObjectWithChildMixin for RenderSingleChildViewport {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderObject for RenderSingleChildViewport {
    reveal_rendering::render_object_accessors!();

    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        self.offset(app)
            .add_listener(app, self.has_scrolled_listener());
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.offset(app)
            .remove_listener(app, &self.has_scrolled_listener());
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        match self.child(app) {
            None => self.set_size(app, constraints.smallest()),
            Some(child) => {
                let inner_constraints = self.get_inner_constraints(app, constraints);
                child.layout(app, inner_constraints, true);
                let size = constraints.constrain(child.size(app));
                self.set_size(app, size);
            }
        }

        let offset = self.offset(app);
        if offset.has_pixels(app) {
            let pixels = offset.pixels(app);
            let max_scroll_extent = self.max_scroll_extent(app);
            let min_scroll_extent = self.min_scroll_extent(app);
            if pixels > max_scroll_extent {
                offset.correct_by(app, max_scroll_extent - pixels);
            } else if pixels < min_scroll_extent {
                offset.correct_by(app, min_scroll_extent - pixels);
            }
        }

        let viewport_extent = self.viewport_extent(app);
        offset.apply_viewport_dimension(app, viewport_extent);
        let (min_scroll_extent, max_scroll_extent) =
            (self.min_scroll_extent(app), self.max_scroll_extent(app));
        offset.apply_content_dimensions(app, min_scroll_extent, max_scroll_extent);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.child(app).is_none() {
            return;
        }
        let paint_offset = self.paint_offset(app);
        if self.should_clip_at_paint_offset(app, paint_offset) {
            let bounds = Offset::ZERO & self.size(app);
            let clip_behavior = self.clip_behavior(app);
            context.push_clip_rect(
                app,
                offset,
                bounds,
                |app, context, offset| self.paint_contents(app, context, offset),
                clip_behavior,
            );
        } else {
            self.paint_contents(app, context, offset);
        }
    }

    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        let offset = self.offset(app);
        if !offset.allow_implicit_scrolling(app) {
            return reveal_rendering::RenderObjectBase::show_on_screen(
                self, app, descendant, rect, duration, curve,
            );
        }

        let new_rect = show_in_viewport(
            app,
            descendant,
            rect,
            self.as_abstract_viewport(),
            offset,
            duration,
            Rc::clone(&curve),
        );
        reveal_rendering::RenderObjectBase::show_on_screen(
            self, app, None, new_rect, duration, curve,
        );
    }

    fn interface(self: RenderHandle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        (id == TypeId::of::<AnyRenderAbstractViewport>())
            .then(|| Box::new(self.as_abstract_viewport()) as Box<dyn Any>)
    }
}

impl RenderBox for RenderSingleChildViewport {
    reveal_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        // We don't actually use the offset argument in `BoxParentData`, so let's avoid
        // allocating it at all.
        if child.parent_data(app).is_none() {
            child.set_parent_data(app, EmptyParentData);
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_min_intrinsic_width(app, height),
            None => 0.0,
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_max_intrinsic_width(app, height),
            None => 0.0,
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_min_intrinsic_height(app, width),
            None => 0.0,
        }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        match self.child(app) {
            Some(child) => child.get_max_intrinsic_height(app, width),
            None => 0.0,
        }
    }

    // We don't override `compute_distance_to_actual_baseline`, because we want the default
    // behavior (returning none). Otherwise, as you scroll, it would shift in its parent if the
    // parent was baseline-aligned, which makes no sense.

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let Some(child) = self.child(app) else {
            return constraints.smallest();
        };
        let inner_constraints = self.get_inner_constraints(app, constraints);
        let child_size = child.get_dry_layout(app, inner_constraints);
        constraints.constrain(child_size)
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        _child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        translate(transform, self.paint_offset(app));
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };
        let paint_offset = self.paint_offset(app);
        result.add_with_paint_offset(Some(paint_offset), position, |result, transformed| {
            debug_assert_eq!(transformed, position + -paint_offset);
            child.hit_test(app, result, transformed)
        })
    }
}

impl RenderAbstractViewport for RenderSingleChildViewport {
    fn get_offset_to_reveal(
        self: RenderHandle<Self>,
        app: &App,
        target: AnyRenderObject,
        alignment: f64,
        rect: Option<Rect>,
        _axis: Option<Axis>,
    ) -> RevealedOffset {
        let rect = rect.unwrap_or_else(|| target.paint_bounds(app));
        let Some(target_box) = target.as_box() else {
            return RevealedOffset::new(self.offset(app).pixels(app), rect);
        };

        let child = self.child(app).expect("the content is revealed in");
        let transform = target_box
            .as_object()
            .get_transform_to(app, Some(child.as_object()));
        let bounds = transform_rect(&transform, rect);
        let content_size = child.size(app);
        let size = self.size(app);

        let (main_axis_extent, leading_scroll_offset, target_main_axis_extent) =
            match self.axis_direction(app) {
                AxisDirection::Up => (
                    size.height(),
                    content_size.height() - bounds.bottom,
                    bounds.height(),
                ),
                AxisDirection::Left => (
                    size.width(),
                    content_size.width() - bounds.right,
                    bounds.width(),
                ),
                AxisDirection::Right => (size.width(), bounds.left, bounds.width()),
                AxisDirection::Down => (size.height(), bounds.top, bounds.height()),
            };

        let target_offset =
            leading_scroll_offset - (main_axis_extent - target_main_axis_extent) * alignment;
        let target_rect = bounds.shift(self.paint_offset_for_position(app, target_offset));
        RevealedOffset::new(target_offset, target_rect)
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::any::Any;

    use reveal_embedder::{Size, TextDirection, valo::Op};
    use reveal_gestures::{HitTestEntry, HitTestResult};
    use reveal_painting::{EdgeInsets, transform_point};
    use reveal_rendering::{BoxHitTestEntry, RenderPadding};

    use super::*;
    use crate::test_harness::{VIEW_HEIGHT, VIEW_WIDTH, binding_cell, binding_mount, binding_pump};
    use crate::widgets::basic::{Directionality, Listener, SizedBox};
    use crate::widgets::media_query::{MediaQuery, MediaQueryData};
    use crate::widgets::scroll_controller::{ScrollController, ScrollControllerLeaf};

    const CONTENT_WIDTH: f64 = 100.0;
    const CONTENT_HEIGHT: f64 = 500.0;
    /// The height of the target inside the content, and the space above it.
    const TARGET_EXTENT: f64 = 50.0;
    const LEADING_EXTENT: f64 = 300.0;

    /// The tree every test mounts: a text direction and a media query, which a `Scrollable`
    /// reads for its gesture settings.
    fn wrap<K>(child: impl IntoWidget<K>) -> WidgetRef {
        let query: WidgetRef = MediaQuery::new(
            MediaQueryData::new().size(Size::new(VIEW_WIDTH, VIEW_HEIGHT)),
            child,
        )
        .into_widget();
        Directionality::new(TextDirection::Ltr, query).into_widget()
    }

    /// A box that answers a hit test, so a hit can tell which child was reached.
    fn opaque(width: f64, height: f64) -> WidgetRef {
        Listener {
            behavior: HitTestBehavior::Opaque,
            child: Some(SizedBox::new().width(width).height(height).into_widget()),
            ..Default::default()
        }
        .into_widget()
    }

    /// Content taller than the viewport, with an opaque target `LEADING_EXTENT` down it.
    fn tall_content() -> WidgetRef {
        Padding::new(EdgeInsetsGeometry::Insets(EdgeInsets::only(
            0.0,
            LEADING_EXTENT,
            0.0,
            CONTENT_HEIGHT - LEADING_EXTENT - TARGET_EXTENT,
        )))
        .child(opaque(CONTENT_WIDTH, TARGET_EXTENT))
        .into_widget()
    }

    fn find<T: RenderObject>(app: &App, object: AnyRenderObject) -> Option<RenderHandle<T>> {
        if let Some(found) = object.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        object.visit_children(app, &mut |child| {
            found = found.or_else(|| find::<T>(app, child));
        });
        found
    }

    fn mount(
        cell: &AppCell,
        view: SingleChildScrollView,
    ) -> RenderHandle<RenderSingleChildViewport> {
        binding_mount(cell, wrap(view));
        let mut app = cell.borrow_mut();
        let root = crate::binding::WidgetsBinding::instance(&mut app)
            .root_element(&app)
            .expect("a mounted root element")
            .find_render_object(&app)
            .expect("a mounted view has a render object");
        find::<RenderSingleChildViewport>(&app, root).expect("a RenderSingleChildViewport")
    }

    /// The offset the viewport paints its content at.
    fn content_offset(app: &App, viewport: RenderHandle<RenderSingleChildViewport>) -> Offset {
        let child = viewport.child(app).expect("the content");
        let mut transform = Matrix4::IDENTITY;
        viewport
            .as_object()
            .apply_paint_transform(app, child.as_object(), &mut transform);
        transform_point(&transform, Offset::ZERO)
    }

    /// The boxes a hit test at `position` walked, innermost first.
    fn hit_boxes(
        app: &mut App,
        viewport: RenderHandle<RenderSingleChildViewport>,
        position: Offset,
    ) -> Vec<AnyRenderBox> {
        let mut result = HitTestResult::new();
        viewport
            .as_box()
            .hit_test(app, &mut BoxHitTestResult::wrap(&mut result), position);
        result
            .path()
            .iter()
            .filter_map(|entry: &HitTestEntry| {
                let target: &dyn Any = entry.target();
                target
                    .downcast_ref::<BoxHitTestEntry>()
                    .map(BoxHitTestEntry::target)
            })
            .collect()
    }

    /// The valo ops the viewport's own layer records.
    fn scene_ops(app: &App, viewport: RenderHandle<RenderSingleChildViewport>) -> Vec<Op> {
        let mut canvas = reveal_embedder::Canvas::new();
        viewport
            .as_object()
            .debug_layer(app)
            .expect("the viewport is a repaint boundary")
            .add_to_scene(app, &mut canvas);
        canvas.build().ops().to_vec()
    }

    #[test]
    fn a_vertical_view_gives_its_child_unbounded_height_and_keeps_the_viewport_height() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        drop(app);
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .controller(controller.as_controller())
                .child(SizedBox::new().width(CONTENT_WIDTH).height(CONTENT_HEIGHT)),
        );
        let app = cell.borrow();

        let child = viewport.child(&app).expect("the content");
        assert_eq!(child.size(&app), Size::new(CONTENT_WIDTH, CONTENT_HEIGHT));
        assert_eq!(viewport.size(&app), Size::new(CONTENT_WIDTH, VIEW_HEIGHT));
        assert_eq!(
            controller.position(&app).max_scroll_extent(&app),
            CONTENT_HEIGHT - VIEW_HEIGHT
        );
    }

    #[test]
    fn a_horizontal_view_gives_its_child_unbounded_width_and_keeps_the_viewport_width() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        drop(app);
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .scroll_direction(Axis::Horizontal)
                .controller(controller.as_controller())
                .child(SizedBox::new().width(CONTENT_HEIGHT).height(CONTENT_WIDTH)),
        );
        let app = cell.borrow();

        let child = viewport.child(&app).expect("the content");
        assert_eq!(child.size(&app), Size::new(CONTENT_HEIGHT, CONTENT_WIDTH));
        assert_eq!(viewport.size(&app), Size::new(VIEW_WIDTH, CONTENT_WIDTH));
        assert_eq!(
            controller.position(&app).max_scroll_extent(&app),
            CONTENT_HEIGHT - VIEW_WIDTH
        );
    }

    #[test]
    fn the_content_offset_follows_the_controller() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        drop(app);
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .controller(controller.as_controller())
                .child(SizedBox::new().width(CONTENT_WIDTH).height(CONTENT_HEIGHT)),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(content_offset(&app, viewport), Offset::ZERO);

        controller.jump_to(&mut app, 120.0);
        binding_pump(&mut app, Duration::ZERO);
        assert_eq!(content_offset(&app, viewport), Offset::new(0.0, -120.0));
    }

    #[test]
    fn a_reversed_view_starts_at_the_end_of_its_content() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        drop(app);
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .reverse(true)
                .controller(controller.as_controller())
                .child(SizedBox::new().width(CONTENT_WIDTH).height(CONTENT_HEIGHT)),
        );
        let mut app = cell.borrow_mut();

        assert_eq!(viewport.axis_direction(&app), AxisDirection::Up);
        // At scroll offset zero the content's trailing edge sits on the viewport's.
        assert_eq!(
            content_offset(&app, viewport),
            Offset::new(0.0, VIEW_HEIGHT - CONTENT_HEIGHT)
        );

        controller.jump_to(&mut app, 120.0);
        binding_pump(&mut app, Duration::ZERO);
        assert_eq!(
            content_offset(&app, viewport),
            Offset::new(0.0, VIEW_HEIGHT - CONTENT_HEIGHT + 120.0)
        );
    }

    #[test]
    fn get_offset_to_reveal_answers_the_offset_that_brings_a_descendant_to_an_edge() {
        let cell = binding_cell();
        let viewport = mount(&cell, SingleChildScrollView::new().child(tall_content()));
        let app = cell.borrow();
        let padding = find::<RenderPadding>(&app, viewport.as_object()).expect("the padding");
        let target = padding.child(&app).expect("the target").as_object();

        let leading = viewport.get_offset_to_reveal(&app, target, 0.0, None, None);
        assert_eq!(leading.offset, LEADING_EXTENT);
        assert_eq!(
            leading.rect,
            Rect::from_ltwh(0.0, 0.0, CONTENT_WIDTH, TARGET_EXTENT)
        );

        let trailing = viewport.get_offset_to_reveal(&app, target, 1.0, None, None);
        assert_eq!(
            trailing.offset,
            LEADING_EXTENT - (VIEW_HEIGHT - TARGET_EXTENT)
        );
        assert_eq!(
            trailing.rect,
            Rect::from_ltwh(
                0.0,
                VIEW_HEIGHT - TARGET_EXTENT,
                CONTENT_WIDTH,
                TARGET_EXTENT
            )
        );
    }

    #[test]
    fn a_hit_test_reaches_the_content_through_the_scroll_offset() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        drop(app);
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .controller(controller.as_controller())
                .child(tall_content()),
        );
        let mut app = cell.borrow_mut();
        let padding = find::<RenderPadding>(&app, viewport.as_object()).expect("the padding");
        let target = padding.child(&app).expect("the target");

        let near_the_top = Offset::new(50.0, 10.0);
        assert!(
            !hit_boxes(&mut app, viewport, near_the_top).contains(&target),
            "the target is scrolled out of view"
        );

        controller.jump_to(&mut app, LEADING_EXTENT);
        binding_pump(&mut app, Duration::ZERO);
        assert!(
            hit_boxes(&mut app, viewport, near_the_top).contains(&target),
            "scrolling the target to the leading edge puts it under the pointer"
        );
    }

    #[test]
    fn overflowing_content_is_clipped_unless_the_clip_behavior_says_otherwise() {
        let cell = binding_cell();
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .child(SizedBox::new().width(CONTENT_WIDTH).height(CONTENT_HEIGHT)),
        );
        let app = cell.borrow();
        assert!(
            scene_ops(&app, viewport)
                .iter()
                .any(|op| matches!(op, Op::ClipPath { .. })),
            "content taller than the viewport is clipped"
        );

        let cell = binding_cell();
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .clip_behavior(Clip::None)
                .child(SizedBox::new().width(CONTENT_WIDTH).height(CONTENT_HEIGHT)),
        );
        let app = cell.borrow();
        assert!(
            !scene_ops(&app, viewport)
                .iter()
                .any(|op| matches!(op, Op::ClipPath { .. })),
            "Clip::None paints the content unclipped"
        );

        let cell = binding_cell();
        let viewport = mount(
            &cell,
            SingleChildScrollView::new()
                .child(SizedBox::new().width(CONTENT_WIDTH).height(VIEW_HEIGHT)),
        );
        let app = cell.borrow();
        assert!(
            !scene_ops(&app, viewport)
                .iter()
                .any(|op| matches!(op, Op::ClipPath { .. })),
            "content that fits needs no clip"
        );
    }
}
