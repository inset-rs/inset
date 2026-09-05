//! Flutter counterpart: `widgets/sliver_persistent_header.dart`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curve;
use reveal_embedder::{Matrix4, Offset, Rect};
use reveal_foundation::{App, Handle, ListenableObject, Listener};
use reveal_rendering::{
    AnyRenderBox, AnyRenderObject, AnyRenderSliverFloatingPersistentHeader,
    FloatingHeaderSnapConfiguration, OverScrollHeaderStretchConfiguration, PaintingContext,
    PersistentHeaderShowOnScreenConfiguration, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderSliver, RenderSliverData,
    RenderSliverFloatingPersistentHeader, RenderSliverFloatingPersistentHeaderData,
    RenderSliverFloatingPinnedPersistentHeader, RenderSliverHelpers, RenderSliverPersistentHeader,
    RenderSliverPersistentHeaderData, RenderSliverPinnedPersistentHeader,
    RenderSliverPinnedPersistentHeaderData, RenderSliverScrollingPersistentHeader,
    RenderSliverScrollingPersistentHeaderData, SliverHitTestResult, TickerProviderRef,
};

use crate::framework::{
    AnyElement, BuildContext, Element, ElementData, IntoWidget, KeyRef, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget, Slot, State, StateData,
    StatefulWidget, StatelessWidget, Widget, WidgetKind, WidgetRef, downcast_widget,
};
use crate::widgets::scroll_position::AnyScrollPosition;
use crate::widgets::scrollable::Scrollable;

// ---------------------------------------------------------------------------------------------
// SliverPersistentHeaderDelegate

/// Delegate for configuring a [`SliverPersistentHeader`].
pub trait SliverPersistentHeaderDelegate: Debug + 'static {
    /// The widget to place inside the [`SliverPersistentHeader`].
    ///
    /// The `context` is the [`BuildContext`] of the sliver.
    ///
    /// The `shrink_offset` is a distance from [`max_extent`](Self::max_extent) towards
    /// [`min_extent`](Self::min_extent) representing the current amount by which the sliver has
    /// been shrunk. When the shrink offset is zero, the contents will be rendered with a
    /// dimension of `max_extent` in the main axis. When it equals the difference between
    /// `max_extent` and `min_extent` (a positive number), the contents will be rendered with a
    /// dimension of `min_extent` in the main axis. The shrink offset will always be a positive
    /// number in that range.
    ///
    /// The `overlaps_content` argument is true if subsequent slivers (if any) will be rendered
    /// beneath this one, and false if the sliver will not have any contents below it. Typically
    /// this is used to decide whether to draw a shadow to simulate the sliver being above the
    /// contents below it.
    fn build(
        &self,
        app: &mut App,
        context: BuildContext,
        shrink_offset: f64,
        overlaps_content: bool,
    ) -> WidgetRef;

    /// The smallest size to allow the header to reach, when it shrinks at the start of the
    /// viewport.
    ///
    /// This must return a value equal to or less than [`max_extent`](Self::max_extent).
    ///
    /// This value should not change over the lifetime of the delegate. It should be based
    /// entirely on the constructor arguments passed to the delegate. See
    /// [`should_rebuild`](Self::should_rebuild), which must return true if a new delegate would
    /// return a different value.
    fn min_extent(&self) -> f64;

    /// The size of the header when it is not shrinking at the top of the viewport.
    ///
    /// This must return a value equal to or greater than [`min_extent`](Self::min_extent).
    fn max_extent(&self) -> f64;

    /// A `TickerProvider` to use when animating the header's size changes.
    ///
    /// Must not be `None` if the persistent header is a floating header, and
    /// [`snap_configuration`](Self::snap_configuration) or
    /// [`show_on_screen_configuration`](Self::show_on_screen_configuration) is not `None`.
    fn vsync(&self) -> Option<TickerProviderRef> {
        None
    }

    /// Specifies how floating headers should animate in and out of view.
    ///
    /// If the value of this property is `None`, then floating headers will not animate into
    /// place.
    ///
    /// This is only used for floating headers (those with [`SliverPersistentHeader::floating`]
    /// set to true).
    fn snap_configuration(&self) -> Option<FloatingHeaderSnapConfiguration> {
        None
    }

    /// Specifies a callback and offset for execution.
    ///
    /// If the value of this property is `None`, then the callback will not be triggered.
    ///
    /// This is only used for stretching headers.
    fn stretch_configuration(&self) -> Option<OverScrollHeaderStretchConfiguration> {
        None
    }

    /// Specifies how floating headers and pinned headers should behave in response to
    /// `RenderObject::show_on_screen` calls.
    fn show_on_screen_configuration(&self) -> Option<PersistentHeaderShowOnScreenConfiguration> {
        None
    }

    /// Whether this delegate is meaningfully different from the old delegate.
    ///
    /// If this returns false, then the header might not be rebuilt, even though the instance of
    /// the delegate changed.
    ///
    /// This must return true if `old_delegate` and this object would return different values for
    /// [`min_extent`](Self::min_extent), [`max_extent`](Self::max_extent),
    /// [`snap_configuration`](Self::snap_configuration), or would return a meaningfully different
    /// widget tree from [`build`](Self::build) for the same arguments.
    ///
    /// Dart's `covariant SliverPersistentHeaderDelegate oldDelegate`: the caller has already
    /// compared [`delegate_type`](Self::delegate_type), so the argument downcasts through
    /// [`as_any`](Self::as_any).
    fn should_rebuild(&self, old_delegate: &dyn SliverPersistentHeaderDelegate) -> bool;

    /// The concrete delegate, for Dart's `oldDelegate as Foo`.
    fn as_any(&self) -> &dyn Any;

    /// The concrete delegate's type, Dart's `runtimeType`.
    fn delegate_type(&self) -> TypeId {
        self.as_any().type_id()
    }
}

