//! Flutter counterpart: `widgets/sliver_prototype_extent_list.dart`.

use std::any::Any;
use std::rc::Rc;

use waft_embedder::{Matrix4, Offset};
use waft_foundation::{App, Handle};
use waft_painting::Axis;
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, ContainerRenderObjectData, ContainerRenderObjectMixin,
    PaintingContext, PipelineOwner, RenderHandle, RenderObject, RenderObjectData, RenderSliver,
    RenderSliverData, RenderSliverFixedExtentBoxAdaptor, RenderSliverFixedExtentBoxAdaptorData,
    RenderSliverHelpers, RenderSliverMultiBoxAdaptor, RenderSliverMultiBoxAdaptorData,
    RenderSliverWithKeepAliveMixin, SliverHitTestResult, SliverMultiBoxAdaptorParentData,
};

use crate::framework::{
    AnyElement, BuildContext, Element, ElementData, IntoWidget, KeyRef, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget, Slot, WidgetRef,
    downcast_widget,
};
use crate::widgets::scroll_delegate::{
    ChildIndexGetter, NullableIndexedWidgetBuilder, SliverChildBuilderDelegate,
    SliverChildDelegateRef, SliverChildListDelegate,
};
use crate::widgets::sliver::{
    SliverMultiBoxAdaptorElementBase, SliverMultiBoxAdaptorElementData, SliverMultiBoxAdaptorKind,
    SliverMultiBoxAdaptorWidget, SliverWithKeepAliveWidget, builder_constructor_setters,
    list_constructor_setters,
};

/// A sliver that places its box children in a linear array and constrains them to have the same
/// extent as a prototype item along the main axis.
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
///
/// [`SliverPrototypeExtentList`] arranges its children in a line along the main axis starting at
/// offset zero and without gaps. Each child is constrained to the same extent as the
/// [`prototype_item`](Self::prototype_item) along the main axis and the
/// `SliverConstraints::cross_axis_extent` along the cross axis.
///
/// [`SliverPrototypeExtentList`] is more efficient than `SliverList` because
/// [`SliverPrototypeExtentList`] does not need to lay out its children to obtain their extent
/// along the main axis. It's a little more flexible than `SliverFixedExtentList` because there's
/// no need to determine the appropriate item extent in pixels.
///
/// See also:
///
///  * `SliverFixedExtentList`, whose children are forced to a given pixel extent.
///  * `SliverVariedExtentList`, which supports children with varying (but known upfront)
///    extents.
///  * `SliverList`, which does not require its children to have the same extent in the main
///    axis.
///  * `SliverFillViewport`, which sizes its children based on the size of the viewport,
///    regardless of what else is in the scroll view.
#[derive(Debug)]
pub struct SliverPrototypeExtentList {
    pub key: Option<KeyRef>,
    /// The delegate that provides the children for this widget.
    pub delegate: SliverChildDelegateRef,
    /// Defines the main axis extent of all of this sliver's children.
    ///
    /// The [`prototype_item`](Self::prototype_item) is laid out before the rest of the sliver's
    /// children and its size along the main axis fixes the size of each child. The
    /// [`prototype_item`](Self::prototype_item) is essentially `Offstage`: it is not painted and
    /// it cannot respond to input.
    pub prototype_item: WidgetRef,
}

impl SliverPrototypeExtentList {
    /// Creates a sliver that places its box children in a linear array and constrains them to
    /// have the same extent as a prototype item along the main axis.
    pub fn new<K>(
        delegate: SliverChildDelegateRef,
        prototype_item: impl IntoWidget<K>,
    ) -> SliverPrototypeExtentList {
        SliverPrototypeExtentList {
            key: None,
            delegate,
            prototype_item: prototype_item.into_widget(),
        }
    }

