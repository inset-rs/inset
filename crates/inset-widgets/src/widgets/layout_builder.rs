//! Flutter counterpart: `widgets/layout_builder.dart`.
//!
//! Dart's `AbstractLayoutBuilder<LayoutInfoType>` / `ConstrainedLayoutBuilder<ConstraintType>`
//! generics collapse into [`LayoutBuilder`] over `BoxConstraints`; `SliverLayoutBuilder`
//! waits with slivers and brings the abstraction back with it.

use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

use inset_embedder::{Offset, Size, TextBaseline};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, DryLayoutFailure,
    PaintingContext, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderObjectWithLayoutCallbackData,
    RenderObjectWithLayoutCallbackMixin, debug_checking_intrinsics,
};
use inset_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};

use crate::framework::{
    AnyElement, BuildContext, BuildScope, BuildScopeCallback, Element, ElementData, KeyRef,
    RenderObjectElement, RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget,
    Slot, Widget, WidgetKind, WidgetRef, downcast_widget,
};

/// The signature of the [`LayoutBuilder`] builder function.
pub type LayoutWidgetBuilder = Rc<dyn Fn(&mut App, BuildContext, BoxConstraints) -> WidgetRef>;

/// Builds a widget tree that can depend on the parent widget's size.
///
/// Similar to the `Builder` widget except that the framework calls the
/// [`builder`](Self::builder) function at layout time and provides the parent widget's
/// constraints. This is useful when the parent constrains the child's size and doesn't
/// depend on the child's intrinsic size. The [`LayoutBuilder`]'s final size will match its
/// child's size.
///
/// The [`builder`](Self::builder) function is called in the following situations:
///
/// * The first time the widget is laid out.
/// * When the parent widget passes different layout constraints.
/// * When the parent widget updates this widget.
/// * When the dependencies that the [`builder`](Self::builder) function subscribes to change.
///
/// The [`builder`](Self::builder) function is _not_ called during layout if the parent
/// passes the same constraints repeatedly.
///
/// If the child should be smaller than the parent, consider wrapping the child in an `Align`
/// widget. If the child might want to be bigger, consider wrapping it in a
/// `SingleChildScrollView` or `OverflowBox`.
///
/// See also:
///
///  * `SliverLayoutBuilder`, the sliver counterpart of this widget.
///  * `Builder`, which calls a `build` function at build time.
///  * `StatefulBuilder`, which passes its `build` function a `setState` callback.
///  * `CustomSingleChildLayout`, which positions its child during layout.
pub struct LayoutBuilder {
    pub key: Option<KeyRef>,
    /// Called at layout time to construct the widget tree.
    ///
    /// The builder must not return null.
    pub builder: LayoutWidgetBuilder,
}

impl LayoutBuilder {
    /// Creates a widget that defers its building until layout.
    pub fn new(
        builder: impl Fn(&mut App, BuildContext, BoxConstraints) -> WidgetRef + 'static,
    ) -> LayoutBuilder {
        LayoutBuilder {
            key: None,
            builder: Rc::new(builder),
        }
    }

    /// Dart `LayoutBuilder(key:)`.
    pub fn key(mut self, key: KeyRef) -> LayoutBuilder {
        self.key = Some(key);
        self
    }

    /// The tree node: a render object widget with its own element, outside the `IntoWidget`
    /// kinds.
    pub fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }

    /// Whether [`builder`](Self::builder) needs to be called again even if the layout
    /// constraints are the same.
    ///
    /// When this widget's configuration is updated, the [`builder`](Self::builder) callback
    /// most likely needs to be called to build this widget's child. However, subclasses may
    /// provide ways in which the widget can be updated without needing to rebuild the child.
    /// Such subclasses can use this method to tell the framework when the child widget
    /// should be rebuilt.
    ///
    /// When this method is called by the framework, the newly configured widget is asked if
    /// it requires a rebuild, and it is passed the old widget as a parameter.
    pub fn update_should_rebuild(&self, old_widget: &LayoutBuilder) -> bool {
        let _ = old_widget;
        true
    }
}

impl fmt::Debug for LayoutBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutBuilder").finish_non_exhaustive()
    }
}

impl Widget for LayoutBuilder {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        LayoutBuilderElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> TypeId {
        TypeId::of::<LayoutBuilder>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl RenderObjectWidget for LayoutBuilder {
    type RenderObject = RenderLayoutBuilder;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderLayoutBuilder::new(app).as_object()
    }

