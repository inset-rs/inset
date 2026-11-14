//! Flutter counterpart: `widgets/image_filter.dart`.

use std::fmt::{self, Debug};

use waft_embedder::{ImageFilter, Matrix4, Offset, Size, TextBaseline};
use waft_foundation::App;
use waft_rendering::{
    AnyOffsetLayer, AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult,
    ImageFilterLayer, OffsetLayerMixin, PaintingContext, RenderBox, RenderBoxData, RenderHandle,
    RenderObject, RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin,
    RenderProxyBoxMixin,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, WidgetRef,
};

/// Applies an [`ImageFilter`] to its child.
///
/// An image filter will always apply its filter operation to the child widget, even if said
/// filter is conceptually a "no-op", such as an [`ImageFilter::blur`] with a radius of 0 or an
/// `ImageFilter.matrix` with an identity matrix. Setting [`enabled`](Self::enabled) to `false`
/// is a more efficient manner of disabling an image filter.
///
/// The framework does not attempt to optimize out "no-op" filters because it cannot tell the
/// difference between an intentional no-op and a filter that is only incidentally a no-op.
/// Consider an `ImageFilter.matrix` that is animated and happens to pass through the identity
/// matrix. If the framework identified it as a no-op it would drop and then recreate the layer
/// during the animation which would be more expensive than keeping it around.
///
/// See also:
///
///  * [`BackdropFilter`](crate::BackdropFilter), which applies an [`ImageFilter`] to everything
///    beneath its child.
///  * [`ImageFilter::shader`], which runs a fragment program over the child.
pub struct ImageFiltered {
    pub key: Option<KeyRef>,
    /// The image filter to apply to the child of this widget.
    pub image_filter: ImageFilter,
    /// Whether or not to apply the image filter operation to the child of this widget.
    ///
    /// Prefer setting enabled to `false` instead of creating a "no-op" filter type for
    /// performance reasons.
    pub enabled: bool,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl ImageFiltered {
    /// Creates a widget that applies an [`ImageFilter`] to its child; Dart's named arguments are
    /// the setters.
    pub fn new(image_filter: ImageFilter) -> ImageFiltered {
        ImageFiltered {
            key: None,
            image_filter,
            enabled: true,
            child: None,
        }
    }

    /// Dart `ImageFiltered(key:)`.
    pub fn key(mut self, key: KeyRef) -> ImageFiltered {
        self.key = Some(key);
        self
    }

    /// Dart `ImageFiltered(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> ImageFiltered {
        self.enabled = enabled;
        self
    }

    /// Dart `ImageFiltered(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ImageFiltered {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for ImageFiltered {
    type RenderObject = ImageFilterRenderObject;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        ImageFilterRenderObject::new(app, self.image_filter.clone(), self.enabled).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<ImageFilterRenderObject>,
    ) {
        render_object.set_enabled(app, self.enabled);
        render_object.set_image_filter(app, self.image_filter.clone());
    }
}

impl SingleChildRenderObjectWidget for ImageFiltered {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for ImageFiltered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageFiltered")
            .field("key", &self.key)
            .field("image_filter", &self.image_filter)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

/// The render object of [`ImageFiltered`]: a repaint boundary, while it has a child and is
/// enabled, whose layer is an [`ImageFilterLayer`] of its filter.
///
/// Dart's `_ImageFilterRenderObject`.
pub struct ImageFilterRenderObject {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    enabled: bool,
    image_filter: ImageFilter,
}

impl ImageFilterRenderObject {
    fn new(app: &mut App, image_filter: ImageFilter, enabled: bool) -> RenderHandle<Self> {
        RenderHandle::new_box(
            app,
            ImageFilterRenderObject {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                enabled,
                image_filter,
            },
        )
    }

    /// Whether the filter is applied.
    pub fn enabled(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).enabled
    }

    /// Sets [`enabled`](Self::enabled).
    pub fn set_enabled(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.enabled(app) == value {
            return;
        }
        let was_repaint_boundary = self.as_object().is_repaint_boundary(app);
        self.get_mut(app).enabled = value;
        if self.as_object().is_repaint_boundary(app) != was_repaint_boundary {
            self.as_object().mark_needs_compositing_bits_update(app);
        }
        self.as_object().mark_needs_paint(app);
    }

    /// The filter applied to the child.
    pub fn image_filter(self: RenderHandle<Self>, app: &App) -> &ImageFilter {
        &self.get(app).image_filter
    }

    /// Sets [`image_filter`](Self::image_filter).
    pub fn set_image_filter(self: RenderHandle<Self>, app: &mut App, value: ImageFilter) {
        if value != self.get(app).image_filter {
            self.get_mut(app).image_filter = value;
            self.as_object().mark_needs_composited_layer_update(app);
        }
    }
}

impl RenderObjectWithChildMixin for ImageFilterRenderObject {
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

impl RenderProxyBoxMixin for ImageFilterRenderObject {}

impl RenderObject for ImageFilterRenderObject {
    waft_rendering::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, app: &App) -> bool {
        self.child(app).is_some() && self.enabled(app)
    }