/// A shared [`SliverPersistentHeaderDelegate`].
pub type SliverPersistentHeaderDelegateRef = Rc<dyn SliverPersistentHeaderDelegate>;

// ---------------------------------------------------------------------------------------------
// SliverPersistentHeader

/// A sliver whose size varies when the sliver is scrolled to the edge of the viewport opposite
/// the sliver's `GrowthDirection`.
///
/// In the normal case of a `CustomScrollView` with no centered sliver, this sliver will vary its
/// size when scrolled to the leading edge of the viewport.
///
/// This is the layout primitive that `SliverAppBar` uses for its shrinking/growing effect.
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
#[derive(Debug)]
pub struct SliverPersistentHeader {
    pub key: Option<KeyRef>,
    /// Configuration for the sliver's layout.
    ///
    /// The delegate provides the following information:
    ///
    ///  * The minimum and maximum dimensions of the sliver.
    ///  * The builder for generating the widgets of the sliver.
    ///  * The instructions for snapping the scroll offset, if [`floating`](Self::floating) is
    ///    true.
    pub delegate: SliverPersistentHeaderDelegateRef,
    /// Whether to stick the header to the start of the viewport once it has reached its minimum
    /// size.
    ///
    /// If this is false, the header will continue scrolling off the screen after it has shrunk to
    /// its minimum extent.
    pub pinned: bool,
    /// Whether the header should immediately grow again if the user reverses scroll direction.
    ///
    /// If this is false, the header only grows again once the user reaches the part of the
    /// viewport that contains the sliver.
    ///
    /// The delegate's `SliverPersistentHeaderDelegate::snap_configuration` is ignored unless
    /// this is true.
    pub floating: bool,
}

impl SliverPersistentHeader {
    /// Creates a sliver that varies its size when it is scrolled to the start of a viewport.
    pub fn new(delegate: SliverPersistentHeaderDelegateRef) -> SliverPersistentHeader {
        SliverPersistentHeader {
            key: None,
            delegate,
            pinned: false,
            floating: false,
        }
    }

    /// Dart `SliverPersistentHeader(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverPersistentHeader {
        self.key = Some(key);
        self
    }

    /// Dart `SliverPersistentHeader(pinned:)`.
    pub fn pinned(mut self, pinned: bool) -> SliverPersistentHeader {
        self.pinned = pinned;
        self
    }

    /// Dart `SliverPersistentHeader(floating:)`.
    pub fn floating(mut self, floating: bool) -> SliverPersistentHeader {
        self.floating = floating;
        self
    }
}

impl StatelessWidget for SliverPersistentHeader {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let delegate = Rc::clone(&self.delegate);
        match (self.floating, self.pinned) {
            (true, true) => header_widget(SliverFloatingPinnedPersistentHeader { delegate }),
            (false, true) => header_widget(SliverPinnedPersistentHeader { delegate }),
            (true, false) => header_widget(SliverFloatingPersistentHeader { delegate }),
            (false, false) => header_widget(SliverScrollingPersistentHeader { delegate }),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The four private render object widgets

/// Flutter's `_SliverPersistentHeaderRenderObjectWidget`.
trait SliverPersistentHeaderRenderObjectWidget: RenderObjectWidget + Sized
where
    Self::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    /// The delegate that lays this header out and builds its child.
    fn delegate(&self) -> &SliverPersistentHeaderDelegateRef;

    /// Whether this header floats, in which case its child is wrapped in a [`FloatingHeader`].
    ///
    /// Dart's `floating` field, which the two floating widgets pass as `super(floating: true)`.
    fn floating(&self) -> bool {
        false
    }
}

/// The erased form of a [`SliverPersistentHeaderRenderObjectWidget`], which is private in Dart
/// and so has no [`IntoWidget`] kind of its own.
struct SliverPersistentHeaderRenderObject<W: SliverPersistentHeaderRenderObjectWidget>(W)
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets;

/// Dart's `_SliverScrollingPersistentHeader(delegate: ..)` as a tree node.
fn header_widget<W: SliverPersistentHeaderRenderObjectWidget>(widget: W) -> WidgetRef
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    Rc::new(SliverPersistentHeaderRenderObject(widget))
}