    // updateRenderObject is redundant with the logic in the LayoutBuilderElement below.
    fn update_render_object(
        &self,
        _app: &mut App,
        _context: BuildContext,
        _render_object: RenderHandle<RenderLayoutBuilder>,
    ) {
    }
}

/// Dart's `_LayoutBuilderElement`: builds its child from within the render object's layout.
pub struct LayoutBuilderElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    child: Option<AnyElement>,
    /// Dart's `late final _buildScope`, created right after the element so its
    /// `scheduleRebuild` can name it.
    build_scope: Option<Handle<BuildScope>>,
    // To schedule a rebuild, markNeedsLayout needs to be called on this Element's
    // render object (as the rebuilding is done in its performLayout call). However,
    // the render tree should typically be kept clean during the postFrameCallbacks
    // and the idle phase, so the layout data can be safely read.
    deferred_callback_scheduled: bool,
    // The constraints that were used to invoke the layout callback with last time,
    // during layout. The `previous_layout_info` value is compared to the new one
    // to determine whether the builder needs to be called.
    previous_layout_info: Option<BoxConstraints>,
    needs_build: bool,
}

impl LayoutBuilderElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<LayoutBuilderElement> {
        debug_assert!(downcast_widget::<LayoutBuilder>(&*widget).is_some());
        let this = app.create(LayoutBuilderElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            child: None,
            build_scope: None,
            deferred_callback_scheduled: false,
            previous_layout_info: None,
            needs_build: true,
        });
        let build_scope = BuildScope::new(
            app,
            Some(Listener::handle_method(this, Self::schedule_rebuild)),
        );
        app.get_mut(this).build_scope = Some(build_scope);
        this
    }

    fn render_layout_builder(self: Handle<Self>, app: &App) -> RenderHandle<RenderLayoutBuilder> {
        self.typed_render_object(app)
    }

    fn schedule_rebuild(self: Handle<Self>, app: &mut App) {
        if app.get(self).deferred_callback_scheduled {
            return;
        }
        let defer_mark_needs_layout = match SchedulerBinding::scheduler_phase(app) {
            SchedulerPhase::Idle | SchedulerPhase::PostFrameCallbacks => true,
            SchedulerPhase::TransientCallbacks
            | SchedulerPhase::MidFrameMicrotasks
            | SchedulerPhase::PersistentCallbacks => false,
        };
        if !defer_mark_needs_layout {
            self.render_layout_builder(app)
                .schedule_layout_callback(app);
            return;
        }
        app.get_mut(self).deferred_callback_scheduled = true;
        SchedulerBinding::schedule_frame_callback(
            app,
            FrameCallback::new(move |app, _time_stamp| self.frame_callback(app)),
            false,
            true,
        );
    }

    fn frame_callback(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).deferred_callback_scheduled = false;
        // This method is only called when the render tree is stable, if the Element
        // is deactivated it will never be reincorporated back to the tree.
        if self.as_element().mounted(app) {
            self.render_layout_builder(app)
                .schedule_layout_callback(app);
        }
    }

    fn rebuild_with_constraints(self: Handle<Self>, app: &mut App) {
        let render_object = self.render_layout_builder(app);
        let layout_info = render_object.layout_info(app);
        let this = self.as_element();
        let needs_callback =
            app.get(self).needs_build || Some(layout_info) != app.get(self).previous_layout_info;
        let callback: Option<BuildScopeCallback> = needs_callback.then(|| {
            Box::new(move |app: &mut App| {
                debug_assert!(layout_info == render_object.layout_info(app));
                let builder = Self::widget_of(this.widget(app)).builder.clone();
                let built = builder(app, this, layout_info);
                // A panic in the builder unwinds; there is no `ErrorWidget`.
                let child = app.get(self).child;
                let child = this.update_child(app, child, Some(built), None);
                debug_assert!(child.is_some());
                let state = app.get_mut(self);
                state.child = child;
                state.needs_build = false;
                state.previous_layout_info = Some(layout_info);
            }) as BuildScopeCallback
        });
        let owner = this.owner(app).expect("a laying-out element has an owner");
        owner.build_scope(app, this, callback);
    }
}

impl RenderObjectElementWidget for LayoutBuilderElement {
    type Widget = LayoutBuilder;

    fn widget_of(widget: &WidgetRef) -> &LayoutBuilder {
        downcast_widget::<LayoutBuilder>(&**widget)
            .expect("a LayoutBuilderElement holds its widget")
    }
}