    /// A sliver that places its box children in a linear array and constrains them to have the
    /// same extent as a prototype item along the main axis.
    ///
    /// This constructor is appropriate for sliver lists with a large (or infinite) number of
    /// children whose extent is already determined.
    ///
    /// Providing an `item_count` improves the ability of the sliver to estimate the maximum
    /// scroll extent.
    ///
    /// `item_builder` will be called only with indices greater than or equal to zero and less
    /// than `item_count`.
    ///
    /// The `prototype_item` argument is used to determine the extent of each item.
    pub fn builder<K>(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
        prototype_item: impl IntoWidget<K>,
    ) -> SliverPrototypeExtentListBuilder {
        SliverPrototypeExtentListBuilder::new(Rc::new(item_builder), prototype_item.into_widget())
    }

    /// A sliver that places multiple box children in a linear array along the main axis.
    ///
    /// This constructor uses a list of widgets to build the sliver.
    pub fn list<K>(
        children: Vec<WidgetRef>,
        prototype_item: impl IntoWidget<K>,
    ) -> SliverPrototypeExtentListList {
        SliverPrototypeExtentListList::new(children, prototype_item.into_widget())
    }

    /// Dart `SliverPrototypeExtentList(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverPrototypeExtentList {
        self.key = Some(key);
        self
    }
}

impl RenderObjectWidget for SliverPrototypeExtentList {
    type RenderObject = RenderSliverPrototypeExtentList;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let element = context
            .downcast::<SliverPrototypeExtentListElement>(app)
            .expect("a SliverPrototypeExtentList creates its render object from its own element");
        RenderSliverPrototypeExtentList::new(app, element).as_object()
    }
}

impl SliverWithKeepAliveWidget for SliverPrototypeExtentList {}

impl SliverMultiBoxAdaptorWidget for SliverPrototypeExtentList {
    fn delegate(&self) -> &SliverChildDelegateRef {
        &self.delegate
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        SliverPrototypeExtentListElement::new(app, this).as_element()
    }
}

/// Dart's `SliverPrototypeExtentList.builder(..)` arguments.
pub struct SliverPrototypeExtentListBuilder {
    key: Option<KeyRef>,
    delegate: SliverChildBuilderDelegate,
    prototype_item: WidgetRef,
}

impl SliverPrototypeExtentListBuilder {
    fn new(
        item_builder: NullableIndexedWidgetBuilder,
        prototype_item: WidgetRef,
    ) -> SliverPrototypeExtentListBuilder {
        SliverPrototypeExtentListBuilder {
            key: None,
            delegate: SliverChildBuilderDelegate::from_builder(item_builder),
            prototype_item,
        }
    }

    builder_constructor_setters!(SliverPrototypeExtentListBuilder);

    /// Dart `findChildIndexCallback:`.
    pub fn find_child_index_callback(
        mut self,
        value: ChildIndexGetter,
    ) -> SliverPrototypeExtentListBuilder {
        self.delegate = self.delegate.find_child_index_callback(value);
        self
    }