impl<W: SliverPersistentHeaderRenderObjectWidget> Widget for SliverPersistentHeaderRenderObject<W>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    fn key(&self) -> Option<&KeyRef> {
        RenderObjectWidget::key(&self.0)
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        SliverPersistentHeaderElement::<W>::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl<W: SliverPersistentHeaderRenderObjectWidget> Debug for SliverPersistentHeaderRenderObject<W>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Flutter's `_SliverScrollingPersistentHeader`.
#[derive(Debug)]
struct SliverScrollingPersistentHeader {
    delegate: SliverPersistentHeaderDelegateRef,
}

impl RenderObjectWidget for SliverScrollingPersistentHeader {
    type RenderObject = RenderSliverScrollingPersistentHeaderForWidgets;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSliverScrollingPersistentHeaderForWidgets::new(
            app,
            self.delegate.stretch_configuration(),
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverScrollingPersistentHeaderForWidgets>,
    ) {
        render_object.set_stretch_configuration(app, self.delegate.stretch_configuration());
    }
}

impl SliverPersistentHeaderRenderObjectWidget for SliverScrollingPersistentHeader {
    fn delegate(&self) -> &SliverPersistentHeaderDelegateRef {
        &self.delegate
    }
}

/// Flutter's `_SliverPinnedPersistentHeader`.
#[derive(Debug)]
struct SliverPinnedPersistentHeader {
    delegate: SliverPersistentHeaderDelegateRef,
}

impl RenderObjectWidget for SliverPinnedPersistentHeader {
    type RenderObject = RenderSliverPinnedPersistentHeaderForWidgets;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSliverPinnedPersistentHeaderForWidgets::new(
            app,
            self.delegate.stretch_configuration(),
            self.delegate.show_on_screen_configuration(),
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverPinnedPersistentHeaderForWidgets>,
    ) {
        render_object.set_stretch_configuration(app, self.delegate.stretch_configuration());
        render_object
            .pinned_data_mut(app)
            .show_on_screen_configuration = self.delegate.show_on_screen_configuration();
    }
}

impl SliverPersistentHeaderRenderObjectWidget for SliverPinnedPersistentHeader {
    fn delegate(&self) -> &SliverPersistentHeaderDelegateRef {
        &self.delegate
    }
}

/// Flutter's `_SliverFloatingPersistentHeader`.
#[derive(Debug)]
struct SliverFloatingPersistentHeader {
    delegate: SliverPersistentHeaderDelegateRef,
}

impl RenderObjectWidget for SliverFloatingPersistentHeader {
    type RenderObject = RenderSliverFloatingPersistentHeaderForWidgets;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSliverFloatingPersistentHeaderForWidgets::new(
            app,
            self.delegate.vsync(),
            self.delegate.snap_configuration(),
            self.delegate.stretch_configuration(),
            self.delegate.show_on_screen_configuration(),
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverFloatingPersistentHeaderForWidgets>,
    ) {
        render_object.set_vsync(app, self.delegate.vsync());
        render_object.set_snap_configuration(app, self.delegate.snap_configuration());
        render_object.set_stretch_configuration(app, self.delegate.stretch_configuration());
        render_object
            .set_show_on_screen_configuration(app, self.delegate.show_on_screen_configuration());
    }
}

impl SliverPersistentHeaderRenderObjectWidget for SliverFloatingPersistentHeader {
    fn delegate(&self) -> &SliverPersistentHeaderDelegateRef {
        &self.delegate
    }

    fn floating(&self) -> bool {
        true
    }
}

/// Flutter's `_SliverFloatingPinnedPersistentHeader`.
#[derive(Debug)]
struct SliverFloatingPinnedPersistentHeader {
    delegate: SliverPersistentHeaderDelegateRef,
}

impl RenderObjectWidget for SliverFloatingPinnedPersistentHeader {
    type RenderObject = RenderSliverFloatingPinnedPersistentHeaderForWidgets;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSliverFloatingPinnedPersistentHeaderForWidgets::new(
            app,
            self.delegate.vsync(),
            self.delegate.snap_configuration(),
            self.delegate.stretch_configuration(),
            self.delegate.show_on_screen_configuration(),
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverFloatingPinnedPersistentHeaderForWidgets>,
    ) {
        render_object.set_vsync(app, self.delegate.vsync());
        render_object.set_snap_configuration(app, self.delegate.snap_configuration());
        render_object.set_stretch_configuration(app, self.delegate.stretch_configuration());
        render_object
            .set_show_on_screen_configuration(app, self.delegate.show_on_screen_configuration());
    }
}

impl SliverPersistentHeaderRenderObjectWidget for SliverFloatingPinnedPersistentHeader {
    fn delegate(&self) -> &SliverPersistentHeaderDelegateRef {
        &self.delegate
    }

    fn floating(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------------------------
// _FloatingHeader

/// Flutter's `_FloatingHeader`.
#[derive(Debug)]
struct FloatingHeader {
    child: WidgetRef,
}

impl FloatingHeader {
    fn new<K>(child: impl IntoWidget<K>) -> FloatingHeader {
        FloatingHeader {
            child: child.into_widget(),
        }
    }
}

impl StatefulWidget for FloatingHeader {
    type State = FloatingHeaderState;

    fn create_state(&self) -> FloatingHeaderState {
        FloatingHeaderState {
            state: StateData::new(),
            position: None,
        }
    }
}

/// A wrapper for the widget created by [`SliverPersistentHeaderElement`] that starts and stops
/// the floating app bar's snap-into-view or snap-out-of-view animation. It also informs the float
/// when pointer scrolling by updating the last known `ScrollDirection` when scrolling began.
struct FloatingHeaderState {
    state: StateData<FloatingHeader>,
    position: Option<AnyScrollPosition>,
}

impl FloatingHeaderState {
    /// Dart's `_headerRenderer`: the enclosing floating header render object, found through
    /// `RenderObject::interface`, which answers Dart's
    /// `findAncestorRenderObjectOfType<RenderSliverFloatingPersistentHeader>()`.
    fn header_renderer(
        self: Handle<Self>,
        app: &App,
    ) -> Option<AnyRenderSliverFloatingPersistentHeader> {
        let mut ancestor = self.context(app).parent(app);
        while let Some(current) = ancestor {
            if current.is_render_object_element()
                && let Some(render_object) = current.render_object(app)
                && let Some(header) =
                    render_object.interface::<AnyRenderSliverFloatingPersistentHeader>()
            {
                return Some(header);
            }
            ancestor = current.parent(app);
        }
        None
    }

    /// Dart's `_isScrollingListener`.
    fn is_scrolling_listener(self: Handle<Self>, app: &mut App) {
        let position = app
            .get(self)
            .position
            .expect("the listener is only registered while a position is held");

        // When a scroll stops, then maybe snap the app bar into view.
        // Similarly, when a scroll starts, then maybe stop the snap animation.
        // Update the scrolling direction as well for pointer scrolling updates.
        let header = self.header_renderer(app);
        let direction = position.user_scroll_direction(app);
        if *app.get(position.is_scrolling_notifier(app)).value() {
            if let Some(header) = header {
                header.update_scroll_start_direction(app, direction);
                // Only SliverAppBars support snapping, headers will not snap.
                header.maybe_stop_snap_animation(app, direction);
            }
        } else if let Some(header) = header {
            // Only SliverAppBars support snapping, headers will not snap.
            header.maybe_start_snap_animation(app, direction);
        }
    }

    /// The tear-off Dart passes to `isScrollingNotifier.addListener` and `removeListener`.
    fn is_scrolling_listener_tear_off(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::is_scrolling_listener)
    }
}

impl State for FloatingHeaderState {
    type Widget = FloatingHeader;
    crate::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        if let Some(position) = app.get(self).position {
            position
                .is_scrolling_notifier(app)
                .remove_listener(app, &self.is_scrolling_listener_tear_off());
        }
        let context = self.context(app);
        let position = Scrollable::maybe_of(app, context, None).map(|state| state.position(app));
        app.get_mut(self).position = position;
        if let Some(position) = position {
            position
                .is_scrolling_notifier(app)
                .add_listener(app, self.is_scrolling_listener_tear_off());
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(position) = app.get(self).position {
            position
                .is_scrolling_notifier(app)
                .remove_listener(app, &self.is_scrolling_listener_tear_off());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        self.widget(app).child.clone()
    }
}

impl Debug for FloatingHeaderState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FloatingHeaderState")
            .field("position", &self.position)
            .finish()
    }
}

// ---------------------------------------------------------------------------------------------
// _SliverPersistentHeaderElement

/// Flutter's `_SliverPersistentHeaderElement`: builds the delegate's child during layout.
struct SliverPersistentHeaderElement<W: SliverPersistentHeaderRenderObjectWidget>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    element: ElementData,
    render_object_element: RenderObjectElementData,
    floating: bool,
    child: Option<AnyElement>,
    marker: std::marker::PhantomData<fn() -> W>,
}