    fn is_repaint_boundary(self: RenderHandle<Self>, app: &App) -> bool {
        self.always_needs_compositing(app)
    }

    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<AnyOffsetLayer>,
    ) -> AnyOffsetLayer {
        let layer = match old_layer {
            Some(old) => app
                .handle::<ImageFilterLayer>(old.id())
                .expect("ImageFiltered reuses an ImageFilterLayer"),
            None => ImageFilterLayer::new(app),
        };
        let image_filter = self.image_filter(app).clone();
        layer.set_image_filter(app, image_filter);
        layer.as_offset_layer()
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
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

impl RenderBox for ImageFilterRenderObject {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/widgets/image_filter_test.dart`; its golden cases wait on golden tests.

    use waft_embedder::{FilterQuality, FragmentProgram};
    use waft_foundation::{AppCell, Handle};
    use waft_rendering::{AnyContainerLayer, ErasedLayer};

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{RepaintBoundary, SizedBox};

    fn filtered(image_filter: ImageFilter, enabled: bool) -> WidgetRef {
        RepaintBoundary::new()
            .child(
                ImageFiltered::new(image_filter)
                    .enabled(enabled)
                    .child(SizedBox::new().width(10.0).height(10.0)),
            )
            .into_widget()
    }

    /// The `ImageFiltered` under the root's repaint boundary.
    fn render_object(harness: &Harness, app: &App) -> RenderHandle<ImageFilterRenderObject> {
        harness
            .render_root(app)
            .child(app)
            .expect("the repaint boundary")
            .as_object()
            .downcast::<waft_rendering::RenderRepaintBoundary>(app)
            .expect("the repaint boundary")
            .child(app)
            .expect("the image filter")
            .as_object()
            .downcast::<ImageFilterRenderObject>(app)
            .expect("the image filter")
    }

    fn image_filter_layer(layer: AnyContainerLayer, app: &App) -> Handle<ImageFilterLayer> {
        app.handle::<ImageFilterLayer>(layer.as_layer().id())
            .expect("an ImageFilterLayer")
    }

    /// Dart's `Image filter - reuses its layer`.
    #[test]
    fn the_layer_is_reused_when_the_filter_changes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, filtered(ImageFilter::blur(5.0, 5.0), true));
        harness.pump(&mut app);
        let object = render_object(&harness, &app).as_object();
        let original = object.debug_layer(&app).expect("a layer");

        harness.set_child(&mut app, filtered(ImageFilter::blur(10.0, 10.0), true));
        harness.pump(&mut app);
        let layer = object.debug_layer(&app).expect("a layer");
        assert_eq!(layer, original);
        assert_eq!(
            image_filter_layer(layer, &app).image_filter(&app),
            Some(&ImageFilter::blur(10.0, 10.0))
        );
    }

    /// Dart's `Image filter - enabled and disabled`.
    #[test]
    fn a_disabled_filter_makes_no_layer() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, filtered(ImageFilter::blur(5.0, 5.0), false));
        harness.pump(&mut app);
        let object = render_object(&harness, &app).as_object();
        assert!(object.debug_layer(&app).is_none());

        harness.set_child(&mut app, filtered(ImageFilter::blur(5.0, 5.0), true));
        harness.pump(&mut app);
        let layer = object.debug_layer(&app).expect("a layer once enabled");
        image_filter_layer(layer, &app);
    }

    /// Dart's `ImageFiltered does not crash at zero area`.
    #[test]
    fn a_filter_of_zero_area_lays_out() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            SizedBox::shrink()
                .child(ImageFiltered::new(ImageFilter::blur(5.0, 5.0)).child(SizedBox::new()))
                .into_widget(),
        );
        harness.pump(&mut app);
        let image_filtered = harness
            .render_root(&app)
            .child(&app)
            .expect("the sized box")
            .as_object()
            .downcast::<waft_rendering::RenderConstrainedBox>(&app)
            .expect("the sized box")
            .child(&app)
            .expect("the image filter");
        assert_eq!(image_filtered.size(&app), Size::ZERO);
    }

    /// A fragment program as the filter, as the stretch effect filters its child.
    #[test]
    fn a_shader_filter_reaches_the_layer() {
        let program = FragmentProgram::from_wgsl(
            "struct U { u_size: vec2<f32> }
             @group(2) @binding(0) var<uniform> u: U;
             @group(1) @binding(0) var u_texture: texture_2d<f32>;
             @group(1) @binding(1) var u_texture_sampler: sampler;
             fn shade(position: vec2<f32>) -> vec4<f32> {
                 return textureSampleLevel(u_texture, u_texture_sampler, position / u.u_size, 0.0);
             }",
        )
        .expect("a valid program");
        let filter = ImageFilter::shader(&program.fragment_shader(), FilterQuality::Low);
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, filtered(filter.clone(), true));
        harness.pump(&mut app);
        let layer = render_object(&harness, &app)
            .as_object()
            .debug_layer(&app)
            .expect("a layer");
        assert_eq!(
            image_filter_layer(layer, &app).image_filter(&app),
            Some(&filter)
        );
    }
}