    /// The `SliverPrototypeExtentList` this constructor describes.
    pub fn build(self) -> SliverPrototypeExtentList {
        let mut list =
            SliverPrototypeExtentList::new(self.delegate.into_delegate(), self.prototype_item);
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverPrototypeExtentListBuilder {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// Dart's `SliverPrototypeExtentList.list(..)` arguments.
pub struct SliverPrototypeExtentListList {
    key: Option<KeyRef>,
    delegate: SliverChildListDelegate,
    prototype_item: WidgetRef,
}

impl SliverPrototypeExtentListList {
    fn new(children: Vec<WidgetRef>, prototype_item: WidgetRef) -> SliverPrototypeExtentListList {
        SliverPrototypeExtentListList {
            key: None,
            delegate: SliverChildListDelegate::new(children),
            prototype_item,
        }
    }

    list_constructor_setters!(SliverPrototypeExtentListList);

    /// The `SliverPrototypeExtentList` this constructor describes.
    pub fn build(self) -> SliverPrototypeExtentList {
        let mut list =
            SliverPrototypeExtentList::new(self.delegate.into_delegate(), self.prototype_item);
        if let Some(key) = self.key {
            list = list.key(key);
        }
        list
    }
}

impl IntoWidget<SliverMultiBoxAdaptorKind> for SliverPrototypeExtentListList {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

thread_local! {
    /// Dart's `static final Object _prototypeSlot = Object()`, the identity a `Slot::Custom`
    /// compares.
    static PROTOTYPE_SLOT: Rc<dyn Any> = Rc::new(());
}

/// Dart's `_SliverPrototypeExtentListElement`: builds the lazily built children and, besides
/// them, the prototype item in a slot of its own.
///
/// Dart's `didAdoptChild` override, which keeps the prototype from being given an index, has
/// nothing to do here: the render object adopts the prototype with `adopt_child`, which does not
/// tell the child manager (see `RenderSliverMultiBoxAdaptor::did_adopt_child`).
struct SliverPrototypeExtentListElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    adaptor: SliverMultiBoxAdaptorElementData,
    prototype: Option<AnyElement>,
}

impl SliverPrototypeExtentListElement {
    fn new(app: &mut App, widget: WidgetRef) -> Handle<SliverPrototypeExtentListElement> {
        debug_assert!(downcast_widget::<SliverPrototypeExtentList>(&*widget).is_some());
        app.create(SliverPrototypeExtentListElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            adaptor: SliverMultiBoxAdaptorElementData::new(false),
            prototype: None,
        })
        .into_tree_node(app)
    }

    /// Dart's `renderObject` override, cast to the prototype list.
    fn render_object(
        self: Handle<Self>,
        app: &App,
    ) -> RenderHandle<RenderSliverPrototypeExtentList> {
        self.typed_render_object(app)
    }

    fn prototype_slot() -> Slot {
        Slot::Custom(PROTOTYPE_SLOT.with(Rc::clone))
    }

    /// Dart's `_prototype = updateChild(_prototype, widget.prototypeItem, _prototypeSlot)`.
    fn update_prototype(self: Handle<Self>, app: &mut App) {
        let prototype_item = Self::widget_of(self.as_element().widget(app))
            .prototype_item
            .clone();
        let prototype = app.get(self).prototype;
        let prototype = SliverMultiBoxAdaptorElementBase::update_child(
            self,
            app,
            prototype,
            Some(prototype_item),
            Some(Self::prototype_slot()),
        );
        app.get_mut(self).prototype = prototype;
    }
}

impl SliverMultiBoxAdaptorElementBase for SliverPrototypeExtentListElement {
    fn adaptor_data(self: Handle<Self>, app: &App) -> &SliverMultiBoxAdaptorElementData {
        &app.get(self).adaptor
    }

    fn adaptor_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SliverMultiBoxAdaptorElementData {
        &mut app.get_mut(self).adaptor
    }
}

impl RenderObjectElementWidget for SliverPrototypeExtentListElement {
    type Widget = SliverPrototypeExtentList;

    fn widget_of(widget: &WidgetRef) -> &SliverPrototypeExtentList {
        downcast_widget::<SliverPrototypeExtentList>(&**widget)
            .expect("a _SliverPrototypeExtentListElement holds a SliverPrototypeExtentList")
    }
}

impl RenderObjectElement for SliverPrototypeExtentListElement {
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

impl Element for SliverPrototypeExtentListElement {
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

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        if slot == Some(Self::prototype_slot()) {
            debug_assert!(child.as_box().is_some());
            let child = child.as_box().expect("the prototype is a box");
            self.render_object(app).set_child(app, Some(child));
        } else {
            SliverMultiBoxAdaptorElementBase::insert_render_object_child(self, app, child, slot);
        }
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        if new_slot == Some(Self::prototype_slot()) {
            // There's only one prototype child so it cannot be moved.
            debug_assert!(false);
        } else {
            SliverMultiBoxAdaptorElementBase::move_render_object_child(
                self, app, child, old_slot, new_slot,
            );
        }
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let render_object = self.render_object(app);
        if render_object.child(app).map(AnyRenderBox::as_object) == Some(child) {
            render_object.set_child(app, None);
        } else {
            SliverMultiBoxAdaptorElementBase::remove_render_object_child(self, app, child, slot);
        }
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        if let Some(prototype) = app.get(self).prototype {
            visitor(prototype);
        }
        SliverMultiBoxAdaptorElementBase::visit_children(self, app, visitor);
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
        self.update_prototype(app);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        SliverMultiBoxAdaptorElementBase::update(self, app, new_widget.clone());
        debug_assert!(Rc::ptr_eq(self.as_element().widget(app), &new_widget));
        self.update_prototype(app);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        SliverMultiBoxAdaptorElementBase::perform_rebuild(self, app);
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

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        SliverMultiBoxAdaptorElementBase::forget_child(self, app, child);
    }

    fn debug_visit_onstage_children(
        self: Handle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyElement),
    ) {
        SliverMultiBoxAdaptorElementBase::debug_visit_onstage_children(self, app, visitor);
    }
}

/// Dart's `_RenderSliverPrototypeExtentList`: a fixed-extent list whose item extent is the
/// main-axis size of a prototype child, laid out before the rest and never painted or hit.
pub struct RenderSliverPrototypeExtentList {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
    fixed_extent: RenderSliverFixedExtentBoxAdaptorData,
    child: Option<AnyRenderBox>,
}

impl RenderSliverPrototypeExtentList {
    fn new(
        app: &mut App,
        child_manager: Handle<SliverPrototypeExtentListElement>,
    ) -> RenderHandle<RenderSliverPrototypeExtentList> {
        RenderHandle::new_sliver(
            app,
            RenderSliverPrototypeExtentList {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager.as_child_manager()),
                fixed_extent: RenderSliverFixedExtentBoxAdaptorData::new(),
                child: None,
            },
        )
    }

