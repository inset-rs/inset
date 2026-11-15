//! Flutter counterpart: `widgets/snapshot_widget.dart` (`SnapshotMode`, `SnapshotController`,
//! `SnapshotWidget`, `_RenderSnapshotWidget`, `SnapshotPainter`, `_DefaultSnapshotPainter`).
//!
//! `OffsetLayer.toImageSync` is deferred, so the render object never takes a raster: where
//! Flutter would rasterise the child and hand the image to
//! [`SnapshotPainter::paint_snapshot`], the painter's [`paint`](SnapshotPainter::paint) paints
//! the child live. The raster fields and the snapshot branch stay as Dart has them, empty until
//! the raster lands; a later editor who fills `child_raster` must also restore Dart's
//! `_paintAndDetachToImage`, with its platform-view check and [`SnapshotMode`] branches.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};

use waft_embedder::valo::Sampling;
use waft_embedder::{Image, Matrix4, Offset, Paint, Rect, Size, TextBaseline};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Listenable, ListenableObject,
    Listener, Retained,
};
use waft_painting::ClipContext;
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, PaintingContext,
    PipelineOwner, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectBase,
    RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
};

use crate::framework::{
    BuildContext, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, WidgetRef,
};
use crate::widgets::media_query::MediaQuery;
use waft_foundation::Ref;

/// Controls how the [`SnapshotWidget`] paints its child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotMode {
    /// The child is snapshotted, but only if all descendants can be snapshotted.
    ///
    /// If there is a platform view in the children of a snapshot widget, the
    /// snapshot will not be used and the child will be rendered using
    /// [`SnapshotPainter::paint`]. This uses an un-snapshotted child and by default
    /// paints it with no additional modification.
    Permissive,

    /// An error is thrown if the child cannot be snapshotted.
    ///
    /// This setting is the default state of the [`SnapshotWidget`].
    Normal,

    /// The child is snapshotted and any child platform views are ignored.
    ///
    /// This mode can be useful if there is a platform view descendant that does
    /// not need to be included in the snapshot.
    Forced,
}

/// A controller for the [`SnapshotWidget`] that controls when the child image is displayed
/// and when to regenerated the child image.
///
/// When the value of [`allow_snapshotting`](Self::allow_snapshotting) is true, the
/// [`SnapshotWidget`] will paint the child widgets based on the [`SnapshotMode`] of the
/// snapshot widget.
///
/// The controller notifies its listeners when the value of
/// [`allow_snapshotting`](Self::allow_snapshotting) changes or when [`clear`](Self::clear) is
/// called.
///
/// To force [`SnapshotWidget`] to recreate the child image, call [`clear`](Self::clear).
pub struct SnapshotController {
    change_notifier: ChangeNotifierData,
    allow_snapshotting: bool,
}

impl SnapshotController {
    /// Create a new [`SnapshotController`].
    ///
    /// By default, [`allow_snapshotting`](Self::allow_snapshotting) is `false`: Dart's
    /// `SnapshotController({bool allowSnapshotting = false})`.
    pub fn new(app: &mut App, allow_snapshotting: bool) -> Ref<SnapshotController> {
        app.create(SnapshotController {
            change_notifier: ChangeNotifierData::new(),
            allow_snapshotting,
        })
    }

    /// Reset the snapshot held by any listening [`SnapshotWidget`].
    ///
    /// This has no effect if [`allow_snapshotting`](Self::allow_snapshotting) is `false`.
    pub fn clear(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }

    /// Whether a snapshot of this child widget is painted in its place.
    pub fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        app.get(self).allow_snapshotting
    }

    /// Sets [`allow_snapshotting`](Self::allow_snapshotting).
    pub fn set_allow_snapshotting(self: Handle<Self>, app: &mut App, value: bool) {
        if value == self.allow_snapshotting(app) {
            return;
        }
        app.get_mut(self).allow_snapshotting = value;
        self.notify_listeners(app);
    }

    /// Discards any resources used by the object. Dart's inherited `ChangeNotifier.dispose`.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        ChangeNotifierData::dispose(app, self);
    }
}

impl ChangeNotifier for SnapshotController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl Debug for SnapshotController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapshotController")
            .field("allow_snapshotting", &self.allow_snapshotting)
            .finish()
    }
}

/// A widget that can replace its child with a snapshotted version of the child.
///
/// A snapshot is a frozen texture-backed representation of all child pictures
/// and layers stored as an [`Image`].
///
/// This widget is useful for performing short animations that would otherwise
/// be expensive or that cannot rely on raster caching. For example, scale and
/// skew animations are often expensive to perform on complex children, as are
/// blurs. For a short animation, a widget that contains these expensive effects
/// can be replaced with a snapshot of itself and manipulated instead.
///
/// For example, the Android Q `ZoomPageTransitionsBuilder` uses a snapshot widget
/// for the forward and entering route to avoid the expensive scale animation.
/// This also has the effect of briefly pausing any animations on the page.
///
/// Generally, this widget should not be used in places where users expect the
/// child widget to continue animating or to be responsive, such as an unbounded
/// animation.
///
/// Caveats:
///
/// * The contents of platform views cannot be captured by a snapshot
///   widget. If a platform view is encountered, then the snapshot widget will
///   determine how to render its children based on the [`SnapshotMode`]. This
///   defaults to [`SnapshotMode::Normal`] which will throw an exception if a
///   platform view is encountered.
///
/// * On the CanvasKit backend of Flutter, the performance of using this widget may regress
///   performance due to the fact that both the UI and engine share a single thread.
#[derive(Debug)]
pub struct SnapshotWidget {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The controller that determines when to display the children as a snapshot.
    pub controller: Ref<SnapshotController>,

