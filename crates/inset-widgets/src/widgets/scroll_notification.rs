//! Flutter counterpart: `widgets/scroll_notification.dart`.

use std::any::Any;
use std::cell::Cell;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_foundation::{App, Handle};
use inset_gestures::{DragEndDetails, DragStartDetails, DragUpdateDetails};
use inset_rendering::ScrollDirection;

use crate::framework::{BuildContext, Element, Notification, NotificationNode, NotificationTarget};
use crate::widgets::scroll_metrics::ScrollMetrics;

/// The fields of Dart's `ViewportNotificationMixin`.
///
/// The counter is a [`Cell`] because a notification bubbles as `&dyn Notification`: the
/// element that increments it never holds the notification exclusively.
#[derive(Debug, Default)]
pub struct ViewportNotificationData {
    depth: Cell<u32>,
}

impl ViewportNotificationData {
    /// A notification that has not bubbled through any viewport yet.
    pub fn new() -> ViewportNotificationData {
        ViewportNotificationData::default()
    }

    /// Dart's `ScrollUpdateNotification(depth:)`, which seeds the counter.
    pub fn with_depth(depth: u32) -> ViewportNotificationData {
        ViewportNotificationData {
            depth: Cell::new(depth),
        }
    }

    /// The counter a viewport element increments, for
    /// [`Notification::viewport_depth`](crate::Notification::viewport_depth).
    pub fn depth_cell(&self) -> &Cell<u32> {
        &self.depth
    }
}

/// Mixin for [`Notification`]s that track how many `RenderAbstractViewport` they
/// have bubbled through.
///
/// This is used by [`ScrollNotification`] and `OverscrollIndicatorNotification`.
pub trait ViewportNotificationMixin: Notification {
    /// Dart's mixin fields, held under the field `viewport_notification`.
    fn viewport_notification_data(&self) -> &ViewportNotificationData;

    /// The number of viewports that this notification has bubbled through.
    ///
    /// Typically listeners only respond to notifications with a
    /// [`depth`](Self::depth) of zero.
    ///
    /// Specifically, this is the number of widgets representing
    /// `RenderAbstractViewport` render objects through which this notification
    /// has bubbled.
    fn depth(&self) -> u32 {
        self.viewport_notification_data().depth.get()
    }

    /// Dart's `debugFillDescription`, which [`Debug`] writes out.
    fn debug_fill_description(&self, description: &mut Vec<String>) {
        let depth = self.depth();
        let locality = if depth == 0 { "local" } else { "remote" };
        description.push(format!("depth: {depth} ({locality})"));
    }
}

/// A mixin that allows [`Element`]s containing `Viewport`-like widgets to correctly
/// modify the notification depth of a [`ViewportNotificationMixin`].
///
/// The two members are the `Element` virtuals Dart reaches through
/// `NotifiableElementMixin`; an element that mixes this in forwards both from its
/// `impl Element`.
///
/// See also:
///   * `Viewport`, which creates a custom `MultiChildRenderObjectElement` that mixes
///     this in.
pub trait ViewportElementMixin: Element {
    /// `NotifiableElementMixin.attachNotificationTree`: a node for this element.
    fn attach_notification_tree(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        let parent_tree = this
            .parent(app)
            .and_then(|parent| parent.notification_tree(app));
        this.set_notification_tree(app, Some(Rc::new(NotificationNode::new(parent_tree, this))));
    }

    /// Counts this viewport, then lets the notification keep bubbling.
    fn on_notification(self: Handle<Self>, app: &mut App, notification: &dyn Notification) -> bool {
        let _ = app;
        if let Some(depth) = notification.viewport_depth() {
            depth.set(depth.get() + 1);
        }
        false
    }
}

/// The fields of Dart's `ScrollNotification` base class.
pub struct ScrollNotificationData {
    /// A description of a `Scrollable`'s contents, useful for modeling the state
    /// of its viewport.
    pub metrics: Rc<dyn ScrollMetrics>,

    /// The build context of the widget that fired this notification.
    ///
    /// This can be used to find the scrollable's render objects to determine the
    /// size of the viewport, for instance.
    pub context: Option<BuildContext>,
}