    /// The prototype child.
    pub fn child(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.get(app).child
    }

    /// Sets [`child`](Self::child).
    pub fn set_child(self: RenderHandle<Self>, app: &mut App, value: Option<AnyRenderBox>) {
        if let Some(child) = self.get(app).child {
            self.as_object().drop_child(app, child.as_object());
        }
        self.get_mut(app).child = value;
        if let Some(child) = value {
            self.as_object().adopt_child(app, child.as_object());
        }
        self.mark_needs_layout(app);
    }
}

impl RenderSliverFixedExtentBoxAdaptor for RenderSliverPrototypeExtentList {
    waft_rendering::render_sliver_fixed_extent_accessors!();

    fn item_extent(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        let child = self.child(app);
        debug_assert!(child.is_some_and(|child| child.has_size(app)));
        let size = child.expect("the prototype is laid out").size(app);
        Some(if self.constraints(app).axis() == Axis::Vertical {
            size.height()
        } else {
            size.width()
        })
    }
}

impl ContainerRenderObjectMixin for RenderSliverPrototypeExtentList {
    type ChildType = AnyRenderBox;
    type ParentDataType = SliverMultiBoxAdaptorParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }

    fn insert(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        after: Option<AnyRenderBox>,
    ) {
        RenderSliverMultiBoxAdaptor::insert(self, app, child, after)
    }

    fn move_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        after: Option<AnyRenderBox>,
    ) {
        RenderSliverMultiBoxAdaptor::move_child(self, app, child, after)
    }

    fn remove(self: RenderHandle<Self>, app: &mut App, child: AnyRenderBox) {
        RenderSliverMultiBoxAdaptor::remove(self, app, child)
    }

    fn remove_all(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverMultiBoxAdaptor::remove_all(self, app)
    }
}

impl RenderSliverHelpers for RenderSliverPrototypeExtentList {}

impl RenderSliverWithKeepAliveMixin for RenderSliverPrototypeExtentList {}

impl RenderSliverMultiBoxAdaptor for RenderSliverPrototypeExtentList {
    waft_rendering::render_sliver_multi_box_adaptor_accessors!();