impl<W: SliverPersistentHeaderRenderObjectWidget> SliverPersistentHeaderElement<W>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    fn create(app: &mut App, widget: WidgetRef) -> Handle<SliverPersistentHeaderElement<W>> {
        let floating = downcast_widget::<W>(&*widget)
            .expect("a header element is created from its own widget")
            .floating();
        app.create(SliverPersistentHeaderElement::<W> {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            floating,
            child: None,
            marker: std::marker::PhantomData,
        })
    }

    fn header(self: Handle<Self>, app: &App) -> RenderHandle<W::RenderObject> {
        self.typed_render_object(app)
    }

    /// Flutter's `_updateChild`.
    fn update_child(
        self: Handle<Self>,
        app: &mut App,
        delegate: SliverPersistentHeaderDelegateRef,
        shrink_offset: f64,
        overlaps_content: bool,
    ) {
        let new_widget = delegate.build(app, self.as_element(), shrink_offset, overlaps_content);
        let new_widget = if app.get(self).floating {
            FloatingHeader::new(new_widget).into_widget()
        } else {
            new_widget
        };
        let child = app.get(self).child;
        let child = self
            .as_element()
            .update_child(app, child, Some(new_widget), None);
        app.get_mut(self).child = child;
    }

    /// Flutter's `_build`, run from the render object's layout callback.
    fn build(self: Handle<Self>, app: &mut App, shrink_offset: f64, overlaps_content: bool) {
        let owner = self
            .as_element()
            .owner(app)
            .expect("a mounted element has an owner");
        owner.build_scope(
            app,
            self.as_element(),
            Some(Box::new(move |app| {
                let delegate = Rc::clone(Self::widget_of(self.as_element().widget(app)).delegate());
                self.update_child(app, delegate, shrink_offset, overlaps_content);
            })),
        );
    }

    /// The element as its render object's `_element` field.
    fn as_header_element(self: Handle<Self>) -> SliverPersistentHeaderElementRef {
        SliverPersistentHeaderElementRef(Rc::new(self))
    }
}

impl<W: SliverPersistentHeaderRenderObjectWidget> RenderObjectElementWidget
    for SliverPersistentHeaderElement<W>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    type Widget = W;

    fn widget_of(widget: &WidgetRef) -> &W {
        downcast_widget::<W>(&**widget)
            .expect("a SliverPersistentHeaderElement holds its header widget")
    }
}

impl<W: SliverPersistentHeaderRenderObjectWidget> RenderObjectElement
    for SliverPersistentHeaderElement<W>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
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

impl<W: SliverPersistentHeaderRenderObjectWidget> Element for SliverPersistentHeaderElement<W>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
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

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
        let element = self.as_header_element();
        self.header(app).set_element(app, Some(element));
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        self.header(app).set_element(app, None);
        RenderObjectElement::unmount(self, app);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        RenderObjectElement::update(self, app, new_widget.clone());
        let new_delegate = Rc::clone(Self::widget_of(&new_widget).delegate());
        let old_delegate = Rc::clone(Self::widget_of(&old_widget).delegate());
        if !Rc::ptr_eq(&new_delegate, &old_delegate)
            && (new_delegate.delegate_type() != old_delegate.delegate_type()
                || new_delegate.should_rebuild(&*old_delegate))
        {
            let render_object = self.header(app);
            let shrink_offset = render_object.last_shrink_offset(app);
            let overlaps_content = render_object.last_overlaps_content(app);
            self.update_child(app, new_delegate, shrink_offset, overlaps_content);
            render_object.trigger_rebuild(app);
        }
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
        self.header(app).trigger_rebuild(app);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(app.get(self).child == Some(child));
        app.get_mut(self).child = None;
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        if let Some(child) = app.get(self).child {
            visitor(child);
        }
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
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
        _slot: Option<Slot>,
    ) {
        let render_object = self.header(app);
        render_object.set_child(app, Some(child.as_box().expect("a header child is a box")));
    }

    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        debug_assert!(false, "a persistent header has one child");
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        _child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        self.header(app).set_child(app, None);
    }
}