/// A [`Notification`] related to scrolling.
///
/// `Scrollable` widgets notify their ancestors about scrolling-related changes.
/// The notifications have the following lifecycle:
///
///  * A [`ScrollStartNotification`], which indicates that the widget has started
///    scrolling.
///  * Zero or more [`ScrollUpdateNotification`]s, which indicate that the widget
///    has changed its scroll position, mixed with zero or more
///    [`OverscrollNotification`]s, which indicate that the widget has not changed
///    its scroll position because the change would have caused its scroll
///    position to go outside its scroll bounds.
///  * Interspersed with the [`ScrollUpdateNotification`]s and
///    [`OverscrollNotification`]s are zero or more [`UserScrollNotification`]s,
///    which indicate that the user has changed the direction in which they are
///    scrolling.
///  * A [`ScrollEndNotification`], which indicates that the widget has stopped
///    scrolling.
///  * A [`UserScrollNotification`], with a [`direction`](UserScrollNotification::direction)
///    of [`ScrollDirection::Idle`].
///
/// Notifications bubble up through the tree, which means a given
/// `NotificationListener` will receive notifications for all descendant
/// `Scrollable` widgets. To focus on notifications from the nearest
/// `Scrollable` descendant, check that the [`depth`](ViewportNotificationMixin::depth)
/// of the notification is zero.
///
/// When a scroll notification is received by a `NotificationListener`, the
/// listener will have already completed build and layout, and it is therefore
/// too late for that widget to call `State::set_state`. Any attempt to adjust the
/// build or layout based on a scroll notification would result in a layout that
/// lagged one frame behind, which is a poor user experience. Scroll
/// notifications are therefore primarily useful for paint effects (since paint
/// happens after layout).
///
/// To drive layout based on the scroll position, consider listening to the
/// `ScrollPosition` directly (or indirectly via a `ScrollController`).
pub trait ScrollNotification: ViewportNotificationMixin {
    /// Dart's base-class fields, held under the field `scroll_notification`.
    fn scroll_notification_data(&self) -> &ScrollNotificationData;

    /// A description of a `Scrollable`'s contents, useful for modeling the state
    /// of its viewport.
    fn metrics(&self) -> &Rc<dyn ScrollMetrics> {
        &self.scroll_notification_data().metrics
    }

    /// The build context of the widget that fired this notification.
    fn context(&self) -> Option<BuildContext> {
        self.scroll_notification_data().context
    }
}

impl NotificationTarget for dyn ScrollNotification {
    fn cast(notification: &dyn Notification) -> Option<&dyn ScrollNotification> {
        notification.as_scroll_notification()
    }
}

/// Dart's `ScrollNotification.debugFillDescription`: the mixin's depth, then the metrics.
fn fill_scroll_description(notification: &dyn ScrollNotification, description: &mut Vec<String>) {
    ViewportNotificationMixin::debug_fill_description(notification, description);
    description.push(format!("{:?}", notification.metrics()));
}

/// A notification that a `Scrollable` widget has started scrolling.
///
/// See also:
///
///  * [`ScrollEndNotification`], which indicates that scrolling has stopped.
///  * [`ScrollNotification`], which describes the notification lifecycle.
pub struct ScrollStartNotification {
    viewport_notification: ViewportNotificationData,
    scroll_notification: ScrollNotificationData,

    /// If the `Scrollable` started scrolling because of a drag, the details about
    /// that drag start.
    ///
    /// Otherwise, null.
    pub drag_details: Option<DragStartDetails>,
}

impl ScrollStartNotification {
    /// Creates a notification that a `Scrollable` widget has started scrolling.
    pub fn new(
        metrics: Rc<dyn ScrollMetrics>,
        context: Option<BuildContext>,
    ) -> ScrollStartNotification {
        ScrollStartNotification {
            viewport_notification: ViewportNotificationData::new(),
            scroll_notification: ScrollNotificationData { metrics, context },
            drag_details: None,
        }
    }