    fn paint_extent_of(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> f64 {
        RenderSliverFixedExtentBoxAdaptor::paint_extent_of(self, app, child)
    }
}

impl RenderObject for RenderSliverPrototypeExtentList {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        self.child(app)
            .expect("the prototype is mounted with the list")
            .layout(
                app,
                constraints.as_box_constraints(0.0, f64::INFINITY, None),
                true,
            );
        RenderSliverFixedExtentBoxAdaptor::perform_layout(self, app);
    }

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderSliverMultiBoxAdaptor::setup_parent_data(self, app, child);
        RenderSliverWithKeepAliveMixin::setup_parent_data(self, app, child);
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        RenderSliverMultiBoxAdaptor::did_attach(self, app, owner);
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderSliverMultiBoxAdaptor::did_detach(self, app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        if let Some(child) = self.child(app) {
            self.as_object().redepth_child(app, child.as_object());
        }
        RenderSliverMultiBoxAdaptor::redepth_children(self, app);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
        RenderSliverMultiBoxAdaptor::visit_children(self, app, visitor);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderSliverMultiBoxAdaptor::apply_paint_transform(self, app, child, transform)
    }

    fn paints_child(self: RenderHandle<Self>, app: &App, child: AnyRenderObject) -> bool {
        RenderSliverMultiBoxAdaptor::paints_child(self, app, child)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderSliverMultiBoxAdaptor::paint(self, app, context, offset)
    }
}

impl RenderSliver for RenderSliverPrototypeExtentList {
    waft_rendering::render_sliver_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        RenderSliverMultiBoxAdaptor::hit_test_children(
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
        RenderSliverMultiBoxAdaptor::child_main_axis_position(self, app, child)
    }

    fn child_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> Option<f64> {
        RenderSliverMultiBoxAdaptor::child_scroll_offset(self, app, child)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Color, TextDirection};
    use waft_foundation::ValueKey;
    use waft_painting::AlignmentGeometry;

    use super::*;
    use crate::framework::{GlobalKey, StatelessWidget};
    use crate::testing::{
        WidgetTester, expect, find, finds_nothing, finds_one_widget, test_widgets,
    };
    use crate::widgets::app::WidgetsApp;
    use crate::widgets::basic::{Directionality, WidgetBuilder};
    use crate::widgets::container::Container;
    use crate::widgets::navigator::{RouteSettings, RouteSettingsRef};
    use crate::widgets::pages::{PageRouteBuilder, PageRouteRef};
    use crate::widgets::routes::RoutePageBuilder;
    use crate::widgets::scroll_view::CustomScrollView;
    use crate::widgets::text::Text;

    /// The test file's `TestItem`.
    #[derive(Debug)]
    struct TestItem {
        key: Option<KeyRef>,
        item: i32,
        width: Option<f64>,
        height: Option<f64>,
    }

    impl TestItem {
        fn new(item: i32) -> TestItem {
            TestItem {
                key: None,
                item,
                width: None,
                height: None,
            }
        }

        fn key(mut self, key: Option<KeyRef>) -> TestItem {
            self.key = key;
            self
        }

        fn width(mut self, width: Option<f64>) -> TestItem {
            self.width = width;
            self
        }

        fn height(mut self, height: Option<f64>) -> TestItem {
            self.height = height;
            self
        }
    }