// ---------------------------------------------------------------------------------------------
// _RenderSliverPersistentHeaderForWidgetsMixin

/// The element behind a header render object, erased: Dart's `_element` field.
///
/// A `Handle<SliverPersistentHeaderElement<W>>` cannot be held by a render object that does not
/// know `W`, so the mixin holds this `Rc` over the two members it calls.
#[derive(Clone)]
pub struct SliverPersistentHeaderElementRef(Rc<dyn ErasedHeaderElement>);

impl PartialEq for SliverPersistentHeaderElementRef {
    fn eq(&self, other: &SliverPersistentHeaderElementRef) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Debug for SliverPersistentHeaderElementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SliverPersistentHeaderElementRef")
    }
}

/// The object side of [`SliverPersistentHeaderElementRef`].
trait ErasedHeaderElement {
    /// The delegate of the widget this element holds.
    fn delegate(&self, app: &App) -> SliverPersistentHeaderDelegateRef;

    /// `_SliverPersistentHeaderElement._build`.
    fn build(&self, app: &mut App, shrink_offset: f64, overlaps_content: bool);
}

impl<W: SliverPersistentHeaderRenderObjectWidget> ErasedHeaderElement
    for Handle<SliverPersistentHeaderElement<W>>
where
    W::RenderObject: RenderSliverPersistentHeaderForWidgets,
{
    fn delegate(&self, app: &App) -> SliverPersistentHeaderDelegateRef {
        Rc::clone(
            SliverPersistentHeaderElement::<W>::widget_of(self.as_element().widget(app)).delegate(),
        )
    }

    fn build(&self, app: &mut App, shrink_offset: f64, overlaps_content: bool) {
        SliverPersistentHeaderElement::<W>::build(*self, app, shrink_offset, overlaps_content);
    }
}

/// Flutter's `_RenderSliverPersistentHeaderForWidgetsMixin` field.
#[derive(Default)]
pub struct RenderSliverPersistentHeaderForWidgetsData {
    element: Option<SliverPersistentHeaderElementRef>,
}

impl RenderSliverPersistentHeaderForWidgetsData {
    /// Creates the state of a header whose element has not mounted yet.
    pub fn new() -> RenderSliverPersistentHeaderForWidgetsData {
        RenderSliverPersistentHeaderForWidgetsData { element: None }
    }
}

/// Flutter's `_RenderSliverPersistentHeaderForWidgetsMixin`: a persistent header whose extents
/// and child come from a [`SliverPersistentHeaderDelegate`].
pub trait RenderSliverPersistentHeaderForWidgets: RenderSliverPersistentHeader {
    /// Mixin field access.
    fn for_widgets_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderSliverPersistentHeaderForWidgetsData;

    /// See [`for_widgets_data`](Self::for_widgets_data).
    fn for_widgets_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverPersistentHeaderForWidgetsData;

    /// Sets Dart's `_element`; the element does this at mount and clears it at unmount.
    fn set_element(
        self: RenderHandle<Self>,
        app: &mut App,
        element: Option<SliverPersistentHeaderElementRef>,
    ) {
        self.for_widgets_data_mut(app).element = element;
    }

    /// The delegate of the widget the element holds.
    fn delegate(self: RenderHandle<Self>, app: &App) -> SliverPersistentHeaderDelegateRef {
        self.for_widgets_data(app)
            .element
            .as_ref()
            .expect("the element is set at mount")
            .0
            .delegate(app)
    }

    /// The body of Flutter's `minExtent` override.
    fn min_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        self.delegate(app).min_extent()
    }

    /// The body of Flutter's `maxExtent` override.
    fn max_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        self.delegate(app).max_extent()
    }

    /// The body of Flutter's `updateChild` override.
    fn update_child(
        self: RenderHandle<Self>,
        app: &mut App,
        shrink_offset: f64,
        overlaps_content: bool,
    ) {
        let element = self
            .for_widgets_data(app)
            .element
            .clone()
            .expect("the element is set at mount");
        element.0.build(app, shrink_offset, overlaps_content);
    }

    /// Flutter's `triggerRebuild`.
    fn trigger_rebuild(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverPersistentHeader::mark_needs_layout(self, app);
    }
}