    /// Configuration that controls how the snapshot widget decides to paint its children.
    ///
    /// Defaults to [`SnapshotMode::Normal`], which throws an error when a platform view
    /// or texture view is encountered.
    ///
    /// See [`SnapshotMode`] for more information.
    pub mode: SnapshotMode,

    /// Whether or not changes in render object size should automatically re-create
    /// the snapshot.
    ///
    /// Defaults to false.
    pub autoresize: bool,

    /// The painter used to paint the child snapshot or child widgets.
    ///
    /// `None` is Dart's default, `const _DefaultSnapshotPainter()`: one painter per app,
    /// reached through [`effective_painter`](Self::effective_painter), which paints the child
    /// exactly as it would paint itself.
    pub painter: Option<SnapshotPainterRef>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl SnapshotWidget {
    /// Create a new [`SnapshotWidget`].
    ///
    /// The [`controller`](Self::controller) and [`child`](Self::child) arguments are required;
    /// Dart's `child` is required but nullable.
    pub fn new(controller: Ref<SnapshotController>, child: Option<WidgetRef>) -> SnapshotWidget {
        SnapshotWidget {
            key: None,
            controller,
            mode: SnapshotMode::Normal,
            autoresize: false,
            painter: None,
            child,
        }
    }

    /// Dart `SnapshotWidget(key:)`.
    pub fn key(mut self, key: KeyRef) -> SnapshotWidget {
        self.key = Some(key);
        self
    }

    /// Dart `SnapshotWidget(mode:)`.
    pub fn mode(mut self, mode: SnapshotMode) -> SnapshotWidget {
        self.mode = mode;
        self
    }

    /// Dart `SnapshotWidget(painter:)`.
    pub fn painter(mut self, painter: SnapshotPainterRef) -> SnapshotWidget {
        self.painter = Some(painter);
        self
    }

    /// Dart `SnapshotWidget(autoresize:)`.
    pub fn autoresize(mut self, autoresize: bool) -> SnapshotWidget {
        self.autoresize = autoresize;
        self
    }

    /// Dart's `painter` field: the given painter, or the app's one default painter.
    pub fn effective_painter(&self, app: &mut App) -> SnapshotPainterRef {
        self.painter.clone().unwrap_or_else(|| {
            app.singleton::<DefaultSnapshotPainter>()
                .upgrade(app)
                .expect("the app keeps its singletons")
                .into_erased()
        })
    }
}

impl RenderObjectWidget for SnapshotWidget {
    type RenderObject = RenderSnapshotWidget;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let device_pixel_ratio = MediaQuery::device_pixel_ratio_of(app, context);
        let painter = self.effective_painter(app);
        RenderSnapshotWidget::new(
            app,
            device_pixel_ratio,
            self.controller.clone(),
            self.mode,
            painter,
            self.autoresize,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderSnapshotWidget>,
    ) {
        render_object.set_controller(app, self.controller.clone());
        render_object.set_mode(app, self.mode);
        let device_pixel_ratio = MediaQuery::device_pixel_ratio_of(app, context);
        render_object.set_device_pixel_ratio(app, device_pixel_ratio);
        let painter = self.effective_painter(app);
        render_object.set_painter(app, painter);
        render_object.set_autoresize(app, self.autoresize);
    }
}

impl SingleChildRenderObjectWidget for SnapshotWidget {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A render object that conditionally converts its child into an [`Image`]
/// and then paints it in place of the child.
///
/// Flutter's private `_RenderSnapshotWidget`.
pub struct RenderSnapshotWidget {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    device_pixel_ratio: f64,
    painter: SnapshotPainterRef,
    controller: Ref<SnapshotController>,
    mode: SnapshotMode,
    autoresize: bool,
    /// Always `None` until `OffsetLayer.toImageSync` lands (see the module docs).
    child_raster: Option<Image>,
    child_raster_size: Option<Size>,
    /// Set to true if the snapshot mode was not forced and a platform view
    /// was encountered while attempting to snapshot the child.
    disable_snapshot_attempt: bool,
    last_cached_size: Option<Size>,
}

impl RenderSnapshotWidget {
    /// Create a new [`RenderSnapshotWidget`].
    fn new(
        app: &mut App,
        device_pixel_ratio: f64,
        controller: Ref<SnapshotController>,
        mode: SnapshotMode,
        painter: SnapshotPainterRef,
        autoresize: bool,
    ) -> RenderHandle<RenderSnapshotWidget> {
        RenderHandle::new_box(
            app,
            RenderSnapshotWidget {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                device_pixel_ratio,
                painter,
                controller,
                mode,
                autoresize,
                child_raster: None,
                child_raster_size: None,
                disable_snapshot_attempt: false,
                last_cached_size: None,
            },
        )
    }

    /// The device pixel ratio used to create the child image.
    pub fn device_pixel_ratio(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).device_pixel_ratio
    }

    /// Sets [`device_pixel_ratio`](Self::device_pixel_ratio).
    pub fn set_device_pixel_ratio(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if value == self.device_pixel_ratio(app) {
            return;
        }
        self.get_mut(app).device_pixel_ratio = value;
        if self.get(app).child_raster.is_none() {
            return;
        }
        self.get_mut(app).child_raster = None;
        self.mark_needs_paint(app);
    }