    /// Dart `ScrollStartNotification(dragDetails:)`.
    pub fn drag_details(mut self, drag_details: DragStartDetails) -> ScrollStartNotification {
        self.drag_details = Some(drag_details);
        self
    }
}

impl Notification for ScrollStartNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        Some(&self.viewport_notification.depth)
    }

    fn as_scroll_notification(&self) -> Option<&dyn ScrollNotification> {
        Some(self)
    }
}

impl ViewportNotificationMixin for ScrollStartNotification {
    fn viewport_notification_data(&self) -> &ViewportNotificationData {
        &self.viewport_notification
    }
}

impl ScrollNotification for ScrollStartNotification {
    fn scroll_notification_data(&self) -> &ScrollNotificationData {
        &self.scroll_notification
    }
}

impl Debug for ScrollStartNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        fill_scroll_description(self, &mut description);
        if let Some(drag_details) = &self.drag_details {
            description.push(format!("{drag_details:?}"));
        }
        write!(f, "ScrollStartNotification({})", description.join(", "))
    }
}

/// A notification that a `Scrollable` widget has changed its scroll position.
///
/// See also:
///
///  * [`OverscrollNotification`], which indicates that a `Scrollable` widget
///    has not changed its scroll position because the change would have caused
///    its scroll position to go outside its scroll bounds.
///  * [`ScrollNotification`], which describes the notification lifecycle.
pub struct ScrollUpdateNotification {
    viewport_notification: ViewportNotificationData,
    scroll_notification: ScrollNotificationData,

    /// If the `Scrollable` changed its scroll position because of a drag, the
    /// details about that drag update.
    ///
    /// Otherwise, null.
    pub drag_details: Option<DragUpdateDetails>,

    /// The distance by which the `Scrollable` was scrolled, in logical pixels.
    pub scroll_delta: Option<f64>,
}

impl ScrollUpdateNotification {
    /// Creates a notification that a `Scrollable` widget has changed its scroll
    /// position.
    pub fn new(metrics: Rc<dyn ScrollMetrics>, context: BuildContext) -> ScrollUpdateNotification {
        ScrollUpdateNotification {
            viewport_notification: ViewportNotificationData::new(),
            scroll_notification: ScrollNotificationData {
                metrics,
                context: Some(context),
            },
            drag_details: None,
            scroll_delta: None,
        }
    }

    /// Dart `ScrollUpdateNotification(dragDetails:)`.
    pub fn drag_details(mut self, drag_details: DragUpdateDetails) -> ScrollUpdateNotification {
        self.drag_details = Some(drag_details);
        self
    }

    /// Dart `ScrollUpdateNotification(scrollDelta:)`.
    pub fn scroll_delta(mut self, scroll_delta: f64) -> ScrollUpdateNotification {
        self.scroll_delta = Some(scroll_delta);
        self
    }

    /// Dart `ScrollUpdateNotification(depth:)`, which seeds the viewport depth. Named
    /// `with_depth` because `depth` is the mixin's getter.
    pub fn with_depth(mut self, depth: u32) -> ScrollUpdateNotification {
        self.viewport_notification = ViewportNotificationData::with_depth(depth);
        self
    }
}

impl Notification for ScrollUpdateNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        Some(&self.viewport_notification.depth)
    }

    fn as_scroll_notification(&self) -> Option<&dyn ScrollNotification> {
        Some(self)
    }
}

impl ViewportNotificationMixin for ScrollUpdateNotification {
    fn viewport_notification_data(&self) -> &ViewportNotificationData {
        &self.viewport_notification
    }
}

impl ScrollNotification for ScrollUpdateNotification {
    fn scroll_notification_data(&self) -> &ScrollNotificationData {
        &self.scroll_notification
    }
}

impl Debug for ScrollUpdateNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        fill_scroll_description(self, &mut description);
        description.push(match self.scroll_delta {
            Some(scroll_delta) => format!("scrollDelta: {scroll_delta:?}"),
            None => "scrollDelta: null".to_string(),
        });
        if let Some(drag_details) = &self.drag_details {
            description.push(format!("{drag_details:?}"));
        }
        write!(f, "ScrollUpdateNotification({})", description.join(", "))
    }
}