/// The `RenderObjectWithChildMixin`, `RenderSliverHelpers`, `RenderSliverPersistentHeader` and
/// `RenderSliverPersistentHeaderForWidgets` halves every header leaf writes the same way.
macro_rules! header_for_widgets_leaf {
    ($leaf:ident) => {
        impl RenderObjectWithChildMixin for $leaf {
            type ChildType = AnyRenderBox;

            fn child_data(
                self: RenderHandle<Self>,
                app: &App,
            ) -> &RenderObjectWithChildData<AnyRenderBox> {
                &self.get(app).child
            }

            fn child_data_mut(
                self: RenderHandle<Self>,
                app: &mut App,
            ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
                &mut self.get_mut(app).child
            }
        }

        impl RenderSliverHelpers for $leaf {}

        impl RenderSliverPersistentHeader for $leaf {
            reveal_rendering::render_sliver_persistent_header_accessors!();

            fn max_extent(self: RenderHandle<Self>, app: &App) -> f64 {
                RenderSliverPersistentHeaderForWidgets::max_extent(self, app)
            }

            fn min_extent(self: RenderHandle<Self>, app: &App) -> f64 {
                RenderSliverPersistentHeaderForWidgets::min_extent(self, app)
            }

            fn update_child(
                self: RenderHandle<Self>,
                app: &mut App,
                shrink_offset: f64,
                overlaps_content: bool,
            ) {
                RenderSliverPersistentHeaderForWidgets::update_child(
                    self,
                    app,
                    shrink_offset,
                    overlaps_content,
                );
            }
        }

        impl RenderSliverPersistentHeaderForWidgets for $leaf {
            fn for_widgets_data(
                self: RenderHandle<Self>,
                app: &App,
            ) -> &RenderSliverPersistentHeaderForWidgetsData {
                &self.get(app).for_widgets
            }

            fn for_widgets_data_mut(
                self: RenderHandle<Self>,
                app: &mut App,
            ) -> &mut RenderSliverPersistentHeaderForWidgetsData {
                &mut self.get_mut(app).for_widgets
            }
        }
    };
}

/// The `RenderObject` and `RenderSliver` halves shared by every header leaf: the persistent
/// header bodies, plus the `perform_layout` and `child_main_axis_position` of the variant.
macro_rules! header_protocol {
    ($leaf:ident, $perform_layout:path, $child_position:path $(, $extra:item)*) => {
        impl RenderObject for $leaf {
            reveal_rendering::render_object_accessors!();

            fn visit_children(
                self: RenderHandle<Self>,
                app: &App,
                visitor: &mut dyn FnMut(AnyRenderObject),
            ) {
                if let Some(child) = self.child(app) {
                    visitor(child.as_object());
                }
            }

            fn apply_paint_transform(
                self: RenderHandle<Self>,
                app: &App,
                child: AnyRenderObject,
                transform: &mut Matrix4,
            ) {
                RenderSliverPersistentHeader::apply_paint_transform(self, app, child, transform);
            }

            fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
                $perform_layout(self, app)
            }

            fn paint(
                self: RenderHandle<Self>,
                app: &mut App,
                context: &mut PaintingContext,
                offset: Offset,
            ) {
                RenderSliverPersistentHeader::paint(self, app, context, offset);
            }

            $($extra)*
        }

        impl RenderSliver for $leaf {
            reveal_rendering::render_sliver_accessors!();

            fn hit_test_children(
                self: RenderHandle<Self>,
                app: &mut App,
                result: &mut SliverHitTestResult<'_>,
                main_axis_position: f64,
                cross_axis_position: f64,
            ) -> bool {
                RenderSliverPersistentHeader::hit_test_children(
                    self,
                    app,
                    result,
                    main_axis_position,
                    cross_axis_position,
                )
            }

            fn child_main_axis_position(
                self: RenderHandle<Self>,
                app: &App,
                child: AnyRenderObject,
            ) -> f64 {
                $child_position(self, app, child)
            }
        }
    };
}

/// Flutter's `_RenderSliverScrollingPersistentHeaderForWidgets`.
pub struct RenderSliverScrollingPersistentHeaderForWidgets {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    header: RenderSliverPersistentHeaderData,
    scrolling: RenderSliverScrollingPersistentHeaderData,
    for_widgets: RenderSliverPersistentHeaderForWidgetsData,
}

impl RenderSliverScrollingPersistentHeaderForWidgets {
    /// Creates a scrolling persistent header whose extents come from a delegate.
    pub fn new(
        app: &mut App,
        stretch_configuration: Option<OverScrollHeaderStretchConfiguration>,
    ) -> RenderHandle<RenderSliverScrollingPersistentHeaderForWidgets> {
        RenderHandle::new_sliver(
            app,
            RenderSliverScrollingPersistentHeaderForWidgets {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                header: RenderSliverPersistentHeaderData::new(stretch_configuration),
                scrolling: RenderSliverScrollingPersistentHeaderData::new(),
                for_widgets: RenderSliverPersistentHeaderForWidgetsData::new(),
            },
        )
    }
}

header_for_widgets_leaf!(RenderSliverScrollingPersistentHeaderForWidgets);

impl RenderSliverScrollingPersistentHeader for RenderSliverScrollingPersistentHeaderForWidgets {
    fn scrolling_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderSliverScrollingPersistentHeaderData {
        &self.get(app).scrolling
    }

    fn scrolling_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverScrollingPersistentHeaderData {
        &mut self.get_mut(app).scrolling
    }
}

header_protocol!(
    RenderSliverScrollingPersistentHeaderForWidgets,
    RenderSliverScrollingPersistentHeader::perform_layout,
    RenderSliverScrollingPersistentHeader::child_main_axis_position
);

/// Flutter's `_RenderSliverPinnedPersistentHeaderForWidgets`.
pub struct RenderSliverPinnedPersistentHeaderForWidgets {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    header: RenderSliverPersistentHeaderData,
    pinned: RenderSliverPinnedPersistentHeaderData,
    for_widgets: RenderSliverPersistentHeaderForWidgetsData,
}

impl RenderSliverPinnedPersistentHeaderForWidgets {
    /// Creates a pinned persistent header whose extents come from a delegate.
    pub fn new(
        app: &mut App,
        stretch_configuration: Option<OverScrollHeaderStretchConfiguration>,
        show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
    ) -> RenderHandle<RenderSliverPinnedPersistentHeaderForWidgets> {
        RenderHandle::new_sliver(
            app,
            RenderSliverPinnedPersistentHeaderForWidgets {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                header: RenderSliverPersistentHeaderData::new(stretch_configuration),
                pinned: RenderSliverPinnedPersistentHeaderData {
                    show_on_screen_configuration,
                },
                for_widgets: RenderSliverPersistentHeaderForWidgetsData::new(),
            },
        )
    }
}