impl RenderObjectElement for LayoutBuilderElement {
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

impl Element for LayoutBuilderElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn build_scope(self: Handle<Self>, app: &App) -> Handle<BuildScope> {
        app.get(self).build_scope.expect("created with the element")
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        if let Some(child) = app.get(self).child {
            visitor(child);
        }
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(app.get(self).child == Some(child));
        app.get_mut(self).child = None;
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot); // Creates the renderObject.
        let render_object = self.render_layout_builder(app);
        render_object.update_callback(
            app,
            Listener::handle_method(self, Self::rebuild_with_constraints),
        );
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        debug_assert!(!Rc::ptr_eq(&old_widget, &new_widget));
        RenderObjectElement::update(self, app, new_widget.clone());
        let render_object = self.render_layout_builder(app);
        render_object.update_callback(
            app,
            Listener::handle_method(self, Self::rebuild_with_constraints),
        );
        if Self::widget_of(&new_widget).update_should_rebuild(Self::widget_of(&old_widget)) {
            app.get_mut(self).needs_build = true;
            render_object.schedule_layout_callback(app);
        }
    }

    fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        // Calling super.markNeedsBuild is not needed. This Element does not need
        // to performRebuild since this call already does what performRebuild does,
        // So the element is clean as soon as this method returns and does not have
        // to be added to the dirty list or marked as dirty.
        self.render_layout_builder(app)
            .schedule_layout_callback(app);
        app.get_mut(self).needs_build = true;
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        // This gets called if markNeedsBuild() is called on us.
        // That might happen if, e.g., our builder uses Inherited widgets.
        // Force the callback to be called, even if the layout constraints are the
        // same. This is because that callback may depend on the updated widget
        // configuration, or an inherited widget.
        self.render_layout_builder(app)
            .schedule_layout_callback(app);
        app.get_mut(self).needs_build = true;
        RenderObjectElement::perform_rebuild(self, app); // Calls widget.updateRenderObject (a no-op in this case).
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        let render_object = self.render_layout_builder(app);
        render_object.set_callback(app, None);
        let build_scope = app.get_mut(self).build_scope.take();
        RenderObjectElement::unmount(self, app);
        if let Some(build_scope) = build_scope {
            app.destroy(build_scope);
        }
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
        debug_assert!(slot.is_none());
        let render_object = self.render_layout_builder(app);
        render_object.set_child(app, Some(child.as_box().expect("a box child")));
    }

    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        debug_assert!(false, "a layout builder has one child");
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        let render_object = self.render_layout_builder(app);
        debug_assert!(render_object.child(app).map(|current| current.as_object()) == Some(child));
        render_object.set_child(app, None);
    }
}

/// Generic mixin for `RenderObject`s created by `AbstractLayoutBuilder`s, which run a
/// builder callback during layout (Dart's `RenderAbstractLayoutBuilderMixin` /
/// `RenderConstrainedLayoutBuilder`).
///
/// The callback is `LayoutBuilderElement::rebuild_with_constraints`; it reads the constraints
/// from the render object rather than receiving them.
pub trait RenderAbstractLayoutBuilderMixin: RenderObjectWithLayoutCallbackMixin {
    /// The layout callback, if set.
    fn callback(self: RenderHandle<Self>, app: &App) -> Option<Listener>;

    /// Sets (or clears) the layout callback without scheduling it.
    fn set_callback(self: RenderHandle<Self>, app: &mut App, callback: Option<Listener>);

    /// Change the layout callback and schedule it.
    fn update_callback(self: RenderHandle<Self>, app: &mut App, value: Listener) {
        if self.callback(app).as_ref() == Some(&value) {
            return;
        }
        self.set_callback(app, Some(value));
        self.schedule_layout_callback(app);
    }

    /// The information the builder is given: this box's constraints.
    fn layout_info(self: RenderHandle<Self>, app: &App) -> BoxConstraints {
        self.constraints(app)
    }
}

/// Dart's `_RenderLayoutBuilder`: a box that runs its layout callback, then sizes to its
/// child (or fills its constraints without one).
pub struct RenderLayoutBuilder {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    layout_callback: RenderObjectWithLayoutCallbackData,
    callback: Option<Listener>,
}

impl RenderLayoutBuilder {
    /// Creates the render object with no callback and no child.
    pub fn new(app: &mut App) -> RenderHandle<RenderLayoutBuilder> {
        RenderHandle::new_box(
            app,
            RenderLayoutBuilder {
                render_object: RenderObjectData::default(),
                render_box: RenderBoxData::default(),
                child: RenderObjectWithChildData::default(),
                layout_callback: RenderObjectWithLayoutCallbackData::default(),
                callback: None,
            },
        )
    }
}

impl RenderObjectWithChildMixin for RenderLayoutBuilder {
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

impl RenderObjectWithLayoutCallbackMixin for RenderLayoutBuilder {
    fn layout_callback_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderObjectWithLayoutCallbackData {
        &self.get(app).layout_callback
    }

    fn layout_callback_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithLayoutCallbackData {
        &mut self.get_mut(app).layout_callback
    }

