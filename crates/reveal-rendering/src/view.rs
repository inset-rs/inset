//! Flutter counterpart: `rendering/view.dart` (`ViewConfiguration`, `RenderView`).
//!
//! `applyPaintTransform`, `updateSystemChrome`, and semantics wait.

use reveal_embedder::{Canvas, Matrix4, Offset, Rect, Size, View, ViewRef};
use reveal_foundation::App;
use reveal_gestures::{HitTestEntry, HitTestResult, HitTestTarget, PointerEvent};

use crate::box_::{AnyRenderBox, BoxConstraints, BoxHitTestResult};
use crate::layer::CompositedLayer;
use crate::object::{
    AnyRenderObject, Constraints, RenderHandle, RenderObject, RenderObjectData, RenderObjectVTable,
    resolve,
};
use crate::painting_context::PaintingContext;

/// The layout constraints for the root render object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewConfiguration {
    /// The constraints of the output surface in logical pixels.
    pub logical_constraints: BoxConstraints,
    /// The constraints of the output surface in physical pixels.
    pub physical_constraints: BoxConstraints,
    /// The pixel density of the output surface.
    pub device_pixel_ratio: f64,
}

impl ViewConfiguration {
    /// Creates a view configuration.
    ///
    /// By default, the view has zero constraints and a pixel ratio of 1.0.
    pub const fn new(
        physical_constraints: BoxConstraints,
        logical_constraints: BoxConstraints,
        device_pixel_ratio: f64,
    ) -> ViewConfiguration {
        ViewConfiguration {
            logical_constraints,
            physical_constraints,
            device_pixel_ratio,
        }
    }

    /// Creates a view configuration that matches `view`.
    pub fn from_view(view: &dyn View) -> ViewConfiguration {
        let metrics = view.metrics();
        let physical_constraints =
            BoxConstraints::from_view_constraints(metrics.physical_constraints);
        let device_pixel_ratio = metrics.device_pixel_ratio;
        ViewConfiguration {
            logical_constraints: physical_constraints / device_pixel_ratio,
            physical_constraints,
            device_pixel_ratio,
        }
    }

    /// Creates a transformation matrix that applies the
    /// [`device_pixel_ratio`](Self::device_pixel_ratio).
    pub fn to_matrix(&self) -> Matrix4 {
        Matrix4::scale(
            self.device_pixel_ratio as f32,
            self.device_pixel_ratio as f32,
        )
    }

    /// Returns whether [`to_matrix`](Self::to_matrix) would return a different value for
    /// this configuration than it would for the given `old_configuration`.
    pub fn should_update_matrix(&self, old_configuration: &ViewConfiguration) -> bool {
        old_configuration.device_pixel_ratio != self.device_pixel_ratio
    }

    /// Transforms the provided `logical_size` to physical pixels.
    pub fn to_physical_size(&self, logical_size: Size) -> Size {
        self.physical_constraints
            .constrain(logical_size * self.device_pixel_ratio)
    }
}

impl Default for ViewConfiguration {
    fn default() -> ViewConfiguration {
        ViewConfiguration::new(
            BoxConstraints::new().max_width(0.0).max_height(0.0),
            BoxConstraints::new().max_width(0.0).max_height(0.0),
            1.0,
        )
    }
}

/// The root of the render tree.
///
/// The view represents the total output surface of the render tree and handles
/// bootstrapping the rendering pipeline. The view has a unique child
/// [`crate::RenderBox`], which is required to fill the entire output surface.
///
/// Not a box: its constraints come from its [`configuration`](Self::configuration), and its
/// child is the one slot Flutter's `RenderObjectWithChildMixin` gives it.
pub struct RenderView {
    render_object: RenderObjectData,
    child: Option<AnyRenderBox>,
    size: Size,
    configuration: Option<ViewConfiguration>,
    view: ViewRef,
    root_transform: Option<Matrix4>,
}