/// A notification that a `Scrollable` widget has not changed its scroll position
/// because the change would have caused its scroll position to go outside of
/// its scroll bounds.
///
/// See also:
///
///  * [`ScrollUpdateNotification`], which indicates that a `Scrollable` widget
///    has changed its scroll position.
///  * [`ScrollNotification`], which describes the notification lifecycle.
pub struct OverscrollNotification {
    viewport_notification: ViewportNotificationData,
    scroll_notification: ScrollNotificationData,

    /// If the `Scrollable` overscrolled because of a drag, the details about that
    /// drag update.
    ///
    /// Otherwise, null.
    pub drag_details: Option<DragUpdateDetails>,

    /// The number of logical pixels that the `Scrollable` avoided scrolling.
    ///
    /// This will be negative for overscroll on the "start" side and positive for
    /// overscroll on the "end" side.
    pub overscroll: f64,

    /// The velocity at which the `ScrollPosition` was changing when this
    /// overscroll happened.
    ///
    /// This will typically be 0.0 for touch-driven overscrolls, and positive
    /// for overscrolls that happened from a `BallisticScrollActivity` or
    /// `DrivenScrollActivity`.
    pub velocity: f64,
}

impl OverscrollNotification {
    /// Creates a notification that a `Scrollable` widget has changed its scroll
    /// position outside of its scroll bounds.
    pub fn new(
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        overscroll: f64,
    ) -> OverscrollNotification {
        debug_assert!(overscroll.is_finite());
        debug_assert!(overscroll != 0.0);
        OverscrollNotification {
            viewport_notification: ViewportNotificationData::new(),
            scroll_notification: ScrollNotificationData {
                metrics,
                context: Some(context),
            },
            drag_details: None,
            overscroll,
            velocity: 0.0,
        }
    }

    /// Dart `OverscrollNotification(dragDetails:)`.
    pub fn drag_details(mut self, drag_details: DragUpdateDetails) -> OverscrollNotification {
        self.drag_details = Some(drag_details);
        self
    }

    /// Dart `OverscrollNotification(velocity:)`.
    pub fn velocity(mut self, velocity: f64) -> OverscrollNotification {
        self.velocity = velocity;
        self
    }
}

impl Notification for OverscrollNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        Some(&self.viewport_notification.depth)
    }

    fn as_scroll_notification(&self) -> Option<&dyn ScrollNotification> {
        Some(self)
    }
}

impl ViewportNotificationMixin for OverscrollNotification {
    fn viewport_notification_data(&self) -> &ViewportNotificationData {
        &self.viewport_notification
    }
}

impl ScrollNotification for OverscrollNotification {
    fn scroll_notification_data(&self) -> &ScrollNotificationData {
        &self.scroll_notification
    }
}

impl Debug for OverscrollNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        fill_scroll_description(self, &mut description);
        description.push(format!("overscroll: {:.1}", self.overscroll));
        description.push(format!("velocity: {:.1}", self.velocity));
        if let Some(drag_details) = &self.drag_details {
            description.push(format!("{drag_details:?}"));
        }
        write!(f, "OverscrollNotification({})", description.join(", "))
    }
}

/// A notification that a `Scrollable` widget has stopped scrolling.
///
/// See also:
///
///  * [`ScrollStartNotification`], which indicates that scrolling has started.
///  * [`ScrollNotification`], which describes the notification lifecycle.
pub struct ScrollEndNotification {
    viewport_notification: ViewportNotificationData,
    scroll_notification: ScrollNotificationData,

    /// If the `Scrollable` stopped scrolling because of a drag, the details about
    /// that drag end.
    ///
    /// Otherwise, null.
    ///
    /// If a drag ends with some residual velocity, a typical [`ScrollPhysics`] will
    /// start a ballistic scroll, which delays the [`ScrollEndNotification`] until
    /// the ballistic simulation completes, at which time
    /// [`drag_details`](Self::drag_details) will be null. If the residual velocity is too
    /// small to trigger ballistic scrolling, then the [`ScrollEndNotification`] will be
    /// dispatched immediately and [`drag_details`](Self::drag_details) will be non-null.
    ///
    /// [`ScrollPhysics`]: crate::widgets::scroll_physics::ScrollPhysics
    pub drag_details: Option<DragEndDetails>,
}