    /// The painter used to paint the child snapshot or child widgets.
    pub fn painter(self: RenderHandle<Self>, app: &App) -> AnySnapshotPainter {
        self.get(app).painter.handle()
    }

    /// Sets [`painter`](Self::painter).
    pub fn set_painter(self: RenderHandle<Self>, app: &mut App, value: SnapshotPainterRef) {
        if value.handle() == self.painter(app) {
            return;
        }
        let old_painter = self.painter(app);
        old_painter.remove_listener(app, &self.paint_listener());
        self.get_mut(app).painter = value;
        let painter = self.painter(app);
        if old_painter.runtime_type() != painter.runtime_type()
            || painter.should_repaint(app, old_painter)
        {
            self.mark_needs_paint(app);
        }
        if self.attached(app) {
            painter.add_listener(app, self.paint_listener());
        }
    }

    /// A controller that determines whether to paint the child normally or to
    /// paint a snapshotted version of that child.
    pub fn controller(self: RenderHandle<Self>, app: &App) -> Handle<SnapshotController> {
        self.get(app).controller.handle()
    }

    /// Sets [`controller`](Self::controller).
    pub fn set_controller(self: RenderHandle<Self>, app: &mut App, value: Ref<SnapshotController>) {
        if value.handle() == self.controller(app) {
            return;
        }
        self.controller(app)
            .remove_listener(app, &self.raster_value_changed_listener());
        let old_value = self.controller(app).allow_snapshotting(app);
        self.get_mut(app).controller = value;
        if self.attached(app) {
            self.controller(app)
                .add_listener(app, self.raster_value_changed_listener());
            if old_value != self.controller(app).allow_snapshotting(app) {
                self.on_raster_value_changed(app);
            }
        }
    }

    /// How the snapshot widget will handle platform views in child layers.
    pub fn mode(self: RenderHandle<Self>, app: &App) -> SnapshotMode {
        self.get(app).mode
    }

    /// Sets [`mode`](Self::mode).
    pub fn set_mode(self: RenderHandle<Self>, app: &mut App, value: SnapshotMode) {
        if value == self.mode(app) {
            return;
        }
        self.get_mut(app).mode = value;
        self.mark_needs_paint(app);
    }

    /// Whether or not changes in render object size should automatically re-rasterize.
    pub fn autoresize(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).autoresize
    }

    /// Sets [`autoresize`](Self::autoresize).
    pub fn set_autoresize(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.autoresize(app) {
            return;
        }
        self.get_mut(app).autoresize = value;
        self.mark_needs_paint(app);
    }

    /// Dart's `_onRasterValueChanged`.
    fn on_raster_value_changed(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).disable_snapshot_attempt = false;
        self.clear_child_raster(app);
        self.mark_needs_paint(app);
    }

    /// Dart's `_childRaster?.dispose(); _childRaster = null; _childRasterSize = null;`.
    fn clear_child_raster(self: RenderHandle<Self>, app: &mut App) {
        let this = self.get_mut(app);
        this.child_raster = None;
        this.child_raster_size = None;
    }

    /// The `markNeedsPaint` tear-off, equal to itself across registrations.
    fn paint_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), mark_needs_paint)
    }

    /// The `_onRasterValueChanged` tear-off, equal to itself across registrations.
    fn raster_value_changed_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), on_raster_value_changed)
    }

    /// Dart's `painter.paint(context, offset, size, super.paint)`: the painter paints the child
    /// live, through the proxy box's own paint.
    fn paint_with_painter(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let size = self.size(app);
        self.painter(app)
            .paint(app, context, offset, size, &mut |app, context, offset| {
                RenderProxyBoxMixin::paint(self, app, context, offset)
            });
    }
}