impl RenderView {
    /// Creates the root of the render tree.
    ///
    /// Typically created by the binding (e.g., `RendererBinding`).
    ///
    /// Providing a `configuration` is optional, but a configuration must be set
    /// before calling [`prepare_initial_frame`](Self::prepare_initial_frame).
    pub fn new(
        app: &mut App,
        child: Option<AnyRenderBox>,
        configuration: Option<ViewConfiguration>,
        view: ViewRef,
    ) -> RenderHandle<RenderView> {
        let this = RenderHandle::from_handle(app.create(RenderView {
            render_object: RenderObjectData::new(),
            child: None,
            size: Size::ZERO,
            configuration: None,
            view,
            root_transform: None,
        }));
        this.render_object_data_mut(app).was_repaint_boundary = true;
        if let Some(configuration) = configuration {
            this.set_configuration(app, configuration);
        }
        this.set_child(app, child);
        this
    }

    /// The erased `RenderObject` edge.
    pub fn as_object(self: RenderHandle<Self>) -> AnyRenderObject {
        AnyRenderObject::from_vtable(self.id(), const { &VTABLE })
    }

    /// The render object's unique child. Flutter's `RenderObjectWithChildMixin.child`.
    pub fn child(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.get(app).child
    }

    /// Sets the unique child, adopting or dropping as Flutter's setter does.
    pub fn set_child(self: RenderHandle<Self>, app: &mut App, value: Option<AnyRenderBox>) {
        if let Some(old) = self.get(app).child {
            self.as_object().drop_child(app, old.as_object());
        }
        self.get_mut(app).child = value;
        if let Some(new) = value {
            self.as_object().adopt_child(app, new.as_object());
        }
    }

    /// The current layout size of the view.
    pub fn size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).size
    }

    /// The constraints used for the root layout.
    ///
    /// Typically, this configuration is set by the `RendererBinding`, when the
    /// [`RenderView`] is registered with it. It will also update the configuration
    /// if necessary. Therefore, if used in conjunction with the `RendererBinding`
    /// this property must not be set manually as the `RendererBinding` will just
    /// override it.
    ///
    /// # Panics
    ///
    /// If no configuration has been set.
    pub fn configuration(self: RenderHandle<Self>, app: &App) -> ViewConfiguration {
        self.get(app)
            .configuration
            .expect("RenderView has not been given a configuration yet")
    }

    /// Sets [`configuration`](Self::configuration).
    pub fn set_configuration(self: RenderHandle<Self>, app: &mut App, value: ViewConfiguration) {
        if self.get(app).configuration == Some(value) {
            return;
        }
        let old_configuration = self.get(app).configuration;
        self.get_mut(app).configuration = Some(value);
        if self.get(app).root_transform.is_none() {
            // [prepare_initial_frame] has not been called yet, nothing more to do for now.
            return;
        }
        if old_configuration.is_none_or(|old| value.should_update_matrix(&old)) {
            let root_layer = self.update_matrices_and_create_new_root_layer(app);
            self.as_object().replace_root_layer(app, root_layer);
        }
        debug_assert!(self.get(app).root_transform.is_some());
        self.as_object().mark_needs_layout(app);
    }

    /// Whether a [`configuration`](Self::configuration) has been set.
    pub fn has_configuration(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).configuration.is_some()
    }

    /// The constraints most recently supplied for layout: the configuration's logical
    /// constraints.
    ///
    /// # Panics
    ///
    /// If no configuration has been set.
    pub fn constraints(self: RenderHandle<Self>, app: &App) -> BoxConstraints {
        debug_assert!(
            self.has_configuration(app),
            "Constraints are not available because RenderView has not been given a configuration yet."
        );
        self.configuration(app).logical_constraints
    }

    /// The host view into which this render tree is rendered.
    pub fn flutter_view(self: RenderHandle<Self>, app: &App) -> ViewRef {
        self.get(app).view.clone()
    }

    /// Bootstrap the rendering pipeline by preparing the first frame.
    ///
    /// This should only be called once, and must be called before changing
    /// [`configuration`](Self::configuration). It is typically called immediately after calling
    /// the constructor.
    ///
    /// This does not actually schedule the first frame. Call
    /// `SchedulerBinding::ensure_visual_update` for that.
    pub fn prepare_initial_frame(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(
            self.as_object().owner(app).is_some(),
            "attach the RenderView to a PipelineOwner before calling prepare_initial_frame"
        );
        debug_assert!(
            self.get(app).root_transform.is_none(),
            "prepare_initial_frame must only be called once"
        );
        debug_assert!(
            self.has_configuration(app),
            "set a configuration before calling prepare_initial_frame"
        );
        self.as_object().schedule_initial_layout(app);
        let root_layer = self.update_matrices_and_create_new_root_layer(app);
        self.as_object().schedule_initial_paint(app, root_layer);
        debug_assert!(self.get(app).root_transform.is_some());
    }

    fn update_matrices_and_create_new_root_layer(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> CompositedLayer {
        debug_assert!(self.has_configuration(app));
        let root_transform = self.configuration(app).to_matrix();
        self.get_mut(app).root_transform = Some(root_transform);
        CompositedLayer::transform_layer(root_transform, Offset::ZERO)
    }

    /// Determines the set of render objects located at the given position.
    ///
    /// Returns true if the given point is contained in this render object or one of its
    /// descendants. Adds any render objects that contain the point to the given hit test result.
    ///
    /// The `position` argument is in the coordinate system of the render view, which is to say,
    /// in logical pixels. This is not necessarily the same coordinate system as that expected by
    /// the root layer (which is in physical pixels).
    pub fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut HitTestResult,
        position: Offset,
    ) -> bool {
        if let Some(child) = self.child(app) {
            child.hit_test(app, &mut BoxHitTestResult::wrap(result), position);
        }
        result.add(HitTestEntry::new(self));
        true
    }

    /// Uploads the composited layer tree to the host.
    ///
    /// Actually causes the output of the rendering pipeline to appear on screen.
    pub fn composite_frame(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(
            self.has_configuration(app),
            "set the RenderView configuration before calling composite_frame"
        );
        debug_assert!(
            self.get(app).root_transform.is_some(),
            "call prepare_initial_frame before calling composite_frame"
        );
        let node = self.as_object();
        let layer = node
            .debug_layer(app)
            .or_else(|| node.layer(app))
            .expect("call prepare_initial_frame before calling composite_frame");
        let mut builder = Canvas::new();
        layer.add_to_scene(app, &mut builder);
        let scene = builder.build();
        debug_assert!(
            self.configuration(app)
                .logical_constraints
                .is_satisfied_by(self.size(app))
        );
        self.get(app).view.present(&scene);
    }

    /// An estimate of the bounds within which this render object will paint, in physical
    /// pixels.
    pub fn paint_bounds(self: RenderHandle<Self>, app: &App) -> Rect {
        Offset::ZERO & (self.size(app) * self.configuration(app).device_pixel_ratio)
    }
}