impl ScrollEndNotification {
    /// Creates a notification that a `Scrollable` widget has stopped scrolling.
    pub fn new(metrics: Rc<dyn ScrollMetrics>, context: BuildContext) -> ScrollEndNotification {
        ScrollEndNotification {
            viewport_notification: ViewportNotificationData::new(),
            scroll_notification: ScrollNotificationData {
                metrics,
                context: Some(context),
            },
            drag_details: None,
        }
    }

    /// Dart `ScrollEndNotification(dragDetails:)`.
    pub fn drag_details(mut self, drag_details: DragEndDetails) -> ScrollEndNotification {
        self.drag_details = Some(drag_details);
        self
    }
}

impl Notification for ScrollEndNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        Some(&self.viewport_notification.depth)
    }

    fn as_scroll_notification(&self) -> Option<&dyn ScrollNotification> {
        Some(self)
    }
}

impl ViewportNotificationMixin for ScrollEndNotification {
    fn viewport_notification_data(&self) -> &ViewportNotificationData {
        &self.viewport_notification
    }
}

impl ScrollNotification for ScrollEndNotification {
    fn scroll_notification_data(&self) -> &ScrollNotificationData {
        &self.scroll_notification
    }
}

impl Debug for ScrollEndNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        fill_scroll_description(self, &mut description);
        if let Some(drag_details) = &self.drag_details {
            description.push(format!("{drag_details:?}"));
        }
        write!(f, "ScrollEndNotification({})", description.join(", "))
    }
}

/// A notification that the user has changed the [`ScrollDirection`] in which they
/// are scrolling, or have stopped scrolling.
///
/// For the direction that the `ScrollView` is oriented to, and the direction
/// contents are being laid out in, see `AxisDirection` and `GrowthDirection`.
///
/// See also:
///
///  * [`ScrollNotification`], which describes the notification lifecycle.
pub struct UserScrollNotification {
    viewport_notification: ViewportNotificationData,
    scroll_notification: ScrollNotificationData,

    /// The direction in which the user is scrolling.
    ///
    /// This does not represent the current `AxisDirection` or `GrowthDirection`
    /// of the `Viewport`, which respectively represent the direction that the
    /// scroll offset is increasing in, and the direction that contents are being
    /// laid out in.
    pub direction: ScrollDirection,
}

impl UserScrollNotification {
    /// Creates a notification that the user has changed the direction in which
    /// they are scrolling.
    pub fn new(
        metrics: Rc<dyn ScrollMetrics>,
        context: BuildContext,
        direction: ScrollDirection,
    ) -> UserScrollNotification {
        UserScrollNotification {
            viewport_notification: ViewportNotificationData::new(),
            scroll_notification: ScrollNotificationData {
                metrics,
                context: Some(context),
            },
            direction,
        }
    }
}

impl Notification for UserScrollNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn viewport_depth(&self) -> Option<&Cell<u32>> {
        Some(&self.viewport_notification.depth)
    }

    fn as_scroll_notification(&self) -> Option<&dyn ScrollNotification> {
        Some(self)
    }
}

impl ViewportNotificationMixin for UserScrollNotification {
    fn viewport_notification_data(&self) -> &ViewportNotificationData {
        &self.viewport_notification
    }
}

impl ScrollNotification for UserScrollNotification {
    fn scroll_notification_data(&self) -> &ScrollNotificationData {
        &self.scroll_notification
    }
}

impl Debug for UserScrollNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description = Vec::new();
        fill_scroll_description(self, &mut description);
        description.push(format!("direction: {:?}", self.direction));
        write!(f, "UserScrollNotification({})", description.join(", "))
    }
}

/// A predicate for [`ScrollNotification`], used to customize widgets that
/// listen to notifications from their children.
pub type ScrollNotificationPredicate = Rc<dyn Fn(&dyn ScrollNotification) -> bool>;