header_for_widgets_leaf!(RenderSliverPinnedPersistentHeaderForWidgets);

impl RenderSliverPinnedPersistentHeader for RenderSliverPinnedPersistentHeaderForWidgets {
    fn pinned_data(self: RenderHandle<Self>, app: &App) -> &RenderSliverPinnedPersistentHeaderData {
        &self.get(app).pinned
    }

    fn pinned_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverPinnedPersistentHeaderData {
        &mut self.get_mut(app).pinned
    }
}

header_protocol!(
    RenderSliverPinnedPersistentHeaderForWidgets,
    RenderSliverPinnedPersistentHeader::perform_layout,
    RenderSliverPinnedPersistentHeader::child_main_axis_position,
    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        RenderSliverPinnedPersistentHeader::show_on_screen(
            self, app, descendant, rect, duration, curve,
        );
    }
);

/// Flutter's `_RenderSliverFloatingPersistentHeaderForWidgets`.
pub struct RenderSliverFloatingPersistentHeaderForWidgets {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    header: RenderSliverPersistentHeaderData,
    floating: RenderSliverFloatingPersistentHeaderData,
    for_widgets: RenderSliverPersistentHeaderForWidgetsData,
}

impl RenderSliverFloatingPersistentHeaderForWidgets {
    /// Creates a floating persistent header whose extents come from a delegate.
    pub fn new(
        app: &mut App,
        vsync: Option<TickerProviderRef>,
        snap_configuration: Option<FloatingHeaderSnapConfiguration>,
        stretch_configuration: Option<OverScrollHeaderStretchConfiguration>,
        show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
    ) -> RenderHandle<RenderSliverFloatingPersistentHeaderForWidgets> {
        RenderHandle::new_sliver(
            app,
            RenderSliverFloatingPersistentHeaderForWidgets {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                header: RenderSliverPersistentHeaderData::new(stretch_configuration),
                floating: RenderSliverFloatingPersistentHeaderData::new(
                    vsync,
                    snap_configuration,
                    show_on_screen_configuration,
                ),
                for_widgets: RenderSliverPersistentHeaderForWidgetsData::new(),
            },
        )
    }
}

header_for_widgets_leaf!(RenderSliverFloatingPersistentHeaderForWidgets);

impl RenderSliverFloatingPersistentHeader for RenderSliverFloatingPersistentHeaderForWidgets {
    fn floating_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderSliverFloatingPersistentHeaderData {
        &self.get(app).floating
    }

    fn floating_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverFloatingPersistentHeaderData {
        &mut self.get_mut(app).floating
    }
}

header_protocol!(
    RenderSliverFloatingPersistentHeaderForWidgets,
    RenderSliverFloatingPersistentHeader::perform_layout,
    RenderSliverFloatingPersistentHeader::child_main_axis_position,
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverFloatingPersistentHeader::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    },
    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        RenderSliverFloatingPersistentHeader::show_on_screen(
            self, app, descendant, rect, duration, curve,
        );
    },
    fn interface(self: RenderHandle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        (id == TypeId::of::<AnyRenderSliverFloatingPersistentHeader>())
            .then(|| Box::new(self.as_floating_persistent_header()) as Box<dyn Any>)
    }
);

/// Flutter's `_RenderSliverFloatingPinnedPersistentHeaderForWidgets`.
pub struct RenderSliverFloatingPinnedPersistentHeaderForWidgets {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    header: RenderSliverPersistentHeaderData,
    floating: RenderSliverFloatingPersistentHeaderData,
    for_widgets: RenderSliverPersistentHeaderForWidgetsData,
}

impl RenderSliverFloatingPinnedPersistentHeaderForWidgets {
    /// Creates a floating pinned persistent header whose extents come from a delegate.
    pub fn new(
        app: &mut App,
        vsync: Option<TickerProviderRef>,
        snap_configuration: Option<FloatingHeaderSnapConfiguration>,
        stretch_configuration: Option<OverScrollHeaderStretchConfiguration>,
        show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
    ) -> RenderHandle<RenderSliverFloatingPinnedPersistentHeaderForWidgets> {
        RenderHandle::new_sliver(
            app,
            RenderSliverFloatingPinnedPersistentHeaderForWidgets {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                header: RenderSliverPersistentHeaderData::new(stretch_configuration),
                floating: RenderSliverFloatingPersistentHeaderData::new(
                    vsync,
                    snap_configuration,
                    show_on_screen_configuration,
                ),
                for_widgets: RenderSliverPersistentHeaderForWidgetsData::new(),
            },
        )
    }
}

header_for_widgets_leaf!(RenderSliverFloatingPinnedPersistentHeaderForWidgets);

impl RenderSliverFloatingPersistentHeader for RenderSliverFloatingPinnedPersistentHeaderForWidgets {
    fn floating_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderSliverFloatingPersistentHeaderData {
        &self.get(app).floating
    }

    fn floating_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverFloatingPersistentHeaderData {
        &mut self.get_mut(app).floating
    }

    fn update_geometry(self: RenderHandle<Self>, app: &mut App) -> f64 {
        RenderSliverFloatingPinnedPersistentHeader::update_geometry(self, app)
    }
}

impl RenderSliverFloatingPinnedPersistentHeader
    for RenderSliverFloatingPinnedPersistentHeaderForWidgets
{
}