impl RenderObject for RenderView {
    fn render_object_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectData {
        &self.get(app).render_object
    }

    fn render_object_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderObjectData {
        &mut self.get_mut(app).render_object
    }

    fn perform_resize(self: RenderHandle<Self>, _app: &mut App) {
        panic!("RenderView is never sized by its parent");
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(self.get(app).root_transform.is_some());
        let constraints = self.constraints(app);
        let sized_by_child = !constraints.is_tight();
        let child = self.child(app);
        if let Some(child) = child {
            child.layout(app, constraints, sized_by_child);
        }
        let size = match child {
            Some(child) if sized_by_child => child.size(app),
            _ => constraints.smallest(),
        };
        self.get_mut(app).size = size;
        debug_assert!(size.is_finite());
        debug_assert!(constraints.is_satisfied_by(size));
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let root_transform = self
            .get(app)
            .root_transform
            .as_ref()
            .expect("the view has a configuration");
        crate::object::multiply(transform, root_transform);
        debug_assert!(child.parent(app).map(AnyRenderObject::id) == Some(self.id()));
    }

    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        true
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
}

/// Flutter's `RenderObject.handleEvent` default: the view itself does nothing with events.
impl HitTestTarget for RenderHandle<RenderView> {
    fn handle_event(&self, _app: &mut App, _event: &PointerEvent, _entry: &HitTestEntry) {}
}