/// A [`ScrollNotificationPredicate`] that checks whether `notification.depth() == 0`,
/// which means that the notification did not bubble through any intervening scrolling
/// widgets.
pub fn default_scroll_notification_predicate(notification: &dyn ScrollNotification) -> bool {
    notification.depth() == 0
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;

    use inset_painting::AxisDirection;

    use super::*;
    use crate::framework::{
        AnyElement, ComponentElement, ComponentElementData, ElementData, IntoWidget, KeyRef,
        ProxyElement, Slot, Widget, WidgetKind, WidgetRef, component_element_overrides,
        downcast_widget,
    };
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::notification_listener::NotificationListener;
    use crate::widgets::scroll_metrics::FixedScrollMetrics;

    /// A stand-in for `Viewport`: a proxy whose element mixes in
    /// [`ViewportElementMixin`], so a notification bubbling past it is one viewport deeper.
    #[derive(Debug)]
    struct TestViewport {
        child: WidgetRef,
    }

    fn viewport<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Rc::new(TestViewport {
            child: child.into_widget(),
        })
    }

    impl Widget for TestViewport {
        fn key(&self) -> Option<&KeyRef> {
            None
        }

        fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
            let element = app.create(TestViewportElement {
                element: ElementData::new(this),
                component: ComponentElementData::default(),
            });
            element.as_element()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn widget_type(&self) -> std::any::TypeId {
            std::any::TypeId::of::<TestViewport>()
        }

        fn kind(&self) -> WidgetKind {
            WidgetKind::Other
        }
    }

    struct TestViewportElement {
        element: ElementData,
        component: ComponentElementData,
    }

    impl ComponentElement for TestViewportElement {
        fn component_data(self: Handle<Self>, app: &App) -> &ComponentElementData {
            &app.get(self).component
        }

        fn component_data_mut(self: Handle<Self>, app: &mut App) -> &mut ComponentElementData {
            &mut app.get_mut(self).component
        }

        fn build(self: Handle<Self>, app: &mut App) -> WidgetRef {
            self.proxied_child(app)
        }
    }

    impl ProxyElement for TestViewportElement {
        fn proxied_child(self: Handle<Self>, app: &App) -> WidgetRef {
            downcast_widget::<TestViewport>(&**self.as_element().widget(app))
                .expect("a TestViewportElement holds its TestViewport")
                .child
                .clone()
        }

        fn notify_clients(self: Handle<Self>, _app: &mut App, _old_widget: WidgetRef) {}
    }

    impl ViewportElementMixin for TestViewportElement {}

    impl Element for TestViewportElement {
        crate::element_accessors!();
        component_element_overrides!();

        fn mount(
            self: Handle<Self>,
            app: &mut App,
            parent: Option<AnyElement>,
            new_slot: Option<Slot>,
        ) {
            ComponentElement::mount(self, app, parent, new_slot);
        }

        fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
            ProxyElement::update(self, app, new_widget);
        }

        fn perform_rebuild(self: Handle<Self>, app: &mut App) {
            ComponentElement::perform_rebuild(self, app);
        }

        fn attach_notification_tree(self: Handle<Self>, app: &mut App) {
            ViewportElementMixin::attach_notification_tree(self, app);
        }

        fn on_notification(
            self: Handle<Self>,
            app: &mut App,
            notification: &dyn Notification,
        ) -> bool {
            ViewportElementMixin::on_notification(self, app, notification)
        }
    }

    fn test_metrics() -> Rc<dyn ScrollMetrics> {
        Rc::new(FixedScrollMetrics::new(
            Some(0.0),
            Some(400.0),
            Some(20.0),
            Some(600.0),
            AxisDirection::Down,
            3.0,
        ))
    }

    type Log = Rc<RefCell<Vec<(&'static str, u32)>>>;

    fn listener(log: &Log, name: &'static str, child: WidgetRef) -> WidgetRef {
        let log = Rc::clone(log);
        NotificationListener::<ScrollUpdateNotification>::new(child)
            .on_notification(move |_app, notification| {
                log.borrow_mut().push((name, notification.depth()));
                false
            })
            .into_widget()
    }

    #[test]
    fn a_family_listener_receives_every_kind_of_scroll_notification() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen: Rc<RefCell<Vec<String>>> = Rc::default();
        let dispatcher = Builder::new(|app, context| {
            ScrollStartNotification::new(test_metrics(), Some(context))
                .dispatch(app, Some(context));
            ScrollUpdateNotification::new(test_metrics(), context)
                .scroll_delta(4.0)
                .dispatch(app, Some(context));
            ScrollEndNotification::new(test_metrics(), context).dispatch(app, Some(context));
            SizedBox::shrink().into_widget()
        });
        let tree = NotificationListener::<dyn ScrollNotification>::new(dispatcher)
            .on_notification({
                let seen = Rc::clone(&seen);
                move |_app, notification| {
                    seen.borrow_mut().push(format!("{notification:?}"));
                    false
                }
            })
            .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);
        let seen = seen.borrow();
        assert_eq!(seen.len(), 3);
        assert!(
            seen[0].starts_with("ScrollStartNotification"),
            "{}",
            seen[0]
        );
        assert!(
            seen[1].starts_with("ScrollUpdateNotification"),
            "{}",
            seen[1]
        );
        assert!(seen[2].starts_with("ScrollEndNotification"), "{}", seen[2]);
    }

    #[test]
    fn a_scroll_update_notification_bubbles_and_each_viewport_adds_to_its_depth() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log: Log = Rc::default();
        let dispatcher = Builder::new(|app, context| {
            ScrollUpdateNotification::new(test_metrics(), context)
                .scroll_delta(4.0)
                .dispatch(app, Some(context));
            SizedBox::shrink().into_widget()
        });
        let tree = listener(
            &log,
            "outer",
            viewport(listener(
                &log,
                "middle",
                viewport(listener(&log, "inner", dispatcher.into_widget())),
            )),
        );
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);

        assert_eq!(
            *log.borrow(),
            vec![("inner", 0), ("middle", 1), ("outer", 2)]
        );
    }

    #[test]
    fn a_notification_reports_its_depth_and_its_metrics() {
        let notification = ScrollUpdateNotification::new(
            test_metrics(),
            // A `BuildContext` is required; the element tree is not walked by `Debug`.
            {
                let cell = AppCell::new();
                let mut app = cell.borrow_mut();
                let harness = Harness::mount(&mut app, SizedBox::shrink().into_widget());
                harness.root.as_element()
            },
        )
        .scroll_delta(4.0)
        .with_depth(2);

        assert_eq!(notification.depth(), 2);
        assert!(default_scroll_notification_predicate(
            &ScrollUpdateNotification::new(
                test_metrics(),
                notification.context().expect("a context was given"),
            )
        ));
        assert!(!default_scroll_notification_predicate(&notification));
        assert_eq!(
            format!("{notification:?}"),
            "ScrollUpdateNotification(depth: 2 (remote), \
             FixedScrollMetrics(20.0..[600.0]..380.0), scrollDelta: 4.0)"
        );
    }

    #[test]
    fn the_overscroll_notification_reports_its_overscroll_and_velocity() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, SizedBox::shrink().into_widget());
        let context = harness.root.as_element();
        let notification =
            OverscrollNotification::new(test_metrics(), context, -12.0).velocity(3.0);
        assert_eq!(notification.overscroll, -12.0);
        assert_eq!(
            format!("{notification:?}"),
            "OverscrollNotification(depth: 0 (local), \
             FixedScrollMetrics(20.0..[600.0]..380.0), overscroll: -12.0, velocity: 3.0)"
        );

        let user_scroll =
            UserScrollNotification::new(test_metrics(), context, ScrollDirection::Reverse);
        assert_eq!(
            format!("{user_scroll:?}"),
            "UserScrollNotification(depth: 0 (local), \
             FixedScrollMetrics(20.0..[600.0]..380.0), direction: Reverse)"
        );
    }
}