header_protocol!(
    RenderSliverFloatingPinnedPersistentHeaderForWidgets,
    RenderSliverFloatingPersistentHeader::perform_layout,
    RenderSliverFloatingPersistentHeader::child_main_axis_position,
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverFloatingPersistentHeader::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    },
    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        RenderSliverFloatingPersistentHeader::show_on_screen(
            self, app, descendant, rect, duration, curve,
        );
    },
    fn interface(self: RenderHandle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        (id == TypeId::of::<AnyRenderSliverFloatingPersistentHeader>())
            .then(|| Box::new(self.as_floating_persistent_header()) as Box<dyn Any>)
    }
);

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;

    use reveal_embedder::TextDirection;
    use reveal_rendering::{
        ContainerRenderObjectMixin, FixedViewportOffset, RenderBox, RenderViewport,
        ScrollCacheExtent, ViewportOffset,
    };

    use super::*;
    use crate::framework::IntoWidget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Directionality, SizedBox, SliverToBoxAdapter};
    use crate::widgets::viewport::Viewport;

    /// The shrink offsets the delegate was asked to build for, in order.
    type ShrinkOffsets = Rc<RefCell<Vec<f64>>>;

    #[derive(Debug)]
    struct TestHeaderDelegate {
        min_extent: f64,
        max_extent: f64,
        shrink_offsets: ShrinkOffsets,
    }

    impl SliverPersistentHeaderDelegate for TestHeaderDelegate {
        fn build(
            &self,
            _app: &mut App,
            _context: BuildContext,
            shrink_offset: f64,
            _overlaps_content: bool,
        ) -> WidgetRef {
            self.shrink_offsets.borrow_mut().push(shrink_offset);
            SizedBox::expand().into_widget()
        }

        fn min_extent(&self) -> f64 {
            self.min_extent
        }

        fn max_extent(&self) -> f64 {
            self.max_extent
        }

        fn should_rebuild(&self, _old_delegate: &dyn SliverPersistentHeaderDelegate) -> bool {
            true
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn header_tree(
        offset: Handle<FixedViewportOffset>,
        shrink_offsets: &ShrinkOffsets,
        pinned: bool,
        floating: bool,
    ) -> WidgetRef {
        let delegate = Rc::new(TestHeaderDelegate {
            min_extent: 40.0,
            max_extent: 120.0,
            shrink_offsets: Rc::clone(shrink_offsets),
        }) as SliverPersistentHeaderDelegateRef;
        Directionality::new(
            TextDirection::Ltr,
            Viewport::new(offset.as_viewport_offset())
                .scroll_cache_extent(ScrollCacheExtent::Pixels(0.0))
                .slivers(vec![
                    SliverPersistentHeader::new(delegate)
                        .pinned(pinned)
                        .floating(floating)
                        .into_widget(),
                    SliverToBoxAdapter::new()
                        .child(SizedBox::new().height(1000.0))
                        .into_widget(),
                ]),
        )
        .into_widget()
    }

    fn viewport_of(harness: &Harness, app: &App) -> RenderHandle<RenderViewport> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderViewport>(app)
            .expect("a RenderViewport")
    }

    fn scroll_to(
        harness: &Harness,
        app: &mut App,
        offset: Handle<FixedViewportOffset>,
        pixels: f64,
    ) {
        let current = offset.pixels(app);
        offset.correct_by(app, pixels - current);
        viewport_of(harness, app).mark_needs_layout(app);
        harness.pump(app);
    }

    /// A pinned header stays at the leading edge, shrinking to its minimum extent as it is
    /// scrolled, and rebuilds its child at each shrink offset.
    #[test]
    fn a_pinned_header_shrinks_to_its_minimum_extent_and_stays() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let shrink_offsets: ShrinkOffsets = Rc::default();
        let harness = Harness::mount(&mut app, header_tree(offset, &shrink_offsets, true, false));
        harness.pump(&mut app);

        let header = viewport_of(&harness, &app)
            .first_child(&app)
            .expect("the header");
        assert_eq!(header.geometry(&app).paint_extent, 120.0);
        assert_eq!(header.geometry(&app).layout_extent, 120.0);
        assert_eq!(header.geometry(&app).max_scroll_obstruction_extent, 40.0);
        assert_eq!(*shrink_offsets.borrow(), vec![0.0]);

        scroll_to(&harness, &mut app, offset, 100.0);

        // Shrunk to the minimum extent, still painting at the leading edge.
        assert_eq!(header.geometry(&app).paint_extent, 40.0);
        assert_eq!(header.geometry(&app).layout_extent, 20.0);
        assert_eq!(*shrink_offsets.borrow(), vec![0.0, 100.0]);
    }

    /// A floating header scrolls off as it shrinks: its paint extent falls with the scroll
    /// offset and it reports no scroll obstruction.
    #[test]
    fn a_floating_header_scrolls_off_as_it_shrinks() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let shrink_offsets: ShrinkOffsets = Rc::default();
        let harness = Harness::mount(&mut app, header_tree(offset, &shrink_offsets, false, true));
        harness.pump(&mut app);

        let header = viewport_of(&harness, &app)
            .first_child(&app)
            .expect("the header");
        assert_eq!(header.geometry(&app).scroll_extent, 120.0);
        assert_eq!(header.geometry(&app).paint_extent, 120.0);
        assert_eq!(*shrink_offsets.borrow(), vec![0.0]);

        scroll_to(&harness, &mut app, offset, 100.0);

        assert_eq!(header.geometry(&app).paint_extent, 20.0);
        assert_eq!(header.geometry(&app).max_scroll_obstruction_extent, 0.0);
        assert_eq!(*shrink_offsets.borrow(), vec![0.0, 100.0]);
    }
}