/// The view is its own protocol: not a box, one object, so its vtable is a plain constant.
static VTABLE: RenderObjectVTable = RenderObjectVTable::of::<RenderView>(
    |app, id, child| <RenderView as RenderObject>::setup_parent_data(resolve(id), app, child),
    |app, id| RenderView::paint_bounds(resolve(id), app),
    |app, id, child, transform| {
        <RenderView as RenderObject>::apply_paint_transform(resolve(id), app, child, transform)
    },
    None,
    None,
);

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_embedder::{Picture, ViewId, ViewMetrics};

    use super::*;
    use crate::box_::RenderBox;
    use crate::layer::CompositedLayerKind;
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::RenderConstrainedBox;
    use crate::shifted_box::RenderPadding;
    use reveal_painting::EdgeInsetsGeometry;

    struct TestView {
        metrics: ViewMetrics,
        presented: Rc<Cell<u32>>,
    }

    impl View for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            self.metrics
        }

        fn present(&self, _picture: &Picture) {
            self.presented.set(self.presented.get() + 1);
        }
    }

    fn test_view(presented: &Rc<Cell<u32>>) -> ViewRef {
        Rc::new(TestView {
            metrics: ViewMetrics::default(),
            presented: Rc::clone(presented),
        })
    }

    /// `view_test.dart`'s `createViewConfiguration`.
    fn create_view_configuration(size: Size, device_pixel_ratio: f64) -> ViewConfiguration {
        let constraints = BoxConstraints::tight(size);
        ViewConfiguration::new(
            constraints * device_pixel_ratio,
            constraints,
            device_pixel_ratio,
        )
    }

    /// `view_test.dart`: `ViewConfiguration == and hashCode`.
    #[test]
    fn view_configuration_equality() {
        let a = create_view_configuration(Size::new(800.0, 600.0), 1.0);
        let b = create_view_configuration(Size::new(800.0, 600.0), 1.0);
        let c = create_view_configuration(Size::new(800.0, 600.0), 3.0);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// `view_test.dart`: `Constraints are derived from configuration`.
    #[test]
    fn constraints_are_derived_from_configuration() {
        let mut app = App::new();
        let presented = Rc::new(Cell::new(0));
        let view = RenderView::new(&mut app, None, None, test_view(&presented));
        assert!(!view.has_configuration(&app));
        let constraints = BoxConstraints::new()
            .min_width(1.0)
            .max_width(2.0)
            .min_height(3.0)
            .max_height(4.0);
        view.set_configuration(
            &mut app,
            ViewConfiguration::new(constraints * 3.0, constraints, 3.0),
        );
        assert_eq!(view.constraints(&app), constraints);
    }

    /// `view_test.dart`: `Config can be set and changed after instantiation without calling
    /// prepareInitialFrame first`.
    #[test]
    fn configuration_can_change_before_prepare_initial_frame() {
        let mut app = App::new();
        let presented = Rc::new(Cell::new(0));
        let view = RenderView::new(&mut app, None, None, test_view(&presented));
        view.set_configuration(
            &mut app,
            create_view_configuration(Size::new(100.0, 200.0), 3.0),
        );
        view.set_configuration(
            &mut app,
            create_view_configuration(Size::new(200.0, 300.0), 2.0),
        );
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(view.as_object()));
        view.prepare_initial_frame(&mut app);
    }

    /// `view_test.dart`: `does not replace the root layer unnecessarily` (and when the view
    /// resizes). Ours observes the replacement through the paint mark.
    #[test]
    fn does_not_replace_the_root_layer_unnecessarily() {
        let mut app = App::new();
        let presented = Rc::new(Cell::new(0));
        let view = RenderView::new(
            &mut app,
            None,
            Some(create_view_configuration(Size::new(100.0, 100.0), 1.0)),
            test_view(&presented),
        );
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(view.as_object()));
        view.prepare_initial_frame(&mut app);
        owner.flush_layout(&mut app);
        owner.flush_paint(&mut app);
        assert!(!view.as_object().debug_needs_paint(&app));

        view.set_configuration(
            &mut app,
            create_view_configuration(Size::new(100.0, 1117.0), 1.0),
        );
        assert!(
            !view.as_object().debug_needs_paint(&app),
            "a resize keeps the root layer"
        );

        view.set_configuration(
            &mut app,
            create_view_configuration(Size::new(100.0, 1117.0), 5.0),
        );
        assert!(
            view.as_object().debug_needs_paint(&app),
            "a new ratio replaces it"
        );
        let layer = view.as_object().debug_layer(&app).expect("root layer");
        assert_eq!(
            layer.composited().kind,
            CompositedLayerKind::Transform {
                transform: Matrix4::scale(5.0, 5.0)
            }
        );
    }

    /// `view_test.dart`: `accounts for device pixel ratio in paintBounds`, plus one frame
    /// composited to the host.
    #[test]
    fn accounts_for_device_pixel_ratio_in_paint_bounds_and_presents() {
        let mut app = App::new();
        let presented = Rc::new(Cell::new(0));
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(10.0, 10.0)), None);
        let view = RenderView::new(
            &mut app,
            Some(child.as_box()),
            Some(create_view_configuration(Size::new(800.0, 600.0), 2.0)),
            test_view(&presented),
        );
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(view.as_object()));
        view.prepare_initial_frame(&mut app);
        owner.flush_layout(&mut app);
        owner.flush_paint(&mut app);
        view.composite_frame(&mut app);

        assert_eq!(view.size(&app), Size::new(800.0, 600.0));
        assert_eq!(
            view.paint_bounds(&app),
            Offset::ZERO & Size::new(1600.0, 1200.0)
        );
        assert_eq!(presented.get(), 1);
    }

    #[test]
    fn coordinates_convert_through_the_paint_transforms_below_the_view() {
        let mut app = App::new();
        let presented = Rc::new(Cell::new(0));
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(30.0, 40.0)), None);
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::from_ltrb(10.0, 20.0, 0.0, 0.0),
            None,
            Some(child.as_box()),
        );
        let view = RenderView::new(
            &mut app,
            Some(padding.as_box()),
            Some(create_view_configuration(Size::new(800.0, 600.0), 2.0)),
            test_view(&presented),
        );
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(view.as_object()));
        view.prepare_initial_frame(&mut app);
        owner.flush_layout(&mut app);

        // The root view's own transform (logical to physical) is left out, as in Dart.
        let child = child.as_box();
        assert_eq!(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::new(10.0, 20.0)
        );
        assert_eq!(
            child.global_to_local(&app, Offset::new(15.0, 25.0), None),
            Offset::new(5.0, 5.0)
        );
        assert_eq!(
            child.local_to_global(&app, Offset::new(1.0, 1.0), Some(padding.as_object())),
            Offset::new(11.0, 21.0)
        );
        assert_eq!(
            padding.as_box().global_to_local(
                &app,
                Offset::new(10.0, 20.0),
                Some(child.as_object())
            ),
            Offset::new(20.0, 40.0)
        );
        assert!(
            child
                .as_object()
                .paint_bounds(&app)
                .inflate(70.0)
                .contains(child.global_to_local(&app, Offset::new(100.0, 100.0), None))
        );
    }

    #[test]
    fn a_proxy_box_adds_no_paint_transform() {
        let mut app = App::new();
        let presented = Rc::new(Cell::new(0));
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(30.0, 40.0)), None);
        let proxy =
            RenderConstrainedBox::new(&mut app, BoxConstraints::new(), Some(child.as_box()));
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::from_ltrb(10.0, 20.0, 0.0, 0.0),
            None,
            Some(proxy.as_box()),
        );
        let view = RenderView::new(
            &mut app,
            Some(padding.as_box()),
            Some(create_view_configuration(Size::new(800.0, 600.0), 2.0)),
            test_view(&presented),
        );
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(view.as_object()));
        view.prepare_initial_frame(&mut app);
        owner.flush_layout(&mut app);

        assert_eq!(
            child.as_box().local_to_global(&app, Offset::ZERO, None),
            Offset::new(10.0, 20.0)
        );
        assert_eq!(
            child
                .as_box()
                .global_to_local(&app, Offset::new(15.0, 25.0), None),
            Offset::new(5.0, 5.0)
        );
    }
}