    fn layout_callback(self: RenderHandle<Self>, app: &mut App) {
        let callback = self
            .get(app)
            .callback
            .clone()
            .expect("a callback is set before layout");
        callback.call(app);
    }
}

impl RenderAbstractLayoutBuilderMixin for RenderLayoutBuilder {
    fn callback(self: RenderHandle<Self>, app: &App) -> Option<Listener> {
        self.get(app).callback.clone()
    }

    fn set_callback(self: RenderHandle<Self>, app: &mut App, callback: Option<Listener>) {
        self.get_mut(app).callback = callback;
    }
}

impl RenderObject for RenderLayoutBuilder {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        self.run_layout_callback(app);
        if let Some(child) = self.child(app) {
            child.layout(app, constraints, true);
            let size = constraints.constrain(child.size(app));
            self.set_size(app, size);
        } else {
            self.set_size(app, constraints.biggest());
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            context.paint_child(app, child.as_object(), offset);
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
}

impl RenderLayoutBuilder {
    fn debug_throw_if_not_checking_intrinsics(self: RenderHandle<Self>) -> bool {
        let _ = self;
        debug_assert!(
            debug_checking_intrinsics(),
            "LayoutBuilder does not support returning intrinsic dimensions.\n\
             Calculating the intrinsic dimensions would require running the layout callback \
             speculatively, which might mutate the live render object tree."
        );
        true
    }
}

impl RenderBox for RenderLayoutBuilder {
    inset_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, _app: &mut App, _height: f64) -> f64 {
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, _app: &mut App, _height: f64) -> f64 {
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, _app: &mut App, _width: f64) -> f64 {
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, _app: &mut App, _width: f64) -> f64 {
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        _constraints: BoxConstraints,
    ) -> Size {
        self.debug_cannot_compute_dry_layout(DryLayoutFailure::Reason(
            "Calculating the dry layout would require running the layout callback \
             speculatively, which might mutate the live render object tree.",
        ));
        Size::ZERO
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        _app: &mut App,
        _constraints: BoxConstraints,
        _baseline: TextBaseline,
    ) -> Option<f64> {
        self.debug_cannot_compute_dry_layout(DryLayoutFailure::Reason(
            "Calculating the dry baseline would require running the layout callback \
             speculatively, which might mutate the live render object tree.",
        ));
        None
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        self.child(app)
            .is_some_and(|child| child.hit_test(app, result, position))
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;

    use inset_embedder::{Size, TextDirection};

    use super::*;
    use crate::framework::IntoWidget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Directionality, SizedBox};

    type Seen = Rc<RefCell<Vec<BoxConstraints>>>;

    fn builder(seen: &Seen) -> WidgetRef {
        let seen = Rc::clone(seen);
        LayoutBuilder::new(move |_app, _context, constraints| {
            seen.borrow_mut().push(constraints);
            SizedBox::square(Some(10.0)).into_widget()
        })
        .into_widget()
    }

    fn sized(width: f64, child: WidgetRef) -> WidgetRef {
        SizedBox::new()
            .width(width)
            .height(50.0)
            .child(child)
            .into_widget()
    }

    #[test]
    fn the_builder_runs_at_layout_with_the_constraints_and_only_when_they_change() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen: Seen = Rc::default();
        let harness = Harness::mount(&mut app, sized(100.0, builder(&seen)));
        harness.pump(&mut app);
        assert_eq!(
            *seen.borrow(),
            vec![BoxConstraints::tight(Size::new(100.0, 50.0))]
        );

        harness.pump(&mut app);
        assert_eq!(seen.borrow().len(), 1);

        harness.set_child(&mut app, sized(80.0, builder(&seen)));
        harness.pump(&mut app);
        assert_eq!(seen.borrow().len(), 2);
        assert_eq!(
            seen.borrow()[1],
            BoxConstraints::tight(Size::new(80.0, 50.0))
        );
    }

    #[test]
    fn an_inherited_dependency_change_rebuilds_the_builder_during_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let dependent = {
            let seen = Rc::clone(&seen);
            LayoutBuilder::new(move |app, context, _constraints| {
                seen.borrow_mut().push(Directionality::of(app, context));
                SizedBox::square(Some(10.0)).into_widget()
            })
            .into_widget()
        };
        let under = |direction: TextDirection, child: WidgetRef| {
            Directionality::new(direction, child).into_widget()
        };
        let harness = Harness::mount(&mut app, under(TextDirection::Ltr, dependent.clone()));
        harness.pump(&mut app);
        harness.set_child(&mut app, under(TextDirection::Rtl, dependent));
        harness.pump(&mut app);
        assert_eq!(*seen.borrow(), vec![TextDirection::Ltr, TextDirection::Rtl]);
    }
}