    impl StatelessWidget for TestItem {
        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            let mut container = Container::new()
                .alignment(AlignmentGeometry::CENTER)
                .child(Text::new(format!("Item {}", self.item)).text_direction(TextDirection::Ltr));
            if let Some(width) = self.width {
                container = container.width(width);
            }
            if let Some(height) = self.height {
                container = container.height(height);
            }
            container.into_widget()
        }
    }

    /// The test file's `buildFrame`.
    fn build_frame(
        count: Option<i32>,
        width: Option<f64>,
        height: Option<f64>,
        scroll_direction: Option<Axis>,
        prototype_key: Option<KeyRef>,
    ) -> WidgetRef {
        let mut delegate = SliverChildBuilderDelegate::new(|_app, _context, index| {
            Some(TestItem::new(index).into_widget())
        });
        if let Some(count) = count {
            delegate = delegate.child_count(count);
        }
        Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new()
                .scroll_direction(scroll_direction.unwrap_or(Axis::Vertical))
                .slivers([SliverPrototypeExtentList::new(
                    delegate.into_delegate(),
                    TestItem::new(-1)
                        .width(width)
                        .height(height)
                        .key(prototype_key),
                )
                .into_widget()]),
        )
        .into_widget()
    }

    /// flutter_test's `TestWidgetsApp._defaultPageRouteBuilder`: a page with no transition.
    fn page_route(app: &mut App, settings: &RouteSettings, builder: WidgetBuilder) -> PageRouteRef {
        let page_builder: RoutePageBuilder =
            Rc::new(move |app, context, _animation, _secondary| builder(app, context));
        let route = PageRouteBuilder::new(app, page_builder)
            .settings(app, RouteSettingsRef::Settings(settings.clone()));
        route.into_erased()
    }

    /// flutter_test's `TestWidgetsApp(home: home)`.
    fn test_widgets_app<K>(home: impl IntoWidget<K>) -> WidgetRef {
        WidgetsApp::new(Color::new(0xFFFFFFFF))
            .page_route_builder(page_route)
            .home(home)
            .into_widget()
    }

    fn item_builder(_app: &mut App, _context: BuildContext, index: i32) -> Option<WidgetRef> {
        Some(TestItem::new(index).into_widget())
    }

    /// The case's check that items `0..count` lie along the main axis at `extent` apart, each
    /// `extent` long.
    fn expect_items_along(tester: &WidgetTester, count: i32, axis: Axis, extent: f64) {
        for i in 0..count {
            let item = find::widget_with_text::<Container>(&format!("Item {i}"));
            expect(tester, &item, finds_one_widget());
            let top_left = tester.get_top_left(&item);
            let size = tester.get_size(&item);
            match axis {
                Axis::Vertical => {
                    assert_eq!(top_left.dy(), f64::from(i) * extent);
                    assert_eq!(size.height(), extent);
                }
                Axis::Horizontal => {
                    assert_eq!(top_left.dx(), f64::from(i) * extent);
                    assert_eq!(size.width(), extent);
                }
            }
        }
    }

    fn expect_texts(tester: &WidgetTester, items: impl IntoIterator<Item = i32>, found: bool) {
        for i in items {
            let matcher = if found {
                finds_one_widget()
            } else {
                finds_nothing()
            };
            expect(tester, &find::text(&format!("Item {i}")), matcher);
        }
    }

    #[test]
    fn the_builder_constructor_sizes_twenty_items_to_the_prototype() {
        test_widgets(|tester| {
            tester.pump_widget(test_widgets_app(
                CustomScrollView::new().slivers([SliverPrototypeExtentList::builder(
                    item_builder,
                    TestItem::new(-1).height(Some(100.0)),
                )
                .item_count(20)
                .into_widget()]),
            ));

            // The viewport is 600 pixels high, lazily created items are 100 pixels high.
            expect_items_along(tester, 6, Axis::Vertical, 100.0);
            expect_texts(tester, 7..20, false);
        });
    }

    #[test]
    fn the_builder_constructor_sizes_eight_items_to_the_prototype() {
        test_widgets(|tester| {
            tester.pump_widget(test_widgets_app(
                CustomScrollView::new().slivers([SliverPrototypeExtentList::builder(
                    item_builder,
                    TestItem::new(-1).height(Some(100.0)),
                )
                .item_count(8)
                .into_widget()]),
            ));

            // The viewport is 600 pixels high, lazily created items are 100 pixels high.
            expect_items_along(tester, 6, Axis::Vertical, 100.0);
            expect(tester, &find::text("Item 7"), finds_nothing());
        });
    }

    #[test]
    fn a_vertical_prototype_extent_list_scrolls_to_its_end() {
        test_widgets(|tester| {
            tester.pump_widget(build_frame(Some(20), None, Some(100.0), None, None));

            // The viewport is 600 pixels high, lazily created items are 100 pixels high.
            expect_items_along(tester, 6, Axis::Vertical, 100.0);
            expect_texts(tester, 7..20, false);

            // Fling scroll to the end.
            tester.fling(find::text("Item 2"), Offset::new(0.0, -200.0), 5000.0);
            tester.pump_and_settle();

            expect_texts(tester, (14..=19).rev(), true);
            expect_texts(tester, (0..=13).rev(), false);
        });
    }

    #[test]
    fn a_horizontal_prototype_extent_list_scrolls_to_its_end() {
        test_widgets(|tester| {
            tester.pump_widget(build_frame(
                Some(20),
                Some(100.0),
                None,
                Some(Axis::Horizontal),
                None,
            ));

            // The viewport is 800 pixels wide, lazily created items are 100 pixels wide.
            expect_items_along(tester, 8, Axis::Horizontal, 100.0);
            expect_texts(tester, 9..20, false);

            // Fling scroll to the end.
            tester.fling(find::text("Item 3"), Offset::new(-200.0, 0.0), 5000.0);
            tester.pump_and_settle();

            expect_texts(tester, (12..=19).rev(), true);
            expect_texts(tester, (0..=11).rev(), false);
        });
    }

    #[test]
    fn changing_the_prototype_item_changes_the_item_extent() {
        test_widgets(|tester| {
            tester.pump_widget(build_frame(Some(10), None, Some(60.0), None, None));

            // The viewport is 600 pixels high, each of the 10 items is 60 pixels high
            expect_texts(tester, 0..10, true);

            tester.pump_widget(build_frame(Some(10), None, Some(120.0), None, None));

            // Now the items are 120 pixels high, so only 5 fit.
            expect_texts(tester, 0..5, true);
            expect_texts(tester, 5..10, false);

            tester.pump_widget(build_frame(Some(10), None, Some(60.0), None, None));

            // Now they all fit again
            expect_texts(tester, 0..10, true);
        });
    }

    #[test]
    fn the_first_item_can_also_be_the_prototype() {
        test_widgets(|tester| {
            let items: Rc<Vec<WidgetRef>> = Rc::new(
                (0..10)
                    .map(|index| {
                        TestItem::new(index)
                            .key(Some(ValueKey::new(index).into()))
                            .height((index == 0).then_some(60.0))
                            .into_widget()
                    })
                    .collect(),
            );

            let delegate_items = Rc::clone(&items);
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                CustomScrollView::new().slivers([SliverPrototypeExtentList::new(
                    SliverChildBuilderDelegate::new(move |_app, _context, index| {
                        Some(Rc::clone(&delegate_items[index as usize]))
                    })
                    .child_count(10)
                    .into_delegate(),
                    Rc::clone(&items[0]),
                )
                .into_widget()]),
            ));

            // Item 0 exists in the list and as the prototype item.
            assert_eq!(
                find::text("Item 0")
                    .skip_offstage(false)
                    .evaluate(tester)
                    .len(),
                2
            );

            expect_texts(tester, 1..10, true);
        });
    }

    /// Regression test for https://github.com/flutter/flutter/issues/67117: the paint transform
    /// of the prototype item, which is not visible, is zero rather than an assertion error.
    #[test]
    fn the_prototype_items_paint_transform_is_zero() {
        test_widgets(|tester| {
            let prototype_key = GlobalKey::new();
            tester.pump_widget(build_frame(
                Some(20),
                None,
                Some(100.0),
                None,
                Some(prototype_key.clone().into()),
            ));

            let scroll_view = tester.render_object(find::by_type::<CustomScrollView>());
            let app = tester.cell.borrow();
            let prototype = prototype_key
                .current_context(&app)
                .and_then(|context| context.find_render_object(&app))
                .expect("the prototype is mounted");

            // vector_math's `Matrix4.zero()`.
            let zero = Matrix4::from_flutter_array(&[0.0; 16]);
            assert_eq!(prototype.get_transform_to(&app, Some(scroll_view)), zero);
        });
    }
}