fn mark_needs_paint(this: Handle<RenderSnapshotWidget>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

fn on_raster_value_changed(this: Handle<RenderSnapshotWidget>, app: &mut App) {
    RenderHandle::from_handle(this).on_raster_value_changed(app);
}

impl RenderObjectWithChildMixin for RenderSnapshotWidget {
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

impl RenderProxyBoxMixin for RenderSnapshotWidget {}

impl RenderObject for RenderSnapshotWidget {
    waft_rendering::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        self.controller(app)
            .add_listener(app, self.raster_value_changed_listener());
        self.painter(app).add_listener(app, self.paint_listener());
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).disable_snapshot_attempt = false;
        self.controller(app)
            .remove_listener(app, &self.raster_value_changed_listener());
        self.painter(app)
            .remove_listener(app, &self.paint_listener());
        self.clear_child_raster(app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        self.controller(app)
            .remove_listener(app, &self.raster_value_changed_listener());
        self.painter(app)
            .remove_listener(app, &self.paint_listener());
        self.clear_child_raster(app);
        RenderObjectBase::dispose(self, app);
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
        let size = self.size(app);
        if size.is_empty() {
            self.clear_child_raster(app);
            return;
        }
        if !self.controller(app).allow_snapshotting(app) || self.get(app).disable_snapshot_attempt {
            self.clear_child_raster(app);
            self.paint_with_painter(app, context, offset);
            return;
        }

        let this = self.get(app);
        if this.autoresize && Some(size) != this.last_cached_size && this.last_cached_size.is_some()
        {
            self.get_mut(app).child_raster = None;
        }

        // Dart rasterises the child here (`_paintAndDetachToImage`); without
        // `OffsetLayer.toImageSync` no raster is taken, so the painter paints the child live.
        let this = self.get(app);
        match (this.child_raster.clone(), this.child_raster_size) {
            (Some(child_raster), Some(child_raster_size)) => {
                let device_pixel_ratio = this.device_pixel_ratio;
                self.painter(app).paint_snapshot(
                    app,
                    context,
                    offset,
                    size,
                    &child_raster,
                    child_raster_size,
                    device_pixel_ratio,
                );
            }
            _ => self.paint_with_painter(app, context, offset),
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

impl RenderBox for RenderSnapshotWidget {
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

/// A painter used to paint either a snapshot or the child widgets that
/// would be a snapshot.
///
/// The painter can call `notify_listeners` to have the [`SnapshotWidget`]
/// re-paint (re-using the same raster). This allows animations to be performed
/// without re-snapshotting of children. For certain scale or perspective changing
/// transforms, such as a rotation, this can be significantly faster than performing
/// the same animation at the widget level.
///
/// By default, the [`SnapshotWidget`] includes a delegate that draws the child raster
/// exactly as the child widgets would have been drawn. Nevertheless, this can
/// also be used to efficiently transform the child raster and apply complex paint
/// effects.
///
/// Dart's abstract `SnapshotPainter extends ChangeNotifier`: a painter is an arena object that
/// holds [`ChangeNotifierData`] and implements [`ChangeNotifier`], which gives it the
/// [`ListenableObject`] side this trait asks for; the default painter implements that side
/// itself, as Dart's `_DefaultSnapshotPainter implements SnapshotPainter` does.
pub trait SnapshotPainter: ListenableObject {
    /// Called whenever the `image` that represents a [`SnapshotWidget`]s child should be
    /// painted.
    ///
    /// The image is rasterized at the physical pixel resolution and should be scaled down by
    /// `pixel_ratio` to account for device independent pixels.
    ///
    /// The default painter paints the snapshot accounting for the fact that the image width
    /// and height will be given in physical pixels, while the image must be painted with
    /// device independent pixels. That is, the width and height of the image is the widget and
    /// height of the provided `size`, multiplied by the `pixel_ratio`. In addition, the actual
    /// size of the scene captured by the `image` is not the image's width or height, but
    /// indeed `source_size`, because the former is a rounded inaccurate integer.
    #[allow(clippy::too_many_arguments)]
    fn paint_snapshot(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        image: &Image,
        source_size: Size,
        pixel_ratio: f64,
    );

    /// Paint the child via `painter`, applying any effects that would have been painted
    /// in [`paint_snapshot`](Self::paint_snapshot).
    ///
    /// This method is called when snapshotting is disabled, or when
    /// [`SnapshotMode::Permissive`] is used and a child platform view prevents snapshotting.
    ///
    /// The `offset` and `size` are the location and dimensions of the render object.
    fn paint(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        painter: &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
    );

    /// Called whenever a new instance of the snapshot widget delegate class is
    /// provided to the [`SnapshotWidget`] object, or any time that a new
    /// [`SnapshotPainter`] object is created with a new instance of the
    /// delegate class (which amounts to the same thing, because the latter is
    /// implemented in terms of the former).
    ///
    /// If the new instance represents different information than the old
    /// instance, then the method should return true, otherwise it should return
    /// false.
    ///
    /// If the method returns false, then the [`paint`](Self::paint) call might be optimized
    /// away.
    ///
    /// It's possible that the [`paint`](Self::paint) method will get called even if
    /// [`should_repaint`](Self::should_repaint) returns false (e.g. if an ancestor or
    /// descendant needed to be repainted). It's also possible that the [`paint`](Self::paint)
    /// method will get called without [`should_repaint`](Self::should_repaint) being called at
    /// all (e.g. if the box changes size).
    ///
    /// Changing the delegate will not cause the child image retained by the
    /// [`SnapshotWidget`] to be updated. Instead, [`SnapshotController::clear`] can
    /// be used to force the generation of a new image.
    ///
    /// The `old_painter` argument is of the same type as this painter: Dart's `covariant`
    /// parameter, reached with [`AnySnapshotPainter::downcast`].
    fn should_repaint(self: Handle<Self>, app: &App, old_painter: AnySnapshotPainter) -> bool;

    /// The type-erased `SnapshotPainter` handle. Free: the vtable is a `const`, and the id is
    /// copied.
    fn as_snapshot_painter(self: Handle<Self>) -> AnySnapshotPainter {
        AnySnapshotPainter {
            id: self.id(),
            vtable: const { &SnapshotPainterVTable::of::<Self>() },
        }
    }
}

/// The dispatch signature of [`SnapshotPainter::paint_snapshot`].
type PaintSnapshotFn =
    fn(&mut App, HandleId, &mut PaintingContext, Offset, Size, &Image, Size, f64);

/// The dispatch signature of [`SnapshotPainter::paint`].
type PaintFn = fn(
    &mut App,
    HandleId,
    &mut PaintingContext,
    Offset,
    Size,
    &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
);

/// The vtable of an erased [`AnySnapshotPainter`]: one `&'static` table per concrete
/// [`SnapshotPainter`] type.
struct SnapshotPainterVTable {
    runtime_type: fn() -> TypeId,
    type_name: fn() -> &'static str,
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    paint_snapshot: PaintSnapshotFn,
    paint: PaintFn,
    should_repaint: fn(&App, HandleId, AnySnapshotPainter) -> bool,
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl SnapshotPainterVTable {
    const fn of<T: SnapshotPainter>() -> SnapshotPainterVTable {
        SnapshotPainterVTable {
            runtime_type: TypeId::of::<T>,
            type_name: std::any::type_name::<T>,
            add_listener: |app, id, listener| {
                ListenableObject::add_listener(resolve::<T>(id), app, listener)
            },
            remove_listener: |app, id, listener| {
                ListenableObject::remove_listener(resolve::<T>(id), app, listener)
            },
            paint_snapshot: |app, id, context, offset, size, image, source_size, pixel_ratio| {
                T::paint_snapshot(
                    resolve(id),
                    app,
                    context,
                    offset,
                    size,
                    image,
                    source_size,
                    pixel_ratio,
                )
            },
            paint: |app, id, context, offset, size, painter| {
                T::paint(resolve(id), app, context, offset, size, painter)
            },
            should_repaint: |app, id, old_painter| T::should_repaint(resolve(id), app, old_painter),
        }
    }
}

/// Erased [`SnapshotPainter`]: what a field or parameter Dart types as `SnapshotPainter`
/// becomes.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnySnapshotPainter {
    id: HandleId,
    vtable: &'static SnapshotPainterVTable,
}

impl From<AnySnapshotPainter> for HandleId {
    fn from(handle: AnySnapshotPainter) -> HandleId {
        handle.id
    }
}

/// A field Dart types as the erased object, kept: AnySnapshotPainter with its count.
pub type SnapshotPainterRef = Retained<AnySnapshotPainter>;

impl AnySnapshotPainter {
    /// A [`SnapshotPainterRef`] while something still keeps the object; see
    /// [`Handle::upgrade`](waft_foundation::Handle::upgrade).
    pub fn upgrade(self, app: &App) -> Option<SnapshotPainterRef> {
        Retained::upgrade(app, self)
    }
}

/// A kept object erases to a [`SnapshotPainterRef`] with [`Retained::erase`]: Dart's upcast.
impl<T: SnapshotPainter> From<Handle<T>> for AnySnapshotPainter {
    fn from(handle: Handle<T>) -> AnySnapshotPainter {
        handle.as_snapshot_painter()
    }
}

impl AnySnapshotPainter {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `runtimeType`.
    pub fn runtime_type(self) -> TypeId {
        (self.vtable.runtime_type)()
    }

    /// Dart's `painter as P`: the typed handle when this painter is a `P`, else `None`.
    pub fn downcast<P: 'static>(self, app: &App) -> Option<Handle<P>> {
        app.handle::<P>(self.id)
    }

    /// See [`SnapshotPainter::paint_snapshot`].
    #[allow(clippy::too_many_arguments)]
    pub fn paint_snapshot(
        self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        image: &Image,
        source_size: Size,
        pixel_ratio: f64,
    ) {
        (self.vtable.paint_snapshot)(
            app,
            self.id,
            context,
            offset,
            size,
            image,
            source_size,
            pixel_ratio,
        )
    }

    /// See [`SnapshotPainter::paint`].
    pub fn paint(
        self,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        painter: &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
    ) {
        (self.vtable.paint)(app, self.id, context, offset, size, painter)
    }

    /// See [`SnapshotPainter::should_repaint`].
    pub fn should_repaint(self, app: &App, old_painter: AnySnapshotPainter) -> bool {
        (self.vtable.should_repaint)(app, self.id, old_painter)
    }
}

impl PartialEq for AnySnapshotPainter {
    fn eq(&self, other: &AnySnapshotPainter) -> bool {
        self.id == other.id
    }
}

impl Eq for AnySnapshotPainter {}

impl Hash for AnySnapshotPainter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnySnapshotPainter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl Listenable for AnySnapshotPainter {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener)
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener)
    }
}

/// Flutter's private `_DefaultSnapshotPainter`: paints the child as it would paint itself, and
/// the snapshot exactly over the render object. It never notifies, so it keeps no listeners.
///
/// Dart's `const` constructor makes every default painter one object; here it is the app's
/// singleton.
#[derive(Debug, Default)]
struct DefaultSnapshotPainter;

impl ListenableObject for DefaultSnapshotPainter {
    fn add_listener(self: Handle<Self>, _app: &mut App, _listener: Listener) {}

    fn remove_listener(self: Handle<Self>, _app: &mut App, _listener: &Listener) {}
}

impl SnapshotPainter for DefaultSnapshotPainter {
    fn paint(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        _size: Size,
        painter: &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
    ) {
        painter(app, context, offset);
    }

    fn paint_snapshot(
        self: Handle<Self>,
        _app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        size: Size,
        image: &Image,
        source_size: Size,
        _pixel_ratio: f64,
    ) {
        let src = Rect::from_ltwh(0.0, 0.0, source_size.width(), source_size.height());
        let dst = Rect::from_ltwh(offset.dx(), offset.dy(), size.width(), size.height());
        // Valo's default sampling, linear with linear mipmaps, is Dart's `FilterQuality.medium`.
        let sampling = Sampling::default();
        context.canvas().draw_image_rect(
            image,
            src.into(),
            dst.into(),
            sampling,
            &Paint::default(),
        );
    }

    fn should_repaint(self: Handle<Self>, _app: &App, _old_painter: AnySnapshotPainter) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::cell::{Cell, RefCell};
    use std::marker::PhantomData;
    use std::rc::Rc;
    use waft_foundation::Ref;

    use waft_embedder::{Canvas, Color};
    use waft_foundation::AppCell;
    use waft_painting::TextDirection;
    use waft_rendering::{
        AnyLayer, CustomPainter, LayerHandle, OpacityLayer, PictureLayer, TransformLayer,
    };

    use super::*;
    use crate::framework::IntoWidget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{
        Center, CustomPaint, Directionality, RepaintBoundary, SizedBox, StateSetter,
        StatefulBuilder,
    };
    use crate::widgets::container::Container;
    use crate::widgets::media_query::MediaQueryData;

    const YELLOW: Color = Color::new(0xFFAABB11);

    struct First;
    struct Second;

    /// Flutter's `TestPainter`, and with `Second` its `TestPainter2`: a painter of another
    /// runtime type that behaves the same.
    struct TestPainterOf<M: 'static> {
        change_notifier: ChangeNotifierData,
        count: u32,
        should_repaint_value: bool,
        last_image: Option<Image>,
        added_listener_count: u32,
        removed_listener_count: u32,
        marker: PhantomData<M>,
    }

    type TestPainter = TestPainterOf<First>;
    type TestPainter2 = TestPainterOf<Second>;

    impl<M: 'static> TestPainterOf<M> {
        fn new(app: &mut App) -> Ref<TestPainterOf<M>> {
            app.create(TestPainterOf {
                change_notifier: ChangeNotifierData::new(),
                count: 0,
                should_repaint_value: false,
                last_image: None,
                added_listener_count: 0,
                removed_listener_count: 0,
                marker: PhantomData,
            })
        }
    }

    impl<M: 'static> ChangeNotifier for TestPainterOf<M> {
        fn change_notifier_data(&self) -> &ChangeNotifierData {
            &self.change_notifier
        }

        fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
            &mut self.change_notifier
        }

        fn will_add_listener(self: Handle<Self>, app: &mut App) {
            app.get_mut(self).added_listener_count += 1;
        }

        fn did_remove_listener(self: Handle<Self>, app: &mut App) {
            app.get_mut(self).removed_listener_count += 1;
        }
    }

    impl<M: 'static> SnapshotPainter for TestPainterOf<M> {
        fn paint_snapshot(
            self: Handle<Self>,
            app: &mut App,
            _context: &mut PaintingContext,
            _offset: Offset,
            _size: Size,
            image: &Image,
            _source_size: Size,
            _pixel_ratio: f64,
        ) {
            let this = app.get_mut(self);
            this.count += 1;
            this.last_image = Some(image.clone());
        }

        fn paint(
            self: Handle<Self>,
            app: &mut App,
            _context: &mut PaintingContext,
            _offset: Offset,
            _size: Size,
            _painter: &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
        ) {
            app.get_mut(self).count += 1;
        }

        fn should_repaint(self: Handle<Self>, app: &App, old_painter: AnySnapshotPainter) -> bool {
            debug_assert!(old_painter.downcast::<Self>(app).is_some());
            app.get(self).should_repaint_value
        }
    }

    /// The shape of Material's zoom painters: the child painted through a transform and an
    /// opacity layer the painter keeps across paints.
    struct LayeredPainter {
        change_notifier: ChangeNotifierData,
        opacity_handle: LayerHandle<Handle<OpacityLayer>>,
        transform_handle: LayerHandle<Handle<TransformLayer>>,
    }

    impl ChangeNotifier for LayeredPainter {
        fn change_notifier_data(&self) -> &ChangeNotifierData {
            &self.change_notifier
        }

        fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
            &mut self.change_notifier
        }
    }

    impl SnapshotPainter for LayeredPainter {
        fn paint_snapshot(
            self: Handle<Self>,
            _app: &mut App,
            _context: &mut PaintingContext,
            _offset: Offset,
            _size: Size,
            _image: &Image,
            _source_size: Size,
            _pixel_ratio: f64,
        ) {
        }

        fn paint(
            self: Handle<Self>,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
            _size: Size,
            painter: &mut dyn FnMut(&mut App, &mut PaintingContext, Offset),
        ) {
            let old_transform = app.get(self).transform_handle.layer();
            let transform = context.push_transform(
                app,
                true,
                offset,
                Matrix4::IDENTITY,
                |app, context, offset| {
                    let old_opacity = app.get(self).opacity_handle.layer();
                    let opacity = context.push_opacity(app, offset, 128, painter, old_opacity);
                    LayerHandle::set_layer(
                        app,
                        |app| &mut app.get_mut(self).opacity_handle,
                        Some(opacity),
                    );
                },
                old_transform,
            );
            LayerHandle::set_layer(
                app,
                |app| &mut app.get_mut(self).transform_handle,
                transform,
            );
        }

        fn should_repaint(
            self: Handle<Self>,
            _app: &App,
            _old_painter: AnySnapshotPainter,
        ) -> bool {
            false
        }
    }

    /// Counts the paints of the child it stands in for.
    struct ChildPaints(Rc<Cell<u32>>);

    impl CustomPainter for ChildPaints {
        fn paint(&self, _app: &mut App, _canvas: &mut Canvas, _size: Size) {
            self.0.set(self.0.get() + 1);
        }

        fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
            false
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Flutter's `TestDependencies`: a text direction and a media query of `device_pixel_ratio`.
    fn test_dependencies<K>(device_pixel_ratio: f64, child: impl IntoWidget<K>) -> WidgetRef {
        let query = MediaQuery::new(
            MediaQueryData::new().device_pixel_ratio(device_pixel_ratio),
            child,
        )
        .into_widget();
        Directionality::new(TextDirection::Ltr, query).into_widget()
    }

    /// A centred 100 x 100 box under a snapshot widget.
    fn centered(snapshot: SnapshotWidget) -> WidgetRef {
        Center::new()
            .child(test_dependencies(1.0, snapshot))
            .into_widget()
    }

    fn sized_box(width: f64, height: f64) -> Option<WidgetRef> {
        Some(SizedBox::new().width(width).height(height).into_widget())
    }

    /// Flutter's `tester.layers`: the layer tree from the root, depth first.
    fn layers(app: &App, harness: &Harness) -> Vec<AnyLayer> {
        harness.layers(app)
    }

    #[test]
    fn changing_the_device_pixel_ratio_does_not_repaint_if_snapshotting_is_not_enabled() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, false);
        let painter = TestPainter::new(&mut app);
        let device_pixel_ratio = Rc::new(Cell::new(1.0));
        let local_set_state: Rc<RefCell<Option<StateSetter>>> = Rc::default();
        let harness = Harness::mount(
            &mut app,
            StatefulBuilder::new({
                let (device_pixel_ratio, local_set_state) =
                    (Rc::clone(&device_pixel_ratio), Rc::clone(&local_set_state));
                let painter = painter.clone();
                move |_app, _context, set_state| {
                    *local_set_state.borrow_mut() = Some(set_state);
                    Center::new()
                        .child(test_dependencies(
                            device_pixel_ratio.get(),
                            SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                                .painter(painter.erase()),
                        ))
                        .into_widget()
                }
            })
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 1);

        let set_state = local_set_state.borrow().clone().expect("the builder ran");
        set_state(&mut app, &mut |_app| device_pixel_ratio.set(2.0));
        harness.pump(&mut app);

        // Not repainted as dpr was not used.
        assert_eq!(app.get(&painter).count, 1);
    }

    #[test]
    fn the_snapshot_widget_paints_its_child_as_a_single_picture_layer() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, true);
        let child = Container::new().width(100.0).height(100.0).color(YELLOW);
        let harness = Harness::mount(
            &mut app,
            RepaintBoundary::new()
                .child(centered(SnapshotWidget::new(
                    controller.clone(),
                    Some(child.into_widget()),
                )))
                .into_widget(),
        );
        harness.pump(&mut app);

        let found = layers(&app, &harness);
        assert_eq!(found.len(), 3);
        assert!(app.handle::<PictureLayer>(found[2].id()).is_some());

        controller.set_allow_snapshotting(&mut app, false);
        harness.pump(&mut app);

        let found = layers(&app, &harness);
        assert_eq!(found.len(), 3);
        assert!(app.handle::<PictureLayer>(found[2].id()).is_some());
    }

    #[test]
    fn the_snapshot_widget_can_update_the_painter_type() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, true);
        let painter1 = TestPainter::new(&mut app);
        let painter2 = TestPainter2::new(&mut app);
        let harness = Harness::mount(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), Some(SizedBox::new().into_widget()))
                    .painter(painter1.erase()),
            ),
        );
        harness.pump(&mut app);

        harness.set_child(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), Some(SizedBox::new().into_widget()))
                    .painter(painter2.erase()),
            ),
        );
        harness.pump(&mut app);
    }

    #[test]
    fn a_child_with_an_empty_size_does_not_fail_to_paint() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, true);
        let painter = TestPainter::new(&mut app);
        let harness = Harness::mount(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), Some(SizedBox::new().into_widget()))
                    .painter(painter.erase()),
            ),
        );
        harness.pump(&mut app);

        assert_eq!(app.get(&painter).count, 0, "an empty box paints nothing");
    }

    #[test]
    fn a_controller_notification_repaints_the_snapshot_widget() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, false);
        let painter = TestPainter::new(&mut app);
        let harness = Harness::mount(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                    .painter(painter.erase()),
            ),
        );
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 1);

        controller.clear(&mut app);
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 2);

        controller.set_allow_snapshotting(&mut app, true);
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 3);

        controller.set_allow_snapshotting(&mut app, true);
        harness.pump(&mut app);
        assert_eq!(
            app.get(&painter).count,
            3,
            "an unchanged value does not notify"
        );
    }

    #[test]
    fn with_snapshotting_allowed_the_painter_paints_the_child_live() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, true);
        let painter = TestPainter::new(&mut app);
        let harness = Harness::mount(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                    .painter(painter.erase()),
            ),
        );
        harness.pump(&mut app);

        assert_eq!(app.get(&painter).count, 1);
        assert!(
            app.get(&painter).last_image.is_none(),
            "no raster is taken until `toImageSync` lands"
        );
    }

    #[test]
    fn the_painter_is_listened_to_while_attached() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, false);
        let painter = TestPainter::new(&mut app);
        let harness = Harness::mount(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                    .painter(painter.erase()),
            ),
        );
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).added_listener_count, 1);
        assert_eq!(app.get(&painter).count, 1);

        painter.notify_listeners(&mut app);
        harness.pump(&mut app);
        assert_eq!(
            app.get(&painter).count,
            2,
            "a painter notification repaints"
        );

        harness.set_child(&mut app, SizedBox::new().into_widget());
        harness.pump(&mut app);
        assert_eq!(
            app.get(&painter).removed_listener_count,
            2,
            "detach and dispose each remove the listener"
        );
        assert!(!app.get(&painter).change_notifier.has_listeners());
        assert!(!app.get(&controller).change_notifier.has_listeners());
    }

    #[test]
    fn a_new_painter_of_the_same_type_repaints_only_when_it_asks() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, false);
        let painter1 = TestPainter::new(&mut app);
        let painter2 = TestPainter::new(&mut app);
        let painter3 = TestPainter::new(&mut app);
        app.get_mut(&painter3).should_repaint_value = true;
        let tree = |painter: Ref<TestPainter>| {
            centered(
                SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                    .painter(painter.erase()),
            )
        };
        let harness = Harness::mount(&mut app, tree(painter1.clone()));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter1).count, 1);

        harness.set_child(&mut app, tree(painter2.clone()));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter1).removed_listener_count, 1);
        assert_eq!(app.get(&painter2).added_listener_count, 1);
        assert_eq!(app.get(&painter2).count, 0, "should_repaint is false");

        harness.set_child(&mut app, tree(painter3.clone()));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter2).removed_listener_count, 1);
        assert_eq!(app.get(&painter3).added_listener_count, 1);
        assert_eq!(app.get(&painter3).count, 1, "should_repaint is true");
    }

    #[test]
    fn a_new_painter_of_another_type_repaints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, false);
        let painter1 = TestPainter::new(&mut app);
        let painter2 = TestPainter2::new(&mut app);
        let harness = Harness::mount(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                    .painter(painter1.erase()),
            ),
        );
        harness.pump(&mut app);

        harness.set_child(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                    .painter(painter2.erase()),
            ),
        );
        harness.pump(&mut app);
        assert_eq!(app.get(&painter2).count, 1);
    }

    #[test]
    fn a_new_controller_repaints_only_when_allow_snapshotting_differs() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller1 = SnapshotController::new(&mut app, false);
        let controller2 = SnapshotController::new(&mut app, false);
        let controller3 = SnapshotController::new(&mut app, true);
        let painter = TestPainter::new(&mut app);
        let tree = |controller: Ref<SnapshotController>| {
            centered(
                SnapshotWidget::new(controller, sized_box(100.0, 100.0)).painter(painter.erase()),
            )
        };
        let harness = Harness::mount(&mut app, tree(controller1.clone()));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 1);

        harness.set_child(&mut app, tree(controller2.clone()));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 1);
        assert!(!app.get(&controller1).change_notifier.has_listeners());
        assert!(app.get(&controller2).change_notifier.has_listeners());

        harness.set_child(&mut app, tree(controller3.clone()));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 2);
        assert!(!app.get(&controller2).change_notifier.has_listeners());

        controller3.clear(&mut app);
        harness.pump(&mut app);
        assert_eq!(
            app.get(&painter).count,
            3,
            "the new controller is listened to"
        );
    }

    #[test]
    fn changing_the_mode_or_autoresize_repaints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, false);
        let painter = TestPainter::new(&mut app);
        let tree = |mode: SnapshotMode, autoresize: bool| {
            centered(
                SnapshotWidget::new(controller.clone(), sized_box(100.0, 100.0))
                    .mode(mode)
                    .autoresize(autoresize)
                    .painter(painter.erase()),
            )
        };
        let harness = Harness::mount(&mut app, tree(SnapshotMode::Normal, false));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 1);

        harness.set_child(&mut app, tree(SnapshotMode::Permissive, false));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 2);

        harness.set_child(&mut app, tree(SnapshotMode::Permissive, true));
        harness.pump(&mut app);
        assert_eq!(app.get(&painter).count, 3);

        harness.set_child(&mut app, tree(SnapshotMode::Permissive, true));
        harness.pump(&mut app);
        assert_eq!(
            app.get(&painter).count,
            3,
            "unchanged values do not repaint"
        );
    }

    #[test]
    fn the_default_painter_paints_the_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, false);
        let child_paints = Rc::new(Cell::new(0));
        let child = CustomPaint::new()
            .painter(ChildPaints(Rc::clone(&child_paints)))
            .child(SizedBox::new().width(100.0).height(100.0));
        let harness = Harness::mount(
            &mut app,
            centered(SnapshotWidget::new(
                controller.clone(),
                Some(child.into_widget()),
            )),
        );
        harness.pump(&mut app);
        assert_eq!(child_paints.get(), 1);

        controller.set_allow_snapshotting(&mut app, true);
        harness.pump(&mut app);
        assert_eq!(
            child_paints.get(),
            2,
            "painted live while snapshotting is allowed"
        );
    }

    #[test]
    fn a_painter_paints_the_child_through_the_layers_it_pushes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = SnapshotController::new(&mut app, true);
        let painter = app.create(LayeredPainter {
            change_notifier: ChangeNotifierData::new(),
            opacity_handle: LayerHandle::new(),
            transform_handle: LayerHandle::new(),
        });
        let child_paints = Rc::new(Cell::new(0));
        let child = CustomPaint::new()
            .painter(ChildPaints(Rc::clone(&child_paints)))
            .child(SizedBox::new().width(100.0).height(100.0));
        let harness = Harness::mount(
            &mut app,
            centered(
                SnapshotWidget::new(controller.clone(), Some(child.into_widget()))
                    .painter(painter.erase()),
            ),
        );
        harness.pump(&mut app);
        assert_eq!(child_paints.get(), 1);
        let transform = app.get(&painter).transform_handle.layer().expect("pushed");
        let opacity = app.get(&painter).opacity_handle.layer().expect("pushed");
        let found = layers(&app, &harness);
        let transform_at = found.iter().position(|layer| layer.id() == transform.id());
        let opacity_at = found.iter().position(|layer| layer.id() == opacity.id());
        assert!(transform_at.is_some() && opacity_at > transform_at);

        painter.notify_listeners(&mut app);
        harness.pump(&mut app);
        assert_eq!(child_paints.get(), 2);
        assert_eq!(
            app.get(&painter).transform_handle.layer(),
            Some(transform),
            "reused"
        );
        assert_eq!(
            app.get(&painter).opacity_handle.layer(),
            Some(opacity),
            "reused"
        );
    }
}
